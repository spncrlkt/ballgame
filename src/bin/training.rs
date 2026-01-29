//! Training Mode Binary
//!
//! Play 1v1 games against AI with comprehensive logging.
//! Default: 3 iterations, first point wins (goal mode).
//!
//! Usage:
//!   cargo run --bin training
//!   cargo run --bin training -- --iterations 5 --profile Aggressive

use ballgame::debug_logging::DebugLogConfig;
use ballgame::events::{
    BasketSnapshot, DebugSampleBuffer, EmitterConfig, EventEmitterState, SqliteEventLogger,
    emit_game_events, flush_debug_samples_to_sqlite, push_debug_samples, snapshot_ball,
    snapshot_player, tick_frame_from_time,
};
use ballgame::simulation::SimDatabase;
use ballgame::training::{
    LevelSelector, ReachabilityCollector, TrainingMode, TrainingPhase, TrainingProtocol,
    TrainingSettings, TrainingState, analyze_pursuit_session_from_db, analyze_session_from_db,
    ensure_session_dir, format_pursuit_analysis_markdown, generate_analysis_request,
    print_session_summary, write_analysis_files, write_session_summary,
};
use ballgame::ui::spawn_steal_indicators;
use ballgame::{
    AiCapabilities, AiGoal, AiNavState, AiProfileDatabase, AiState, Ball, BallPlayerContact,
    BallPulse, BallRolling, BallShotGrace, BallSpin, BallState, BallStyle, BallTextures,
    ChargeGaugeBackground, ChargeGaugeFill, ChargingShot, CoyoteTimer, CurrentLevel,
    CurrentPalette, DebugSettings, EventBuffer, EventBus, Facing, GameConfig, GameEvent, Grounded,
    HoldingBall, HumanControlTarget, HumanControlled, InputState, JumpState, LastShotInfo,
    LevelChangeTracker, LevelDatabase, MatchCountdown, NavGraph, PALETTES_FILE, PaletteDatabase,
    PhysicsTweaks, Player, PlayerId, PlayerInput, Score, SnapshotConfig, StealContest,
    StealCooldown, StealTracker, StyleTextures, TargetBasket, Team, TweakPanelState, Velocity, ai,
    ball, constants::*, countdown, emit_level_change_events, helpers::*, input, levels, player,
    scoring, shooting, spawn_countdown_text, steal, tuning, update_event_bus_time, world,
};
use bevy::{app::ScheduleRunnerPlugin, camera::ScalingMode, prelude::*};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;
use world::{Basket, BasketRim, Collider, CornerRamp, LevelPlatform, Platform};

/// Path to ball options file
const BALL_OPTIONS_FILE: &str = "config/ball_options.txt";

/// Parse ball_options.txt to get list of style names
fn load_ball_style_names() -> Vec<String> {
    let content = fs::read_to_string(BALL_OPTIONS_FILE).unwrap_or_else(|e| {
        warn!("Could not read ball options file: {}, using defaults", e);
        String::new()
    });

    let mut styles = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix("style:") {
            styles.push(name.trim().to_string());
        }
    }

    if styles.is_empty() {
        styles = vec!["wedges".to_string(), "half".to_string()];
    }

    styles
}

/// Speed multiplier for reachability training (4x faster exploration)
const REACHABILITY_SPEED_MULTIPLIER: f32 = 4.0;
/// Speed when one trigger is held (normal speed)
const REACHABILITY_SPEED_NORMAL: f32 = 1.0;
/// Speed when both triggers are held (slow motion)
const REACHABILITY_SPEED_SLOW: f32 = 0.5;
/// Trigger threshold for considering it "pressed" (analog value 0.0-1.0)
const TRIGGER_PRESS_THRESHOLD: f32 = 0.5;

#[derive(Resource, Clone)]
struct AllowedTrainingLevels(Option<Vec<String>>);

/// Marker component for shadow trail entities (reachability visualization)
#[derive(Component)]
struct ShadowTrail;

/// Resource tracking shadow trail state for reachability protocol
#[derive(Resource)]
struct ShadowTrailState {
    /// Last position where a shadow was spawned
    last_pos: Option<Vec2>,
    /// Whether shadow trail is enabled (only for Reachability protocol)
    enabled: bool,
}

impl Default for ShadowTrailState {
    fn default() -> Self {
        Self {
            last_pos: None,
            enabled: false,
        }
    }
}

/// Resource tracking automated walk/hop state for AutoReachability protocol
#[derive(Resource)]
struct AutoWalkState {
    /// Current movement direction (-1.0 = left, 1.0 = right)
    direction: f32,
    /// Time until next jump attempt
    jump_timer: f32,
    /// Time until direction change
    direction_timer: f32,
    /// Whether currently holding jump
    jump_held: bool,
    /// How long to hold jump for (varies for different jump heights)
    jump_hold_duration: f32,
    /// Time spent holding jump
    jump_hold_timer: f32,
    /// Whether the automation is enabled
    enabled: bool,
    /// RNG seed for reproducibility (optional)
    rng_counter: u32,
}

impl Default for AutoWalkState {
    fn default() -> Self {
        Self {
            direction: 1.0,
            jump_timer: 0.5,
            direction_timer: 2.0,
            jump_held: false,
            jump_hold_duration: 0.0,
            jump_hold_timer: 0.0,
            enabled: false,
            rng_counter: 0,
        }
    }
}

impl AutoWalkState {
    /// Simple pseudo-random number generator (deterministic)
    fn next_random(&mut self) -> f32 {
        // LCG parameters
        self.rng_counter = self.rng_counter.wrapping_mul(1103515245).wrapping_add(12345);
        ((self.rng_counter >> 16) & 0x7FFF) as f32 / 32768.0
    }

    /// Reset state for a new level
    fn reset_for_level(&mut self) {
        self.direction = if self.next_random() > 0.5 { 1.0 } else { -1.0 };
        self.jump_timer = 0.3 + self.next_random() * 0.5;
        self.direction_timer = 1.5 + self.next_random() * 2.0;
        self.jump_held = false;
        self.jump_hold_duration = 0.0;
        self.jump_hold_timer = 0.0;
    }
}

fn load_allowed_levels(settings: &TrainingSettings) -> Option<Vec<String>> {
    let Some(path) = &settings.offline_levels_file else {
        return None;
    };
    let path = Path::new(path);
    let Ok(content) = fs::read_to_string(path) else {
        warn!(
            "Failed to read offline levels file {}, ignoring",
            path.display()
        );
        return None;
    };
    let levels: Vec<String> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_lowercase())
        .collect();
    if levels.is_empty() {
        warn!("Offline levels file {} was empty, ignoring", path.display());
        None
    } else {
        Some(levels)
    }
}

fn level_allowed(
    level_name: &str,
    settings: &TrainingSettings,
    allowed_levels: Option<&[String]>,
) -> bool {
    let is_excluded = settings
        .exclude_levels
        .iter()
        .any(|exc| level_name.eq_ignore_ascii_case(exc));
    if is_excluded {
        return false;
    }
    match allowed_levels {
        Some(list) => list
            .iter()
            .any(|name| name.eq_ignore_ascii_case(level_name)),
        None => true,
    }
}

/// Create the SQLite event logger for training
fn create_sqlite_logger() -> (SqliteEventLogger, String) {
    // Ensure db directory exists
    std::fs::create_dir_all("db").ok();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let db_path_buf = format!("db/training_{}.db", timestamp);
    let db_path = std::path::Path::new(&db_path_buf);
    let latest_path = std::path::Path::new("db/training.db");
    if let Err(e) = std::fs::remove_file(latest_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            warn!("Failed to remove existing training.db symlink: {}", e);
        }
    }
    let link_target = std::env::current_dir()
        .map(|cwd| cwd.join(&db_path_buf))
        .unwrap_or_else(|_| db_path.to_path_buf());
    if let Err(e) = std::os::unix::fs::symlink(&link_target, latest_path) {
        warn!("Failed to update training.db symlink: {}", e);
    }
    match SqliteEventLogger::new(db_path, "training") {
        Ok(logger) => {
            info!("SQLite event logger initialized: {:?}", db_path);
            (logger, db_path_buf)
        }
        Err(e) => {
            warn!(
                "Failed to create SQLite logger ({}), using disabled logger",
                e
            );
            (SqliteEventLogger::disabled(), db_path_buf)
        }
    }
}

fn append_offline_db_path(db_path: &str) {
    let list_path = Path::new("offline_training/db_list.txt");
    if let Some(parent) = list_path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            warn!("Failed to create offline_training dir: {}", err);
            return;
        }
    }
    let existing = std::fs::read_to_string(list_path).unwrap_or_default();
    if existing
        .lines()
        .any(|line| line.trim().starts_with(db_path))
    {
        warn!("Offline DB list already contains {}", db_path);
        return;
    }
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("{}  # {}\n", db_path, timestamp);
    if let Err(err) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(list_path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()))
    {
        warn!("Failed to append offline DB path: {}", err);
    }
}

