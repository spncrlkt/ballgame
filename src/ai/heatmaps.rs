//! Heatmap loading and sampling for AI decision making

use bevy::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::env;

use crate::constants::{
    ARENA_HEIGHT, ARENA_WIDTH, HEATMAP_CELL_SIZE, HEATMAP_GRID_HEIGHT, HEATMAP_GRID_WIDTH,
};
use crate::levels::LevelDatabase;
use crate::scoring::CurrentLevel;
use crate::world::Basket;

const HEATMAP_DIR: &str = "showcase/heatmaps";

#[derive(Clone)]
pub struct HeatmapGrid {
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

    pub fn sample_world(&self, pos: Vec2) -> f32 {
        world_to_cell(pos.x, pos.y)
            .map(|(cx, cy)| self.get(cx, cy))
            .unwrap_or(0.0)
    }
}

#[derive(Resource, Clone)]
pub struct HeatmapBundle {
    pub built_for_level_id: String,
    pub score_left: HeatmapGrid,
    pub score_right: HeatmapGrid,
    pub line_of_sight_left: HeatmapGrid,
    pub line_of_sight_right: HeatmapGrid,
    pub speed: HeatmapGrid,
    pub reachability: HeatmapGrid,
    pub landing_safety: HeatmapGrid,
    pub path_cost: HeatmapGrid,
    pub elevation: HeatmapGrid,
    pub escape_routes: HeatmapGrid,
}

impl Default for HeatmapBundle {
    fn default() -> Self {
        Self {
            built_for_level_id: String::new(),
            score_left: HeatmapGrid::new(),
            score_right: HeatmapGrid::new(),
            line_of_sight_left: HeatmapGrid::new(),
            line_of_sight_right: HeatmapGrid::new(),
            speed: HeatmapGrid::new(),
            reachability: HeatmapGrid::new(),
            landing_safety: HeatmapGrid::new(),
            path_cost: HeatmapGrid::new(),
            elevation: HeatmapGrid::new(),
            escape_routes: HeatmapGrid::new(),
        }
    }
}

fn skip_reachability_heatmaps() -> bool {
    match env::var("BALLGAME_SKIP_REACHABILITY_HEATMAPS") {
        Ok(val) => {
            let val = val.to_lowercase();
            val == "1" || val == "true" || val == "yes"
        }
        Err(_) => false,
    }
}

impl HeatmapBundle {
    pub fn score_for_basket(&self, basket: Basket, pos: Vec2) -> f32 {
        match basket {
            Basket::Left => self.score_left.sample_world(pos),
            Basket::Right => self.score_right.sample_world(pos),
        }
    }

    pub fn line_of_sight_for_basket(&self, basket: Basket, pos: Vec2) -> f32 {
        match basket {
            Basket::Left => self.line_of_sight_left.sample_world(pos),
            Basket::Right => self.line_of_sight_right.sample_world(pos),
        }
    }
}

/// Load all heatmaps for the current level when the level changes.
pub fn load_heatmaps_on_level_change(
    current_level: Res<CurrentLevel>,
    level_db: Res<LevelDatabase>,
    mut heatmaps: ResMut<HeatmapBundle>,
) {
    if !current_level.is_changed() && heatmaps.built_for_level_id == current_level.0 {
        return;
    }

    let Some(level) = level_db
        .get_by_id(&current_level.0)
        .or_else(|| level_db.get_by_name(&current_level.0))
    else {
        panic!("Heatmaps: current level '{}' not found in database", current_level.0);
    };

    let safe_name = sanitize_level_name(level.name.as_str());

    let score_left = load_heatmap_grid(&resolve_heatmap_path(
        "score",
        &safe_name,
        level.id.as_str(),
        Some("left"),
    ));
    let score_right = load_heatmap_grid(&resolve_heatmap_path(
        "score",
        &safe_name,
        level.id.as_str(),
        Some("right"),
    ));
    let line_of_sight_left = load_heatmap_grid(&resolve_heatmap_path(
        "line_of_sight",
        &safe_name,
        level.id.as_str(),
        Some("left"),
    ));
    let line_of_sight_right = load_heatmap_grid(&resolve_heatmap_path(
        "line_of_sight",
        &safe_name,
        level.id.as_str(),
        Some("right"),
    ));
    let speed = load_heatmap_grid(&resolve_heatmap_path(
        "speed",
        &safe_name,
        level.id.as_str(),
        None,
    ));
    let skip_reachability = skip_reachability_heatmaps();
    if skip_reachability {
        warn!(
            "Heatmaps: reachability/path_cost/escape_routes skipped via BALLGAME_SKIP_REACHABILITY_HEATMAPS"
        );
    }
    let reachability = if skip_reachability {
        HeatmapGrid::new()
    } else {
        load_heatmap_grid(&resolve_heatmap_path(
            "reachability",
            &safe_name,
            level.id.as_str(),
            None,
        ))
    };
    let landing_safety = load_heatmap_grid(&resolve_heatmap_path(
        "landing_safety",
        &safe_name,
        level.id.as_str(),
        None,
    ));
    let path_cost = if skip_reachability {
        HeatmapGrid::new()
    } else {
        load_heatmap_grid(&resolve_heatmap_path(
            "path_cost",
            &safe_name,
            level.id.as_str(),
            None,
        ))
    };
    let elevation = load_heatmap_grid(&resolve_heatmap_path(
        "elevation",
        &safe_name,
        level.id.as_str(),
        None,
    ));
    let escape_routes = if skip_reachability {
        HeatmapGrid::new()
    } else {
        load_heatmap_grid(&resolve_heatmap_path(
            "escape_routes",
            &safe_name,
            level.id.as_str(),
            None,
        ))
    };

    *heatmaps = HeatmapBundle {
        built_for_level_id: level.id.clone(),
        score_left,
        score_right,
        line_of_sight_left,
        line_of_sight_right,
        speed,
        reachability,
        landing_safety,
        path_cost,
        elevation,
        escape_routes,
    };
}

