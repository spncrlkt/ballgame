//! Event-level audit and comparison reports for simulation DBs.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{Connection, Result, params};

const EVENT_CODES: &[&str] = &[
    "PU", "DR", "SS", "SR", "SA", "S+", "S-", "SO", "AG", "NS", "NC",
];

const SQL_EVENT_COUNTS: &str = "SELECT event_type, COUNT(*) FROM events GROUP BY event_type";
const SQL_MATCH_COUNTS: &str = "SELECT COUNT(*) FROM matches";
const SQL_AVG_DURATION: &str = "SELECT AVG(duration_secs) FROM matches";
const SQL_GOALS_PER_MATCH: &str = "SELECT AVG(score_left + score_right) FROM matches";
const SQL_SCORELESS_RATE: &str = "SELECT AVG(CASE WHEN score_left + score_right = 0 THEN 1.0 ELSE 0.0 END) FROM matches";
const SQL_SHOTS_TOTAL: &str = "SELECT SUM(shots_attempted), SUM(shots_made) FROM player_stats";
const SQL_AVG_SHOT_QUALITY: &str = "SELECT AVG(avg_shot_quality) FROM player_stats";
const SQL_EVENT_COUNTS_PER_MATCH: &str =
    "SELECT match_id, COUNT(*) FROM events WHERE event_type = ?1 GROUP BY match_id";
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
const SQL_POSSESSION_SUM: &str =
    "SELECT match_id, SUM(possession_time) FROM player_stats GROUP BY match_id";
const SQL_SHOT_START: &str =
    "SELECT match_id, time_ms, data FROM events WHERE event_type = 'SS'";
const SQL_SHOT_RELEASE: &str =
    "SELECT match_id, time_ms, data FROM events WHERE event_type = 'SR'";

