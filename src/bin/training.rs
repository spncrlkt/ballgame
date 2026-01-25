//! Training Mode Binary
//!
//! Play 5 consecutive 1v1 games against AI with comprehensive logging.
//! After the session, use Claude Code to analyze the evlogs.
//!
//! Usage:
//!   cargo run --bin training
//!   cargo run --bin training -- --games 3 --profile Aggressive

use ballgame::ui::spawn_steal_indicators;
use ballgame::{
    AiGoal, AiNavState, AiProfileDatabase, AiState, Ball, BallPlayerContact, BallPulse, BallRolling,
    BallShotGrace, BallSpin, BallState, BallStyle, BallTextures, ChargeGaugeBackground,
    ChargeGaugeFill, ChargingShot, CoyoteTimer, CurrentLevel, CurrentPalette, DebugSettings,
    EventBuffer, Facing, GameConfig, GameEvent, Grounded, HoldingBall, HumanControlled, InputState,
    JumpState, LastShotInfo, LevelDatabase, MatchCountdown, NavGraph, PALETTES_FILE, PaletteDatabase,
    PhysicsTweaks, Player, PlayerInput, Score, SnapshotConfig, StealContest, StealCooldown, StealTracker,
    StyleTextures, TargetBasket, Team, Velocity, ai, ball, constants::*, countdown, helpers::*, input,
    levels, player, scoring, shooting, spawn_countdown_text, steal, world,
};
use ballgame::events::{
    emit_game_events, snapshot_ball, snapshot_player, EmitterConfig, EventEmitterState,
};
use ballgame::training::{
    TrainingMode, TrainingPhase, TrainingSettings, TrainingState, ensure_session_dir,
    evlog_path_for_game, print_session_summary, write_session_summary,
};
use bevy::{camera::ScalingMode, prelude::*};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write as IoWrite;
use world::{Basket, BasketRim, Collider, Platform};

/// Path to ball options file
const BALL_OPTIONS_FILE: &str = "assets/ball_options.txt";

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

fn main() {
    let settings = TrainingSettings::from_args();

    println!("========================================");
    println!("       TRAINING MODE");
    println!("========================================");
    println!();
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
    if let Some(level) = settings.level {
        println!("  Level: {} (fixed)", level);
    } else {
        println!("  Level: random");
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
    training_state.win_score = if settings.mode == TrainingMode::Game {
        settings.win_score
    } else {
        1 // Goal mode: end after first goal
    };
    training_state.time_limit_secs = settings.time_limit_secs;
    training_state.first_point_timeout_secs = settings.first_point_timeout_secs;

    // Pick level - either fixed from settings or random
    if let Some(fixed_level) = settings.level {
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
                let level_name = level.map(|l| l.name.clone()).unwrap_or_default();
                let is_excluded = settings.exclude_levels.iter()
                    .any(|exc| level_name.to_lowercase() == exc.to_lowercase());
                !is_debug && !is_excluded
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

    println!("Starting iteration 1/{} on {}", settings.iterations, training_state.current_level_name);
    println!();

    // Viewport setup from settings
    let viewport_index = settings.viewport_index.min(VIEWPORT_PRESETS.len() - 1);
    let (viewport_width, viewport_height, _) = VIEWPORT_PRESETS[viewport_index];

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
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
        }))
        .insert_resource(ClearColor(initial_bg))
        .insert_resource(palette_db)
        .insert_resource(level_db)
        .insert_resource(settings)
        .insert_resource(training_state)
        .init_resource::<PlayerInput>()
        .init_resource::<DebugSettings>()
        .init_resource::<StealContest>()
        .init_resource::<StealTracker>()
        .init_resource::<Score>()
        .insert_resource(CurrentLevel(1)) // Will be set from training state
        .insert_resource(CurrentPalette(0))
        .init_resource::<PhysicsTweaks>()
        .init_resource::<LastShotInfo>()
        .init_resource::<AiProfileDatabase>()
        .init_resource::<NavGraph>()
        .insert_resource(SnapshotConfig::default())
        .init_resource::<TrainingEventBuffer>()
        .init_resource::<MatchCountdown>()
        // Startup systems
        .add_systems(Startup, training_setup)
        // Input systems chain - paused when game is paused
        .add_systems(
            Update,
            (
                input::capture_input,
                ai::copy_human_input,
                ai::mark_nav_dirty_on_level_change,
                ai::rebuild_nav_graph,
                ai::ai_navigation_update,
                ai::ai_decision_update,
            )
                .chain()
                .run_if(not_paused),
        )
        // Core Update systems - split to avoid tuple issues
        .add_systems(Update, player::respawn_player)
        .add_systems(Update, steal::steal_cooldown_update)
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
                training_state_machine,
                update_training_hud,
                emit_training_events,
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
            )
                .chain()
                .run_if(countdown::not_in_countdown)
                .run_if(not_paused),
        )
        .run();
}

