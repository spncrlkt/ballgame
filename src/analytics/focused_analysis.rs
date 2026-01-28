//! Focused, deep-dive analysis for a single simulation DB.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, Result};

use crate::constants::{ARENA_FLOOR_Y, LEVELS_FILE};
use crate::helpers::basket_x_from_offset;
use crate::levels::LevelDatabase;

#[derive(Debug, Clone)]
pub struct StatSummary {
    pub avg: f64,
    pub med: f64,
    pub p10: Option<f64>,
    pub p90: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct FocusedReport {
    pub db_path: String,
    pub match_count: i64,
    pub avg_duration: f64,
    pub goals_per_match: f64,
    pub shots_per_match: f64,
    pub shot_pct: f64,
    pub avg_shot_quality: f64,
    pub scoreless_rate: f64,
    pub shot_quality: StatSummary,
    pub shot_charge: StatSummary,
    pub charge_durations: StatSummary,
    pub ai_goal_changes: StatSummary,
    pub steal_attempts: StatSummary,
    pub steal_success_rate: Option<f64>,
    pub nav_complete_rate: Option<f64>,
    pub per_level: Vec<PerLevelRow>,
    pub low_shot_levels: Vec<(String, i64)>,
    pub distance_charge_bins: Vec<DistanceChargeBin>,
    pub distance_charge_corr: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct PerLevelRow {
    pub level_name: String,
    pub matches: i64,
    pub goals_per_match: f64,
    pub shots_per_match: f64,
    pub shot_pct: f64,
    pub scoreless_rate: f64,
    pub avg_shot_quality: f64,
}

#[derive(Debug, Clone)]
pub struct DistanceChargeBin {
    pub label: String,
    pub count: usize,
    pub avg_charge: Option<f64>,
}

const SQL_MATCH_COUNTS: &str = "SELECT COUNT(*) FROM matches";
const SQL_AVG_DURATION: &str = "SELECT AVG(duration_secs) FROM matches";
const SQL_GOALS_PER_MATCH: &str = "SELECT AVG(score_left + score_right) FROM matches";
const SQL_SCORELESS_RATE: &str =
    "SELECT AVG(CASE WHEN score_left + score_right = 0 THEN 1.0 ELSE 0.0 END) FROM matches";
const SQL_SHOTS_TOTAL: &str = "SELECT SUM(shots_attempted), SUM(shots_made) FROM player_stats";
const SQL_AVG_SHOT_QUALITY: &str = "SELECT AVG(avg_shot_quality) FROM player_stats";
const SQL_SHOT_START: &str = "SELECT match_id, time_ms, data FROM events WHERE event_type = 'SS'";
const SQL_SHOT_RELEASE: &str = "SELECT match_id, time_ms, data FROM events WHERE event_type = 'SR'";
const SQL_MATCH_LEVELS: &str = "SELECT id, level_name FROM matches";
const SQL_AI_GOAL: &str =
    "SELECT match_id, COUNT(*) FROM events WHERE event_type = 'AG' GROUP BY match_id";
const SQL_STEAL_ATTEMPT: &str =
    "SELECT match_id, COUNT(*) FROM events WHERE event_type = 'SA' GROUP BY match_id";
const SQL_EVENTS: &str = "SELECT event_type, COUNT(*) FROM events GROUP BY event_type";
const SQL_LOW_SHOT_LEVELS: &str = r#"
    SELECT level_name, COUNT(*)
    FROM (
        SELECT m.id, m.level_name, SUM(ps.shots_attempted) as shots
        FROM matches m JOIN player_stats ps ON ps.match_id = m.id
        GROUP BY m.id
    )
    WHERE shots <= 5
    GROUP BY level_name
    ORDER BY COUNT(*) DESC
"#;
const SQL_PER_LEVEL: &str = r#"
    SELECT
        m.level_name,
        COUNT(DISTINCT m.id) as matches,
        AVG(m.score_left + m.score_right) as goals_per_match,
        SUM(ps.shots_attempted) as shots,
        SUM(ps.shots_made) as made,
        AVG(CASE WHEN m.score_left + m.score_right = 0 THEN 1.0 ELSE 0.0 END) as scoreless_rate,
        AVG(ps.avg_shot_quality) as avg_shot_quality
    FROM matches m
    JOIN player_stats ps ON ps.match_id = m.id
    GROUP BY m.level_name
    ORDER BY scoreless_rate DESC, shots ASC
"#;

fn quantiles(mut values: Vec<f64>) -> Option<(f64, f64)> {
    if values.len() < 10 {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p10 = values[(values.len() as f64 * 0.1).floor() as usize];
    let p90 = values[(values.len() as f64 * 0.9).floor() as usize];
    Some((p10, p90))
}

fn summarize(values: &[f64]) -> Option<StatSummary> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let med = sorted[sorted.len() / 2];
    let avg = sorted.iter().sum::<f64>() / sorted.len() as f64;
    let (p10, p90) = quantiles(sorted.clone()).unwrap_or((f64::NAN, f64::NAN));
    Some(StatSummary {
        avg,
        med,
        p10: if p10.is_nan() { None } else { Some(p10) },
        p90: if p90.is_nan() { None } else { Some(p90) },
    })
}

fn summarize_or_empty(values: &[f64]) -> StatSummary {
    summarize(values).unwrap_or(StatSummary {
        avg: 0.0,
        med: 0.0,
        p10: None,
        p90: None,
    })
}

fn parse_event_parts(data: &str) -> Vec<&str> {
    data.split('|').collect()
}

fn parse_event_player(data: &str) -> Option<String> {
    let parts = parse_event_parts(data);
    parts.get(2).map(|p| (*p).to_string())
}

fn parse_shot_start_pos(data: &str) -> Option<(f64, f64)> {
    let parts = parse_event_parts(data);
    if parts.len() < 4 {
        return None;
    }
    let pos = parts.get(3)?;
    let mut iter = pos.split(',');
    let x = iter.next()?.parse::<f64>().ok()?;
    let y = iter.next()?.parse::<f64>().ok()?;
    Some((x, y))
}

fn parse_shot_charge(data: &str) -> Option<f64> {
    let parts = parse_event_parts(data);
    if parts.len() < 4 {
        return None;
    }
    parts.get(3).and_then(|v| v.parse::<f64>().ok())
}

fn parse_shot_quality(data: &str) -> Option<f64> {
    let parts = parse_event_parts(data);
    if parts.len() < 5 {
        return None;
    }
    parts.get(4).and_then(|v| v.parse::<f64>().ok())
}

fn pearson_corr(pairs: &[(f64, f64)]) -> Option<f64> {
    if pairs.len() < 2 {
        return None;
    }
    let (sum_x, sum_y) = pairs
        .iter()
        .fold((0.0, 0.0), |(sx, sy), (x, y)| (sx + x, sy + y));
    let n = pairs.len() as f64;
    let mean_x = sum_x / n;
    let mean_y = sum_y / n;
    let mut num = 0.0;
    let mut den_x = 0.0;
    let mut den_y = 0.0;
    for (x, y) in pairs {
        let dx = x - mean_x;
        let dy = y - mean_y;
        num += dx * dy;
        den_x += dx * dx;
        den_y += dy * dy;
    }
    let denom = (den_x * den_y).sqrt();
    if denom <= f64::EPSILON {
        None
    } else {
        Some(num / denom)
    }
}

pub fn run_focused_analysis(db_path: &Path) -> Result<FocusedReport> {
    let conn = Connection::open(db_path)?;

    let match_count: i64 = conn.query_row(SQL_MATCH_COUNTS, [], |row| row.get(0))?;
    let avg_duration: f64 = conn.query_row(SQL_AVG_DURATION, [], |row| row.get(0))?;
    let goals_per_match: f64 = conn.query_row(SQL_GOALS_PER_MATCH, [], |row| row.get(0))?;
    let scoreless_rate: f64 = conn.query_row(SQL_SCORELESS_RATE, [], |row| row.get(0))?;
    let (shots_attempted, shots_made): (Option<i64>, Option<i64>) =
        conn.query_row(SQL_SHOTS_TOTAL, [], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let shots_attempted = shots_attempted.unwrap_or(0) as f64;
    let shots_made = shots_made.unwrap_or(0) as f64;
    let shots_per_match = if match_count > 0 {
        shots_attempted / match_count as f64
    } else {
        0.0
    };
    let shot_pct = if shots_attempted > 0.0 {
        shots_made / shots_attempted
    } else {
        0.0
    };
    let avg_shot_quality: f64 = conn.query_row(SQL_AVG_SHOT_QUALITY, [], |row| row.get(0))?;

    let mut events = HashMap::new();
    let mut stmt = conn.prepare(SQL_EVENTS)?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        let (code, count) = row?;
        events.insert(code, count);
    }

    let mut ai_goal_counts = Vec::new();
    let mut stmt = conn.prepare(SQL_AI_GOAL)?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(1))?;
    for row in rows {
        ai_goal_counts.push(row? as f64);
    }

    let mut steal_attempts = Vec::new();
    let mut stmt = conn.prepare(SQL_STEAL_ATTEMPT)?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(1))?;
    for row in rows {
        steal_attempts.push(row? as f64);
    }

