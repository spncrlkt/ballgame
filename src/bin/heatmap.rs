//! Heatmap generator for shot trajectories
//!
//! Generates heatmaps for shot analysis:
//! - **speed** (default): Shot angle (arrow direction) and required speed (color)
//! - **score**: Scoring percentage via Monte Carlo simulation with rim physics
//!
//! Usage:
//!   cargo run --bin heatmap                    # Default: speed heatmap
//!   cargo run --bin heatmap -- speed           # Explicit: speed heatmap
//!   cargo run --bin heatmap -- score           # Scoring percentage heatmaps (per level)
//!   cargo run --bin heatmap -- --type reachability
//!   cargo run --bin heatmap -- --full --level "Catwalk"
//!   cargo run --bin heatmap -- --check
//!   cargo run --bin heatmap -- --full --check
//!   cargo run --bin heatmap -- --full --refresh
//!   cargo run --bin heatmap -- score --level "Catwalk"
//!   cargo run --bin heatmap -- score --level b7569f063af0f78b
//!   cargo run --bin heatmap -- speed --level "Open Floor"
//!
//! Speed outputs land in showcase/heatmaps as:
//!   heatmap_speed_<level>_<uuid>.png
//!   heatmap_speed_<level>_<uuid>.txt (x,y,value)
//! Score outputs land in showcase/heatmaps as:
//!   heatmap_score_<level>_<uuid>_<side>.png
//!   heatmap_score_<level>_<uuid>_<side>.txt (x,y,shot_pct)
//! Other heatmaps land in showcase/heatmaps as:
//!   heatmap_<type>_<level>_<uuid>.png
//!   heatmap_<type>_<level>_<uuid>.txt (x,y,value)
//! Line-of-sight heatmaps include left/right suffixes.
//! Combined sheets are written to showcase/heatmap_<type>_all.png.
//! Full bundles write showcase/heatmaps/heatmap_full_<level>_<uuid>.png.
//! Skips debug/regression levels and training protocol levels unless --level is specified.

use ballgame::training::TrainingProtocol;
use ballgame::{
    AIR_ACCEL, AIR_DECEL, ARENA_FLOOR_Y, ARENA_HEIGHT, ARENA_WIDTH, BALL_BOUNCE, BALL_GRAVITY,
    CORNER_STEP_THICKNESS, GRAVITY_FALL, GRAVITY_RISE, GROUND_ACCEL, GROUND_DECEL, JUMP_VELOCITY,
    LevelDatabase, MOVE_SPEED, PLAYER_SIZE, RIM_THICKNESS, SHOT_DISTANCE_VARIANCE,
    SHOT_MIN_VARIANCE, WALL_THICKNESS, basket_x_from_offset, calculate_shot_trajectory,
};
use bevy::prelude::Vec2;
use image::{Rgb, RgbImage};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::sync::{Mutex, OnceLock};

// Basket dimensions (matching ballgame constants)
const BASKET_SIZE_X: f32 = 60.0;
const BASKET_SIZE_Y: f32 = 80.0;
const BALL_RADIUS: f32 = 12.0;

// Grid settings
const CELL_SIZE: u32 = 20; // pixels per cell
const GRID_WIDTH: u32 = (ARENA_WIDTH as u32) / CELL_SIZE; // 80 cells
const GRID_HEIGHT: u32 = (ARENA_HEIGHT as u32) / CELL_SIZE; // 45 cells

// Speed range for color mapping
const SPEED_MIN: f32 = 300.0; // Green
const SPEED_MAX: f32 = 1400.0; // Red

// Monte Carlo settings
const MONTE_CARLO_TRIALS: u32 = 100;

const LEVELS_FILE: &str = "config/levels.txt";
const LEVEL_HASH_FILE: &str = "config/level_hashes.json";
const OUTPUT_DIR: &str = "showcase/heatmaps";
const HEATMAP_STATS_FILE: &str = "showcase/heatmaps/heatmap_stats.txt";

const REACHABILITY_SAMPLES_PER_START: usize = 15;
const REACHABILITY_DT: f32 = 1.0 / 120.0;
const REACHABILITY_MAX_TIME: f32 = 1.6;
const REACHABILITY_INPUT_INTERVAL: f32 = 0.12;
const REACHABILITY_PASSABLE_THRESHOLD: f32 = 0.01;

const LANDING_TOLERANCE: f32 = 6.0;
const ELEVATION_RANGE: f32 = 500.0;
const SCORE_MASK_THRESHOLD: f32 = 0.1;
const SCORE_PERCENTILE_LOW: f32 = 0.1;
const SCORE_PERCENTILE_HIGH: f32 = 0.9;

// =============================================================================
// SIMULATION TYPE
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum HeatmapKind {
    Speed,
    Score,
    Reachability,
    LandingSafety,
    PathCost,
    LineOfSight,
    Elevation,
    EscapeRoutes,
}

#[derive(Debug, Clone, Copy)]
enum HeatmapMode {
    Single(HeatmapKind),
    Full,
}

struct SimConfig {
    mode: HeatmapMode,
    level_filter: Vec<String>,
    check: bool,
    refresh: bool,
}

fn parse_args() -> SimConfig {
    let mut mode = HeatmapMode::Single(HeatmapKind::Speed);
    let mut level_filter = Vec::new();
    let mut check = false;
    let mut refresh = false;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "score" => mode = HeatmapMode::Single(HeatmapKind::Score),
            "speed" => mode = HeatmapMode::Single(HeatmapKind::Speed),
            "--type" => {
                if let Some(value) = args.next() {
                    if let Some(kind) = parse_heatmap_kind(&value) {
                        mode = HeatmapMode::Single(kind);
                    }
                }
            }
            "--full" => mode = HeatmapMode::Full,
            "--check" => check = true,
            "--refresh" => refresh = true,
            "--level" => {
                if let Some(value) = args.next() {
                    level_filter.push(value);
                }
            }
            _ => {}
        }
    }

    SimConfig {
        mode,
        level_filter,
        check,
        refresh,
    }
}

// =============================================================================
// RIM GEOMETRY
// =============================================================================

struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

/// Build rim geometry for collision detection
/// The basket opening is BASKET_SIZE_X wide, with rims on sides and bottom
fn build_rim_geometry(basket_x: f32, basket_y: f32) -> Vec<Rect> {
    let half_opening = BASKET_SIZE_X / 2.0;

    vec![
        // Outer rim (wall side) - 50% of basket height
        Rect {
            x: basket_x + half_opening,
            y: basket_y,
            width: RIM_THICKNESS,
            height: BASKET_SIZE_Y * 0.5,
        },
        // Inner rim (center side) - 10% of basket height
        Rect {
            x: basket_x - half_opening - RIM_THICKNESS,
            y: basket_y,
            width: RIM_THICKNESS,
            height: BASKET_SIZE_Y * 0.1,
        },
        // Bottom rim
        Rect {
            x: basket_x - half_opening,
            y: basket_y - BASKET_SIZE_Y / 2.0 - RIM_THICKNESS,
            width: BASKET_SIZE_X,
            height: RIM_THICKNESS,
        },
    ]
}

/// Check collision between circle and rectangle, return normal if colliding
fn check_circle_rect_collision(cx: f32, cy: f32, radius: f32, rect: &Rect) -> Option<(f32, f32)> {
    // Find closest point on rectangle to circle center
    let closest_x = cx.clamp(rect.x, rect.x + rect.width);
    let closest_y = cy.clamp(rect.y - rect.height, rect.y);

    let dx = cx - closest_x;
    let dy = cy - closest_y;
    let dist_sq = dx * dx + dy * dy;

    if dist_sq < radius * radius && dist_sq > 0.0 {
        let dist = dist_sq.sqrt();
        Some((dx / dist, dy / dist)) // Normal pointing away from rect
    } else {
        None
    }
}

// =============================================================================
// BALL FLIGHT SIMULATION
// =============================================================================