fn main() {
    let settings = TrainingSettings::from_args();
    let allowed_levels = load_allowed_levels(&settings);

    println!("========================================");
    println!("       TRAINING MODE");
    println!("========================================");
    println!();
    println!("  Protocol: {}", settings.protocol.display_name());
    let mode_str = match settings.mode {
        TrainingMode::Goal => "Goal-by-goal",
        TrainingMode::Game => "Full games",
    };
    println!("  Mode: {}", mode_str);
    println!("  Iterations: {}", settings.iterations);
    if settings.mode == TrainingMode::Game {
        println!("  Win Score: {}", settings.win_score);
    }
    println!("  AI Profile: {}", settings.ai_profile);
    if let Some(ref level) = settings.level {
        println!("  Level: {} (fixed)", level);
    } else {
        println!("  Level: random");
    }
    if let Some(ref style) = settings.ball_style {
        println!("  Ball Style: {}", style);
    } else {
        println!("  Ball Style: random");
    }
    if let Some(seed) = settings.seed {
        println!("  Seed: {} (deterministic)", seed);
    }
    if let Some(t) = settings.time_limit_secs {
        println!("  Time Limit: {}s", t);
    }
    if let Some(t) = settings.first_point_timeout_secs {
        println!("  First Point Timeout: {}s", t);
    }
    println!();
    println!("  Controls:");
    println!("    A/D or Left Stick: Move");
    println!("    Space/W or South: Jump");
    println!("    E or West: Pickup/Steal");
    println!("    F or RB: Throw (hold to charge)");
    println!("    P or Start: Pause/Resume");
    println!("    Escape: Quit training session");
    println!();
    match settings.mode {
        TrainingMode::Goal => println!("  Score a goal to complete each iteration."),
        TrainingMode::Game => println!("  First to {} points wins each game.", settings.win_score),
    }
    println!("========================================");
    println!();

    // Load level database from file
    let level_db = LevelDatabase::load_from_file(LEVELS_FILE);

    // Load palette database
    let palette_db = PaletteDatabase::load_or_create(PALETTES_FILE);

    // Get initial background color from selected palette
    let initial_bg = palette_db
        .get(settings.palette_index)
        .map(|p| p.background)
        .unwrap_or(DEFAULT_BACKGROUND_COLOR);

    // Create training state with settings
    let mut training_state = TrainingState::new(settings.iterations, &settings.ai_profile);
    training_state.protocol = settings.protocol;
    training_state.win_score = if settings.mode == TrainingMode::Game {
        settings.win_score
    } else {
        1 // Goal mode: end after first goal
    };
    training_state.time_limit_secs = settings.time_limit_secs;
    training_state.first_point_timeout_secs = settings.first_point_timeout_secs;

    // Pick level - either fixed from settings, sequential (Reachability), or random
    if settings.protocol.iterates_all_levels() {
        // Reachability protocol: iterate through all non-debug levels sequentially
        training_state.init_level_sequence(&level_db);

        // If -l flag provided, find that level in the sequence and start there
        if let Some(ref level_selector) = settings.level {
            let target_idx = match level_selector {
                LevelSelector::Number(n) => (*n as usize).saturating_sub(1),
                LevelSelector::Name(name) => level_db
                    .all()
                    .iter()
                    .position(|l| l.name.to_lowercase() == name.to_lowercase())
                    .unwrap_or(0),
            };

            // Find this level's position in the sequence
            if let Some(seq_pos) = training_state
                .level_sequence
                .iter()
                .position(|&i| i == target_idx)
            {
                training_state.level_sequence_index = seq_pos;
            }
        }

        if let Some(level_idx) = training_state.current_sequence_level() {
            if let Some(level_data) = level_db.get(level_idx) {
                training_state.current_level = (level_idx + 1) as u32;
                training_state.current_level_name = level_data.name.clone();
                // Initialize reachability collector for this level (with preloaded data if available)
                training_state.reachability_collector = Some(ReachabilityCollector::new_with_preload(
                    level_data.id.clone(),
                    level_data.name.clone(),
                ));
            }
        }
    } else if let Some(ref level_selector) = settings.level {
        // Resolve level selector to number
        let fixed_level = match level_selector {
            LevelSelector::Number(n) => *n,
            LevelSelector::Name(name) => {
                // Find level by name (case-insensitive)
                (0..level_db.len())
                    .find(|&i| {
                        level_db
                            .get(i)
                            .map(|l| l.name.to_lowercase() == name.to_lowercase())
                            .unwrap_or(false)
                    })
                    .map(|i| (i + 1) as u32)
                    .unwrap_or_else(|| {
                        eprintln!("Warning: Level '{}' not found, using level 3", name);
                        3
                    })
            }
        };
        training_state.current_level = fixed_level;
        training_state.current_level_name = level_db
            .get((fixed_level - 1) as usize)
            .map(|l| l.name.clone())
            .unwrap_or_else(|| format!("Level {}", fixed_level));
    } else {
        // Filter out debug levels and excluded levels
        let training_levels: Vec<u32> = (0..level_db.len())
            .filter(|&i| {
                let level = level_db.get(i);
                let is_debug = level.map(|l| l.debug).unwrap_or(true);
                let is_regression = level.map(|l| l.regression).unwrap_or(true);
                let level_name = level.map(|l| l.name.clone()).unwrap_or_default();
                let allowed = level_allowed(&level_name, &settings, allowed_levels.as_deref());
                !is_debug && !is_regression && allowed
            })
            .map(|i| (i + 1) as u32)
            .collect();

        if let Some(&level) = training_levels.choose(&mut rand::thread_rng()) {
            training_state.current_level = level;
            training_state.current_level_name = level_db
                .get((level - 1) as usize)
                .map(|l| l.name.clone())
                .unwrap_or_else(|| format!("Level {}", level));
        }
    }

    // Ensure session directory exists
    if let Err(e) = ensure_session_dir(&training_state) {
        eprintln!("Failed to create session directory: {}", e);
        return;
    }

    if settings.protocol.iterates_all_levels() {
        println!(
            "Reachability exploration: {}",
            training_state.current_level_name
        );
        println!("  Explore the level, press LB/Q when done");
    } else {
        println!(
            "Starting iteration 1/{} on {}",
            settings.iterations, training_state.current_level_name
        );
    }
    println!();

    // Viewport setup from settings
    let viewport_index = settings.viewport_index.min(VIEWPORT_PRESETS.len() - 1);
    let (viewport_width, viewport_height, _) = VIEWPORT_PRESETS[viewport_index];

    let args: Vec<String> = std::env::args().collect();
    let debug_config = DebugLogConfig::load_with_args(&args);

    let (sqlite_logger, db_path_buf) = create_sqlite_logger();
    if settings.offline_levels_file.is_some() {
        append_offline_db_path(&db_path_buf);
    }

    let is_headless = settings.headless;

    let mut app = App::new();

    // Add plugins based on headless mode
    if is_headless {
        // Headless mode: minimal plugins for simulation
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f32(1.0 / 60.0),
        )));
        app.add_plugins(bevy::transform::TransformPlugin);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.add_plugins(bevy::prelude::ImagePlugin::default());
        app.add_plugins(bevy::input::InputPlugin);
        app.add_plugins(bevy::state::app::StatesPlugin);
        // Set fixed timestep for physics
        app.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f32(
            1.0 / 60.0,
        )));
    } else {
        // Windowed mode: full plugins
        app.add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: bevy::window::WindowResolution::new(
                        viewport_width as u32,
                        viewport_height as u32,
                    )
                    .with_scale_factor_override(1.0),
                    title: "Ballgame - Training Mode".into(),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
        );
        app.insert_resource(ClearColor(initial_bg));
    }

    app
        .insert_resource(palette_db)
        .insert_resource(level_db)
        .insert_resource(settings)
        .insert_resource(AllowedTrainingLevels(allowed_levels))
        .insert_resource(training_state)
        .init_resource::<ShadowTrailState>()
        .init_resource::<AutoWalkState>()
        .init_resource::<PlayerInput>()
        .init_resource::<TweakPanelState>()
        .init_resource::<DebugSettings>()
        .init_resource::<StealContest>()
        .init_resource::<StealTracker>()
        .init_resource::<Score>()
        .insert_resource(CurrentLevel(String::new())) // Will be set from training state
        .insert_resource(CurrentPalette(0))
        .insert_resource({
            let mut tweaks = PhysicsTweaks::default();
            let _ = tuning::apply_global_tuning(&mut tweaks);
            tweaks
        })
        .init_resource::<LastShotInfo>()
        .init_resource::<AiProfileDatabase>()
        .init_resource::<NavGraph>()
        .init_resource::<AiCapabilities>()
        .init_resource::<ai::HeatmapBundle>()
        .insert_resource(SnapshotConfig::default())
        .init_resource::<TrainingEventBuffer>()
        .init_resource::<MatchCountdown>()
        // Event bus resources
        .insert_resource(EventBus::new())
        .insert_resource(HumanControlTarget(Some(PlayerId::L))) // Left player is human
        .init_resource::<LevelChangeTracker>()
        .insert_resource(debug_config)
        .init_resource::<DebugSampleBuffer>()
        // SQLite event logger - central hub for event storage
        .insert_resource(sqlite_logger)
        // Startup systems
        .add_systems(Startup, (training_setup, setup_reachability_time_scale).chain())
        // Event bus time update (runs every frame for timestamping)
        .add_systems(Update, update_event_bus_time)
        .add_systems(Update, flush_debug_samples_to_sqlite)
        // Dynamic speed control for reachability mode (trigger-based)
        .add_systems(Update, update_reachability_time_scale)
        // Input systems chain - paused when game is paused
        .add_systems(
            Update,
            (
                input::capture_input,
                ai::copy_human_input,
                auto_walk_and_hop,
                ai::mark_nav_dirty_on_level_change,
                ai::load_heatmaps_on_level_change,
                ai::rebuild_nav_graph,
                ai::ai_navigation_update,
                ai::ai_decision_update,
            )
                .chain()
                .run_if(not_paused),
        )
        // Core Update systems - split to avoid tuple issues
        // Note: respawn_player is NOT used in training mode - we have our own setup
        // and restart logic via check_pause_restart
        // Note: steal_cooldown_update is only in FixedUpdate (not here) to avoid double-ticking
        // Level change event emission
        .add_systems(Update, emit_level_change_events)
        .add_systems(
            Update,
            (
                ballgame::ui::animate_pickable_ball,
                ballgame::ui::update_charge_gauge,
                ballgame::ui::update_steal_indicators,
            ),
        )
        // Countdown system
        .add_systems(Update, countdown::update_countdown)
        // Training-specific systems
        .add_systems(
            Update,
            (
                check_advance_level,
                emit_training_events,
                training_state_machine,
                update_training_hud,
                flush_training_events_to_sqlite,
                check_escape_quit,
                check_pause_restart,
            ),
        )
        // Fixed update physics chain - only runs when countdown is finished
        .add_systems(
            FixedUpdate,
            (
                player::apply_input,
                player::apply_gravity,
                ball::ball_gravity,
                ball::ball_spin,
                ball::apply_velocity,
                player::check_collisions,
                ball::ball_collisions,
                ball::ball_state_update,
                ball::ball_player_collision,
                ball::ball_follow_holder,
                ball::pickup_ball,
                steal::steal_cooldown_update,
                shooting::update_shot_charge,
                shooting::throw_ball,
                scoring::check_scoring,
                give_ball_to_human,
                collect_training_debug_samples,
                collect_reachability_positions,
                spawn_shadow_trail,
            )
                .chain()
                .run_if(countdown::not_in_countdown)
                .run_if(not_paused),
        )
        // Level change handling
        .add_systems(
            Update,
            (
                reload_level_geometry_on_change,
                clear_shadow_trail_on_level_change,
            ),
        )
        .run();
}

/// Run condition: game is not paused
fn not_paused(training_state: Res<TrainingState>) -> bool {
    training_state.phase != TrainingPhase::Paused
}