    let sa = events.get("SA").copied().unwrap_or(0) as f64;
    let ss = events.get("S+").copied().unwrap_or(0) as f64;
    let steal_success_rate = if sa > 0.0 { Some(ss / sa) } else { None };
    let ns = events.get("NS").copied().unwrap_or(0) as f64;
    let nc = events.get("NC").copied().unwrap_or(0) as f64;
    let nav_complete_rate = if ns > 0.0 { Some(nc / ns) } else { None };

    let mut charges = Vec::new();
    let mut stmt = conn.prepare(SQL_SHOT_RELEASE)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(2))?;
    for row in rows {
        if let Some(charge) = parse_shot_charge(&row?) {
            charges.push(charge);
        }
    }

    let mut shot_quality = Vec::new();
    let mut stmt = conn.prepare(SQL_SHOT_START)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(2))?;
    for row in rows {
        if let Some(quality) = parse_shot_quality(&row?) {
            shot_quality.push(quality);
        }
    }

    let mut ss = HashMap::<(i64, Option<String>), Vec<i64>>::new();
    let mut sr = HashMap::<(i64, Option<String>), Vec<i64>>::new();
    let mut stmt = conn.prepare(SQL_SHOT_START)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (mid, time_ms, data) = row?;
        let player = parse_event_player(&data);
        ss.entry((mid, player)).or_default().push(time_ms);
    }
    let mut stmt = conn.prepare(SQL_SHOT_RELEASE)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (mid, time_ms, data) = row?;
        let player = parse_event_player(&data);
        sr.entry((mid, player)).or_default().push(time_ms);
    }
    let mut durations = Vec::new();
    for (key, starts) in ss {
        let mut starts = starts;
        let mut ends = sr.remove(&key).unwrap_or_default();
        starts.sort();
        ends.sort();
        let mut j = 0usize;
        for s in starts {
            while j < ends.len() && ends[j] < s {
                j += 1;
            }
            if j < ends.len() {
                durations.push((ends[j] - s) as f64 / 1000.0);
                j += 1;
            }
        }
    }

    let level_db = LevelDatabase::load_from_file(LEVELS_FILE);
    let mut level_baskets = HashMap::<String, (f64, f64, f64)>::new();
    for level in level_db.all() {
        let (left_x, right_x) = basket_x_from_offset(level.basket_push_in);
        let basket_y = ARENA_FLOOR_Y + level.basket_height;
        level_baskets.insert(
            level.name.clone(),
            (left_x as f64, right_x as f64, basket_y as f64),
        );
    }

    let mut match_levels = HashMap::<i64, String>::new();
    let mut stmt = conn.prepare(SQL_MATCH_LEVELS)?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (match_id, level_name) = row?;
        match_levels.insert(match_id, level_name);
    }

    let mut shot_starts = HashMap::<(i64, String), Vec<(i64, (f64, f64))>>::new();
    let mut stmt = conn.prepare(SQL_SHOT_START)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (match_id, time_ms, data) = row?;
        if let (Some(player), Some(pos)) = (parse_event_player(&data), parse_shot_start_pos(&data))
        {
            shot_starts
                .entry((match_id, player))
                .or_default()
                .push((time_ms, pos));
        }
    }

    let mut shot_releases = HashMap::<(i64, String), Vec<(i64, f64)>>::new();
    let mut stmt = conn.prepare(SQL_SHOT_RELEASE)?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (match_id, time_ms, data) = row?;
        if let (Some(player), Some(charge)) = (parse_event_player(&data), parse_shot_charge(&data))
        {
            shot_releases
                .entry((match_id, player))
                .or_default()
                .push((time_ms, charge));
        }
    }

    let mut distance_charge_pairs = Vec::new();
    for (key, starts) in shot_starts {
        let (match_id, player) = key;
        let Some(level_name) = match_levels.get(&match_id) else {
            continue;
        };
        let Some((left_x, right_x, basket_y)) = level_baskets.get(level_name) else {
            continue;
        };
        let target_x = match player.as_str() {
            "L" => *right_x,
            "R" => *left_x,
            _ => continue,
        };
        let target_y = *basket_y;
        let mut starts = starts;
        let mut releases = shot_releases
            .remove(&(match_id, player))
            .unwrap_or_default();
        starts.sort_by_key(|(time, _)| *time);
        releases.sort_by_key(|(time, _)| *time);
        let mut j = 0usize;
        for (start_time, pos) in starts {
            while j < releases.len() && releases[j].0 < start_time {
                j += 1;
            }
            if j < releases.len() {
                let (_release_time, charge) = releases[j];
                let dx = pos.0 - target_x;
                let dy = pos.1 - target_y;
                let distance = (dx * dx + dy * dy).sqrt();
                distance_charge_pairs.push((distance, charge));
                j += 1;
            }
        }
    }

    let distance_bins = [
        (0.0, 200.0),
        (200.0, 400.0),
        (400.0, 600.0),
        (600.0, 800.0),
        (800.0, f64::INFINITY),
    ];
    let mut distance_charge_bins = Vec::new();
    for (min, max) in distance_bins {
        let mut charges = Vec::new();
        for (distance, charge) in &distance_charge_pairs {
            let in_bin = if max.is_infinite() {
                *distance >= min
            } else {
                *distance >= min && *distance < max
            };
            if in_bin {
                charges.push(*charge);
            }
        }
        let avg_charge = if charges.is_empty() {
            None
        } else {
            Some(charges.iter().sum::<f64>() / charges.len() as f64)
        };
        let label = if max.is_infinite() {
            format!("{:.0}+", min)
        } else {
            format!("{:.0}-{:.0}", min, max)
        };
        distance_charge_bins.push(DistanceChargeBin {
            label,
            count: charges.len(),
            avg_charge,
        });
    }
    let distance_charge_corr = pearson_corr(&distance_charge_pairs);

    let mut per_level = Vec::new();
    let mut stmt = conn.prepare(SQL_PER_LEVEL)?;
    let rows = stmt.query_map([], |row| {
        Ok(PerLevelRow {
            level_name: row.get(0)?,
            matches: row.get(1)?,
            goals_per_match: row.get(2)?,
            shots_per_match: {
                let shots: f64 = row.get::<_, f64>(3)?;
                let matches: f64 = row.get::<_, i64>(1)? as f64;
                if matches > 0.0 { shots / matches } else { 0.0 }
            },
            shot_pct: {
                let shots: f64 = row.get::<_, f64>(3)?;
                let made: f64 = row.get::<_, f64>(4)?;
                if shots > 0.0 { made / shots } else { 0.0 }
            },
            scoreless_rate: row.get(5)?,
            avg_shot_quality: row.get(6)?,
        })
    })?;
    for row in rows {
        per_level.push(row?);
    }

    let mut low_shot_levels = Vec::new();
    let mut stmt = conn.prepare(SQL_LOW_SHOT_LEVELS)?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    for row in rows {
        low_shot_levels.push(row?);
    }

    Ok(FocusedReport {
        db_path: db_path.display().to_string(),
        match_count,
        avg_duration,
        goals_per_match,
        shots_per_match,
        shot_pct,
        avg_shot_quality,
        scoreless_rate,
        shot_quality: summarize_or_empty(&shot_quality),
        shot_charge: summarize_or_empty(&charges),
        charge_durations: summarize_or_empty(&durations),
        ai_goal_changes: summarize_or_empty(&ai_goal_counts),
        steal_attempts: summarize_or_empty(&steal_attempts),
        steal_success_rate,
        nav_complete_rate,
        per_level,
        low_shot_levels,
        distance_charge_bins,
        distance_charge_corr,
    })
}