/// Simulate ball flight with rim physics, returns true if ball scores
fn simulate_ball_flight(
    start_x: f32,
    start_y: f32,
    angle: f32,
    speed: f32,
    basket_x: f32,
    basket_y: f32,
) -> bool {
    const DT: f32 = 0.001; // 1ms timestep
    const MAX_TIME: f32 = 5.0;

    let mut x = start_x;
    let mut y = start_y;
    let mut vx = angle.cos() * speed;
    let mut vy = angle.sin() * speed;
    let mut t = 0.0;

    let ball_radius = BALL_RADIUS;
    let rims = build_rim_geometry(basket_x, basket_y);

    // Scoring zone (inside basket)
    let score_left = basket_x - BASKET_SIZE_X / 2.0 + ball_radius;
    let score_right = basket_x + BASKET_SIZE_X / 2.0 - ball_radius;
    let score_top = basket_y;
    let score_bottom = basket_y - BASKET_SIZE_Y / 2.0;

    while t < MAX_TIME {
        // Apply gravity
        vy -= BALL_GRAVITY * DT;
        x += vx * DT;
        y += vy * DT;
        t += DT;

        // Check rim collisions
        for rim in &rims {
            if let Some((nx, ny)) = check_circle_rect_collision(x, y, ball_radius, rim) {
                // Reflect velocity
                let dot = vx * nx + vy * ny;
                vx = (vx - 2.0 * dot * nx) * BALL_BOUNCE;
                vy = (vy - 2.0 * dot * ny) * BALL_BOUNCE;
                // Push out of collision
                x += nx * 2.0;
                y += ny * 2.0;
            }
        }

        // Check if scored (ball center in basket bounds)
        if x > score_left && x < score_right && y < score_top && y > score_bottom {
            return true;
        }

        // Ball fell below floor - miss
        if y < ARENA_FLOOR_Y - 50.0 {
            return false;
        }
    }
    false
}

// =============================================================================
// MONTE CARLO SCORING SIMULATION
// =============================================================================

/// Simulate scoring percentage from a position using Monte Carlo
///
/// This simulates "ideal" fully-charged stationary shots to represent the AI
/// decision quality baseline. The heatmap now includes factors that affect
/// actual shots in throw.rs:
/// - Speed randomness (±10%)
/// - Distance multiplier (1.0→1.05 linear)
/// - Angle variance (based on SHOT_MIN_VARIANCE for ideal shots)
fn simulate_scoring(shooter_x: f32, shooter_y: f32, basket_x: f32, basket_y: f32) -> f32 {
    let mut rng = rand::thread_rng();

    let Some(traj) =
        calculate_shot_trajectory(shooter_x, shooter_y, basket_x, basket_y, BALL_GRAVITY)
    else {
        return 0.0;
    };

    let mut makes = 0;

    for _ in 0..MONTE_CARLO_TRIALS {
        // Apply variance: base + distance penalty (matching throw.rs)
        let distance = ((basket_x - shooter_x).powi(2) + (basket_y - shooter_y).powi(2)).sqrt();
        let distance_variance = distance * SHOT_DISTANCE_VARIANCE;
        let total_variance = SHOT_MIN_VARIANCE + distance_variance;

        // Random angle offset within variance range
        let angle_offset = rng.gen_range(-total_variance..total_variance) * 30f32.to_radians();
        let final_angle = traj.angle + angle_offset;

        // Speed randomness (±10%) - matching throw.rs line 173
        let speed_randomness = rng.gen_range(0.9..1.1);

        // Distance-based speed multiplier - matching throw.rs lines 163-167
        // Simple linear: 1.0 at close range (dx=200), 1.05 at far range (dx=800)
        let dx = (basket_x - shooter_x).abs();
        let t = ((dx - 200.0) / 600.0).clamp(0.0, 1.0);
        let distance_multiplier = 1.0 + 0.05 * t;

        // Final speed with both factors applied
        let final_speed = traj.required_speed * distance_multiplier * speed_randomness;

        if simulate_ball_flight(
            shooter_x,
            shooter_y,
            final_angle,
            final_speed,
            basket_x,
            basket_y,
        ) {
            makes += 1;
        }
    }

    makes as f32 / MONTE_CARLO_TRIALS as f32
}

// =============================================================================
// MAIN
// =============================================================================

fn main() {
    let config = parse_args();
    fs::create_dir_all(OUTPUT_DIR).expect("Failed to create heatmap output directory");
    if config.refresh {
        clear_heatmap_outputs();
    }

    let level_db = LevelDatabase::load_from_file(LEVELS_FILE);
    let training_levels = training_level_names();
    let level_hashes = compute_level_hashes(&level_db);
    let change_set = if config.check {
        let changes = compare_level_hashes(&level_hashes);
        print_level_changes(&level_db, &changes);
        changes
    } else {
        LevelChangeSet::default()
    };

    let eligible_levels = select_target_levels(&level_db, &training_levels, &config, &change_set);
    if eligible_levels.is_empty() {
        println!("No eligible levels found for heatmap generation.");
        return;
    }

    match config.mode {
        HeatmapMode::Single(kind) => {
            println!(
                "Generating {} heatmap: {}x{} cells ({} pixels)",
                heatmap_kind_label(kind),
                GRID_WIDTH,
                GRID_HEIGHT,
                CELL_SIZE
            );
            run_single_kind(kind, &eligible_levels);
        }
        HeatmapMode::Full => {
            println!(
                "Generating full heatmap bundle: {}x{} cells ({} pixels)",
                GRID_WIDTH, GRID_HEIGHT, CELL_SIZE
            );
            run_full_bundle(&eligible_levels);
            if config.check && config.level_filter.is_empty() {
                save_level_hashes(&level_hashes);
            }
        }
    }
}

fn parse_heatmap_kind(value: &str) -> Option<HeatmapKind> {
    match value.to_lowercase().as_str() {
        "speed" => Some(HeatmapKind::Speed),
        "score" => Some(HeatmapKind::Score),
        "reachability" | "reach" => Some(HeatmapKind::Reachability),
        "landing" | "landing_safety" | "landing-safety" => Some(HeatmapKind::LandingSafety),
        "path" | "path_cost" | "path-cost" => Some(HeatmapKind::PathCost),
        "los" | "line_of_sight" | "line-of-sight" => Some(HeatmapKind::LineOfSight),
        "elevation" | "height" => Some(HeatmapKind::Elevation),
        "escape" | "escape_routes" | "escape-routes" => Some(HeatmapKind::EscapeRoutes),
        _ => None,
    }
}

fn heatmap_kind_label(kind: HeatmapKind) -> &'static str {
    match kind {
        HeatmapKind::Speed => "speed",
        HeatmapKind::Score => "score",
        HeatmapKind::Reachability => "reachability",
        HeatmapKind::LandingSafety => "landing_safety",
        HeatmapKind::PathCost => "path_cost",
        HeatmapKind::LineOfSight => "line_of_sight",
        HeatmapKind::Elevation => "elevation",
        HeatmapKind::EscapeRoutes => "escape_routes",
    }
}

fn heatmap_kinds_all() -> &'static [HeatmapKind] {
    &[
        HeatmapKind::Speed,
        HeatmapKind::Score,
        HeatmapKind::Reachability,
        HeatmapKind::LandingSafety,
        HeatmapKind::PathCost,
        HeatmapKind::LineOfSight,
        HeatmapKind::Elevation,
        HeatmapKind::EscapeRoutes,
    ]
}

#[derive(Default)]
struct LevelChangeSet {
    new_ids: Vec<String>,
    changed_ids: Vec<String>,
    removed_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct LevelHashCache {
    levels: HashMap<String, String>,
}

#[derive(Clone)]
struct HeatmapGrid {
    values: Vec<f32>,
}

impl HeatmapGrid {
    fn new() -> Self {
        Self {
            values: vec![0.0; (GRID_WIDTH * GRID_HEIGHT) as usize],
        }
    }

    fn index(cx: u32, cy: u32) -> usize {
        (cy * GRID_WIDTH + cx) as usize
    }

    fn get(&self, cx: u32, cy: u32) -> f32 {
        self.values[Self::index(cx, cy)]
    }

    fn set(&mut self, cx: u32, cy: u32, value: f32) {
        let idx = Self::index(cx, cy);
        self.values[idx] = value;
    }
}

#[derive(Clone, Copy, Debug)]
struct PlatformRect {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

struct LevelOverlayContext<'a> {
    platform_rects: &'a [PlatformRect],
    basket_left_x: f32,
    basket_right_x: f32,
    basket_y: f32,
}

fn stats_header_written() -> &'static Mutex<bool> {
    static HEADER_WRITTEN: OnceLock<Mutex<bool>> = OnceLock::new();
    HEADER_WRITTEN.get_or_init(|| Mutex::new(false))
}

