//! Debug-focused analysis for training sessions.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use image::{Rgb, RgbImage};
use rusqlite::Connection;

use crate::constants::{
    ARENA_HEIGHT, ARENA_WIDTH, HEATMAP_CELL_SIZE, HEATMAP_GRID_HEIGHT, HEATMAP_GRID_WIDTH,
    LEVELS_FILE,
};
use crate::levels::LevelDatabase;

const HEATMAP_DIR: &str = "showcase/heatmaps";
const ANALYSIS_CELL_SIZE: u32 = 6;
const STATIONARY_SPEED_THRESHOLD: f32 = 5.0;

type AnyResult<T> = Result<T, Box<dyn std::error::Error>>;

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct DebugSample {
    match_id: i64,
    time_ms: i64,
    tick_frame: i64,
    player: String,
    pos_x: f32,
    pos_y: f32,
    vel_x: f32,
    vel_y: f32,
    input_move_x: f32,
    input_jump: i64,
    grounded: i64,
    is_jumping: i64,
    coyote_timer: f32,
    jump_buffer_timer: f32,
    facing: f32,
    nav_active: i64,
    nav_path_index: i64,
    nav_action: Option<String>,
    level_id: String,
    human_controlled: i64,
}

#[derive(Debug, Clone, Default)]
struct ControlStats {
    samples: usize,
    speed_sum: f32,
    speed_max: f32,
    input_abs_sum: f32,
    grounded_count: usize,
    jump_press_count: usize,
    nav_active_count: usize,
    stationary_count: usize,
}

impl ControlStats {
    fn update(&mut self, sample: &DebugSample) {
        self.samples += 1;
        let speed = (sample.vel_x * sample.vel_x + sample.vel_y * sample.vel_y).sqrt();
        self.speed_sum += speed;
        self.speed_max = self.speed_max.max(speed);
        self.input_abs_sum += sample.input_move_x.abs();
        if sample.grounded != 0 {
            self.grounded_count += 1;
        }
        if sample.input_jump != 0 {
            self.jump_press_count += 1;
        }
        if sample.nav_active != 0 {
            self.nav_active_count += 1;
        }
        if speed <= STATIONARY_SPEED_THRESHOLD {
            self.stationary_count += 1;
        }
    }

    fn avg_speed(&self) -> f32 {
        if self.samples == 0 {
            0.0
        } else {
            self.speed_sum / self.samples as f32
        }
    }

    fn avg_input_abs(&self) -> f32 {
        if self.samples == 0 {
            0.0
        } else {
            self.input_abs_sum / self.samples as f32
        }
    }

    fn grounded_pct(&self) -> f32 {
        if self.samples == 0 {
            0.0
        } else {
            self.grounded_count as f32 / self.samples as f32 * 100.0
        }
    }

    fn jump_press_rate(&self) -> f32 {
        if self.samples == 0 {
            0.0
        } else {
            self.jump_press_count as f32 / self.samples as f32 * 100.0
        }
    }

    fn nav_active_rate(&self) -> f32 {
        if self.samples == 0 {
            0.0
        } else {
            self.nav_active_count as f32 / self.samples as f32 * 100.0
        }
    }

    fn stationary_pct(&self) -> f32 {
        if self.samples == 0 {
            0.0
        } else {
            self.stationary_count as f32 / self.samples as f32 * 100.0
        }
    }
}

#[derive(Debug, Clone)]
struct CoverageStats {
    total_cells: usize,
    human_cells: usize,
    ai_cells: usize,
    union_cells: usize,
    offgrid_samples: usize,
}

#[derive(Debug, Clone, Default)]
struct HeatmapAccumulator {
    count: usize,
    sum: f32,
    min: f32,
    max: f32,
    zero_count: usize,
}

impl HeatmapAccumulator {
    fn push(&mut self, value: f32) {
        if self.count == 0 {
            self.min = value;
            self.max = value;
        } else {
            self.min = self.min.min(value);
            self.max = self.max.max(value);
        }
        if value <= 0.001 {
            self.zero_count += 1;
        }
        self.sum += value;
        self.count += 1;
    }