fn collect_training_debug_samples(
    debug_config: Res<DebugLogConfig>,
    training_state: Res<TrainingState>,
    current_level: Res<CurrentLevel>,
    mut buffer: ResMut<DebugSampleBuffer>,
    players: Query<
        (
            &Team,
            &Transform,
            &Velocity,
            &InputState,
            &Grounded,
            &JumpState,
            &CoyoteTimer,
            &Facing,
            Option<&AiNavState>,
            Option<&HumanControlled>,
        ),
        With<Player>,
    >,
) {
    if !debug_config.enabled || training_state.phase != TrainingPhase::Playing {
        return;
    }
    let time_ms = (training_state.game_elapsed * 1000.0) as u32;
    let tick_frame = tick_frame_from_time(time_ms);
    push_debug_samples(&mut buffer, time_ms, tick_frame, &current_level.0, &players);
}

/// Collect human player positions for reachability heatmap (Reachability protocol only)
fn collect_reachability_positions(
    mut training_state: ResMut<TrainingState>,
    players: Query<(&Transform, Option<&HumanControlled>), With<Player>>,
) {
    // Only collect during Playing phase for Reachability protocol
    if training_state.phase != TrainingPhase::Playing
        || !training_state.protocol.iterates_all_levels()
    {
        return;
    }

    // Collect human player position
    if let Some(ref mut collector) = training_state.reachability_collector {
        for (transform, human) in &players {
            if human.is_some() {
                let pos = transform.translation;
                collector.positions.push((pos.x, pos.y));
            }
        }
    }
}

/// Spawn shadow trail markers behind the human player (Reachability protocol only)
/// Creates persistent visual markers showing areas the player has visited
fn spawn_shadow_trail(
    mut commands: Commands,
    mut shadow_state: ResMut<ShadowTrailState>,
    training_state: Res<TrainingState>,
    players: Query<(&Transform, Option<&HumanControlled>, &Team), With<Player>>,
    palette_db: Res<PaletteDatabase>,
    current_palette: Res<CurrentPalette>,
) {
    // Only spawn during Playing phase for Reachability protocol
    if !shadow_state.enabled
        || training_state.phase != TrainingPhase::Playing
        || !training_state.protocol.iterates_all_levels()
    {
        return;
    }

    // Find human player position
    for (transform, human, team) in &players {
        if human.is_none() {
            continue;
        }

        let pos = Vec2::new(transform.translation.x, transform.translation.y);

        // Check if we've moved far enough from last shadow
        let should_spawn = match shadow_state.last_pos {
            Some(last) => pos.distance(last) >= SHADOW_TRAIL_MIN_DISTANCE,
            None => true,
        };

        if should_spawn {
            // Get player color and create complementary shadow color
            let palette = palette_db.get(current_palette.0).unwrap_or_else(|| {
                palette_db.get(0).expect("No palettes loaded")
            });

            // Use team color as base
            let player_color = if *team == Team::Left {
                palette.left
            } else {
                palette.right
            };

            // Create complementary color: shift hue by 180°, make it light
            let shadow_color = complementary_light_color(player_color, SHADOW_TRAIL_LIGHTNESS, SHADOW_TRAIL_ALPHA);

            // Spawn shadow sprite
            commands.spawn((
                Sprite::from_color(shadow_color, SHADOW_TRAIL_SIZE),
                Transform::from_xyz(pos.x, pos.y, SHADOW_TRAIL_Z),
                ShadowTrail,
            ));

            shadow_state.last_pos = Some(pos);
        }
    }
}

/// Clear shadow trail when level changes and spawn preloaded shadows
fn clear_shadow_trail_on_level_change(
    mut commands: Commands,
    mut shadow_state: ResMut<ShadowTrailState>,
    training_state: Res<TrainingState>,
    shadows: Query<Entity, With<ShadowTrail>>,
    players: Query<&Team, (With<Player>, With<HumanControlled>)>,
    palette_db: Res<PaletteDatabase>,
    current_palette: Res<CurrentPalette>,
    mut last_level: Local<String>,
) {
    // Detect level change
    if training_state.current_level_name != *last_level {
        // Clear all shadow trail entities
        for entity in &shadows {
            commands.entity(entity).despawn();
        }

        // Reset shadow state
        shadow_state.last_pos = None;

        // Enable shadow trail for Reachability protocol
        shadow_state.enabled = training_state.protocol.iterates_all_levels();

        // Spawn shadows for preloaded positions if this is reachability mode
        if shadow_state.enabled {
            if let Some(ref collector) = training_state.reachability_collector {
                if !collector.preloaded_positions.is_empty() {
                    // Get shadow color from human player's team
                    let team = players.iter().next().copied().unwrap_or(Team::Left);
                    let palette = palette_db.get(current_palette.0).unwrap_or_else(|| {
                        palette_db.get(0).expect("No palettes loaded")
                    });
                    let player_color = if team == Team::Left {
                        palette.left
                    } else {
                        palette.right
                    };
                    let shadow_color = complementary_light_color(
                        player_color,
                        SHADOW_TRAIL_LIGHTNESS,
                        SHADOW_TRAIL_ALPHA,
                    );

                    // Spawn shadows for all preloaded positions
                    for &(x, y) in &collector.preloaded_positions {
                        commands.spawn((
                            Sprite::from_color(shadow_color, SHADOW_TRAIL_SIZE),
                            Transform::from_xyz(x, y, SHADOW_TRAIL_Z),
                            ShadowTrail,
                        ));
                    }

                    info!(
                        "Spawned {} preloaded shadows for {}",
                        collector.preloaded_positions.len(),
                        collector.level_name
                    );
                }
            }
        }

        *last_level = training_state.current_level_name.clone();
    }
}

/// Reload level geometry (platforms + corner ramps) when level changes
/// This is needed because training mode doesn't use the main game's respawn_player system
fn reload_level_geometry_on_change(
    mut commands: Commands,
    current_level: Res<CurrentLevel>,
    level_db: Res<LevelDatabase>,
    palette_db: Res<PaletteDatabase>,
    current_palette: Res<CurrentPalette>,
    level_platforms: Query<Entity, With<LevelPlatform>>,
    corner_ramps: Query<Entity, With<CornerRamp>>,
    mut baskets: Query<&mut Transform, (With<Basket>, Without<Player>, Without<Ball>)>,
    mut last_level_id: Local<String>,
) {
    // Detect level change by comparing level IDs
    if current_level.0 == *last_level_id {
        return;
    }

    // Skip on first frame (initial setup handles this)
    if last_level_id.is_empty() {
        *last_level_id = current_level.0.clone();
        return;
    }

    info!("Reloading level geometry for: {}", current_level.0);

    // Get palette for platform colors
    let palette = palette_db
        .get(current_palette.0)
        .unwrap_or_else(|| palette_db.get(0).expect("No palettes loaded"));

    // Reload level geometry (despawns old, spawns new)
    if let Some((left_x, right_x, basket_y)) = levels::reload_level_geometry(
        &mut commands,
        &level_db,
        &current_level.0,
        palette.platforms,
        level_platforms.iter(),
        corner_ramps.iter(),
    ) {
        // Update basket positions
        for mut basket_transform in &mut baskets {
            if basket_transform.translation.x < 0.0 {
                basket_transform.translation.x = left_x;
            } else {
                basket_transform.translation.x = right_x;
            }
            basket_transform.translation.y = basket_y;
        }
    }

    *last_level_id = current_level.0.clone();
}

/// Automated random walk and hop system for AutoReachability protocol
/// Generates movement and jump inputs to explore all reachable areas of a level
fn auto_walk_and_hop(
    time: Res<Time>,
    training_state: Res<TrainingState>,
    mut auto_state: ResMut<AutoWalkState>,
    mut players: Query<
        (&Transform, &Grounded, &mut InputState),
        (With<Player>, With<HumanControlled>),
    >,
    mut last_level: Local<String>,
) {
    // Only active for AutoReachability protocol during Playing phase
    if !training_state.protocol.is_automated() {
        auto_state.enabled = false;
        return;
    }

    if training_state.phase != TrainingPhase::Playing {
        return;
    }

    // Reset state on level change
    if training_state.current_level_name != *last_level {
        auto_state.reset_for_level();
        auto_state.enabled = true;
        *last_level = training_state.current_level_name.clone();
    }

    if !auto_state.enabled {
        return;
    }

    let dt = time.delta_secs();

    // Update timers
    auto_state.jump_timer -= dt;
    auto_state.direction_timer -= dt;

    // Change direction periodically or randomly
    if auto_state.direction_timer <= 0.0 {
        auto_state.direction = -auto_state.direction;
        // Random duration between 1.0 and 4.0 seconds
        auto_state.direction_timer = 1.0 + auto_state.next_random() * 3.0;
    }

    // Handle jump hold timing
    if auto_state.jump_held {
        auto_state.jump_hold_timer += dt;
        if auto_state.jump_hold_timer >= auto_state.jump_hold_duration {
            auto_state.jump_held = false;
            auto_state.jump_hold_timer = 0.0;
        }
    }

    // Apply inputs to human-controlled player
    for (transform, grounded, mut input_state) in &mut players {
        // Always move in current direction
        input_state.move_x = auto_state.direction;

        // Check if near arena walls and reverse direction
        let wall_margin = ARENA_WIDTH / 2.0 - WALL_THICKNESS - 50.0;
        if transform.translation.x > wall_margin && auto_state.direction > 0.0 {
            auto_state.direction = -1.0;
            auto_state.direction_timer = 1.0 + auto_state.next_random() * 2.0;
        } else if transform.translation.x < -wall_margin && auto_state.direction < 0.0 {
            auto_state.direction = 1.0;
            auto_state.direction_timer = 1.0 + auto_state.next_random() * 2.0;
        }

        // Jump logic - only when grounded and timer expired
        if grounded.0 && auto_state.jump_timer <= 0.0 && !auto_state.jump_held {
            // Start a jump with random hold duration (0.0 = tap, up to 0.3 = full height)
            auto_state.jump_held = true;
            auto_state.jump_hold_duration = auto_state.next_random() * 0.3;
            auto_state.jump_hold_timer = 0.0;

            // Set jump buffer timer to trigger jump
            input_state.jump_buffer_timer = 0.1;

            // Random interval between jumps (0.3 to 1.5 seconds)
            auto_state.jump_timer = 0.3 + auto_state.next_random() * 1.2;

            // Occasionally change direction after landing
            if auto_state.next_random() > 0.7 {
                auto_state.direction = -auto_state.direction;
            }
        }

        // Hold jump if in jump hold phase
        input_state.jump_held = auto_state.jump_held;
    }
}