impl FocusedReport {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Focused Analysis Report\n\n");
        out.push_str(&format!("DB: `{}`\n\n", self.db_path));

        out.push_str("## SQL Used\n");
        out.push_str("```\n");
        out.push_str(SQL_MATCH_COUNTS);
        out.push_str("\n");
        out.push_str(SQL_AVG_DURATION);
        out.push_str("\n");
        out.push_str(SQL_GOALS_PER_MATCH);
        out.push_str("\n");
        out.push_str(SQL_SCORELESS_RATE);
        out.push_str("\n");
        out.push_str(SQL_SHOTS_TOTAL);
        out.push_str("\n");
        out.push_str(SQL_AVG_SHOT_QUALITY);
        out.push_str("\n");
        out.push_str(SQL_SHOT_START);
        out.push_str("\n");
        out.push_str(SQL_SHOT_RELEASE);
        out.push_str("\n");
        out.push_str(SQL_MATCH_LEVELS);
        out.push_str("\n");
        out.push_str(SQL_AI_GOAL);
        out.push_str("\n");
        out.push_str(SQL_STEAL_ATTEMPT);
        out.push_str("\n");
        out.push_str(SQL_EVENTS);
        out.push_str("\n");
        out.push_str(SQL_LOW_SHOT_LEVELS);
        out.push_str("\n");
        out.push_str(SQL_PER_LEVEL);
        out.push_str("\n```\n\n");