fn select_target_levels<'a>(
    level_db: &'a LevelDatabase,
    training_levels: &[&'static str],
    config: &SimConfig,
    changes: &LevelChangeSet,
) -> Vec<&'a ballgame::LevelData> {
    let filter_active = !config.level_filter.is_empty();
    let mut levels = Vec::new();

    if matches!(config.mode, HeatmapMode::Full) && config.check && !filter_active {
        let mut target_ids = Vec::new();
        target_ids.extend(changes.new_ids.iter().cloned());
        target_ids.extend(changes.changed_ids.iter().cloned());

        for id in target_ids {
            if let Some(level) = level_db.get_by_id(&id) {
                if should_skip_level(level, training_levels) {
                    continue;
                }
                levels.push(level);
            }
        }
        return levels;
    }

    for level in level_db.all() {
        if filter_active {
            if level_matches_filter(level, &config.level_filter) {
                levels.push(level);
            }
        } else if !should_skip_level(level, training_levels) {
            levels.push(level);
        }
    }

    levels
}

fn run_single_kind(kind: HeatmapKind, levels: &[&ballgame::LevelData]) {
    let mut generated = Vec::new();
    let mut generated_overlays = Vec::new();

    for level in levels {
        let basket_y = ARENA_FLOOR_Y + level.basket_height;
        let (left_x, right_x) = basket_x_from_offset(level.basket_push_in);
        let platform_rects = build_platform_rects(level);
        let overlay = LevelOverlayContext {
            platform_rects: &platform_rects,
            basket_left_x: left_x,
            basket_right_x: right_x,
            basket_y,
        };
        if kind == HeatmapKind::Score {
            generated.push(generate_score_heatmap(
                level.name.as_str(),
                level.id.as_str(),
                "left",
                left_x,
                basket_y,
                Some(&overlay),
            ));
            generated_overlays.push(overlay_path(
                "score",
                level.name.as_str(),
                level.id.as_str(),
                Some("left"),
            ));
            generated.push(generate_score_heatmap(
                level.name.as_str(),
                level.id.as_str(),
                "right",
                right_x,
                basket_y,
                Some(&overlay),
            ));
            generated_overlays.push(overlay_path(
                "score",
                level.name.as_str(),
                level.id.as_str(),
                Some("right"),
            ));
        } else if kind == HeatmapKind::LineOfSight {
            let los_left = compute_line_of_sight(&platform_rects, left_x, basket_y);
            let los_right = compute_line_of_sight(&platform_rects, right_x, basket_y);
            generated.push(generate_value_heatmap(
                level,
                "line_of_sight",
                &los_left,
                1.0,
                Some("left"),
                Some(&overlay),
            ));
            generated_overlays.push(overlay_path(
                "line_of_sight",
                level.name.as_str(),
                level.id.as_str(),
                Some("left"),
            ));
            generated.push(generate_value_heatmap(
                level,
                "line_of_sight",
                &los_right,
                1.0,
                Some("right"),
                Some(&overlay),
            ));
            generated_overlays.push(overlay_path(
                "line_of_sight",
                level.name.as_str(),
                level.id.as_str(),
                Some("right"),
            ));
        } else {
            let image_path = generate_heatmap_for_kind(
                kind,
                level,
                right_x,
                basket_y,
                &platform_rects,
                None,
                Some(&overlay),
            );
            generated.push(image_path);
            generated_overlays.push(overlay_path(
                heatmap_kind_label(kind),
                level.name.as_str(),
                level.id.as_str(),
                None,
            ));
        }
    }

    let combined = format!("showcase/heatmap_{}_all.png", heatmap_kind_label(kind));
    combine_heatmaps(&generated, &combined);
    let combined_overlay = format!(
        "showcase/heatmap_{}_all_overlay.png",
        heatmap_kind_label(kind)
    );
    combine_heatmaps(&generated_overlays, &combined_overlay);
    match kind {
        HeatmapKind::Speed => println!(
            "Speed range: {} (green) to {} (red) pixels/sec",
            SPEED_MIN, SPEED_MAX
        ),
        HeatmapKind::Score => println!(
            "Score range: 0% (red) to 100% (green), {} trials per cell",
            MONTE_CARLO_TRIALS
        ),
        _ => {}
    }
}

fn run_full_bundle(levels: &[&ballgame::LevelData]) {
    let mut per_kind: HashMap<HeatmapKind, Vec<String>> = HashMap::new();
    let mut per_kind_overlays: HashMap<HeatmapKind, Vec<String>> = HashMap::new();

    for level in levels {
        let basket_y = ARENA_FLOOR_Y + level.basket_height;
        let (left_x, right_x) = basket_x_from_offset(level.basket_push_in);
        let platform_rects = build_platform_rects(level);
        let reachability = compute_reachability(&platform_rects);
        let overlay = LevelOverlayContext {
            platform_rects: &platform_rects,
            basket_left_x: left_x,
            basket_right_x: right_x,
            basket_y,
        };

        let mut level_images = Vec::new();
        for &kind in heatmap_kinds_all() {
            if kind == HeatmapKind::Score {
                let left_path = generate_score_heatmap(
                    level.name.as_str(),
                    level.id.as_str(),
                    "left",
                    left_x,
                    basket_y,
                    Some(&overlay),
                );
                let right_path = generate_score_heatmap(
                    level.name.as_str(),
                    level.id.as_str(),
                    "right",
                    right_x,
                    basket_y,
                    Some(&overlay),
                );
                level_images.push(left_path.clone());
                level_images.push(right_path.clone());
                per_kind.entry(kind).or_default().push(left_path);
                per_kind.entry(kind).or_default().push(right_path);
                per_kind_overlays
                    .entry(kind)
                    .or_default()
                    .push(overlay_path(
                        "score",
                        level.name.as_str(),
                        level.id.as_str(),
                        Some("left"),
                    ));
                per_kind_overlays
                    .entry(kind)
                    .or_default()
                    .push(overlay_path(
                        "score",
                        level.name.as_str(),
                        level.id.as_str(),
                        Some("right"),
                    ));
            } else if kind == HeatmapKind::LineOfSight {
                let los_left = compute_line_of_sight(&platform_rects, left_x, basket_y);
                let los_right = compute_line_of_sight(&platform_rects, right_x, basket_y);
                let left_path = generate_value_heatmap(
                    level,
                    "line_of_sight",
                    &los_left,
                    1.0,
                    Some("left"),
                    Some(&overlay),
                );
                let right_path = generate_value_heatmap(
                    level,
                    "line_of_sight",
                    &los_right,
                    1.0,
                    Some("right"),
                    Some(&overlay),
                );
                level_images.push(left_path.clone());
                level_images.push(right_path.clone());
                per_kind.entry(kind).or_default().push(left_path);
                per_kind.entry(kind).or_default().push(right_path);
                per_kind_overlays
                    .entry(kind)
                    .or_default()
                    .push(overlay_path(
                        "line_of_sight",
                        level.name.as_str(),
                        level.id.as_str(),
                        Some("left"),
                    ));
                per_kind_overlays
                    .entry(kind)
                    .or_default()
                    .push(overlay_path(
                        "line_of_sight",
                        level.name.as_str(),
                        level.id.as_str(),
                        Some("right"),
                    ));
            } else {
                let image_path = generate_heatmap_for_kind(
                    kind,
                    level,
                    right_x,
                    basket_y,
                    &platform_rects,
                    Some(&reachability),
                    Some(&overlay),
                );
                level_images.push(image_path.clone());
                per_kind.entry(kind).or_default().push(image_path);
                per_kind_overlays
                    .entry(kind)
                    .or_default()
                    .push(overlay_path(
                        heatmap_kind_label(kind),
                        level.name.as_str(),
                        level.id.as_str(),
                        None,
                    ));
            }
        }

        let safe_name = sanitize_level_name(level.name.as_str());
        let level_bundle = format!("{}/heatmap_full_{}_{}.png", OUTPUT_DIR, safe_name, level.id);
        combine_heatmaps(&level_images, &level_bundle);
    }

    for &kind in heatmap_kinds_all() {
        if let Some(images) = per_kind.get(&kind) {
            let combined = format!("showcase/heatmap_{}_all.png", heatmap_kind_label(kind));
            combine_heatmaps(images, &combined);
        }
        if let Some(overlays) = per_kind_overlays.get(&kind) {
            let combined_overlay = format!(
                "showcase/heatmap_{}_all_overlay.png",
                heatmap_kind_label(kind)
            );
            combine_heatmaps(overlays, &combined_overlay);
        }
    }

    if let Some(images) = per_kind.get(&HeatmapKind::LineOfSight) {
        let left: Vec<String> = images
            .iter()
            .filter(|path| path.ends_with("_left.png"))
            .cloned()
            .collect();
        let right: Vec<String> = images
            .iter()
            .filter(|path| path.ends_with("_right.png"))
            .cloned()
            .collect();
        combine_heatmaps(&left, "showcase/heatmap_line_of_sight_left_all.png");
        combine_heatmaps(&right, "showcase/heatmap_line_of_sight_right_all.png");
    }
    if let Some(overlays) = per_kind_overlays.get(&HeatmapKind::LineOfSight) {
        let left: Vec<String> = overlays
            .iter()
            .filter(|path| path.ends_with("_left_overlay.png"))
            .cloned()
            .collect();
        let right: Vec<String> = overlays
            .iter()
            .filter(|path| path.ends_with("_right_overlay.png"))
            .cloned()
            .collect();
        combine_heatmaps(&left, "showcase/heatmap_line_of_sight_left_all_overlay.png");
        combine_heatmaps(&right, "showcase/heatmap_line_of_sight_right_all_overlay.png");
    }
}