/// Give the ball to the human player (left team) after scoring
/// This runs after check_scoring to override the default ball reset behavior
fn give_ball_to_human(
    mut commands: Commands,
    mut balls: Query<(Entity, &mut Transform, &mut BallState), With<Ball>>,
    players: Query<(Entity, &Transform, &Team), (With<Player>, Without<Ball>)>,
    training_settings: Res<TrainingSettings>,
) {
    if !training_settings.drive_mode {
        return;
    }
    for (ball_entity, mut ball_transform, mut ball_state) in &mut balls {
        // Only act if ball is free (just reset after a score)
        if !matches!(*ball_state, BallState::Free) {
            continue;
        }

        // Find the human player (left team)
        for (player_entity, player_transform, team) in &players {
            if *team == Team::Left {
                // Give ball to human player - keep ball's z for proper rendering
                ball_transform.translation.x = player_transform.translation.x;
                ball_transform.translation.y = player_transform.translation.y;
                *ball_state = BallState::Held(player_entity);
                commands
                    .entity(player_entity)
                    .insert(HoldingBall(ball_entity));
                break;
            }
        }
    }
}

/// Event buffer for training mode logging
#[derive(Resource)]
pub struct TrainingEventBuffer {
    pub buffer: EventBuffer,
    /// Shared emitter state for detecting changes
    pub emitter_state: EventEmitterState,
    /// Track elapsed time
    pub elapsed: f32,
}

impl Default for TrainingEventBuffer {
    fn default() -> Self {
        Self {
            buffer: EventBuffer::default(),
            emitter_state: EventEmitterState::with_config(EmitterConfig {
                // Training only tracks right player (AI opponent)
                track_both_ai_goals: false,
            }),
            elapsed: 0.0,
        }
    }
}

/// HUD text marker
#[derive(Component)]
pub struct TrainingHudText;

/// Setup the training game world
fn training_setup(
    mut commands: Commands,
    level_db: Res<LevelDatabase>,
    palette_db: Res<PaletteDatabase>,
    asset_server: Res<AssetServer>,
    profile_db: Res<AiProfileDatabase>,
    mut training_state: ResMut<TrainingState>,
    training_settings: Res<TrainingSettings>,
    mut current_level: ResMut<CurrentLevel>,
    mut event_buffer: ResMut<TrainingEventBuffer>,
    sqlite_logger: Res<SqliteEventLogger>,
) {
    // Set current level from training state (convert level number to level ID)
    let level_id = level_db
        .all()
        .get((training_state.current_level as usize).saturating_sub(1))
        .map(|l| l.id.clone())
        .unwrap_or_else(|| {
            level_db
                .all()
                .first()
                .map(|l| l.id.clone())
                .unwrap_or_default()
        });
    current_level.0 = level_id;

    // Camera
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 0.0, 0.0),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: ARENA_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Get palette
    let initial_palette = palette_db.get(0).expect("No palettes loaded");

    // Get level ID from training state
    let level_id = level_db
        .all()
        .get((training_state.current_level as usize).saturating_sub(1))
        .map(|l| l.id.clone())
        .unwrap_or_else(|| {
            level_db
                .all()
                .first()
                .map(|l| l.id.clone())
                .unwrap_or_default()
        });

    // Find AI profile ID
    let ai_profile_id = profile_db
        .get_by_name(&training_state.ai_profile)
        .map(|p| p.id.clone())
        .unwrap_or_else(|| profile_db.default_profile().id.clone());

    // Left player - HUMAN controlled
    let left_player = commands
        .spawn((
            Sprite::from_color(initial_palette.left, PLAYER_SIZE),
            Transform::from_translation(PLAYER_SPAWN_LEFT),
            Player,
            Velocity::default(),
            Grounded(false),
            CoyoteTimer::default(),
            JumpState::default(),
            Facing::default(),
        ))
        .insert((
            ChargingShot::default(),
            TargetBasket(Basket::Right),
            Collider,
            Team::Left,
            InputState::default(),
            AiState {
                current_goal: AiGoal::Idle, // Not used, human controlled
                profile_id: profile_db.default_profile().id.clone(),
                ..default()
            },
            AiNavState::default(),
            StealCooldown::default(),
            HumanControlled, // Mark as human controlled
        ))
        .id();

    // Right player - AI controlled (or Idle for solo mode)
    let ai_initial_goal = if training_settings.protocol.is_solo_mode() {
        AiGoal::Idle
    } else {
        AiGoal::ChaseBall
    };

    // Position AI off-screen in solo mode (still exists for entity queries)
    let right_spawn = if training_settings.protocol.is_solo_mode() {
        Vec3::new(ARENA_WIDTH + 500.0, 0.0, 0.0) // Off-screen right
    } else {
        PLAYER_SPAWN_RIGHT
    };

    let right_player = commands
        .spawn((
            Sprite::from_color(initial_palette.right, PLAYER_SIZE),
            Transform::from_translation(right_spawn),
            Player,
            Velocity::default(),
            Grounded(false),
            CoyoteTimer::default(),
            JumpState::default(),
            Facing(-1.0),
        ))
        .insert((
            ChargingShot::default(),
            TargetBasket(Basket::Left),
            Collider,
            Team::Right,
            InputState::default(),
            AiState {
                current_goal: ai_initial_goal,
                profile_id: ai_profile_id.clone(),
                ..default()
            },
            AiNavState::default(),
            StealCooldown::default(),
        ))
        .id();

    // Charge gauges for left player
    let gauge_x = -PLAYER_SIZE.x / 4.0;
    let gauge_bg = commands
        .spawn((
            Sprite::from_color(
                Color::BLACK,
                Vec2::new(CHARGE_GAUGE_WIDTH, CHARGE_GAUGE_HEIGHT),
            ),
            Transform::from_xyz(gauge_x, 0.0, 0.5),
            ChargeGaugeBackground,
        ))
        .id();
    commands.entity(left_player).add_child(gauge_bg);

    let gauge_fill = commands
        .spawn((
            Sprite::from_color(
                Color::srgb(0.0, 0.8, 0.0),
                Vec2::new(CHARGE_GAUGE_WIDTH - 2.0, CHARGE_GAUGE_HEIGHT - 2.0),
            ),
            Transform::from_xyz(gauge_x, 0.0, 0.6).with_scale(Vec3::new(1.0, 0.0, 1.0)),
            ChargeGaugeFill,
        ))
        .id();
    commands.entity(left_player).add_child(gauge_fill);

    // Charge gauge for right player
    let right_gauge_x = PLAYER_SIZE.x / 4.0;
    let right_gauge_bg = commands
        .spawn((
            Sprite::from_color(
                Color::BLACK,
                Vec2::new(CHARGE_GAUGE_WIDTH, CHARGE_GAUGE_HEIGHT),
            ),
            Transform::from_xyz(right_gauge_x, 0.0, 0.5),
            ChargeGaugeBackground,
        ))
        .id();
    commands.entity(right_player).add_child(right_gauge_bg);

    let right_gauge_fill = commands
        .spawn((
            Sprite::from_color(
                Color::srgb(0.0, 0.8, 0.0),
                Vec2::new(CHARGE_GAUGE_WIDTH - 2.0, CHARGE_GAUGE_HEIGHT - 2.0),
            ),
            Transform::from_xyz(right_gauge_x, 0.0, 0.6).with_scale(Vec3::new(1.0, 0.0, 1.0)),
            ChargeGaugeFill,
        ))
        .id();
    commands.entity(right_player).add_child(right_gauge_fill);

    // Steal indicators
    spawn_steal_indicators(&mut commands, left_player, 1.0);
    spawn_steal_indicators(&mut commands, right_player, -1.0);

    // Load ball textures
    let style_names = load_ball_style_names();
    let num_palettes = palette_db.len();
    let mut styles_map = HashMap::new();
    for style_name in &style_names {
        let textures = StyleTextures {
            textures: (0..num_palettes)
                .map(|i| asset_server.load(format!("textures/balls/ball_{}_{}.png", style_name, i)))
                .collect(),
        };
        styles_map.insert(style_name.clone(), textures);
    }

    let ball_textures = BallTextures {
        styles: styles_map,
        style_order: style_names.clone(),
    };
    commands.insert_resource(ball_textures.clone());

    // Spawn ball - use settings or random
    let ball_style_name = if let Some(ref style) = training_settings.ball_style {
        style.clone()
    } else {
        // Random style from available options
        style_names
            .choose(&mut rand::thread_rng())
            .cloned()
            .unwrap_or_else(|| "wedges".to_string())
    };
    if let Some(textures) = ball_textures.get(&ball_style_name) {
        let (ball_spawn_pos, ball_state) = if training_settings.drive_mode {
            (
                Vec3::new(PLAYER_SPAWN_LEFT.x, PLAYER_SPAWN_LEFT.y, BALL_SPAWN.z),
                BallState::Held(left_player),
            )
        } else {
            (BALL_SPAWN, BallState::Free)
        };

        let ball_entity = commands
            .spawn((
                Sprite {
                    image: textures.textures[0].clone(),
                    custom_size: Some(BALL_SIZE),
                    ..default()
                },
                Transform::from_translation(ball_spawn_pos),
                Ball,
                ball_state,
                Velocity::default(),
                BallPlayerContact::default(),
                BallPulse::default(),
                BallRolling::default(),
                BallShotGrace::default(),
                BallSpin::default(),
                BallStyle::new(&ball_style_name),
            ))
            .id();

        if training_settings.drive_mode {
            // Give the human player the ball
            commands
                .entity(left_player)
                .insert(HoldingBall(ball_entity));
        }
    }

    // Arena floor
    commands.spawn((
        Sprite::from_color(
            initial_palette.platforms,
            Vec2::new(ARENA_WIDTH - WALL_THICKNESS * 2.0, 40.0),
        ),
        Transform::from_xyz(0.0, ARENA_FLOOR_Y, 0.0),
        Platform,
    ));

    // Walls
    commands.spawn((
        Sprite::from_color(initial_palette.platforms, Vec2::new(WALL_THICKNESS, 5000.0)),
        Transform::from_xyz(-ARENA_WIDTH / 2.0 + WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
    ));
    commands.spawn((
        Sprite::from_color(initial_palette.platforms, Vec2::new(WALL_THICKNESS, 5000.0)),
        Transform::from_xyz(ARENA_WIDTH / 2.0 - WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
    ));

    // Level platforms
    levels::spawn_level_platforms(
        &mut commands,
        &level_db,
        &level_id,
        initial_palette.platforms,
    );

    // Baskets
    let initial_level = level_db.get_by_id(&level_id);
    let basket_y = initial_level
        .map(|l| ARENA_FLOOR_Y + l.basket_height)
        .unwrap_or(ARENA_FLOOR_Y + 400.0);
    let basket_push_in = initial_level
        .map(|l| l.basket_push_in)
        .unwrap_or(BASKET_PUSH_IN);
    let (left_basket_x, right_basket_x) = basket_x_from_offset(basket_push_in);

    let rim_outer_height = BASKET_SIZE.y * 0.5;
    let rim_inner_height = BASKET_SIZE.y * 0.1;
    let rim_outer_y = -BASKET_SIZE.y / 2.0 + rim_outer_height / 2.0;
    let rim_inner_y = -BASKET_SIZE.y / 2.0 + rim_inner_height / 2.0;
    let rim_bottom_width = BASKET_SIZE.x + RIM_THICKNESS;

    // Left basket
    commands
        .spawn((
            Sprite::from_color(initial_palette.left, BASKET_SIZE),
            Transform::from_xyz(left_basket_x, basket_y, -0.1),
            Basket::Left,
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite::from_color(
                    initial_palette.right_rim,
                    Vec2::new(RIM_THICKNESS, rim_outer_height),
                ),
                Transform::from_xyz(-BASKET_SIZE.x / 2.0, rim_outer_y, 0.1),
                Platform,
                BasketRim,
            ));
            parent.spawn((
                Sprite::from_color(
                    initial_palette.right_rim,
                    Vec2::new(RIM_THICKNESS, rim_inner_height),
                ),
                Transform::from_xyz(BASKET_SIZE.x / 2.0, rim_inner_y, 0.1),
                Platform,
                BasketRim,
            ));
            parent.spawn((
                Sprite::from_color(
                    initial_palette.right_rim,
                    Vec2::new(rim_bottom_width, RIM_THICKNESS),
                ),
                Transform::from_xyz(0.0, -BASKET_SIZE.y / 2.0, 0.1),
                Platform,
                BasketRim,
            ));
        });

    // Right basket
    commands
        .spawn((
            Sprite::from_color(initial_palette.right, BASKET_SIZE),
            Transform::from_xyz(right_basket_x, basket_y, -0.1),
            Basket::Right,
        ))
        .with_children(|parent| {
            parent.spawn((
                Sprite::from_color(
                    initial_palette.left_rim,
                    Vec2::new(RIM_THICKNESS, rim_inner_height),
                ),
                Transform::from_xyz(-BASKET_SIZE.x / 2.0, rim_inner_y, 0.1),
                Platform,
                BasketRim,
            ));
            parent.spawn((
                Sprite::from_color(
                    initial_palette.left_rim,
                    Vec2::new(RIM_THICKNESS, rim_outer_height),
                ),
                Transform::from_xyz(BASKET_SIZE.x / 2.0, rim_outer_y, 0.1),
                Platform,
                BasketRim,
            ));
            parent.spawn((
                Sprite::from_color(
                    initial_palette.left_rim,
                    Vec2::new(rim_bottom_width, RIM_THICKNESS),
                ),
                Transform::from_xyz(0.0, -BASKET_SIZE.y / 2.0, 0.1),
                Platform,
                BasketRim,
            ));
        });

    // Corner ramps
    let initial_step_count = initial_level
        .map(|l| l.step_count)
        .unwrap_or(CORNER_STEP_COUNT);
    let initial_corner_height = initial_level
        .map(|l| l.corner_height)
        .unwrap_or(CORNER_STEP_TOTAL_HEIGHT);
    let initial_corner_width = initial_level
        .map(|l| l.corner_width)
        .unwrap_or(CORNER_STEP_TOTAL_WIDTH);
    let initial_step_push_in = initial_level
        .map(|l| l.step_push_in)
        .unwrap_or(STEP_PUSH_IN);
    levels::spawn_corner_ramps(
        &mut commands,
        initial_step_count,
        initial_corner_height,
        initial_corner_width,
        initial_step_push_in,
        initial_palette.platforms,
    );

    // Training HUD
    commands.spawn((
        Text2d::new(format!(
            "Game {}/{} | {} | You 0 - 0 AI",
            training_state.game_number,
            training_state.games_total,
            training_state.current_level_name
        )),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(TEXT_PRIMARY),
        Transform::from_xyz(0.0, ARENA_HEIGHT / 2.0 - 30.0, 1.0),
        TrainingHudText,
    ));

    // Countdown text (3-2-1 before match starts)
    spawn_countdown_text(&mut commands);

    // Initialize event buffer for this game
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    event_buffer.buffer.start_session(&timestamp);

    // Start match in SQLite (events will be flushed to SQLite during gameplay)
    let seed: u64 = rand::random();
    let match_id = sqlite_logger.start_match(
        training_state.current_level,
        &training_state.current_level_name,
        "Player",
        &training_state.ai_profile,
        seed,
    );
    training_state.current_match_id = match_id;
    training_state.sqlite_session_id = Some(sqlite_logger.session_id().to_string());

    // Log match start
    event_buffer.buffer.log(
        0.0,
        GameEvent::MatchStart {
            level: training_state.current_level,
            level_name: training_state.current_level_name.clone(),
            left_profile: "Player".to_string(),
            right_profile: training_state.ai_profile.clone(),
            seed,
        },
    );

    // Log config
    event_buffer.buffer.log(
        0.0,
        GameEvent::Config(GameConfig {
            gravity_rise: GRAVITY_RISE,
            gravity_fall: GRAVITY_FALL,
            jump_velocity: JUMP_VELOCITY,
            move_speed: MOVE_SPEED,
            ground_accel: GROUND_ACCEL,
            air_accel: AIR_ACCEL,
            ball_gravity: BALL_GRAVITY,
            ball_bounce: BALL_BOUNCE,
            ball_air_friction: BALL_AIR_FRICTION,
            ball_ground_friction: BALL_GROUND_FRICTION,
            shot_max_power: SHOT_MAX_POWER,
            shot_max_speed: SHOT_MAX_SPEED,
            shot_charge_time: SHOT_CHARGE_TIME,
            shot_max_variance: SHOT_MAX_VARIANCE,
            shot_min_variance: SHOT_MIN_VARIANCE,
            steal_range: STEAL_RANGE,
            steal_success_chance: STEAL_SUCCESS_CHANCE,
            steal_cooldown: STEAL_COOLDOWN,
            preset_movement: None,
            preset_ball: None,
            preset_shooting: None,
            preset_composite: None,
        }),
    );
}