/// Run condition: game is not paused
fn not_paused(training_state: Res<TrainingState>) -> bool {
    training_state.phase != TrainingPhase::Paused
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
    training_state: Res<TrainingState>,
    mut current_level: ResMut<CurrentLevel>,
    mut event_buffer: ResMut<TrainingEventBuffer>,
) {
    // Set current level from training state
    current_level.0 = training_state.current_level;

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

    // Get level
    let level_index = (training_state.current_level as usize).saturating_sub(1);

    // Find AI profile index
    let ai_profile_index = profile_db
        .profiles()
        .iter()
        .position(|p| p.name == training_state.ai_profile)
        .unwrap_or(0);

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
                profile_index: 0,
                ..default()
            },
            AiNavState::default(),
            StealCooldown::default(),
            HumanControlled, // Mark as human controlled
        ))
        .id();

    // Right player - AI controlled
    let right_player = commands
        .spawn((
            Sprite::from_color(initial_palette.right, PLAYER_SIZE),
            Transform::from_translation(PLAYER_SPAWN_RIGHT),
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
                current_goal: AiGoal::ChaseBall,
                profile_index: ai_profile_index,
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
            Sprite::from_color(Color::BLACK, Vec2::new(CHARGE_GAUGE_WIDTH, CHARGE_GAUGE_HEIGHT)),
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
            Sprite::from_color(Color::BLACK, Vec2::new(CHARGE_GAUGE_WIDTH, CHARGE_GAUGE_HEIGHT)),
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
                .map(|i| asset_server.load(format!("ball_{}_{}.png", style_name, i)))
                .collect(),
        };
        styles_map.insert(style_name.clone(), textures);
    }

    let ball_textures = BallTextures {
        styles: styles_map,
        style_order: style_names.clone(),
    };
    commands.insert_resource(ball_textures.clone());

    // Spawn ball
    let ball_style_name = ball_textures.default_style().cloned().unwrap_or_else(|| "wedges".to_string());
    if let Some(textures) = ball_textures.get(&ball_style_name) {
        commands.spawn((
            Sprite {
                image: textures.textures[0].clone(),
                custom_size: Some(BALL_SIZE),
                ..default()
            },
            Transform::from_translation(BALL_SPAWN),
            Ball,
            BallState::default(),
            Velocity::default(),
            BallPlayerContact::default(),
            BallPulse::default(),
            BallRolling::default(),
            BallShotGrace::default(),
            BallSpin::default(),
            BallStyle::new(&ball_style_name),
        ));
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
    levels::spawn_level_platforms(&mut commands, &level_db, level_index, initial_palette.platforms);

    // Baskets
    let initial_level = level_db.get(level_index);
    let basket_y = initial_level
        .map(|l| ARENA_FLOOR_Y + l.basket_height)
        .unwrap_or(ARENA_FLOOR_Y + 400.0);
    let basket_push_in = initial_level.map(|l| l.basket_push_in).unwrap_or(BASKET_PUSH_IN);
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
                Sprite::from_color(initial_palette.right_rim, Vec2::new(RIM_THICKNESS, rim_outer_height)),
                Transform::from_xyz(-BASKET_SIZE.x / 2.0, rim_outer_y, 0.1),
                Platform,
                BasketRim,
            ));
            parent.spawn((
                Sprite::from_color(initial_palette.right_rim, Vec2::new(RIM_THICKNESS, rim_inner_height)),
                Transform::from_xyz(BASKET_SIZE.x / 2.0, rim_inner_y, 0.1),
                Platform,
                BasketRim,
            ));
            parent.spawn((
                Sprite::from_color(initial_palette.right_rim, Vec2::new(rim_bottom_width, RIM_THICKNESS)),
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
                Sprite::from_color(initial_palette.left_rim, Vec2::new(RIM_THICKNESS, rim_inner_height)),
                Transform::from_xyz(-BASKET_SIZE.x / 2.0, rim_inner_y, 0.1),
                Platform,
                BasketRim,
            ));
            parent.spawn((
                Sprite::from_color(initial_palette.left_rim, Vec2::new(RIM_THICKNESS, rim_outer_height)),
                Transform::from_xyz(BASKET_SIZE.x / 2.0, rim_outer_y, 0.1),
                Platform,
                BasketRim,
            ));
            parent.spawn((
                Sprite::from_color(initial_palette.left_rim, Vec2::new(rim_bottom_width, RIM_THICKNESS)),
                Transform::from_xyz(0.0, -BASKET_SIZE.y / 2.0, 0.1),
                Platform,
                BasketRim,
            ));
        });

    // Corner ramps
    let initial_step_count = initial_level.map(|l| l.step_count).unwrap_or(CORNER_STEP_COUNT);
    let initial_corner_height = initial_level.map(|l| l.corner_height).unwrap_or(CORNER_STEP_TOTAL_HEIGHT);
    let initial_corner_width = initial_level.map(|l| l.corner_width).unwrap_or(CORNER_STEP_TOTAL_WIDTH);
    let initial_step_push_in = initial_level.map(|l| l.step_push_in).unwrap_or(STEP_PUSH_IN);
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
            training_state.game_number, training_state.games_total, training_state.current_level_name
        )),
        TextFont { font_size: 20.0, ..default() },
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

    // Log match start
    event_buffer.buffer.log(
        0.0,
        GameEvent::MatchStart {
            level: training_state.current_level,
            level_name: training_state.current_level_name.clone(),
            left_profile: "Player".to_string(),
            right_profile: training_state.ai_profile.clone(),
            seed: rand::random(),
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

/// Training state machine - handles game flow
fn training_state_machine(
    mut training_state: ResMut<TrainingState>,
    mut score: ResMut<Score>,
    mut steal_tracker: ResMut<StealTracker>,
    mut event_buffer: ResMut<TrainingEventBuffer>,
    mut countdown: ResMut<MatchCountdown>,
    balls: Query<&BallState, With<Ball>>,
    time: Res<Time>,
    mut app_exit: MessageWriter<AppExit>,
    level_db: Res<LevelDatabase>,
    mut current_level: ResMut<CurrentLevel>,
) {
    match training_state.phase {
        TrainingPhase::WaitingToStart => {
            // Wait for first ball pickup to start timer
            for ball_state in &balls {
                if matches!(ball_state, BallState::Held(_)) {
                    training_state.start_game_timer();
                    break;
                }
            }
        }

        TrainingPhase::Playing => {
            training_state.update_elapsed();
            event_buffer.elapsed = training_state.game_elapsed;

            // Check win condition
            if score.left >= training_state.win_score || score.right >= training_state.win_score {
                // Log match end
                event_buffer.buffer.log(
                    training_state.game_elapsed,
                    GameEvent::MatchEnd {
                        score_left: score.left,
                        score_right: score.right,
                        duration: training_state.game_elapsed,
                    },
                );

                // Save evlog
                let evlog_path = evlog_path_for_game(&training_state);
                if let Err(e) = write_evlog(&event_buffer, &evlog_path) {
                    eprintln!("Failed to write evlog: {}", e);
                }

                // Record result
                training_state.record_result(score.left, score.right, evlog_path);

                let winner = if score.left >= training_state.win_score { "You win!" } else { "AI wins!" };
                println!(
                    "Game {} complete: {} ({}-{})",
                    training_state.game_number - 1 + 1, // game_number not yet incremented
                    winner,
                    score.left,
                    score.right
                );
            }
        }

        TrainingPhase::Paused => {
            // Do nothing - game is paused, waiting for Start to resume
        }

        TrainingPhase::GameEnded => {
            training_state.transition_timer += time.delta_secs();

            // Wait 2 seconds before prompting for notes
            if training_state.transition_timer > 2.0 {
                training_state.phase = TrainingPhase::AwaitingNotes;
            }
        }

        TrainingPhase::AwaitingNotes => {
            // Prompt for notes (blocking stdin read)
            println!(
                "\nEnter notes for Game {} (or press Enter to skip):",
                training_state.game_number
            );

            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_ok() {
                let notes = input.trim();
                if !notes.is_empty() {
                    // Store notes in last game result
                    if let Some(last_result) = training_state.game_results.last_mut() {
                        last_result.notes = Some(notes.to_string());
                    }
                }
            }

            // Move to next phase
            training_state.transition_timer = 0.0;
            if training_state.game_number >= training_state.games_total {
                training_state.phase = TrainingPhase::SessionComplete;
            } else {
                // Pick new random level
                // Filter out debug levels and Pit (too hard for training)
                let training_levels: Vec<u32> = (0..level_db.len())
                    .filter(|&i| {
                        let level = level_db.get(i);
                        let is_debug = level.map(|l| l.debug).unwrap_or(true);
                        let is_pit = level.map(|l| l.name.to_lowercase() == "pit").unwrap_or(false);
                        !is_debug && !is_pit
                    })
                    .map(|i| (i + 1) as u32)
                    .collect();

                if let Some(&level) = training_levels.choose(&mut rand::thread_rng()) {
                    training_state.current_level = level;
                    training_state.current_level_name = level_db
                        .get((level - 1) as usize)
                        .map(|l| l.name.clone())
                        .unwrap_or_else(|| format!("Level {}", level));
                    current_level.0 = level;
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
                event_buffer.buffer.log(
                    0.0,
                    GameEvent::MatchStart {
                        level: training_state.current_level,
                        level_name: training_state.current_level_name.clone(),
                        left_profile: "Player".to_string(),
                        right_profile: training_state.ai_profile.clone(),
                        seed: rand::random(),
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
    players: Query<
        (
            Entity,
            &Team,
            &Transform,
            &Velocity,
            &ChargingShot,
            &AiState,
            &StealCooldown,
            Option<&HoldingBall>,
            &InputState,
        ),
        With<Player>,
    >,
    balls: Query<(&Transform, &Velocity, &BallState), With<Ball>>,
) {
    if training_state.phase != TrainingPhase::Playing {
        return;
    }

    let time = training_state.game_elapsed;

    // Convert query results to snapshots
    let player_snapshots: Vec<_> = players
        .iter()
        .map(|(entity, team, transform, velocity, charging, ai_state, steal_cooldown, holding, input_state)| {
            snapshot_player(
                entity,
                team,
                transform,
                velocity,
                charging,
                ai_state,
                steal_cooldown,
                holding,
                input_state,
            )
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
        ball_snapshot.as_ref(),
    );
}

/// Check for escape key to quit
fn check_escape_quit(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_exit: MessageWriter<AppExit>,
    training_state: Res<TrainingState>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        println!("\nTraining session cancelled by user.");

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
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut training_state: ResMut<TrainingState>,
    mut score: ResMut<Score>,
    mut steal_tracker: ResMut<StealTracker>,
    mut event_buffer: ResMut<TrainingEventBuffer>,
    mut countdown: ResMut<MatchCountdown>,
    level_db: Res<LevelDatabase>,
    _settings: Res<TrainingSettings>,
    mut current_level: ResMut<CurrentLevel>,
    mut players: Query<&mut Transform, With<Player>>,
    mut balls: Query<(&mut Transform, &mut BallState, &mut Velocity), (With<Ball>, Without<Player>)>,
) {
    // Check for Start button (keyboard P or gamepad Start)
    let start_pressed = keyboard.just_pressed(KeyCode::KeyP)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::Start));

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

    println!("\nRestarting game with new level...");

    // Pick new random level (excluding debug and Pit)
    let training_levels: Vec<u32> = (0..level_db.len())
        .filter(|&i| {
            let level = level_db.get(i);
            let is_debug = level.map(|l| l.debug).unwrap_or(true);
            let is_pit = level.map(|l| l.name.to_lowercase() == "pit").unwrap_or(false);
            !is_debug && !is_pit
        })
        .map(|i| (i + 1) as u32)
        .collect();

    if let Some(&level) = training_levels.choose(&mut rand::thread_rng()) {
        training_state.current_level = level;
        training_state.current_level_name = level_db
            .get((level - 1) as usize)
            .map(|l| l.name.clone())
            .unwrap_or_else(|| format!("Level {}", level));
        current_level.0 = level;
    }

    // Reset score and steal tracker
    score.left = 0;
    score.right = 0;
    steal_tracker.reset();

    // Reset players to spawn positions
    for mut transform in &mut players {
        if transform.translation.x < 0.0 {
            transform.translation = PLAYER_SPAWN_LEFT;
        } else {
            transform.translation = PLAYER_SPAWN_RIGHT;
        }
    }

    // Reset ball
    for (mut transform, mut ball_state, mut velocity) in &mut balls {
        transform.translation = BALL_SPAWN;
        *ball_state = BallState::Free;
        velocity.0 = Vec2::ZERO;
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
    event_buffer.buffer.log(
        0.0,
        GameEvent::MatchStart {
            level: training_state.current_level,
            level_name: training_state.current_level_name.clone(),
            left_profile: "Player".to_string(),
            right_profile: training_state.ai_profile.clone(),
            seed: rand::random(),
        },
    );

    println!(
        "Game {}/{} on {}",
        training_state.game_number,
        training_state.games_total,
        training_state.current_level_name
    );
}

/// Write evlog to file
fn write_evlog(event_buffer: &TrainingEventBuffer, path: &std::path::Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let content = event_buffer.buffer.serialize();
    let mut file = File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}