fn generate_heatmap_for_kind(
    kind: HeatmapKind,
    level: &ballgame::LevelData,
    basket_x: f32,
    basket_y: f32,
    platform_rects: &[PlatformRect],
    reachability_cache: Option<&HeatmapGrid>,
    overlay: Option<&LevelOverlayContext<'_>>,
) -> String {
    let mut owned_reachability = None;

    match kind {
        HeatmapKind::Speed => generate_speed_heatmap(
            level.name.as_str(),
            level.id.as_str(),
            basket_x,
            basket_y,
            overlay,
        ),
        HeatmapKind::Score => generate_score_heatmap(
            level.name.as_str(),
            level.id.as_str(),
            "right",
            basket_x,
            basket_y,
            overlay,
        ),
        HeatmapKind::Reachability => {
            let reachability = if let Some(cache) = reachability_cache {
                cache
            } else {
                if owned_reachability.is_none() {
                    owned_reachability = Some(compute_reachability(platform_rects));
                }
                owned_reachability.as_ref().expect("reachability cache")
            };
            generate_value_heatmap(level, "reachability", &reachability, 1.0, None, overlay)
        }
        HeatmapKind::LandingSafety => {
            let safety = compute_landing_safety(platform_rects);
            generate_value_heatmap(level, "landing_safety", &safety, 1.0, None, overlay)
        }
        HeatmapKind::PathCost => {
            let reachability = if let Some(cache) = reachability_cache {
                cache
            } else {
                if owned_reachability.is_none() {
                    owned_reachability = Some(compute_reachability(platform_rects));
                }
                owned_reachability.as_ref().expect("reachability cache")
            };
            let cost = compute_path_cost(reachability);
            generate_value_heatmap(level, "path_cost", &cost, 1.0, None, overlay)
        }
        HeatmapKind::LineOfSight => {
            let los = compute_line_of_sight(platform_rects, basket_x, basket_y);
            generate_value_heatmap(level, "line_of_sight", &los, 1.0, None, overlay)
        }
        HeatmapKind::Elevation => {
            let elevation = compute_elevation(basket_y);
            generate_value_heatmap(level, "elevation", &elevation, 1.0, None, overlay)
        }
        HeatmapKind::EscapeRoutes => {
            let reachability = if let Some(cache) = reachability_cache {
                cache
            } else {
                if owned_reachability.is_none() {
                    owned_reachability = Some(compute_reachability(platform_rects));
                }
                owned_reachability.as_ref().expect("reachability cache")
            };
            let escape = compute_escape_routes(reachability);
            generate_value_heatmap(level, "escape_routes", &escape, 1.0, None, overlay)
        }
    }
}

fn build_platform_rects(level: &ballgame::LevelData) -> Vec<PlatformRect> {
    let mut rects = Vec::new();

    for platform in &level.platforms {
        match platform {
            ballgame::PlatformDef::Mirror { x, y, width } => {
                let world_y = ARENA_FLOOR_Y + *y;
                rects.push(rect_from_center(-x, world_y, *width, 20.0));
                rects.push(rect_from_center(*x, world_y, *width, 20.0));
            }
            ballgame::PlatformDef::Center { y, width } => {
                let world_y = ARENA_FLOOR_Y + *y;
                rects.push(rect_from_center(0.0, world_y, *width, 20.0));
            }
        }
    }

    if level.step_count > 0 {
        let left_wall_inner = -ARENA_WIDTH / 2.0 + WALL_THICKNESS;
        let right_wall_inner = ARENA_WIDTH / 2.0 - WALL_THICKNESS;
        let step_height = level.corner_height / level.step_count as f32;
        let step_width = level.corner_width / level.step_count as f32;
        let floor_top = ARENA_FLOOR_Y + 20.0;

        for i in 0..level.step_count {
            let step_num = (level.step_count - 1 - i) as f32;
            let y = floor_top + step_height * (step_num + 0.5);

            let (x, width) = if i == 0 {
                let right_edge = left_wall_inner + level.step_push_in + step_width;
                let center = (left_wall_inner + right_edge) / 2.0;
                let full_width = right_edge - left_wall_inner;
                (center, full_width)
            } else {
                (
                    left_wall_inner + level.step_push_in + step_width * (i as f32 + 0.5),
                    step_width,
                )
            };
            rects.push(rect_from_center(x, y, width, CORNER_STEP_THICKNESS));
        }

        for i in 0..level.step_count {
            let step_num = (level.step_count - 1 - i) as f32;
            let y = floor_top + step_height * (step_num + 0.5);

            let (x, width) = if i == 0 {
                let left_edge = right_wall_inner - level.step_push_in - step_width;
                let center = (right_wall_inner + left_edge) / 2.0;
                let full_width = right_wall_inner - left_edge;
                (center, full_width)
            } else {
                (
                    right_wall_inner - level.step_push_in - step_width * (i as f32 + 0.5),
                    step_width,
                )
            };
            rects.push(rect_from_center(x, y, width, CORNER_STEP_THICKNESS));
        }
    }

    rects
}

fn rect_from_center(x: f32, y: f32, width: f32, height: f32) -> PlatformRect {
    let half_w = width / 2.0;
    let half_h = height / 2.0;
    PlatformRect {
        left: x - half_w,
        right: x + half_w,
        top: y + half_h,
        bottom: y - half_h,
    }
}

fn generate_value_heatmap(
    level: &ballgame::LevelData,
    label: &str,
    grid: &HeatmapGrid,
    scale: f32,
    side: Option<&str>,
    overlay: Option<&LevelOverlayContext<'_>>,
) -> String {
    let safe_name = sanitize_level_name(level.name.as_str());
    let base_name = if let Some(side) = side {
        format!("heatmap_{}_{}_{}_{}", label, safe_name, level.id, side)
    } else {
        format!("heatmap_{}_{}_{}", label, safe_name, level.id)
    };
    let image_path = format!("{}/{}.png", OUTPUT_DIR, base_name);
    let data_path = format!("{}/{}.txt", OUTPUT_DIR, base_name);

    let img_width = GRID_WIDTH * CELL_SIZE;
    let img_height = GRID_HEIGHT * CELL_SIZE;
    let mut img = RgbImage::new(img_width, img_height);

    let bg_color = Rgb([230, 230, 230]);
    for pixel in img.pixels_mut() {
        *pixel = bg_color;
    }

    let mut data = String::from("x,y,value\n");
    let mut values = Vec::with_capacity((GRID_WIDTH * GRID_HEIGHT) as usize);
    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            let value = grid.get(cx, cy).clamp(0.0, 1.0);
            let color = score_to_color(value);
            fill_cell(&mut img, cx, cy, color);
            values.push(value);

            let (world_x, world_y) = cell_world_coords(cx, cy);
            let _ = writeln!(
                &mut data,
                "{:.2},{:.2},{:.3}",
                world_x,
                world_y,
                value * scale
            );
        }
    }

    img.save(&image_path).expect("Failed to save image");
    if let Some(overlay) = overlay {
        write_level_overlay(
            &img,
            label,
            level.name.as_str(),
            level.id.as_str(),
            side,
            overlay,
        );
    }
    fs::write(&data_path, data).expect("Failed to write heatmap data");
    report_heatmap_stats(label, level.name.as_str(), level.id.as_str(), side, &values);
    println!("Saved {} and {}", image_path, data_path);
    image_path
}