/// Set up time scale for reachability protocols (4x speed for faster exploration)
fn setup_reachability_time_scale(
    training_state: Res<TrainingState>,
    mut time: ResMut<Time<Virtual>>,
) {
    if training_state.protocol.iterates_all_levels() {
        time.set_relative_speed(REACHABILITY_SPEED_MULTIPLIER);
        info!(
            "Reachability mode: time scale set to {}x",
            REACHABILITY_SPEED_MULTIPLIER
        );
    }
}

/// Dynamic speed control for reachability training based on trigger input
/// - No triggers: 4x speed (fast exploration)
/// - One trigger held: 1x speed (normal)
/// - Both triggers held: 0.5x speed (slow motion)
fn update_reachability_time_scale(
    training_state: Res<TrainingState>,
    gamepads: Query<&Gamepad>,
    mut time: ResMut<Time<Virtual>>,
) {
    // Only apply to reachability protocols
    if !training_state.protocol.iterates_all_levels() {
        return;
    }

    // Check trigger states from all connected gamepads
    let mut left_trigger_held = false;
    let mut right_trigger_held = false;

    for gamepad in gamepads.iter() {
        // LeftTrigger2/RightTrigger2 are the analog triggers (LT/RT)
        // get() returns the analog value 0.0-1.0
        if gamepad
            .get(GamepadButton::LeftTrigger2)
            .is_some_and(|v| v > TRIGGER_PRESS_THRESHOLD)
        {
            left_trigger_held = true;
        }
        if gamepad
            .get(GamepadButton::RightTrigger2)
            .is_some_and(|v| v > TRIGGER_PRESS_THRESHOLD)
        {
            right_trigger_held = true;
        }
    }

    // Determine target speed based on trigger state
    let target_speed = if left_trigger_held && right_trigger_held {
        REACHABILITY_SPEED_SLOW // Both triggers: 0.5x
    } else if left_trigger_held || right_trigger_held {
        REACHABILITY_SPEED_NORMAL // One trigger: 1x
    } else {
        REACHABILITY_SPEED_MULTIPLIER // No triggers: 4x
    };

    // Only update if speed changed (avoid spamming the setter)
    let current_speed = time.relative_speed();
    if (current_speed - target_speed).abs() > 0.01 {
        time.set_relative_speed(target_speed);
    }
}