    fn mean(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f32
        }
    }

    fn zero_pct(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.zero_count as f32 / self.count as f32 * 100.0
        }
    }
}

#[derive(Debug, Clone)]
struct HeatmapComparison {
    label: String,
    human: HeatmapAccumulator,
    ai: HeatmapAccumulator,
    note: Option<String>,
}

#[derive(Debug, Clone)]
struct LevelDebugReport {
    level_id: String,
    level_name: String,
    samples_total: usize,
    human_stats: ControlStats,
    ai_stats: ControlStats,
    coverage: CoverageStats,
    heatmaps: Vec<HeatmapComparison>,
    output_files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TrainingDebugReport {
    pub db_path: String,
    pub session_id: Option<String>,
    pub session_type: Option<String>,
    pub session_count: usize,
    pub match_count: usize,
    pub sample_count: usize,
    pub event_count: usize,
    pub debug_event_count: usize,
    pub debug_without_event: usize,
    pub event_without_debug: usize,
    pub per_session: Vec<SessionSummaryRow>,
    per_level: Vec<LevelDebugReport>,
}

impl TrainingDebugReport {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Training Debug Analysis\n\n");
        out.push_str(&format!("DB: `{}`\n\n", self.db_path));
        if let Some(session_id) = &self.session_id {
            out.push_str(&format!("Session ID: `{}`\n\n", session_id));
        }
        if let Some(session_type) = &self.session_type {
            out.push_str(&format!("Session Type: `{}`\n\n", session_type));
        }
        if self.session_count > 1 {
            out.push_str(&format!(
                "Sessions combined: **{}**\n\n",
                self.session_count
            ));
        }
        out.push_str("## Data Consistency\n\n");
        out.push_str(&format!("- Matches: {}\n", self.match_count));
        out.push_str(&format!("- Events: {}\n", self.event_count));
        out.push_str(&format!("- Debug events: {}\n", self.debug_event_count));
        out.push_str(&format!(
            "- Debug without matching event tick: {}\n",
            self.debug_without_event
        ));
        out.push_str(&format!(
            "- Events without matching debug tick: {}\n\n",
            self.event_without_debug
        ));

        if !self.per_session.is_empty() {
            out.push_str("## Session Summary\n\n");
            out.push_str("| Session | Created | Type | Matches | Events | Debug | Levels |\n");
            out.push_str("|---------|---------|------|---------|--------|-------|--------|\n");
            for row in &self.per_session {
                out.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} | {} |\n",
                    row.session_id,
                    row.created_at,
                    row.session_type,
                    row.matches,
                    row.events,
                    row.debug_events,
                    row.levels
                ));
            }
            out.push_str("\n");
        }

        for level in &self.per_level {
            out.push_str(&format!(
                "## Level: {} ({})\n\n",
                level.level_name, level.level_id
            ));
            out.push_str(&format!(
                "- Samples: {} (human {}, AI {})\n",
                level.samples_total, level.human_stats.samples, level.ai_stats.samples
            ));
            out.push_str(&format!(
                "- Coverage: human {:.1}%, AI {:.1}%, union {:.1}% (off-grid {} samples)\n",
                pct(level.coverage.human_cells, level.coverage.total_cells),
                pct(level.coverage.ai_cells, level.coverage.total_cells),
                pct(level.coverage.union_cells, level.coverage.total_cells),
                level.coverage.offgrid_samples
            ));
            out.push_str(&format!(
                "- Human avg speed {:.1}, grounded {:.1}%, jump rate {:.1}%, nav active {:.1}%, stationary {:.1}%\n",
                level.human_stats.avg_speed(),
                level.human_stats.grounded_pct(),
                level.human_stats.jump_press_rate(),
                level.human_stats.nav_active_rate(),
                level.human_stats.stationary_pct()
            ));
            out.push_str(&format!(
                "- AI avg speed {:.1}, grounded {:.1}%, jump rate {:.1}%, nav active {:.1}%, stationary {:.1}%\n",
                level.ai_stats.avg_speed(),
                level.ai_stats.grounded_pct(),
                level.ai_stats.jump_press_rate(),
                level.ai_stats.nav_active_rate(),
                level.ai_stats.stationary_pct()
            ));
            out.push_str(&format!(
                "- Input magnitude avg: human {:.2}, AI {:.2}\n\n",
                level.human_stats.avg_input_abs(),
                level.ai_stats.avg_input_abs()
            ));

            if !level.heatmaps.is_empty() {
                out.push_str("### Heatmap comparisons\n\n");
                for heat in &level.heatmaps {
                    out.push_str(&format!(
                        "- {}: human mean {:.3} (zero {:.1}%), AI mean {:.3} (zero {:.1}%)",
                        heat.label,
                        heat.human.mean(),
                        heat.human.zero_pct(),
                        heat.ai.mean(),
                        heat.ai.zero_pct()
                    ));
                    if let Some(note) = &heat.note {
                        out.push_str(&format!(" [{}]", note));
                    }
                    out.push_str("\n");
                }
                out.push_str("\n");
            }

            if !level.output_files.is_empty() {
                out.push_str("### Output files\n\n");
                for file in &level.output_files {
                    out.push_str(&format!("- `{}`\n", file));
                }
                out.push_str("\n");
            }
        }

        out
    }
}