fn compute_reachability(platform_rects: &[PlatformRect]) -> HeatmapGrid {
    // TODO: Hard fail until reachability uses gameplay tuning (config/gameplay_tuning.json)
    // to ensure the heatmap matches real in-game physics.
    panic!(
        "TODO: reachability heatmap must load gameplay tuning before simulating (see docs/analysis/tuning_workflows.md; workflow skips reachability for now)"
    );

    let mut grid = HeatmapGrid::new();
    let mut counts = vec![0u32; (GRID_WIDTH * GRID_HEIGHT) as usize];
    let mut rng = rand::thread_rng();

    let start_y = ARENA_FLOOR_Y + PLAYER_SIZE.y / 2.0;
    let per_start = REACHABILITY_SAMPLES_PER_START as f32;

    for cx in 0..GRID_WIDTH {
        let (start_x, _) = cell_world_coords(cx, 0);
        for _ in 0..REACHABILITY_SAMPLES_PER_START {
            let mut visited = vec![false; (GRID_WIDTH * GRID_HEIGHT) as usize];
            simulate_jump(
                start_x,
                start_y,
                platform_rects,
                &mut rng,
                &mut |pos: Vec2| {
                    if let Some((gx, gy)) = world_to_cell(pos.x, pos.y) {
                        let idx = HeatmapGrid::index(gx, gy);
                        if !visited[idx] {
                            visited[idx] = true;
                            counts[idx] += 1;
                        }
                    }
                },
            );
        }
    }

    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            let idx = HeatmapGrid::index(cx, cy);
            let value = (counts[idx] as f32 / per_start).clamp(0.0, 1.0);
            grid.set(cx, cy, value);
        }
    }

    grid
}

fn compute_landing_safety(platform_rects: &[PlatformRect]) -> HeatmapGrid {
    let mut grid = HeatmapGrid::new();
    let mut surfaces = platform_rects.to_vec();

    let floor_left = -ARENA_WIDTH / 2.0 + WALL_THICKNESS;
    let floor_right = ARENA_WIDTH / 2.0 - WALL_THICKNESS;
    let floor_rect = PlatformRect {
        left: floor_left,
        right: floor_right,
        top: ARENA_FLOOR_Y,
        bottom: ARENA_FLOOR_Y - 1.0,
    };
    surfaces.push(floor_rect);

    let half_h = PLAYER_SIZE.y / 2.0;
    let min_margin = PLAYER_SIZE.x;
    let max_margin = PLAYER_SIZE.x * 3.0;

    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            let (world_x, world_y) = cell_world_coords(cx, cy);
            let mut best: f32 = 0.0;

            for rect in &surfaces {
                if world_x < rect.left || world_x > rect.right {
                    continue;
                }
                let bottom = world_y - half_h;
                if (bottom - rect.top).abs() <= LANDING_TOLERANCE {
                    let margin = (world_x - rect.left).min(rect.right - world_x);
                    let safety = if margin <= min_margin {
                        0.0
                    } else {
                        ((margin - min_margin) / (max_margin - min_margin)).clamp(0.0, 1.0)
                    };
                    best = best.max(safety);
                }
            }

            grid.set(cx, cy, best);
        }
    }

    grid
}

fn compute_path_cost(reachability: &HeatmapGrid) -> HeatmapGrid {
    let mut grid = HeatmapGrid::new();
    let mut dist = vec![u32::MAX; (GRID_WIDTH * GRID_HEIGHT) as usize];
    let mut queue = VecDeque::new();

    for cx in 0..GRID_WIDTH {
        let (_, world_y) = cell_world_coords(cx, GRID_HEIGHT - 1);
        let passable = reachability.get(cx, GRID_HEIGHT - 1) >= REACHABILITY_PASSABLE_THRESHOLD;
        if passable && world_y <= ARENA_FLOOR_Y + PLAYER_SIZE.y {
            let idx = HeatmapGrid::index(cx, GRID_HEIGHT - 1);
            dist[idx] = 0;
            queue.push_back((cx, GRID_HEIGHT - 1));
        }
    }

    let neighbors = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)];

    while let Some((cx, cy)) = queue.pop_front() {
        let base_idx = HeatmapGrid::index(cx, cy);
        let base_dist = dist[base_idx];

        for (dx, dy) in neighbors {
            let nx = cx as i32 + dx;
            let ny = cy as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let nx = nx as u32;
            let ny = ny as u32;
            if nx >= GRID_WIDTH || ny >= GRID_HEIGHT {
                continue;
            }
            if reachability.get(nx, ny) < REACHABILITY_PASSABLE_THRESHOLD {
                continue;
            }
            let nidx = HeatmapGrid::index(nx, ny);
            if dist[nidx] == u32::MAX {
                dist[nidx] = base_dist + 1;
                queue.push_back((nx, ny));
            }
        }
    }

    let max_dist = dist
        .iter()
        .copied()
        .filter(|d| *d != u32::MAX)
        .max()
        .unwrap_or(0) as f32;
    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            let idx = HeatmapGrid::index(cx, cy);
            let value = if dist[idx] == u32::MAX || max_dist <= 0.0 {
                0.0
            } else {
                1.0 - (dist[idx] as f32 / max_dist)
            };
            grid.set(cx, cy, value);
        }
    }

    grid
}

fn compute_line_of_sight(
    platform_rects: &[PlatformRect],
    basket_x: f32,
    basket_y: f32,
) -> HeatmapGrid {
    let mut grid = HeatmapGrid::new();

    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            let (world_x, world_y) = cell_world_coords(cx, cy);
            let clear = !platform_rects
                .iter()
                .any(|rect| segment_intersects_rect(world_x, world_y, basket_x, basket_y, rect));
            grid.set(cx, cy, if clear { 1.0 } else { 0.0 });
        }
    }

    grid
}

fn compute_elevation(basket_y: f32) -> HeatmapGrid {
    let mut grid = HeatmapGrid::new();

    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            let (_, world_y) = cell_world_coords(cx, cy);
            let dy = world_y - basket_y;
            let value = ((dy / ELEVATION_RANGE) + 1.0).clamp(0.0, 2.0) / 2.0;
            grid.set(cx, cy, value);
        }
    }

    grid
}

fn compute_escape_routes(reachability: &HeatmapGrid) -> HeatmapGrid {
    let mut grid = HeatmapGrid::new();
    let neighbors = [
        (-1i32, -1i32),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];

    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            if reachability.get(cx, cy) < REACHABILITY_PASSABLE_THRESHOLD {
                continue;
            }
            let mut count = 0;
            for (dx, dy) in neighbors {
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;
                if nx < 0 || ny < 0 {
                    continue;
                }
                let nx = nx as u32;
                let ny = ny as u32;
                if nx >= GRID_WIDTH || ny >= GRID_HEIGHT {
                    continue;
                }
                if reachability.get(nx, ny) >= REACHABILITY_PASSABLE_THRESHOLD {
                    count += 1;
                }
            }
            let value = (count as f32 / neighbors.len() as f32).clamp(0.0, 1.0);
            grid.set(cx, cy, value);
        }
    }

    grid
}

fn simulate_jump(
    start_x: f32,
    start_y: f32,
    platform_rects: &[PlatformRect],
    rng: &mut impl Rng,
    mut on_sample: impl FnMut(Vec2),
) {
    let mut x = start_x;
    let mut y = start_y;
    let mut vx = 0.0;
    let mut vy = JUMP_VELOCITY;
    let mut t = 0.0;
    let mut input_dir = 0.0;
    let mut next_input = 0.0;
    let half_w = PLAYER_SIZE.x / 2.0;
    let half_h = PLAYER_SIZE.y / 2.0;

    on_sample(Vec2::new(x, y - half_h));

    while t < REACHABILITY_MAX_TIME {
        if t >= next_input {
            input_dir = match rng.gen_range(0..=2) {
                0 => -1.0,
                1 => 0.0,
                _ => 1.0,
            };
            next_input += REACHABILITY_INPUT_INTERVAL;
        }

        let accel = if y - half_h <= ARENA_FLOOR_Y + 0.5 {
            GROUND_ACCEL
        } else {
            AIR_ACCEL
        };
        let decel = if y - half_h <= ARENA_FLOOR_Y + 0.5 {
            GROUND_DECEL
        } else {
            AIR_DECEL
        };

        let target_vx = input_dir * MOVE_SPEED;
        if input_dir.abs() > 0.01 {
            vx = approach(vx, target_vx, accel * REACHABILITY_DT);
        } else {
            vx = approach(vx, 0.0, decel * REACHABILITY_DT);
        }

        let gravity = if vy > 0.0 { GRAVITY_RISE } else { GRAVITY_FALL };
        vy -= gravity * REACHABILITY_DT;

        let prev_y = y;
        x += vx * REACHABILITY_DT;
        y += vy * REACHABILITY_DT;

        let min_x = -ARENA_WIDTH / 2.0 + WALL_THICKNESS + half_w;
        let max_x = ARENA_WIDTH / 2.0 - WALL_THICKNESS - half_w;
        if x < min_x {
            x = min_x;
            vx = 0.0;
        } else if x > max_x {
            x = max_x;
            vx = 0.0;
        }

        let mut grounded = false;
        if y - half_h <= ARENA_FLOOR_Y {
            y = ARENA_FLOOR_Y + half_h;
            vy = 0.0;
            grounded = true;
        } else if vy <= 0.0 {
            for rect in platform_rects {
                if x < rect.left || x > rect.right {
                    continue;
                }
                let prev_bottom = prev_y - half_h;
                let next_bottom = y - half_h;
                if prev_bottom >= rect.top && next_bottom <= rect.top {
                    y = rect.top + half_h;
                    vy = 0.0;
                    grounded = true;
                    break;
                }
            }
        }

        on_sample(Vec2::new(x, y - half_h));

        if grounded && t > 0.1 {
            break;
        }

        t += REACHABILITY_DT;
    }
}

fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

fn world_to_cell(x: f32, y: f32) -> Option<(u32, u32)> {
    let cx = ((x + ARENA_WIDTH / 2.0) / CELL_SIZE as f32).floor() as i32;
    let cy = ((ARENA_HEIGHT / 2.0 - y) / CELL_SIZE as f32).floor() as i32;
    if cx < 0 || cy < 0 {
        return None;
    }
    let cx = cx as u32;
    let cy = cy as u32;
    if cx >= GRID_WIDTH || cy >= GRID_HEIGHT {
        return None;
    }
    Some((cx, cy))
}

fn segment_intersects_rect(x0: f32, y0: f32, x1: f32, y1: f32, rect: &PlatformRect) -> bool {
    if point_in_rect(x0, y0, rect) || point_in_rect(x1, y1, rect) {
        return true;
    }

    let edges = [
        (rect.left, rect.bottom, rect.right, rect.bottom),
        (rect.right, rect.bottom, rect.right, rect.top),
        (rect.right, rect.top, rect.left, rect.top),
        (rect.left, rect.top, rect.left, rect.bottom),
    ];

    edges
        .iter()
        .any(|&(ax, ay, bx, by)| segments_intersect(x0, y0, x1, y1, ax, ay, bx, by))
}

fn point_in_rect(x: f32, y: f32, rect: &PlatformRect) -> bool {
    x >= rect.left && x <= rect.right && y >= rect.bottom && y <= rect.top
}

fn segments_intersect(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    x3: f32,
    y3: f32,
    x4: f32,
    y4: f32,
) -> bool {
    let d = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if d.abs() < f32::EPSILON {
        return false;
    }
    let t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / d;
    let u = -((x1 - x2) * (y1 - y3) - (y1 - y2) * (x1 - x3)) / d;
    t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0
}

fn compute_level_hashes(level_db: &LevelDatabase) -> HashMap<String, String> {
    let mut hashes = HashMap::new();
    for level in level_db.all() {
        hashes.insert(level.id.clone(), hash_level(level));
    }
    hashes
}

fn hash_level(level: &ballgame::LevelData) -> String {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();

    level.id.hash(&mut hasher);
    level.name.hash(&mut hasher);
    hash_f32(&mut hasher, level.basket_height);
    hash_f32(&mut hasher, level.basket_push_in);
    level.step_count.hash(&mut hasher);
    hash_f32(&mut hasher, level.corner_height);
    hash_f32(&mut hasher, level.corner_width);
    hash_f32(&mut hasher, level.step_push_in);
    level.debug.hash(&mut hasher);
    level.regression.hash(&mut hasher);

    for platform in &level.platforms {
        match platform {
            ballgame::PlatformDef::Mirror { x, y, width } => {
                "mirror".hash(&mut hasher);
                hash_f32(&mut hasher, *x);
                hash_f32(&mut hasher, *y);
                hash_f32(&mut hasher, *width);
            }
            ballgame::PlatformDef::Center { y, width } => {
                "center".hash(&mut hasher);
                hash_f32(&mut hasher, *y);
                hash_f32(&mut hasher, *width);
            }
        }
    }

    format!("{:016x}", hasher.finish())
}

fn hash_f32<H: Hasher>(hasher: &mut H, value: f32) {
    value.to_bits().hash(hasher);
}

fn load_level_hash_cache() -> LevelHashCache {
    match fs::read_to_string(LEVEL_HASH_FILE) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => LevelHashCache::default(),
    }
}