/// Training state machine - handles game flow
fn training_state_machine(
    mut training_state: ResMut<TrainingState>,
    mut score: ResMut<Score>,
    mut steal_tracker: ResMut<StealTracker>,
    mut event_buffer: ResMut<TrainingEventBuffer>,
    mut countdown: ResMut<MatchCountdown>,
    training_settings: Res<TrainingSettings>,
    allowed_levels: Res<AllowedTrainingLevels>,
    balls: Query<&BallState, With<Ball>>,
    time: Res<Time>,
    mut app_exit: MessageWriter<AppExit>,
    level_db: Res<LevelDatabase>,
    mut current_level: ResMut<CurrentLevel>,
    sqlite_logger: Res<SqliteEventLogger>,
) {
    match training_state.phase {
        TrainingPhase::WaitingToStart => {
            // Reachability: start immediately (player has ball)
            // Others: wait for first ball pickup to start timer
            if training_state.protocol.iterates_all_levels() {
                // Start immediately for exploration mode
                training_state.start_game_timer();
            } else {
                for ball_state in &balls {
                    if matches!(ball_state, BallState::Held(_)) {
                        training_state.start_game_timer();
                        break;
                    }
                }
            }
        }

        TrainingPhase::Playing => {
            training_state.update_elapsed();
            event_buffer.elapsed = training_state.game_elapsed;

            // AutoReachability: auto-advance based on time limit
            if training_state.protocol.is_automated() {
                let time_limit = training_state.time_limit_secs.unwrap_or(60.0);
                if training_state.game_elapsed >= time_limit {
                    // Auto-advance to next level
                    auto_advance_level(
                        &mut training_state,
                        &score,
                        &mut event_buffer,
                        &sqlite_logger,
                        &level_db,
                        &mut current_level,
                    );
                }
                return;
            }

            // Reachability (manual): no win condition - player decides when to advance via LB
            if training_state.protocol.iterates_all_levels() {
                // Level transitions handled by check_advance_level system
                return;
            }

            // Check win condition: score reached OR time limit expired
            let score_reached =
                score.left >= training_state.win_score || score.right >= training_state.win_score;
            let time_expired = training_state
                .time_limit_secs
                .map(|limit| training_state.game_elapsed >= limit)
                .unwrap_or(false);

            if score_reached || time_expired {
                // Log match end
                event_buffer.buffer.log(
                    training_state.game_elapsed,
                    GameEvent::MatchEnd {
                        score_left: score.left,
                        score_right: score.right,
                        duration: training_state.game_elapsed,
                    },
                );

                let match_id = sqlite_logger.current_match_id();
                flush_training_events_buffer(&mut event_buffer, &sqlite_logger);

                // End match in SQLite
                sqlite_logger.end_match(score.left, score.right, training_state.game_elapsed);

                // Record result
                training_state.record_result(score.left, score.right, match_id);

                // Determine outcome message
                let outcome = if time_expired && !score_reached {
                    format!("Time expired ({:.1}s)", training_state.game_elapsed)
                } else if score.left >= training_state.win_score {
                    "You win!".to_string()
                } else {
                    "AI wins!".to_string()
                };

                println!(
                    "Iteration {} complete: {} ({}-{})",
                    training_state.game_number, outcome, score.left, score.right
                );
            }
        }

        TrainingPhase::Paused => {
            // Do nothing - game is paused, waiting for Start to resume
        }

        TrainingPhase::GameEnded => {
            training_state.transition_timer += time.delta_secs();

            // Wait 2 seconds before moving to next phase
            if training_state.transition_timer > 2.0 {
                training_state.transition_timer = 0.0;
                if training_state.game_number >= training_state.games_total {
                    training_state.phase = TrainingPhase::SessionComplete;
                } else {
                    // Pick level based on protocol
                    if let Some(fixed_level_name) = training_state.protocol.fixed_level() {
                        // Protocol specifies a fixed level - keep using it
                        // Level is already set, just ensure current_level matches
                        if let Some((idx, level_data)) =
                            level_db.all().iter().enumerate().find(|(_, l)| {
                                l.name.to_lowercase() == fixed_level_name.to_lowercase()
                            })
                        {
                            training_state.current_level = (idx + 1) as u32;
                            training_state.current_level_name = fixed_level_name.to_string();
                            current_level.0 = level_data.id.clone();
                        }
                    } else {
                        // Pick new random level
                        // Filter out debug/regression levels and explicit excludes
                        let training_levels: Vec<(usize, &ballgame::levels::LevelData)> = level_db
                            .all()
                            .iter()
                            .enumerate()
                            .filter(|(_, l)| {
                                let is_debug = l.debug;
                                let is_regression = l.regression;
                                let allowed = level_allowed(
                                    &l.name,
                                    &training_settings,
                                    allowed_levels.0.as_deref(),
                                );
                                !is_debug && !is_regression && allowed
                            })
                            .collect();

                        if let Some(&(idx, level_data)) =
                            training_levels.choose(&mut rand::thread_rng())
                        {
                            training_state.current_level = (idx + 1) as u32;
                            training_state.current_level_name = level_data.name.clone();
                            current_level.0 = level_data.id.clone();
                        }
                    }

                    training_state.next_game();

                    // Reset score and steal tracker for new game
                    score.left = 0;
                    score.right = 0;
                    steal_tracker.reset();

                    // Start countdown for new game
                    countdown.start();

                    // Reset event buffer for new game
                    *event_buffer = TrainingEventBuffer::default();
                    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
                    event_buffer.buffer.start_session(&timestamp);

                    // Start new match in SQLite
                    let seed: u64 = rand::random();
                    let match_id = sqlite_logger.start_match(
                        training_state.current_level,
                        &training_state.current_level_name,
                        "Player",
                        &training_state.ai_profile,
                        seed,
                    );
                    training_state.current_match_id = match_id;

                    event_buffer.buffer.log(
                        0.0,
                        GameEvent::MatchStart {
                            level: training_state.current_level,
                            level_name: training_state.current_level_name.clone(),
                            left_profile: "Player".to_string(),
                            right_profile: training_state.ai_profile.clone(),
                            seed,
                        },
                    );

                    println!(
                        "\nStarting Game {}/{} on {}",
                        training_state.game_number,
                        training_state.games_total,
                        training_state.current_level_name
                    );
                }
            }
        }

        TrainingPhase::StartingNext => {
            // This phase is handled inline above
            training_state.phase = TrainingPhase::WaitingToStart;
        }

        TrainingPhase::SessionComplete => {
            // Write summary and exit
            if let Err(e) = write_session_summary(&training_state) {
                eprintln!("Failed to write session summary: {}", e);
            }
            print_session_summary(&training_state);

            // Run standard analysis (same for all protocols)
            println!("\nAnalyzing training session...");
            let analysis = training_state
                .sqlite_session_id
                .as_deref()
                .and_then(|session_id| {
                    SimDatabase::open(std::path::Path::new("db/training.db"))
                        .ok()
                        .and_then(|db| {
                            analyze_session_from_db(&db, session_id, training_state.protocol)
                        })
                });

            if let Some(ref analysis) = analysis {
                if let Err(e) = write_analysis_files(&training_state.session_dir, analysis) {
                    eprintln!("Failed to write analysis: {}", e);
                }
            } else {
                eprintln!("No SQLite analysis available for this session.");
            }

            // Run protocol-specific analysis (additional output)
            match training_state.protocol {
                TrainingProtocol::Pursuit | TrainingProtocol::Pursuit2 => {
                    // Pursuit-specific analysis (in addition to standard)
                    let pursuit_analysis =
                        training_state
                            .sqlite_session_id
                            .as_deref()
                            .and_then(|session_id| {
                                SimDatabase::open(std::path::Path::new("db/training.db"))
                                    .ok()
                                    .and_then(|db| analyze_pursuit_session_from_db(&db, session_id))
                            });

                    if let Some(pursuit_analysis) = pursuit_analysis {
                        // Write pursuit analysis
                        let md_content = format_pursuit_analysis_markdown(&pursuit_analysis);
                        let md_path = training_state.session_dir.join("pursuit_analysis.md");
                        if let Err(e) = fs::write(&md_path, &md_content) {
                            eprintln!("Failed to write pursuit analysis: {}", e);
                        } else {
                            println!("Pursuit analysis written to: {}", md_path.display());
                        }

                        // Print pursuit summary to terminal
                        println!("\n## Pursuit Test Results\n");
                        println!("Pursuit Score: {:.1}/100", pursuit_analysis.pursuit_score);
                        println!(
                            "Outcomes: {} catches, {} player scores, {} timeouts",
                            pursuit_analysis.ai_catches,
                            pursuit_analysis.player_scores,
                            pursuit_analysis.timeouts
                        );
                        println!(
                            "Avg Distance: {:.0}px | Closing Rate: {:.1}px/s",
                            pursuit_analysis.avg_distance, pursuit_analysis.avg_closing_rate
                        );

                        if pursuit_analysis.pursuit_score >= 70.0 {
                            println!("\nResult: PASS - AI demonstrates good pursuit behavior.");
                        } else if pursuit_analysis.pursuit_score >= 50.0 {
                            println!("\nResult: MARGINAL - AI shows some pursuit but with issues.");
                        } else {
                            println!("\nResult: FAIL - AI is not effectively pursuing the player.");
                        }
                    } else {
                        eprintln!("No SQLite pursuit analysis available for this session.");
                    }
                }
                TrainingProtocol::AdvancedPlatform => {
                    if let Some(ref analysis) = analysis {
                        // Print analysis request to terminal
                        let prompt =
                            generate_analysis_request(&training_state.session_dir, analysis);
                        println!("\n{}", prompt);
                    } else {
                        eprintln!("No analysis available for analysis request.");
                    }
                }
                TrainingProtocol::Reachability | TrainingProtocol::AutoReachability => {
                    // Reachability exploration - summary of levels visited
                    println!("\n## Reachability Exploration Complete\n");
                    println!(
                        "Levels explored: {}/{}",
                        training_state.level_sequence_index + 1,
                        training_state.level_sequence.len()
                    );
                    println!(
                        "\nRun offline analysis with:\n  ./offline_training/analyze_offline.sh"
                    );
                }
            }

            app_exit.write(AppExit::Success);
        }
    }
}

/// Update training HUD text
fn update_training_hud(
    training_state: Res<TrainingState>,
    score: Res<Score>,
    mut hud_query: Query<&mut Text2d, With<TrainingHudText>>,
) {
    for mut text in &mut hud_query {
        // Reachability mode: different HUD format
        if training_state.protocol.iterates_all_levels() {
            let phase_indicator = match training_state.phase {
                TrainingPhase::Paused => " [PAUSED]",
                TrainingPhase::SessionComplete => " [Complete]",
                _ => "",
            };

            text.0 = format!(
                "{} | Time: {:.0}s | [LB: Quit]{}",
                training_state.current_level_name,
                training_state.game_elapsed,
                phase_indicator
            );
            return;
        }

        // Standard training mode HUD
        let phase_indicator = match training_state.phase {
            TrainingPhase::WaitingToStart => " [Pick up the ball to start]",
            TrainingPhase::Paused => " [PAUSED - Press Start to resume]",
            TrainingPhase::GameEnded => " [Game Over - Press Start to retry]",
            TrainingPhase::SessionComplete => " [Session Complete]",
            _ => "",
        };

        text.0 = format!(
            "Game {}/{} | {} | You {} - {} {}{}",
            training_state.game_number,
            training_state.games_total,
            training_state.current_level_name,
            score.left,
            score.right,
            training_state.ai_profile,
            phase_indicator
        );
    }
}