#[derive(Debug, Clone)]
pub struct SessionSummaryRow {
    pub session_id: String,
    pub created_at: String,
    pub session_type: String,
    pub matches: usize,
    pub events: usize,
    pub debug_events: usize,
    pub levels: usize,
}

struct HeatmapGrid {
    values: Vec<f32>,
}

impl HeatmapGrid {
    fn new() -> Self {
        Self {
            values: vec![0.0; (HEATMAP_GRID_WIDTH * HEATMAP_GRID_HEIGHT) as usize],
        }
    }

    fn index(cx: u32, cy: u32) -> usize {
        (cy * HEATMAP_GRID_WIDTH + cx) as usize
    }

    fn set(&mut self, cx: u32, cy: u32, value: f32) {
        let idx = Self::index(cx, cy);
        self.values[idx] = value;
    }

    fn get(&self, cx: u32, cy: u32) -> f32 {
        let idx = Self::index(cx, cy);
        self.values[idx]
    }

    fn sample_world(&self, pos_x: f32, pos_y: f32) -> f32 {
        world_to_cell(pos_x, pos_y)
            .map(|(cx, cy)| self.get(cx, cy))
            .unwrap_or(0.0)
    }
}

struct CountGrid {
    counts: Vec<u32>,
}

impl CountGrid {
    fn new() -> Self {
        Self {
            counts: vec![0; (HEATMAP_GRID_WIDTH * HEATMAP_GRID_HEIGHT) as usize],
        }
    }

    fn add(&mut self, cx: u32, cy: u32) {
        let idx = HeatmapGrid::index(cx, cy);
        self.counts[idx] = self.counts[idx].saturating_add(1);
    }

    fn max(&self) -> u32 {
        self.counts.iter().copied().max().unwrap_or(0)
    }
}