fn save_level_hashes(current: &HashMap<String, String>) {
    let cache = LevelHashCache {
        levels: current.clone(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&cache) {
        if let Err(err) = fs::write(LEVEL_HASH_FILE, json) {
            println!("Failed to write level hash cache: {}", err);
        }
    }
}

fn compare_level_hashes(current: &HashMap<String, String>) -> LevelChangeSet {
    let cache = load_level_hash_cache();
    let mut changes = LevelChangeSet::default();

    for (id, hash) in current {
        match cache.levels.get(id) {
            None => changes.new_ids.push(id.clone()),
            Some(old) if old != hash => changes.changed_ids.push(id.clone()),
            _ => {}
        }
    }

    for id in cache.levels.keys() {
        if !current.contains_key(id) {
            changes.removed_ids.push(id.clone());
        }
    }

    changes
}

fn print_level_changes(level_db: &LevelDatabase, changes: &LevelChangeSet) {
    if changes.new_ids.is_empty()
        && changes.changed_ids.is_empty()
        && changes.removed_ids.is_empty()
    {
        println!("No new or changed levels detected.");
        return;
    }

    if !changes.new_ids.is_empty() {
        println!("New levels:");
        for id in &changes.new_ids {
            if let Some(level) = level_db.get_by_id(id) {
                println!("  {} ({})", level.name, level.id);
            } else {
                println!("  {}", id);
            }
        }
    }

    if !changes.changed_ids.is_empty() {
        println!("Changed levels:");
        for id in &changes.changed_ids {
            if let Some(level) = level_db.get_by_id(id) {
                println!("  {} ({})", level.name, level.id);
            } else {
                println!("  {}", id);
            }
        }
    }

    if !changes.removed_ids.is_empty() {
        println!("Removed levels:");
        for id in &changes.removed_ids {
            println!("  {}", id);
        }
    }
}

fn clear_heatmap_outputs() {
    let showcase_root = "showcase";
    if let Ok(entries) = fs::read_dir(OUTPUT_DIR) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("heatmap_") {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir(showcase_root) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("heatmap_") && name.ends_with(".png") {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }

    let _ = fs::remove_file(LEVEL_HASH_FILE);
    let _ = fs::remove_file(HEATMAP_STATS_FILE);
    println!("Cleared prior heatmap outputs and hash cache.");
}

/// Convert normalized speed (0-1) to RGB color
/// Low speed = green, high speed = red
fn speed_to_color(t: f32) -> Rgb<u8> {
    // Green -> Yellow -> Red gradient
    let r = (t * 2.0).min(1.0);
    let g = ((1.0 - t) * 2.0).min(1.0);
    Rgb([(r * 255.0) as u8, (g * 255.0) as u8, 50])
}

/// Convert score percentage (0-1) to RGB color
/// Low score = red, high score = green
fn score_to_color(pct: f32) -> Rgb<u8> {
    // Red -> Yellow -> Green gradient (opposite of speed)
    let r = ((1.0 - pct) * 2.0).min(1.0);
    let g = (pct * 2.0).min(1.0);
    Rgb([(r * 255.0) as u8, (g * 255.0) as u8, 50])
}

/// Fill a cell with a solid color
fn fill_cell(img: &mut RgbImage, cx: u32, cy: u32, color: Rgb<u8>) {
    let x_start = cx * CELL_SIZE;
    let y_start = cy * CELL_SIZE;

    for dy in 0..CELL_SIZE {
        for dx in 0..CELL_SIZE {
            img.put_pixel(x_start + dx, y_start + dy, color);
        }
    }
}

/// Draw a small arrow in a cell indicating shot angle
fn draw_arrow(img: &mut RgbImage, cx: u32, cy: u32, angle: f32, base_color: Rgb<u8>) {
    let center_x = (cx * CELL_SIZE + CELL_SIZE / 2) as f32;
    let center_y = (cy * CELL_SIZE + CELL_SIZE / 2) as f32;

    // Arrow length (most of half cell size)
    let len = (CELL_SIZE as f32) * 0.7;

    // Note: image Y is inverted from world Y, so negate sin
    let dx = angle.cos() * len;
    let dy = -angle.sin() * len; // Negative because image Y is down

    // Arrow color (darker version of base)
    let arrow_color = Rgb([
        (base_color.0[0] as f32 * 0.5) as u8,
        (base_color.0[1] as f32 * 0.5) as u8,
        (base_color.0[2] as f32 * 0.5) as u8,
    ]);

    // Draw line from center toward direction
    draw_line(
        img,
        center_x as i32,
        center_y as i32,
        (center_x + dx) as i32,
        (center_y + dy) as i32,
        arrow_color,
    );
}

/// Draw a line using Bresenham's algorithm
fn draw_line(img: &mut RgbImage, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgb<u8>) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = x0;
    let mut y = y0;

    loop {
        if x >= 0 && x < img.width() as i32 && y >= 0 && y < img.height() as i32 {
            img.put_pixel(x as u32, y as u32, color);
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

/// Draw a marker for the basket position
fn draw_basket_marker(img: &mut RgbImage, basket_x: f32, basket_y: f32) {
    // Convert world coords to image coords
    let img_x = ((basket_x + ARENA_WIDTH / 2.0) / CELL_SIZE as f32 * CELL_SIZE as f32) as u32;
    let img_y = ((ARENA_HEIGHT / 2.0 - basket_y) / CELL_SIZE as f32 * CELL_SIZE as f32) as u32;

    let basket_color = Rgb([200, 50, 50]); // Red

    // Draw a small cross at basket position
    let size = CELL_SIZE as i32;
    for i in -size..=size {
        let px = (img_x as i32 + i) as u32;
        let py = img_y;
        if px < img.width() && py < img.height() {
            img.put_pixel(px, py, basket_color);
        }
        let px = img_x;
        let py = (img_y as i32 + i) as u32;
        if px < img.width() && py < img.height() {
            img.put_pixel(px, py, basket_color);
        }
    }
}

/// Draw the floor line
fn draw_floor_line(img: &mut RgbImage) {
    // Convert ARENA_FLOOR_Y to image Y
    let floor_img_y = ((ARENA_HEIGHT / 2.0 - ARENA_FLOOR_Y) / 1.0) as u32;

    if floor_img_y < img.height() {
        let floor_color = Rgb([50, 50, 50]); // Dark gray
        for x in 0..img.width() {
            img.put_pixel(x, floor_img_y, floor_color);
        }
    }
}

fn cell_world_coords(cx: u32, cy: u32) -> (f32, f32) {
    // Convert cell to world coordinates (center of cell)
    // Image Y is top-down, world Y is bottom-up
    let world_x = (cx as f32 + 0.5) * CELL_SIZE as f32 - ARENA_WIDTH / 2.0;
    let world_y = ARENA_HEIGHT / 2.0 - (cy as f32 + 0.5) * CELL_SIZE as f32;
    (world_x, world_y)
}

fn generate_speed_heatmap(
    level_name: &str,
    level_id: &str,
    basket_x: f32,
    basket_y: f32,
    overlay: Option<&LevelOverlayContext<'_>>,
) -> String {
    let safe_name = sanitize_level_name(level_name);
    let base_name = format!("heatmap_speed_{}_{}", safe_name, level_id);
    let image_path = format!("{}/{}.png", OUTPUT_DIR, base_name);
    let data_path = format!("{}/{}.txt", OUTPUT_DIR, base_name);

    println!(
        "Generating speed heatmap for {} ({}): {}x{} cells",
        level_name, level_id, GRID_WIDTH, GRID_HEIGHT
    );

    let img_width = GRID_WIDTH * CELL_SIZE;
    let img_height = GRID_HEIGHT * CELL_SIZE;
    let mut img = RgbImage::new(img_width, img_height);

    let bg_color = Rgb([230, 230, 230]);
    for pixel in img.pixels_mut() {
        *pixel = bg_color;
    }

    let total_cells = GRID_WIDTH * GRID_HEIGHT;
    let mut processed = 0;
    let mut data = String::from("x,y,value\n");

    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            let (world_x, world_y) = cell_world_coords(cx, cy);

            if let Some(traj) =
                calculate_shot_trajectory(world_x, world_y, basket_x, basket_y, BALL_GRAVITY)
            {
                let speed = traj.required_speed.clamp(SPEED_MIN, SPEED_MAX);
                let t = (speed - SPEED_MIN) / (SPEED_MAX - SPEED_MIN);

                let color = speed_to_color(t);
                fill_cell(&mut img, cx, cy, color);
                draw_arrow(&mut img, cx, cy, traj.angle, color);
                let _ = writeln!(&mut data, "{:.2},{:.2},{:.3}", world_x, world_y, t);
            } else {
                fill_cell(&mut img, cx, cy, Rgb([80, 80, 80]));
                let _ = writeln!(&mut data, "{:.2},{:.2},{:.3}", world_x, world_y, 0.0);
            }

            processed += 1;
            if processed % 500 == 0 {
                print!(
                    "\rProgress: {:.1}%",
                    (processed as f32 / total_cells as f32) * 100.0
                );
                std::io::stdout().flush().ok();
            }
        }
    }

    println!();
    draw_basket_marker(&mut img, basket_x, basket_y);
    draw_floor_line(&mut img);

    img.save(&image_path).expect("Failed to save image");
    if let Some(overlay) = overlay {
        write_level_overlay(&img, "speed", level_name, level_id, None, overlay);
    }
    fs::write(&data_path, data).expect("Failed to write heatmap data");
    println!("Saved {} and {}", image_path, data_path);
    image_path
}

fn generate_score_heatmap(
    level_name: &str,
    level_id: &str,
    side: &str,
    basket_x: f32,
    basket_y: f32,
    overlay: Option<&LevelOverlayContext<'_>>,
) -> String {
    let safe_name = sanitize_level_name(level_name);
    let base_name = format!("heatmap_score_{}_{}_{}", safe_name, level_id, side);
    let image_path = format!("{}/{}.png", OUTPUT_DIR, base_name);
    let data_path = format!("{}/{}.txt", OUTPUT_DIR, base_name);

    println!(
        "Generating score heatmap for {} ({}, {}): {}x{} cells",
        level_name, level_id, side, GRID_WIDTH, GRID_HEIGHT
    );

    // Create image (multiply by cell size for actual pixels)
    let img_width = GRID_WIDTH * CELL_SIZE;
    let img_height = GRID_HEIGHT * CELL_SIZE;
    let mut img = RgbImage::new(img_width, img_height);

    // Background color (light gray, like the game)
    let bg_color = Rgb([230, 230, 230]);
    for pixel in img.pixels_mut() {
        *pixel = bg_color;
    }

    let total_cells = GRID_WIDTH * GRID_HEIGHT;
    let mut processed = 0;
    let mut grid = HeatmapGrid::new();
    let mut values = Vec::with_capacity(total_cells as usize);
    let mut data = String::from("x,y,shot_pct\n");

    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            let (world_x, world_y) = cell_world_coords(cx, cy);
            let score_pct = simulate_scoring(world_x, world_y, basket_x, basket_y);
            grid.set(cx, cy, score_pct);
            values.push(score_pct);

            let shot_pct = score_pct * 100.0;
            let _ = writeln!(&mut data, "{:.2},{:.2},{:.2}", world_x, world_y, shot_pct);

            processed += 1;
            if processed % 100 == 0 {
                print!(
                    "\rProgress: {:.1}%",
                    (processed as f32 / total_cells as f32) * 100.0
                );
                std::io::stdout().flush().ok();
            }
        }
    }

    report_heatmap_stats("score", level_name, level_id, Some(side), &values);

    let (p_low, p_high) =
        percentile_bounds(&mut values, SCORE_PERCENTILE_LOW, SCORE_PERCENTILE_HIGH);
    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            let score_pct = grid.get(cx, cy);
            let color = if score_pct < SCORE_MASK_THRESHOLD {
                Rgb([80, 80, 80])
            } else {
                let t = if (p_high - p_low).abs() < f32::EPSILON {
                    score_pct
                } else {
                    ((score_pct - p_low) / (p_high - p_low)).clamp(0.0, 1.0)
                };
                score_to_color(t)
            };
            fill_cell(&mut img, cx, cy, color);
        }
    }

    println!(); // Newline after progress

    // Draw basket position marker
    draw_basket_marker(&mut img, basket_x, basket_y);

    // Draw floor line
    draw_floor_line(&mut img);

    img.save(&image_path).expect("Failed to save image");
    if let Some(overlay) = overlay {
        write_level_overlay(&img, "score", level_name, level_id, Some(side), overlay);
    }
    fs::write(&data_path, data).expect("Failed to write heatmap data");
    println!("Saved {} and {}", image_path, data_path);
    image_path
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