/// Emit game events during training
///
/// This is a thin wrapper around the shared `emit_game_events` function.
fn emit_training_events(
    mut event_buffer: ResMut<TrainingEventBuffer>,
    training_state: Res<TrainingState>,
    score: Res<Score>,
    steal_contest: Res<StealContest>,
    shot_info: Res<LastShotInfo>,
    players: Query<
        (
            Entity,
            &Team,
            &Transform,
            &Velocity,
            &TargetBasket,
            &ChargingShot,
            &AiState,
            &StealCooldown,
            Option<&HoldingBall>,
            &InputState,
        ),
        With<Player>,
    >,
    baskets: Query<(&Transform, &Basket)>,
    balls: Query<(&Transform, &Velocity, &BallState), With<Ball>>,
    mut event_bus: ResMut<EventBus>,
) {
    if training_state.phase != TrainingPhase::Playing {
        return;
    }

    // Bridge EventBus → EventBuffer
    let bus_events: Vec<_> = event_bus
        .export_events()
        .into_iter()
        .filter(|(_, event)| !matches!(event, GameEvent::Goal { .. }))
        .collect();
    event_buffer.buffer.import_events(bus_events);

    let time = training_state.game_elapsed;

    // Convert query results to snapshots
    let player_snapshots: Vec<_> = players
        .iter()
        .map(
            |(
                entity,
                team,
                transform,
                velocity,
                target,
                charging,
                ai_state,
                steal_cooldown,
                holding,
                input_state,
            )| {
                snapshot_player(
                    entity,
                    team,
                    transform,
                    velocity,
                    target,
                    charging,
                    ai_state,
                    steal_cooldown,
                    holding,
                    input_state,
                )
            },
        )
        .collect();

    let basket_snapshots: Vec<_> = baskets
        .iter()
        .map(|(transform, basket)| BasketSnapshot {
            basket: *basket,
            position: (transform.translation.x, transform.translation.y),
        })
        .collect();

    let ball_snapshot = balls
        .iter()
        .next()
        .map(|(transform, velocity, state)| snapshot_ball(transform, velocity, state));

    // Destructure to get separate mutable borrows
    let TrainingEventBuffer {
        ref mut emitter_state,
        ref mut buffer,
        ..
    } = *event_buffer;

    // Use the shared emitter
    emit_game_events(
        emitter_state,
        buffer,
        time,
        &score,
        &steal_contest,
        &player_snapshots,
        &basket_snapshots,
        ball_snapshot.as_ref(),
        Some(&shot_info),
    );
}

fn flush_training_events_buffer(
    event_buffer: &mut TrainingEventBuffer,
    sqlite_logger: &SqliteEventLogger,
) {
    let events = event_buffer.buffer.drain_events();
    if events.is_empty() {
        return;
    }

    sqlite_logger.log_events(&events);
}

fn flush_training_events_to_sqlite(
    mut event_buffer: ResMut<TrainingEventBuffer>,
    sqlite_logger: Res<SqliteEventLogger>,
) {
    flush_training_events_buffer(&mut event_buffer, &sqlite_logger);
}

/// Export reachability heatmap data to CSV file
/// Merges with existing long-running data - each session accumulates onto the baseline
fn export_reachability_heatmap(collector: &ReachabilityCollector) {
    use std::io::{BufRead, Write};

    const CELL_SIZE: f32 = 20.0;
    const GRID_WIDTH: usize = 80;
    const GRID_HEIGHT: usize = 45;
    const SCALE_FACTOR: f32 = 10000.0; // Scale for converting normalized values to counts

    // Sanitize level name and determine file path
    let safe_name = sanitize_level_name(&collector.level_name);
    fs::create_dir_all("showcase/heatmaps").ok();
    let path = format!(
        "showcase/heatmaps/heatmap_reachability_{}_{}.txt",
        safe_name, collector.level_id
    );

    // Load existing cumulative counts from file (if it exists)
    let mut cumulative_grid = vec![0u32; GRID_WIDTH * GRID_HEIGHT];
    if let Ok(file) = fs::File::open(&path) {
        let reader = std::io::BufReader::new(file);
        for line in reader.lines().skip(1) {
            // Skip header
            if let Ok(line) = line {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() == 3 {
                    if let (Ok(x), Ok(y), Ok(value)) = (
                        parts[0].parse::<f32>(),
                        parts[1].parse::<f32>(),
                        parts[2].parse::<f32>(),
                    ) {
                        // Convert world coords back to grid index
                        let cx = ((x + ARENA_WIDTH / 2.0) / CELL_SIZE).floor() as i32;
                        let cy = ((ARENA_HEIGHT / 2.0 - y) / CELL_SIZE).floor() as i32;
                        if cx >= 0
                            && cy >= 0
                            && (cx as usize) < GRID_WIDTH
                            && (cy as usize) < GRID_HEIGHT
                        {
                            let idx = cy as usize * GRID_WIDTH + cx as usize;
                            // Convert normalized value back to pseudo-count (scaled)
                            cumulative_grid[idx] = (value * SCALE_FACTOR) as u32;
                        }
                    }
                }
            }
        }
    }

    // Build new session visit counts
    let mut session_grid = vec![0u32; GRID_WIDTH * GRID_HEIGHT];
    for &(x, y) in collector.all_positions() {
        let cx = ((x + ARENA_WIDTH / 2.0) / CELL_SIZE).floor() as i32;
        let cy = ((ARENA_HEIGHT / 2.0 - y) / CELL_SIZE).floor() as i32;

        if cx >= 0 && cy >= 0 && (cx as usize) < GRID_WIDTH && (cy as usize) < GRID_HEIGHT {
            let idx = cy as usize * GRID_WIDTH + cx as usize;
            session_grid[idx] += 1;
        }
    }

    // Normalize session data to SCALE_FACTOR max and add to cumulative
    let session_max = session_grid.iter().max().copied().unwrap_or(1).max(1);
    for idx in 0..(GRID_WIDTH * GRID_HEIGHT) {
        if session_grid[idx] > 0 {
            let normalized_session =
                (session_grid[idx] as f32 / session_max as f32 * SCALE_FACTOR) as u32;
            cumulative_grid[idx] = cumulative_grid[idx].saturating_add(normalized_session);
        }
    }

    // Find max for final normalization
    let max_count = cumulative_grid.iter().max().copied().unwrap_or(1).max(1);

    // Write CSV
    let Ok(mut file) = fs::File::create(&path) else {
        eprintln!("Failed to create heatmap file: {}", path);
        return;
    };

    if writeln!(file, "x,y,value").is_err() {
        eprintln!("Failed to write heatmap header");
        return;
    }

    for cy in 0..GRID_HEIGHT {
        for cx in 0..GRID_WIDTH {
            let world_x = (cx as f32 + 0.5) * CELL_SIZE - ARENA_WIDTH / 2.0;
            let world_y = ARENA_HEIGHT / 2.0 - (cy as f32 + 0.5) * CELL_SIZE;
            let count = cumulative_grid[cy * GRID_WIDTH + cx];
            let value = if count > 0 {
                count as f32 / max_count as f32
            } else {
                0.0
            };
            let _ = writeln!(file, "{:.2},{:.2},{:.3}", world_x, world_y, value);
        }
    }
}

/// Create a complementary color that is light and semi-transparent
/// Shifts hue by 180°, increases lightness, and applies alpha
fn complementary_light_color(base: Color, lightness: f32, alpha: f32) -> Color {
    // Convert to HSL via Srgba
    let srgba = base.to_srgba();
    let r = srgba.red;
    let g = srgba.green;
    let b = srgba.blue;

    // RGB to HSL conversion
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;

    let (h, s) = if (max - min).abs() < 0.0001 {
        (0.0, 0.0)
    } else {
        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        let h = if (max - r).abs() < 0.0001 {
            ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
        } else if (max - g).abs() < 0.0001 {
            ((b - r) / d + 2.0) / 6.0
        } else {
            ((r - g) / d + 4.0) / 6.0
        };
        (h, s)
    };

    // Shift hue by 180° (0.5 in 0-1 range) for complementary color
    let new_h = (h + 0.5) % 1.0;
    // Blend lightness toward target (make it lighter)
    let new_l = l + (lightness - l) * 0.8;
    // Keep saturation but reduce it slightly for softer look
    let new_s = s * 0.6;

    // HSL to RGB conversion
    let (r2, g2, b2) = if new_s.abs() < 0.0001 {
        (new_l, new_l, new_l)
    } else {
        let q = if new_l < 0.5 {
            new_l * (1.0 + new_s)
        } else {
            new_l + new_s - new_l * new_s
        };
        let p = 2.0 * new_l - q;
        let hue_to_rgb = |t: f32| {
            let t = if t < 0.0 { t + 1.0 } else if t > 1.0 { t - 1.0 } else { t };
            if t < 1.0 / 6.0 {
                p + (q - p) * 6.0 * t
            } else if t < 0.5 {
                q
            } else if t < 2.0 / 3.0 {
                p + (q - p) * (2.0 / 3.0 - t) * 6.0
            } else {
                p
            }
        };
        (
            hue_to_rgb(new_h + 1.0 / 3.0),
            hue_to_rgb(new_h),
            hue_to_rgb(new_h - 1.0 / 3.0),
        )
    };

    Color::srgba(r2, g2, b2, alpha)
}

/// Sanitize level name for use in filenames
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

/// Auto-advance to next level (for AutoReachability protocol)
/// Called when time limit is reached
fn auto_advance_level(
    training_state: &mut TrainingState,
    score: &Score,
    event_buffer: &mut TrainingEventBuffer,
    sqlite_logger: &SqliteEventLogger,
    level_db: &LevelDatabase,
    current_level: &mut CurrentLevel,
) {
    // Export reachability heatmap
    if let Some(collector) = training_state.reachability_collector.take() {
        export_reachability_heatmap(&collector);
        println!(
            "  Exported reachability heatmap: {} ({:.1}s, {} samples)",
            collector.level_name,
            collector.elapsed_secs(),
            collector.positions.len()
        );
    }

    // Log match end for current level
    event_buffer.buffer.log(
        training_state.game_elapsed,
        GameEvent::MatchEnd {
            score_left: score.left,
            score_right: score.right,
            duration: training_state.game_elapsed,
        },
    );

    flush_training_events_buffer(event_buffer, sqlite_logger);
    sqlite_logger.end_match(score.left, score.right, training_state.game_elapsed);

    println!(
        "Level complete (auto): {} ({:.1}s) [{}/{}]",
        training_state.current_level_name,
        training_state.game_elapsed,
        training_state.level_sequence_index + 1,
        training_state.level_sequence.len()
    );

    // Advance to next level in sequence
    if training_state.advance_to_next_level() {
        // More levels to explore
        if let Some(level_idx) = training_state.current_sequence_level() {
            if let Some(level_data) = level_db.get(level_idx) {
                // Update training state
                training_state.current_level = (level_idx + 1) as u32;
                training_state.current_level_name = level_data.name.clone();
                training_state.game_number += 1;
                training_state.game_elapsed = 0.0;
                training_state.game_start_time = None;

                // Update CurrentLevel resource to trigger level change
                current_level.0 = level_data.id.clone();

                // Create new reachability collector
                training_state.reachability_collector = Some(ReachabilityCollector::new_with_preload(
                    level_data.id.clone(),
                    level_data.name.clone(),
                ));

                println!(
                    "\nAuto-advancing to: {} [{}/{}]",
                    level_data.name,
                    training_state.level_sequence_index + 1,
                    training_state.level_sequence.len()
                );

                // Reset to WaitingToStart
                training_state.phase = TrainingPhase::WaitingToStart;
            }
        }
    } else {
        // All levels complete
        println!("\nAll levels explored!");
        training_state.phase = TrainingPhase::SessionComplete;
    }
}