pub fn run_training_debug_analysis(
    db_path: &Path,
    output_dir: &Path,
) -> AnyResult<TrainingDebugReport> {
    let conn = Connection::open(db_path)?;

    let mut sessions = Vec::new();
    let mut stmt =
        conn.prepare("SELECT id, created_at, session_type FROM sessions ORDER BY created_at")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        sessions.push(row?);
    }

    let session_count = sessions.len();
    let (session_id, _created_at, session_type) =
        sessions
            .last()
            .cloned()
            .unwrap_or((String::new(), String::new(), String::new()));
    let session_filter = if session_count > 1 { "" } else { &session_id };
    let match_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM matches WHERE (?1 = '') OR session_id = ?1",
            [session_filter],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize;

    let event_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM events e WHERE (?1 = '') OR e.match_id IN (SELECT id FROM matches WHERE session_id = ?1)",
            [session_filter],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize;

    let debug_event_count: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM debug_events d WHERE (?1 = '') OR d.match_id IN (SELECT id FROM matches WHERE session_id = ?1)",
            [session_filter],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize;

    let debug_without_event: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM debug_events d LEFT JOIN events e ON e.match_id = d.match_id AND e.tick_frame = d.tick_frame WHERE e.id IS NULL AND ((?1 = '') OR d.match_id IN (SELECT id FROM matches WHERE session_id = ?1))",
            [session_filter],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize;

    let event_without_debug: usize = conn
        .query_row(
            "SELECT COUNT(*) FROM events e LEFT JOIN debug_events d ON d.match_id = e.match_id AND d.tick_frame = e.tick_frame WHERE d.id IS NULL AND ((?1 = '') OR e.match_id IN (SELECT id FROM matches WHERE session_id = ?1))",
            [session_filter],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) as usize;

    let mut match_level_names: HashMap<i64, String> = HashMap::new();
    let mut stmt =
        conn.prepare("SELECT id, level_name FROM matches WHERE (?1 = '') OR session_id = ?1")?;
    let rows = stmt.query_map([session_filter], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (match_id, level_name) = row?;
        match_level_names.insert(match_id, level_name);
    }

    let level_db = LevelDatabase::load_from_file(LEVELS_FILE);
    let mut level_names_by_id: HashMap<String, String> = HashMap::new();
    for level in &level_db.levels {
        level_names_by_id.insert(level.id.clone(), level.name.clone());
    }

    let mut samples_by_level: HashMap<String, Vec<DebugSample>> = HashMap::new();
    let mut stmt = conn.prepare(
        "SELECT match_id, time_ms, tick_frame, player, pos_x, pos_y, vel_x, vel_y, input_move_x, input_jump, grounded, is_jumping, coyote_timer, jump_buffer_timer, facing, nav_active, nav_path_index, nav_action, level_id, human_controlled FROM debug_events WHERE (?1 = '') OR match_id IN (SELECT id FROM matches WHERE session_id = ?1) ORDER BY level_id, match_id, time_ms",
    )?;
    let rows = stmt.query_map([session_filter], |row| {
        Ok(DebugSample {
            match_id: row.get(0)?,
            time_ms: row.get(1)?,
            tick_frame: row.get(2)?,
            player: row.get(3)?,
            pos_x: row.get(4)?,
            pos_y: row.get(5)?,
            vel_x: row.get(6)?,
            vel_y: row.get(7)?,
            input_move_x: row.get(8)?,
            input_jump: row.get(9)?,
            grounded: row.get(10)?,
            is_jumping: row.get(11)?,
            coyote_timer: row.get(12)?,
            jump_buffer_timer: row.get(13)?,
            facing: row.get(14)?,
            nav_active: row.get(15)?,
            nav_path_index: row.get(16)?,
            nav_action: row.get(17)?,
            level_id: row.get(18)?,
            human_controlled: row.get(19)?,
        })
    })?;

    for row in rows {
        let sample = row?;
        samples_by_level
            .entry(sample.level_id.clone())
            .or_default()
            .push(sample);
    }

    let mut per_level = Vec::new();
    for (level_id, samples) in samples_by_level {
        let level_name = samples
            .first()
            .and_then(|s| match_level_names.get(&s.match_id).cloned())
            .or_else(|| level_names_by_id.get(&level_id).cloned())
            .unwrap_or_else(|| "Unknown".to_string());

        let mut human_stats = ControlStats::default();
        let mut ai_stats = ControlStats::default();
        let mut human_grid = CountGrid::new();
        let mut ai_grid = CountGrid::new();
        let mut union_grid = CountGrid::new();
        let mut offgrid_samples = 0usize;

        for sample in &samples {
            let is_human = sample.human_controlled != 0;
            if is_human {
                human_stats.update(sample);
            } else {
                ai_stats.update(sample);
            }

            if let Some((cx, cy)) = world_to_cell(sample.pos_x, sample.pos_y) {
                if is_human {
                    human_grid.add(cx, cy);
                } else {
                    ai_grid.add(cx, cy);
                }
                union_grid.add(cx, cy);
            } else {
                offgrid_samples += 1;
            }
        }

        let coverage = CoverageStats {
            total_cells: (HEATMAP_GRID_WIDTH * HEATMAP_GRID_HEIGHT) as usize,
            human_cells: count_nonzero(&human_grid),
            ai_cells: count_nonzero(&ai_grid),
            union_cells: count_nonzero(&union_grid),
            offgrid_samples,
        };

        let mut output_files = Vec::new();
        let safe_name = sanitize_level_name(&level_name);
        let combined_paths =
            write_derived_grid(&union_grid, output_dir, &safe_name, &level_id, "combined")?;
        output_files.extend(combined_paths);
        let human_paths =
            write_derived_grid(&human_grid, output_dir, &safe_name, &level_id, "human")?;
        output_files.extend(human_paths);
        let ai_paths = write_derived_grid(&ai_grid, output_dir, &safe_name, &level_id, "ai")?;
        output_files.extend(ai_paths);

        let heatmaps = build_heatmap_comparisons(&samples, &level_name, &level_id);

        per_level.push(LevelDebugReport {
            level_id,
            level_name,
            samples_total: samples.len(),
            human_stats,
            ai_stats,
            coverage,
            heatmaps,
            output_files,
        });
    }

    per_level.sort_by(|a, b| a.level_name.cmp(&b.level_name));

    let mut per_session = Vec::new();
    for (sid, created_at, sess_type) in sessions {
        let matches: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM matches WHERE session_id = ?1",
                [sid.as_str()],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;
        let events: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM events WHERE match_id IN (SELECT id FROM matches WHERE session_id = ?1)",
                [sid.as_str()],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;
        let debug_events: usize = conn
            .query_row(
                "SELECT COUNT(*) FROM debug_events WHERE match_id IN (SELECT id FROM matches WHERE session_id = ?1)",
                [sid.as_str()],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;
        let levels: usize = conn
            .query_row(
                "SELECT COUNT(DISTINCT level_name) FROM matches WHERE session_id = ?1",
                [sid.as_str()],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0) as usize;
        per_session.push(SessionSummaryRow {
            session_id: sid,
            created_at,
            session_type: sess_type,
            matches,
            events,
            debug_events,
            levels,
        });
    }

    Ok(TrainingDebugReport {
        db_path: db_path.display().to_string(),
        session_id: if session_count > 1 {
            None
        } else if session_id.is_empty() {
            None
        } else {
            Some(session_id)
        },
        session_type: if session_count > 1 {
            None
        } else if session_type.is_empty() {
            None
        } else {
            Some(session_type)
        },
        session_count,
        match_count,
        sample_count: debug_event_count,
        event_count,
        debug_event_count,
        debug_without_event,
        event_without_debug,
        per_session,
        per_level,
    })
}