#[derive(Debug, Clone)]
pub struct StatSummary {
    pub avg: f64,
    pub med: f64,
    pub p10: Option<f64>,
    pub p90: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ShotChargeSummary {
    pub count: usize,
    pub avg: Option<f64>,
    pub med: Option<f64>,
    pub p10: Option<f64>,
    pub p90: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct DbAudit {
    pub match_count: i64,
    pub avg_duration: f64,
    pub goals_per_match: f64,
    pub shots_per_match: f64,
    pub shot_pct: f64,
    pub avg_shot_quality: f64,
    pub scoreless_rate: f64,
    pub events: HashMap<String, i64>,
    pub per_match: HashMap<String, StatSummary>,
    pub shot_charge: ShotChargeSummary,
    pub charge_durations: ShotChargeSummary,
    pub low_shot_levels: Vec<(String, i64)>,
    pub possession_sum: StatSummary,
    pub steal_success_rate: Option<f64>,
    pub nav_complete_rate: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct AuditReport {
    pub base_path: String,
    pub current_path: String,
    pub base: DbAudit,
    pub current: DbAudit,
}

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

fn summarize_opt(values: &[f64]) -> ShotChargeSummary {
    if values.is_empty() {
        return ShotChargeSummary {
            count: 0,
            avg: None,
            med: None,
            p10: None,
            p90: None,
        };
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let med = sorted[sorted.len() / 2];
    let avg = sorted.iter().sum::<f64>() / sorted.len() as f64;
    let (p10, p90) = quantiles(sorted.clone()).unwrap_or((f64::NAN, f64::NAN));
    ShotChargeSummary {
        count: values.len(),
        avg: Some(avg),
        med: Some(med),
        p10: if p10.is_nan() { None } else { Some(p10) },
        p90: if p90.is_nan() { None } else { Some(p90) },
    }
}

fn parse_event_parts(data: &str) -> Vec<&str> {
    data.split('|').collect()
}

fn parse_event_player(data: &str) -> Option<String> {
    let parts = parse_event_parts(data);
    parts.get(2).map(|p| (*p).to_string())
}

fn parse_shot_charge(data: &str) -> Option<f64> {
    let parts = parse_event_parts(data);
    if parts.len() < 4 {
        return None;
    }
    parts.get(3).and_then(|v| v.parse::<f64>().ok())
}


fn audit_db(path: &Path) -> Result<DbAudit> {
    let conn = Connection::open(path)?;
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
    let mut stmt = conn.prepare(SQL_EVENT_COUNTS)?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
    for row in rows {
        let (code, count) = row?;
        events.insert(code, count);
    }

    let mut per_match = HashMap::new();
    for code in EVENT_CODES {
        let mut stmt = conn.prepare(SQL_EVENT_COUNTS_PER_MATCH)?;
        let rows = stmt.query_map(params![code], |row| row.get::<_, i64>(1))?;
        let mut values = Vec::new();
        for row in rows {
            values.push(row? as f64);
        }
        if let Some(summary) = summarize(&values) {
            per_match.insert(code.to_string(), summary);
        }
    }

    let mut charges = Vec::new();
    let mut stmt = conn.prepare(SQL_SHOT_RELEASE)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(2))?;
    for row in rows {
        if let Some(charge) = parse_shot_charge(&row?) {
            charges.push(charge);
        }
    }
    let shot_charge = summarize_opt(&charges);

    let mut ss = HashMap::<(i64, Option<String>), Vec<i64>>::new();
    let mut sr = HashMap::<(i64, Option<String>), Vec<i64>>::new();
    let mut stmt = conn.prepare(SQL_SHOT_START)?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)))?;
    for row in rows {
        let (mid, time_ms, data) = row?;
        let player = parse_event_player(&data);
        ss.entry((mid, player)).or_default().push(time_ms);
    }
    let mut stmt = conn.prepare(SQL_SHOT_RELEASE)?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)))?;
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
    let charge_durations = summarize_opt(&durations);

    let mut low_shot_levels = Vec::new();
    let mut stmt = conn.prepare(SQL_LOW_SHOT_LEVELS)?;
    let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
    for row in rows {
        low_shot_levels.push(row?);
    }

    let mut poss = Vec::new();
    let mut stmt = conn.prepare(SQL_POSSESSION_SUM)?;
    let rows = stmt.query_map([], |row| row.get::<_, f64>(1))?;
    for row in rows {
        poss.push(row?);
    }
    let possession_sum = summarize(&poss).unwrap_or(StatSummary {
        avg: 0.0,
        med: 0.0,
        p10: None,
        p90: None,
    });

    let sa = events.get("SA").copied().unwrap_or(0) as f64;
    let ss_count = events.get("S+").copied().unwrap_or(0) as f64;
    let steal_success_rate = if sa > 0.0 { Some(ss_count / sa) } else { None };
    let ns = events.get("NS").copied().unwrap_or(0) as f64;
    let nc = events.get("NC").copied().unwrap_or(0) as f64;
    let nav_complete_rate = if ns > 0.0 { Some(nc / ns) } else { None };

    Ok(DbAudit {
        match_count,
        avg_duration,
        goals_per_match,
        shots_per_match,
        shot_pct,
        avg_shot_quality,
        scoreless_rate,
        events,
        per_match,
        shot_charge,
        charge_durations,
        low_shot_levels,
        possession_sum,
        steal_success_rate,
        nav_complete_rate,
    })
}

pub fn run_event_audit(base_path: &Path, current_path: &Path) -> Result<AuditReport> {
    Ok(AuditReport {
        base_path: base_path.display().to_string(),
        current_path: current_path.display().to_string(),
        base: audit_db(base_path)?,
        current: audit_db(current_path)?,
    })
}

impl AuditReport {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Event Audit Report\n\n");
        out.push_str(&format!("Base DB: `{}`\n\n", self.base_path));
        out.push_str(&format!("Current DB: `{}`\n\n", self.current_path));

        out.push_str("## SQL Used\n");
        out.push_str("```\n");
        out.push_str(SQL_EVENT_COUNTS);
        out.push_str("\n");
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
        out.push_str(SQL_EVENT_COUNTS_PER_MATCH);
        out.push_str("\n");
        out.push_str(SQL_LOW_SHOT_LEVELS);
        out.push_str("\n");
        out.push_str(SQL_POSSESSION_SUM);
        out.push_str("\n");
        out.push_str(SQL_SHOT_START);
        out.push_str("\n");
        out.push_str(SQL_SHOT_RELEASE);
        out.push_str("\n```\n\n");