fn load_heatmap_grid(path: &Path) -> HeatmapGrid {
    let data = fs::read_to_string(path).unwrap_or_else(|err| {
        panic!(
            "Heatmaps: failed to read {}: {}",
            path.display(),
            err
        )
    });

    let mut lines = data.lines();
    let mut value_scale = 1.0;
    let mut first_line = lines.next().unwrap_or_default().trim();

    if first_line.starts_with("x,y") {
        if first_line.contains("shot_pct") {
            value_scale = 0.01;
        }
    } else {
        // No header line; treat first line as data.
        lines = data.lines();
        first_line = "";
    }

    let mut grid = HeatmapGrid::new();
    let mut filled = vec![false; (HEATMAP_GRID_WIDTH * HEATMAP_GRID_HEIGHT) as usize];

    for line in lines {
        let line = line.trim();
        if line.is_empty() || line == first_line {
            continue;
        }
        let mut parts = line.split(',');
        let (Some(x_str), Some(y_str), Some(value_str)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };

        let Ok(x) = x_str.trim().parse::<f32>() else {
            continue;
        };
        let Ok(y) = y_str.trim().parse::<f32>() else {
            continue;
        };
        let Ok(mut value) = value_str.trim().parse::<f32>() else {
            continue;
        };

        value = (value * value_scale).clamp(0.0, 1.0);
        let Some((cx, cy)) = world_to_cell(x, y) else {
            continue;
        };
        let idx = HeatmapGrid::index(cx, cy);
        filled[idx] = true;
        grid.set(cx, cy, value);
    }

    if filled.iter().any(|v| !v) {
        panic!(
            "Heatmaps: {} missing {} cells",
            path.display(),
            filled.iter().filter(|v| !**v).count()
        );
    }

    grid
}

fn resolve_heatmap_path(label: &str, safe_name: &str, level_id: &str, side: Option<&str>) -> PathBuf {
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
    let Ok(entries) = fs::read_dir(HEATMAP_DIR) else {
        panic!("Heatmaps: missing directory {}", HEATMAP_DIR);
    };

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

    match matches.len() {
        1 => matches.remove(0),
        0 => panic!(
            "Heatmaps: missing {} heatmap for level '{}' (expected {}, fallback by name failed)",
            label,
            safe_name,
            base
        ),
        _ => panic!(
            "Heatmaps: multiple {} heatmaps matched for level '{}' ({:?})",
            label,
            safe_name,
            matches
        ),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn load_score_heatmap_scales_percent() {
        let mut data = String::from("x,y,shot_pct\n");
        for cy in 0..HEATMAP_GRID_HEIGHT {
            for cx in 0..HEATMAP_GRID_WIDTH {
                let world_x = (cx as f32 + 0.5) * HEATMAP_CELL_SIZE as f32 - ARENA_WIDTH / 2.0;
                let world_y = ARENA_HEIGHT / 2.0 - (cy as f32 + 0.5) * HEATMAP_CELL_SIZE as f32;
                let value = if cx == 0 && cy == 0 { 50.0 } else { 0.0 };
                data.push_str(&format!("{:.2},{:.2},{:.2}\n", world_x, world_y, value));
            }
        }

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("heatmap_test_{}.txt", nanos));
        fs::write(&path, data).expect("write test heatmap");

        let grid = load_heatmap_grid(&path);
        let sample_pos = Vec2::new(
            -ARENA_WIDTH / 2.0 + HEATMAP_CELL_SIZE as f32 * 0.5,
            ARENA_HEIGHT / 2.0 - HEATMAP_CELL_SIZE as f32 * 0.5,
        );
        let sample = grid.sample_world(sample_pos);
        assert!(
            (sample - 0.5).abs() < 0.001,
            "expected 0.5, got {}",
            sample
        );

        let _ = fs::remove_file(&path);
    }
}