fn build_heatmap_comparisons(
    samples: &[DebugSample],
    level_name: &str,
    level_id: &str,
) -> Vec<HeatmapComparison> {
    let mut heatmaps = Vec::new();
    let safe_name = sanitize_level_name(level_name);

    let mut map = HashMap::new();
    for label in [
        "reachability",
        "landing_safety",
        "path_cost",
        "speed",
        "elevation",
        "escape_routes",
    ] {
        let grid = load_heatmap_grid(&resolve_heatmap_path(label, &safe_name, level_id, None));
        map.insert(label.to_string(), grid);
    }

    let los_left = load_heatmap_grid(&resolve_heatmap_path(
        "line_of_sight",
        &safe_name,
        level_id,
        Some("left"),
    ));
    let los_right = load_heatmap_grid(&resolve_heatmap_path(
        "line_of_sight",
        &safe_name,
        level_id,
        Some("right"),
    ));

    let mut comparisons: HashMap<String, HeatmapComparison> = HashMap::new();
    for label in [
        "reachability",
        "landing_safety",
        "path_cost",
        "speed",
        "elevation",
        "escape_routes",
    ] {
        let note = if map.get(label).and_then(|g| g.as_ref()).is_none() {
            Some("missing".to_string())
        } else {
            None
        };
        comparisons.insert(
            label.to_string(),
            HeatmapComparison {
                label: label.to_string(),
                human: HeatmapAccumulator::default(),
                ai: HeatmapAccumulator::default(),
                note,
            },
        );
    }

    let mut los_comp = HeatmapComparison {
        label: "line_of_sight".to_string(),
        human: HeatmapAccumulator::default(),
        ai: HeatmapAccumulator::default(),
        note: if los_left.is_none() || los_right.is_none() {
            Some("missing".to_string())
        } else {
            Some("basket-specific".to_string())
        },
    };

    for sample in samples {
        let is_human = sample.human_controlled != 0;
        for (label, grid) in &map {
            if let Some(grid) = grid {
                let value = grid.sample_world(sample.pos_x, sample.pos_y);
                let entry = comparisons.get_mut(label).expect("heatmap entry");
                if is_human {
                    entry.human.push(value);
                } else {
                    entry.ai.push(value);
                }
            }
        }

        let los_grid = match sample.player.as_str() {
            "L" => los_right.as_ref(),
            "R" => los_left.as_ref(),
            _ => None,
        };
        if let Some(grid) = los_grid {
            let value = grid.sample_world(sample.pos_x, sample.pos_y);
            if is_human {
                los_comp.human.push(value);
            } else {
                los_comp.ai.push(value);
            }
        }
    }

    for (_, entry) in comparisons {
        heatmaps.push(entry);
    }
    heatmaps.push(los_comp);
    heatmaps
}