        out.push_str("## Summary\n");
        out.push_str(&format!(
            "- Matches: base {} | current {}\n",
            self.base.match_count, self.current.match_count
        ));
        out.push_str(&format!(
            "- Avg duration: base {:.2}s | current {:.2}s\n",
            self.base.avg_duration, self.current.avg_duration
        ));
        out.push_str(&format!(
            "- Goals/match: base {:.3} | current {:.3}\n",
            self.base.goals_per_match, self.current.goals_per_match
        ));
        out.push_str(&format!(
            "- Shots/match: base {:.3} | current {:.3}\n",
            self.base.shots_per_match, self.current.shots_per_match
        ));
        out.push_str(&format!(
            "- Shot%: base {:.3} | current {:.3}\n",
            self.base.shot_pct, self.current.shot_pct
        ));
        out.push_str(&format!(
            "- Avg shot quality: base {:.3} | current {:.3}\n",
            self.base.avg_shot_quality, self.current.avg_shot_quality
        ));
        out.push_str(&format!(
            "- Scoreless rate: base {:.3} | current {:.3}\n",
            self.base.scoreless_rate, self.current.scoreless_rate
        ));
        if let (Some(b), Some(c)) = (self.base.steal_success_rate, self.current.steal_success_rate) {
            out.push_str(&format!("- Steal success rate: base {:.3} | current {:.3}\n", b, c));
        }
        if let (Some(b), Some(c)) = (self.base.nav_complete_rate, self.current.nav_complete_rate) {
            out.push_str(&format!("- Nav complete rate: base {:.3} | current {:.3}\n", b, c));
        }
        out.push_str("\n");

        out.push_str("## Event Counts Per Match (avg)\n");
        for code in EVENT_CODES {
            if let (Some(b), Some(c)) = (
                self.base.per_match.get(*code),
                self.current.per_match.get(*code),
            ) {
                out.push_str(&format!(
                    "- {}: base {:.2} | current {:.2}\n",
                    code, b.avg, c.avg
                ));
            }
        }
        out.push_str("\n");

        out.push_str("## Shot Charge (from SR events)\n");
        out.push_str(&format!(
            "- Base: count {} avg {:?} med {:?} p10 {:?} p90 {:?}\n",
            self.base.shot_charge.count,
            self.base.shot_charge.avg,
            self.base.shot_charge.med,
            self.base.shot_charge.p10,
            self.base.shot_charge.p90
        ));
        out.push_str(&format!(
            "- Current: count {} avg {:?} med {:?} p10 {:?} p90 {:?}\n\n",
            self.current.shot_charge.count,
            self.current.shot_charge.avg,
            self.current.shot_charge.med,
            self.current.shot_charge.p10,
            self.current.shot_charge.p90
        ));

        out.push_str("## Charge Durations (SS -> SR)\n");
        out.push_str(&format!(
            "- Base: count {} avg {:?} med {:?} p10 {:?} p90 {:?}\n",
            self.base.charge_durations.count,
            self.base.charge_durations.avg,
            self.base.charge_durations.med,
            self.base.charge_durations.p10,
            self.base.charge_durations.p90
        ));
        out.push_str(&format!(
            "- Current: count {} avg {:?} med {:?} p10 {:?} p90 {:?}\n\n",
            self.current.charge_durations.count,
            self.current.charge_durations.avg,
            self.current.charge_durations.med,
            self.current.charge_durations.p10,
            self.current.charge_durations.p90
        ));

        out.push_str("## Low-shot Levels (<= 5 shots/match)\n");
        out.push_str("- Base:\n");
        for (lvl, count) in &self.base.low_shot_levels {
            out.push_str(&format!("  - {}: {}\n", lvl, count));
        }
        out.push_str("- Current:\n");
        for (lvl, count) in &self.current.low_shot_levels {
            out.push_str(&format!("  - {}: {}\n", lvl, count));
        }
        out.push_str("\n");

        out.push_str("## Possession Sum (per match)\n");
        out.push_str(&format!(
            "- Base: avg {:.2} med {:.2} p10 {:?} p90 {:?}\n",
            self.base.possession_sum.avg,
            self.base.possession_sum.med,
            self.base.possession_sum.p10,
            self.base.possession_sum.p90
        ));
        out.push_str(&format!(
            "- Current: avg {:.2} med {:.2} p10 {:?} p90 {:?}\n",
            self.current.possession_sum.avg,
            self.current.possession_sum.med,
            self.current.possession_sum.p10,
            self.current.possession_sum.p90
        ));

        out
    }
}