        out.push_str("## Summary\n");
        out.push_str(&format!("- Matches: {}\n", self.match_count));
        out.push_str(&format!("- Avg duration: {:.2}s\n", self.avg_duration));
        out.push_str(&format!("- Goals/match: {:.3}\n", self.goals_per_match));
        out.push_str(&format!("- Shots/match: {:.3}\n", self.shots_per_match));
        out.push_str(&format!("- Shot%: {:.3}\n", self.shot_pct));
        out.push_str(&format!(
            "- Avg shot quality: {:.3}\n",
            self.avg_shot_quality
        ));
        out.push_str(&format!("- Scoreless rate: {:.3}\n", self.scoreless_rate));
        if let Some(rate) = self.steal_success_rate {
            out.push_str(&format!("- Steal success rate: {:.3}\n", rate));
        }
        if let Some(rate) = self.nav_complete_rate {
            out.push_str(&format!("- Nav complete rate: {:.3}\n", rate));
        }
        out.push_str("\n");

        out.push_str("## Shot Quality Distribution (from SS events)\n");
        out.push_str(&format!(
            "- avg {:.3} | med {:.3} | p10 {:?} | p90 {:?}\n",
            self.shot_quality.avg,
            self.shot_quality.med,
            self.shot_quality.p10,
            self.shot_quality.p90
        ));
        out.push_str("\n");