fn load_heatmap_grid(path: &PathBuf) -> Option<HeatmapGrid> {
    if !path.exists() {
        return None;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return None;
    };

    let mut grid = HeatmapGrid::new();
    for (idx, line) in text.lines().enumerate() {
        if idx == 0 && line.trim().starts_with("x,") {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 3 {
            continue;
        }
        let Ok(x) = parts[0].trim().parse::<f32>() else {
            continue;
        };
        let Ok(y) = parts[1].trim().parse::<f32>() else {
            continue;
        };
        let Ok(value) = parts[2].trim().parse::<f32>() else {
            continue;
        };
        if let Some((cx, cy)) = world_to_cell(x, y) {
            grid.set(cx, cy, value);
        }
    }

    Some(grid)
}

fn resolve_heatmap_path(
    label: &str,
    safe_name: &str,
    level_id: &str,
    side: Option<&str>,
) -> PathBuf {
    let base = match side {
        Some(side) => format!("heatmap_{}_{}_{}_{}", label, safe_name, level_id, side),
        None => format!("heatmap_{}_{}_{}", label, safe_name, level_id),
    };
    let direct = Path::new(HEATMAP_DIR).join(format!("{}.txt", base));
    if direct.exists() {
        return direct;
    }

    let prefix = format!("heatmap_{}_{}", label, safe_name);
    let mut matches = Vec::new();
    if let Ok(entries) = fs::read_dir(HEATMAP_DIR) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if !name.starts_with(&prefix) || !name.ends_with(".txt") {
                continue;
            }
            if let Some(side) = side {
                if !name.ends_with(&format!("_{}.txt", side)) {
                    continue;
                }
            } else if name.ends_with("_left.txt") || name.ends_with("_right.txt") {
                continue;
            }
            matches.push(path);
        }
    }

    matches.into_iter().next().unwrap_or(direct)
}

fn sanitize_level_name(name: &str) -> String {
    let mut out = String::new();
    let mut last_was_underscore = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_underscore = false;
        } else if !last_was_underscore {
            out.push('_');
            last_was_underscore = true;
        }
    }

    out.trim_matches('_').to_string()
}