/// Check for level advance input (manual Reachability protocol only)
/// LB advances to next level in the sequence, cycling through all levels
/// Note: AutoReachability uses auto_advance_level instead (time-based)
fn check_advance_level(
    mut input: ResMut<PlayerInput>,
    mut training_state: ResMut<TrainingState>,
    score: Res<Score>,
    mut event_buffer: ResMut<TrainingEventBuffer>,
    sqlite_logger: Res<SqliteEventLogger>,
    level_db: Res<LevelDatabase>,
    mut current_level: ResMut<CurrentLevel>,
) {
    // Only handle for manual Reachability protocol (not AutoReachability)
    if !training_state.protocol.iterates_all_levels() || training_state.protocol.is_automated() {
        return;
    }

    if training_state.phase != TrainingPhase::Playing {
        return;
    }

    // Grace period: ignore all advance input for first 2 seconds
    if training_state.game_elapsed < 2.0 {
        input.advance_level_pressed = false;
        input.swap_pressed = false;
        return;
    }

    // Check if advance level was pressed
    if !input.advance_level_pressed {
        return;
    }

    // Consume both flags to prevent swap behavior
    input.advance_level_pressed = false;
    input.swap_pressed = false;

    // Export reachability heatmap if sufficient exploration time
    if let Some(collector) = training_state.reachability_collector.take() {
        if collector.elapsed_secs() >= 10.0 {
            export_reachability_heatmap(&collector);
            println!(
                "  Exported reachability heatmap: {} ({:.1}s, {} new + {} preloaded samples)",
                collector.level_name,
                collector.elapsed_secs(),
                collector.positions.len(),
                collector.preloaded_positions.len()
            );
        } else {
            println!(
                "  Skipped heatmap: {} ({:.1}s < 10s threshold)",
                collector.level_name,
                collector.elapsed_secs()
            );
        }
    }

    // Log match end for current level
    event_buffer.buffer.log(
        training_state.game_elapsed,
        GameEvent::MatchEnd {
            score_left: score.left,
            score_right: score.right,
            duration: training_state.game_elapsed,
        },
    );

    flush_training_events_buffer(&mut event_buffer, &sqlite_logger);
    sqlite_logger.end_match(score.left, score.right, training_state.game_elapsed);

    println!(
        "Level complete: {} ({:.1}s) [{}/{}]",
        training_state.current_level_name,
        training_state.game_elapsed,
        training_state.level_sequence_index + 1,
        training_state.level_sequence.len()
    );

    // Advance to next level in sequence
    if training_state.advance_to_next_level() {
        // More levels to explore
        if let Some(level_idx) = training_state.current_sequence_level() {
            if let Some(level_data) = level_db.get(level_idx) {
                // Update training state
                training_state.current_level = (level_idx + 1) as u32;
                training_state.current_level_name = level_data.name.clone();
                training_state.game_number += 1;
                training_state.game_elapsed = 0.0;
                training_state.game_start_time = None;

                // Update CurrentLevel resource to trigger level change
                current_level.0 = level_data.id.clone();

                // Create new reachability collector with preloaded data
                training_state.reachability_collector = Some(ReachabilityCollector::new_with_preload(
                    level_data.id.clone(),
                    level_data.name.clone(),
                ));

                println!(
                    "\nAdvancing to: {} [{}/{}]",
                    level_data.name,
                    training_state.level_sequence_index + 1,
                    training_state.level_sequence.len()
                );

                // Reset to WaitingToStart (timer starts on first movement)
                training_state.phase = TrainingPhase::WaitingToStart;
            }
        }
    } else {
        // All levels complete
        println!("\nAll levels explored!");
        training_state.phase = TrainingPhase::SessionComplete;
    }
}

/// Check for escape key to quit
fn check_escape_quit(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_exit: MessageWriter<AppExit>,
    mut training_state: ResMut<TrainingState>,
    score: Res<Score>,
    mut event_buffer: ResMut<TrainingEventBuffer>,
    sqlite_logger: Res<SqliteEventLogger>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        println!("\nTraining session cancelled by user.");

        // Export reachability heatmap if sufficient exploration time (for Reachability protocol)
        if let Some(collector) = training_state.reachability_collector.take() {
            if collector.elapsed_secs() >= 10.0 {
                export_reachability_heatmap(&collector);
                println!(
                    "  Exported reachability heatmap: {} ({:.1}s, {} samples)",
                    collector.level_name,
                    collector.elapsed_secs(),
                    collector.positions.len()
                );
            } else {
                println!(
                    "  Skipped heatmap: {} ({:.1}s < 10s threshold)",
                    collector.level_name,
                    collector.elapsed_secs()
                );
            }
        }

        // End current match in SQLite if one is active
        if training_state.phase == TrainingPhase::Playing
            || training_state.phase == TrainingPhase::WaitingToStart
        {
            event_buffer.buffer.log(
                training_state.game_elapsed,
                GameEvent::MatchEnd {
                    score_left: score.left,
                    score_right: score.right,
                    duration: training_state.game_elapsed,
                },
            );
            flush_training_events_buffer(&mut event_buffer, &sqlite_logger);
            sqlite_logger.end_match(score.left, score.right, training_state.game_elapsed);
        }

        // Still write summary with completed games
        if !training_state.game_results.is_empty() {
            if let Err(e) = write_session_summary(&training_state) {
                eprintln!("Failed to write session summary: {}", e);
            }
            print_session_summary(&training_state);
        }

        app_exit.write(AppExit::Success);
    }
}

/// Check for Start button to pause/unpause or restart
fn check_pause_restart(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut training_state: ResMut<TrainingState>,
    mut score: ResMut<Score>,
    mut steal_tracker: ResMut<StealTracker>,
    mut event_buffer: ResMut<TrainingEventBuffer>,
    mut countdown: ResMut<MatchCountdown>,
    level_db: Res<LevelDatabase>,
    settings: Res<TrainingSettings>,
    allowed_levels: Res<AllowedTrainingLevels>,
    mut current_level: ResMut<CurrentLevel>,
    mut players: Query<(Entity, &mut Transform, &Team), With<Player>>,
    mut balls: Query<
        (Entity, &mut Transform, &mut BallState, &mut Velocity),
        (With<Ball>, Without<Player>),
    >,
    sqlite_logger: Res<SqliteEventLogger>,
) {
    // Check for Start button (keyboard P or gamepad Start)
    let start_pressed = keyboard.just_pressed(KeyCode::KeyP)
        || gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::Start));

    if !start_pressed {
        return;
    }

    // Toggle pause during Playing
    if training_state.phase == TrainingPhase::Playing {
        training_state.phase = TrainingPhase::Paused;
        println!("\n[PAUSED] Press Start to resume");
        return;
    }

    // Unpause
    if training_state.phase == TrainingPhase::Paused {
        training_state.phase = TrainingPhase::Playing;
        println!("[RESUMED]");
        return;
    }

    // Restart during GameEnded phase
    if training_state.phase != TrainingPhase::GameEnded {
        return;
    }

    // Pick level based on protocol
    if let Some(fixed_level_name) = training_state.protocol.fixed_level() {
        // Protocol specifies a fixed level - keep using it
        println!("\nRestarting iteration on {}...", fixed_level_name);
        if let Some((idx, level_data)) = level_db
            .all()
            .iter()
            .enumerate()
            .find(|(_, l)| l.name.to_lowercase() == fixed_level_name.to_lowercase())
        {
            training_state.current_level = (idx + 1) as u32;
            training_state.current_level_name = fixed_level_name.to_string();
            current_level.0 = level_data.id.clone();
        }
    } else {
        println!("\nRestarting game with new level...");

        // Pick new random level (excluding debug/regression and explicit excludes)
        let training_levels: Vec<(usize, &ballgame::levels::LevelData)> = level_db
            .all()
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                let is_debug = l.debug;
                let is_regression = l.regression;
                let allowed = level_allowed(&l.name, &settings, allowed_levels.0.as_deref());
                !is_debug && !is_regression && allowed
            })
            .collect();

        if let Some(&(idx, level_data)) = training_levels.choose(&mut rand::thread_rng()) {
            training_state.current_level = (idx + 1) as u32;
            training_state.current_level_name = level_data.name.clone();
            current_level.0 = level_data.id.clone();
        }
    }

    // Reset score and steal tracker
    score.left = 0;
    score.right = 0;
    steal_tracker.reset();

    // Reset players to spawn positions and find human player (left team)
    let mut left_player_entity = None;
    for (entity, mut player_transform, team) in &mut players {
        match team {
            Team::Left => {
                player_transform.translation = PLAYER_SPAWN_LEFT;
                left_player_entity = Some(entity);
            }
            Team::Right => {
                player_transform.translation = PLAYER_SPAWN_RIGHT;
            }
        }
        commands.entity(entity).remove::<HoldingBall>();
    }

    // Reset ball - jump ball by default, drive mode gives human possession
    for (ball_entity, mut ball_transform, mut ball_state, mut velocity) in &mut balls {
        if settings.drive_mode {
            if let Some(left_player) = left_player_entity {
                ball_transform.translation.x = PLAYER_SPAWN_LEFT.x;
                ball_transform.translation.y = PLAYER_SPAWN_LEFT.y;
                *ball_state = BallState::Held(left_player);
                velocity.0 = Vec2::ZERO;
                commands
                    .entity(left_player)
                    .insert(HoldingBall(ball_entity));
            }
        } else {
            ball_transform.translation = BALL_SPAWN;
            *ball_state = BallState::Free;
            velocity.0 = Vec2::ZERO;
        }
    }

    // Reset training state for new game (keep same game number for retry)
    training_state.phase = TrainingPhase::WaitingToStart;
    training_state.game_start_time = None;
    training_state.game_elapsed = 0.0;
    training_state.transition_timer = 0.0;

    // Start countdown for new game
    countdown.start();

    // Reset event buffer for new game
    *event_buffer = TrainingEventBuffer::default();
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
    event_buffer.buffer.start_session(&timestamp);

    // Start new match in SQLite
    let seed: u64 = rand::random();
    let match_id = sqlite_logger.start_match(
        training_state.current_level,
        &training_state.current_level_name,
        "Player",
        &training_state.ai_profile,
        seed,
    );
    training_state.current_match_id = match_id;

    event_buffer.buffer.log(
        0.0,
        GameEvent::MatchStart {
            level: training_state.current_level,
            level_name: training_state.current_level_name.clone(),
            left_profile: "Player".to_string(),
            right_profile: training_state.ai_profile.clone(),
            seed,
        },
    );

    println!(
        "Game {}/{} on {}",
        training_state.game_number, training_state.games_total, training_state.current_level_name
    );
}