        out.push_str("## Shot Charge (from SR events)\n");
        out.push_str(&format!(
            "- avg {:.3} | med {:.3} | p10 {:?} | p90 {:?}\n",
            self.shot_charge.avg, self.shot_charge.med, self.shot_charge.p10, self.shot_charge.p90
        ));
        out.push_str("\n");

        out.push_str("## Charge Durations (SS -> SR)\n");
        out.push_str(&format!(
            "- avg {:.3}s | med {:.3}s | p10 {:?} | p90 {:?}\n",
            self.charge_durations.avg,
            self.charge_durations.med,
            self.charge_durations.p10,
            self.charge_durations.p90
        ));
        out.push_str("\n");

        out.push_str("## Distance vs Charge (SS position -> target basket)\n");
        if let Some(corr) = self.distance_charge_corr {
            out.push_str(&format!("- correlation: {:.3}\n", corr));
        } else {
            out.push_str("- correlation: n/a\n");
        }
        out.push_str("distance | shots | avg_charge\n");
        out.push_str("---- | ---- | ----\n");
        for bin in &self.distance_charge_bins {
            let avg = bin
                .avg_charge
                .map(|v| format!("{:.3}", v))
                .unwrap_or_else(|| "n/a".to_string());
            out.push_str(&format!("{} | {} | {}\n", bin.label, bin.count, avg));
        }
        out.push_str("\n");

        out.push_str("## AI Goal Changes / Match\n");
        out.push_str(&format!(
            "- avg {:.2} | med {:.2} | p10 {:?} | p90 {:?}\n",
            self.ai_goal_changes.avg,
            self.ai_goal_changes.med,
            self.ai_goal_changes.p10,
            self.ai_goal_changes.p90
        ));
        out.push_str("\n");

        out.push_str("## Steal Attempts / Match\n");
        out.push_str(&format!(
            "- avg {:.2} | med {:.2} | p10 {:?} | p90 {:?}\n",
            self.steal_attempts.avg,
            self.steal_attempts.med,
            self.steal_attempts.p10,
            self.steal_attempts.p90
        ));
        out.push_str("\n");

        out.push_str("## Low-shot Levels (<= 5 shots/match)\n");
        if self.low_shot_levels.is_empty() {
            out.push_str("- None\n");
        } else {
            for (lvl, count) in &self.low_shot_levels {
                out.push_str(&format!("- {}: {}\n", lvl, count));
            }
        }
        out.push_str("\n");

        out.push_str("## Per-level Summary\n");
        out.push_str("level | matches | goals/m | shots/m | shot% | scoreless | avg_q\n");
        out.push_str("---- | ---- | ---- | ---- | ---- | ---- | ----\n");
        for row in &self.per_level {
            out.push_str(&format!(
                "{} | {} | {:.2} | {:.2} | {:.3} | {:.3} | {:.3}\n",
                row.level_name,
                row.matches,
                row.goals_per_match,
                row.shots_per_match,
                row.shot_pct,
                row.scoreless_rate,
                row.avg_shot_quality
            ));
        }

        out
    }
}