fn world_to_cell(x: f32, y: f32) -> Option<(u32, u32)> {
    let cx = ((x + ARENA_WIDTH / 2.0) / HEATMAP_CELL_SIZE as f32).floor() as i32;
    let cy = ((ARENA_HEIGHT / 2.0 - y) / HEATMAP_CELL_SIZE as f32).floor() as i32;
    if cx < 0 || cy < 0 {
        return None;
    }
    let cx = cx as u32;
    let cy = cy as u32;
    if cx >= HEATMAP_GRID_WIDTH || cy >= HEATMAP_GRID_HEIGHT {
        return None;
    }
    Some((cx, cy))
}

fn count_nonzero(grid: &CountGrid) -> usize {
    grid.counts.iter().filter(|v| **v > 0).count()
}

fn pct(numer: usize, denom: usize) -> f32 {
    if denom == 0 {
        0.0
    } else {
        numer as f32 / denom as f32 * 100.0
    }
}

fn write_derived_grid(
    grid: &CountGrid,
    output_dir: &Path,
    safe_name: &str,
    level_id: &str,
    label: &str,
) -> AnyResult<Vec<String>> {
    fs::create_dir_all(output_dir).ok();

    let max_count = grid.max().max(1) as f32;
    let base = format!("derived_reachability_{}_{}_{}", safe_name, level_id, label);
    let csv_path = output_dir.join(format!("{}.csv", base));
    let png_path = output_dir.join(format!("{}.png", base));

    let mut csv = String::from("x,y,value\n");
    for cy in 0..HEATMAP_GRID_HEIGHT {
        for cx in 0..HEATMAP_GRID_WIDTH {
            let idx = HeatmapGrid::index(cx, cy);
            let count = grid.counts[idx] as f32;
            let value = (count / max_count).clamp(0.0, 1.0);
            let world_x = (cx as f32 + 0.5) * HEATMAP_CELL_SIZE as f32 - ARENA_WIDTH / 2.0;
            let world_y = ARENA_HEIGHT / 2.0 - (cy as f32 + 0.5) * HEATMAP_CELL_SIZE as f32;
            csv.push_str(&format!("{:.2},{:.2},{:.3}\n", world_x, world_y, value));
        }
    }

    fs::write(&csv_path, csv)?;

    let img_width = HEATMAP_GRID_WIDTH * ANALYSIS_CELL_SIZE;
    let img_height = HEATMAP_GRID_HEIGHT * ANALYSIS_CELL_SIZE;
    let mut img = RgbImage::new(img_width, img_height);
    let bg = Rgb([245, 245, 245]);
    for pixel in img.pixels_mut() {
        *pixel = bg;
    }

    for cy in 0..HEATMAP_GRID_HEIGHT {
        for cx in 0..HEATMAP_GRID_WIDTH {
            let idx = HeatmapGrid::index(cx, cy);
            let count = grid.counts[idx] as f32;
            let value = (count / max_count).clamp(0.0, 1.0);
            let color = value_to_color(value);
            fill_cell(&mut img, cx, cy, color);
        }
    }

    img.save(&png_path)?;

    Ok(vec![
        csv_path.display().to_string(),
        png_path.display().to_string(),
    ])
}

fn fill_cell(img: &mut RgbImage, cx: u32, cy: u32, color: Rgb<u8>) {
    let start_x = cx * ANALYSIS_CELL_SIZE;
    let start_y = cy * ANALYSIS_CELL_SIZE;
    for y in start_y..(start_y + ANALYSIS_CELL_SIZE) {
        for x in start_x..(start_x + ANALYSIS_CELL_SIZE) {
            if x < img.width() && y < img.height() {
                img.put_pixel(x, y, color);
            }
        }
    }
}

fn value_to_color(value: f32) -> Rgb<u8> {
    let v = value.clamp(0.0, 1.0);
    let intensity = (255.0 * v) as u8;
    let base = 255u8.saturating_sub(intensity / 2);
    Rgb([intensity, base, base])
}