fn percentile_bounds(values: &mut [f32], low: f32, high: f32) -> (f32, f32) {
    if values.is_empty() {
        return (0.0, 1.0);
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let len = values.len() as f32;
    let low_idx = (len * low).floor().clamp(0.0, len - 1.0) as usize;
    let high_idx = (len * high).floor().clamp(0.0, len - 1.0) as usize;
    (values[low_idx], values[high_idx])
}

fn report_heatmap_stats(
    label: &str,
    level_name: &str,
    level_id: &str,
    side: Option<&str>,
    values: &[f32],
) {
    if values.is_empty() {
        return;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = sorted.len() as f32;
    let min = sorted.first().copied().unwrap_or(0.0);
    let max = sorted.last().copied().unwrap_or(0.0);
    let mean = values.iter().sum::<f32>() / len;
    let variance = values
        .iter()
        .map(|v| {
            let d = v - mean;
            d * d
        })
        .sum::<f32>()
        / len;
    let p10 = sorted[(len * 0.1).floor().clamp(0.0, len - 1.0) as usize];
    let p90 = sorted[(len * 0.9).floor().clamp(0.0, len - 1.0) as usize];

    let low = values.iter().filter(|v| **v < 0.2).count() as f32 / len;
    let mid = values.iter().filter(|v| **v >= 0.2 && **v < 0.5).count() as f32 / len;
    let high = values.iter().filter(|v| **v >= 0.5).count() as f32 / len;

    let side_label = side.map(|s| format!(" {}", s)).unwrap_or_default();
    let line = format!(
        "Heatmap stats [{}{}] {} ({}): min {:.3} max {:.3} mean {:.3} var {:.3} p10 {:.3} p90 {:.3} bands <0.2 {:.1}% 0.2-0.5 {:.1}% >=0.5 {:.1}%",
        label,
        side_label,
        level_name,
        level_id,
        min,
        max,
        mean,
        variance,
        p10,
        p90,
        low * 100.0,
        mid * 100.0,
        high * 100.0
    );
    println!("{}", line);
    write_heatmap_stats_line(&line);

    if p90 - p10 < 0.1 {
        let warning = format!(
            "Warning: low contrast heatmap [{}{}] for {} ({}): p90-p10 {:.3}",
            label,
            side_label,
            level_name,
            level_id,
            p90 - p10
        );
        println!("{}", warning);
        write_heatmap_stats_line(&warning);
    }
}

fn write_heatmap_stats_line(line: &str) {
    let mut header = stats_header_written().lock().expect("heatmap stats lock");
    let needs_header = !*header;

    let mut file = match fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(HEATMAP_STATS_FILE)
    {
        Ok(file) => file,
        Err(err) => {
            println!("Failed to write heatmap stats: {}", err);
            return;
        }
    };

    if needs_header {
        let _ = writeln!(file, "Heatmap stats log (generated by src/bin/heatmap.rs)");
        *header = true;
    }

    let _ = writeln!(file, "{}", line);
}

fn training_level_names() -> Vec<&'static str> {
    [TrainingProtocol::Pursuit, TrainingProtocol::Pursuit2]
        .into_iter()
        .filter_map(|protocol| protocol.fixed_level())
        .collect()
}

fn should_skip_level(level: &ballgame::LevelData, training_levels: &[&'static str]) -> bool {
    if level.debug || level.regression {
        return true;
    }

    training_levels
        .iter()
        .any(|name| level.name.eq_ignore_ascii_case(name))
}

fn level_matches_filter(level: &ballgame::LevelData, filters: &[String]) -> bool {
    filters.iter().any(|filter| {
        level.name.eq_ignore_ascii_case(filter) || level.id.eq_ignore_ascii_case(filter)
    })
}

fn combine_heatmaps(image_paths: &[String], output_path: &str) {
    if image_paths.is_empty() {
        return;
    }

    let mut images = Vec::with_capacity(image_paths.len());
    for path in image_paths {
        match image::open(path) {
            Ok(img) => images.push(img.to_rgb8()),
            Err(err) => {
                println!("Failed to open {}: {}", path, err);
                return;
            }
        }
    }

    let cell_w = images[0].width();
    let cell_h = images[0].height();

    let cols = 4u32;
    let rows = ((images.len() as u32 + cols - 1) / cols).max(1);

    let mut sheet = RgbImage::new(cell_w * cols, cell_h * rows);
    let bg = Rgb([20, 20, 20]);
    for pixel in sheet.pixels_mut() {
        *pixel = bg;
    }

    for (idx, img) in images.into_iter().enumerate() {
        let col = (idx as u32) % cols;
        let row = (idx as u32) / cols;
        let x0 = col * cell_w;
        let y0 = row * cell_h;

        for y in 0..cell_h {
            for x in 0..cell_w {
                let px = img.get_pixel(x, y);
                sheet.put_pixel(x0 + x, y0 + y, *px);
            }
        }
    }

    if let Err(err) = sheet.save(output_path) {
        println!("Failed to write combined heatmap {}: {}", output_path, err);
    } else {
        println!("Saved combined heatmap {}", output_path);
    }
}

fn write_level_overlay(
    base: &RgbImage,
    label: &str,
    level_name: &str,
    level_id: &str,
    side: Option<&str>,
    overlay: &LevelOverlayContext<'_>,
) {
    let safe_name = sanitize_level_name(level_name);
    let side_suffix = side.map(|s| format!("_{}", s)).unwrap_or_default();
    let overlay_dir = format!("{}/overlays", OUTPUT_DIR);
    let _ = fs::create_dir_all(&overlay_dir);
    let overlay_path = format!(
        "{}/heatmap_{}_{}_{}{}_overlay.png",
        overlay_dir, label, safe_name, level_id, side_suffix
    );

    let mut img = base.clone();

    draw_floor_line(&mut img);
    draw_platform_overlays(&mut img, overlay.platform_rects);
    draw_basket_marker_color(
        &mut img,
        overlay.basket_left_x,
        overlay.basket_y,
        Rgb([240, 210, 80]),
    );
    draw_basket_marker_color(
        &mut img,
        overlay.basket_right_x,
        overlay.basket_y,
        Rgb([240, 210, 80]),
    );

    if let Err(err) = img.save(&overlay_path) {
        println!("Failed to save overlay {}: {}", overlay_path, err);
    }
}

fn overlay_path(label: &str, level_name: &str, level_id: &str, side: Option<&str>) -> String {
    let safe_name = sanitize_level_name(level_name);
    let side_suffix = side.map_or(String::new(), |side| format!("_{}", side));
    format!(
        "{}/overlays/heatmap_{}_{}_{}{}_overlay.png",
        OUTPUT_DIR, label, safe_name, level_id, side_suffix
    )
}

fn draw_platform_overlays(img: &mut RgbImage, rects: &[PlatformRect]) {
    let color = Rgb([240, 240, 240]);
    for rect in rects {
        let left = (rect.left + ARENA_WIDTH / 2.0).round() as i32;
        let right = (rect.right + ARENA_WIDTH / 2.0).round() as i32;
        let top = (ARENA_HEIGHT / 2.0 - rect.top).round() as i32;
        let bottom = (ARENA_HEIGHT / 2.0 - rect.bottom).round() as i32;

        draw_rect_outline(img, left, top, right, bottom, color);
    }
}

fn draw_rect_outline(
    img: &mut RgbImage,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    color: Rgb<u8>,
) {
    let left = left.max(0);
    let top = top.max(0);
    let right = right.min(img.width() as i32 - 1);
    let bottom = bottom.min(img.height() as i32 - 1);

    for x in left..=right {
        img.put_pixel(x as u32, top as u32, color);
        img.put_pixel(x as u32, bottom as u32, color);
    }
    for y in top..=bottom {
        img.put_pixel(left as u32, y as u32, color);
        img.put_pixel(right as u32, y as u32, color);
    }
}

fn draw_basket_marker_color(img: &mut RgbImage, basket_x: f32, basket_y: f32, color: Rgb<u8>) {
    let img_x = ((basket_x + ARENA_WIDTH / 2.0) / CELL_SIZE as f32 * CELL_SIZE as f32) as u32;
    let img_y = ((ARENA_HEIGHT / 2.0 - basket_y) / CELL_SIZE as f32 * CELL_SIZE as f32) as u32;

    let size = CELL_SIZE as i32;
    for i in -size..=size {
        let px = (img_x as i32 + i) as u32;
        let py = img_y;
        if px < img.width() && py < img.height() {
            img.put_pixel(px, py, color);
        }
        let px = img_x;
        let py = (img_y as i32 + i) as u32;
        if px < img.width() && py < img.height() {
            img.put_pixel(px, py, color);
        }
    }
}
