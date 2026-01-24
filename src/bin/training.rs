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
    JumpState, LastShotInfo, LevelDatabase, NavGraph, PALETTES_FILE, PaletteDatabase, PhysicsTweaks,
    PlayerId, Player, PlayerInput, Score, SnapshotConfig, StealContest, StealCooldown,
    StyleTextures, TargetBasket, Team, Velocity, ai, ball, constants::*, helpers::*, input, levels,
    player, scoring, shooting, steal, world,
};
use ballgame::training::{
    TrainingPhase, TrainingState, ensure_session_dir, evlog_path_for_game, print_session_summary,
    write_session_summary,
};
use bevy::{camera::ScalingMode, prelude::*};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write as IoWrite;
use world::{Basket, BasketRim, Collider, Platform};

/// Path to ball options file
const BALL_OPTIONS_FILE: &str = "assets/ball_options.txt";

/// Parse command-line arguments
struct TrainingArgs {
    games: u32,
    profile: String,
}

impl Default for TrainingArgs {
    fn default() -> Self {
        Self {
            games: 1,
            profile: "Balanced".to_string(),
        }
    }
}

fn parse_args() -> TrainingArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut result = TrainingArgs::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--games" | "-g" => {
                if let Some(val) = args.get(i + 1) {
                    result.games = val.parse().unwrap_or(5);
                    i += 1;
                }
            }
            "--profile" | "-p" => {
                if let Some(val) = args.get(i + 1) {
                    result.profile = val.clone();
                    i += 1;
                }
            }
            "--help" | "-h" => {
                println!("Training Mode - Play against AI and collect analysis data");
                println!();
                println!("Usage: cargo run --bin training [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -g, --games N       Number of games (default: 1)");
                println!("  -p, --profile NAME  AI profile (default: Balanced)");
                println!("  -h, --help          Show this help");
                println!();
                println!("Available profiles: Balanced, Aggressive, Defensive, Sniper,");
                println!("                    Rusher, Turtle, Chaotic, Patient, Hunter, Goalie");
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }

    result
}

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
    let args = parse_args();

    println!("========================================");
    println!("       TRAINING MODE");
    println!("========================================");
    println!();
    println!("  Games: {}", args.games);
    println!("  AI Profile: {}", args.profile);
    println!();
    println!("  Controls:");
    println!("    A/D or Left Stick: Move");
    println!("    Space/W or South: Jump");
    println!("    E or West: Pickup/Steal");
    println!("    F or RB: Throw (hold to charge)");
    println!("    R or Start: Restart with new level");
    println!("    Escape: Quit training session");
    println!();
    println!("  First to 5 points wins each game.");
    println!("========================================");
    println!();

    // Load level database from file
    let level_db = LevelDatabase::load_from_file(LEVELS_FILE);

    // Load palette database
    let palette_db = PaletteDatabase::load_or_create(PALETTES_FILE);

    // Get initial background color from first palette
    let initial_bg = palette_db
        .get(0)
        .map(|p| p.background)
        .unwrap_or(DEFAULT_BACKGROUND_COLOR);

    // Create training state
    let mut training_state = TrainingState::new(args.games, &args.profile);

    // Pick initial random non-debug level
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
    }

    // Ensure session directory exists
    if let Err(e) = ensure_session_dir(&training_state) {
        eprintln!("Failed to create session directory: {}", e);
        return;
    }

    println!("Starting Game 1/{} on {}", args.games, training_state.current_level_name);
    println!();

    // Viewport setup
    let (viewport_width, viewport_height, _) = VIEWPORT_PRESETS[0];

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
        .insert_resource(training_state)
        .init_resource::<PlayerInput>()
        .init_resource::<DebugSettings>()
        .init_resource::<StealContest>()
        .init_resource::<Score>()
        .insert_resource(CurrentLevel(1)) // Will be set from training state
        .insert_resource(CurrentPalette(0))
        .init_resource::<PhysicsTweaks>()
        .init_resource::<LastShotInfo>()
        .init_resource::<AiProfileDatabase>()
        .init_resource::<NavGraph>()
        .insert_resource(SnapshotConfig::default())
        .init_resource::<TrainingEventBuffer>()
        // Startup systems
        .add_systems(Startup, training_setup)
        // Input systems chain
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
                .chain(),
        )
        // Core Update systems
        .add_systems(
            Update,
            (
                player::respawn_player,
                steal::steal_cooldown_update,
                ballgame::ui::animate_pickable_ball,
                ballgame::ui::update_charge_gauge,
                ballgame::ui::update_steal_indicators,
            ),
        )
        // Training-specific systems
        .add_systems(
            Update,
            (
                training_state_machine,
                update_training_hud,
                emit_training_events,
                check_escape_quit,
                check_restart,
            ),
        )
        // Fixed update physics chain
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
                .chain(),
        )
        .run();
}

/// Event buffer for training mode logging
#[derive(Resource, Default)]
pub struct TrainingEventBuffer {
    pub buffer: EventBuffer,
    /// Track previous score for detecting score changes
    pub prev_score_left: u32,
    pub prev_score_right: u32,
    /// Track previous ball holder for possession events
    pub prev_ball_holder: Option<Entity>,
    /// Track previous charging state for shot events
    pub prev_charging: [bool; 2],
    /// Track last tick time for 50ms sampling
    pub last_tick_time: f32,
    /// Frame counter for tick events
    pub tick_frame_count: u64,
    /// Track previous AI goals for change detection
    pub prev_ai_goal: Option<String>,
    /// Track previous steal cooldowns for steal detection
    pub prev_steal_cooldowns: [f32; 2],
    /// Track elapsed time
    pub elapsed: f32,
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
    mut event_buffer: ResMut<TrainingEventBuffer>,
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

            // Check win condition (first to 5)
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

        TrainingPhase::GameEnded => {
            training_state.transition_timer += time.delta_secs();

            // Wait 2 seconds before next game
            if training_state.transition_timer > 2.0 {
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

                    // Reset score
                    score.left = 0;
                    score.right = 0;

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
fn emit_training_events(
    mut event_buffer: ResMut<TrainingEventBuffer>,
    training_state: Res<TrainingState>,
    score: Res<Score>,
    steal_contest: Res<StealContest>,
    players: Query<
        (Entity, &Team, &Transform, &Velocity, &ChargingShot, &AiState, &StealCooldown, Option<&HoldingBall>),
        With<Player>,
    >,
    balls: Query<(&Transform, &Velocity, &BallState), With<Ball>>,
) {
    if training_state.phase != TrainingPhase::Playing {
        return;
    }

    let time = training_state.game_elapsed;

    // Tick events at 50ms (20 Hz)
    if time - event_buffer.last_tick_time >= 0.05 {
        event_buffer.last_tick_time = time;
        event_buffer.tick_frame_count += 1;
        let frame = event_buffer.tick_frame_count;

        let mut left_pos = (0.0, 0.0);
        let mut left_vel = (0.0, 0.0);
        let mut right_pos = (0.0, 0.0);
        let mut right_vel = (0.0, 0.0);

        for (_, team, transform, velocity, _, _, _, _) in &players {
            let pos = (transform.translation.x, transform.translation.y);
            let vel = (velocity.0.x, velocity.0.y);
            match team {
                Team::Left => {
                    left_pos = pos;
                    left_vel = vel;
                }
                Team::Right => {
                    right_pos = pos;
                    right_vel = vel;
                }
            }
        }

        let mut ball_pos = (0.0, 0.0);
        let mut ball_vel = (0.0, 0.0);
        let mut ball_state_char = 'F';

        for (transform, velocity, ball_state) in &balls {
            ball_pos = (transform.translation.x, transform.translation.y);
            ball_vel = (velocity.0.x, velocity.0.y);
            ball_state_char = match ball_state {
                BallState::Free => 'F',
                BallState::Held(_) => 'H',
                BallState::InFlight { .. } => 'I',
            };
            break;
        }

        event_buffer.buffer.log(
            time,
            GameEvent::Tick {
                frame,
                left_pos,
                left_vel,
                right_pos,
                right_vel,
                ball_pos,
                ball_vel,
                ball_state: ball_state_char,
            },
        );
    }

    // Score changes
    if score.left > event_buffer.prev_score_left {
        event_buffer.buffer.log(
            time,
            GameEvent::Goal {
                player: PlayerId::L,
                score_left: score.left,
                score_right: score.right,
            },
        );
        event_buffer.prev_score_left = score.left;
    }
    if score.right > event_buffer.prev_score_right {
        event_buffer.buffer.log(
            time,
            GameEvent::Goal {
                player: PlayerId::R,
                score_left: score.left,
                score_right: score.right,
            },
        );
        event_buffer.prev_score_right = score.right;
    }

    // AI goal changes (right player only)
    for (_, team, _, _, _, ai_state, _, _) in &players {
        if *team == Team::Right {
            let goal_str = format!("{:?}", ai_state.current_goal);
            if event_buffer.prev_ai_goal.as_ref() != Some(&goal_str) {
                event_buffer.prev_ai_goal = Some(goal_str.clone());
                event_buffer.buffer.log(
                    time,
                    GameEvent::AiGoal {
                        player: PlayerId::R,
                        goal: goal_str,
                    },
                );
            }
        }
    }

    // Steal detection
    for (_, team, _, _, _, _, steal_cooldown, _) in &players {
        let idx = match team {
            Team::Left => 0,
            Team::Right => 1,
        };
        let player_id = match team {
            Team::Left => PlayerId::L,
            Team::Right => PlayerId::R,
        };

        let current_cooldown = steal_cooldown.0;
        let prev_cooldown = event_buffer.prev_steal_cooldowns[idx];

        if current_cooldown > prev_cooldown + 0.3 && current_cooldown > 0.5 {
            event_buffer.buffer.log(time, GameEvent::StealAttempt { attacker: player_id });
            if steal_contest.fail_flash_timer > 0.0 {
                event_buffer.buffer.log(time, GameEvent::StealFail { attacker: player_id });
            } else {
                event_buffer.buffer.log(time, GameEvent::StealSuccess { attacker: player_id });
            }
        }
        event_buffer.prev_steal_cooldowns[idx] = current_cooldown;
    }

    // Ball possession changes
    for (entity, team, transform, _, charging, _, _, holding) in &players {
        let player_id = match team {
            Team::Left => PlayerId::L,
            Team::Right => PlayerId::R,
        };
        let idx = match team {
            Team::Left => 0,
            Team::Right => 1,
        };

        let is_holding = holding.is_some();
        let was_holding = event_buffer.prev_ball_holder == Some(entity);

        if is_holding && !was_holding {
            event_buffer.buffer.log(time, GameEvent::Pickup { player: player_id });
            event_buffer.prev_ball_holder = Some(entity);
        }

        let is_charging = charging.charge_time > 0.0;
        if is_charging && !event_buffer.prev_charging[idx] {
            event_buffer.buffer.log(
                time,
                GameEvent::ShotStart {
                    player: player_id,
                    pos: (transform.translation.x, transform.translation.y),
                    quality: 0.5,
                },
            );
        }
        event_buffer.prev_charging[idx] = is_charging;
    }

    // Ball state changes (drop/shot release)
    for (_, _, ball_state) in &balls {
        match ball_state {
            BallState::InFlight { shooter, power } => {
                if event_buffer.prev_ball_holder.is_some() {
                    let player_id = players
                        .iter()
                        .find(|(e, _, _, _, _, _, _, _)| *e == *shooter)
                        .map(|(_, team, _, _, _, _, _, _)| match team {
                            Team::Left => PlayerId::L,
                            Team::Right => PlayerId::R,
                        });

                    if let Some(pid) = player_id {
                        event_buffer.buffer.log(
                            time,
                            GameEvent::ShotRelease {
                                player: pid,
                                charge: 0.5,
                                angle: 60.0,
                                power: *power,
                            },
                        );
                    }
                    event_buffer.prev_ball_holder = None;
                }
            }
            BallState::Free => {
                if event_buffer.prev_ball_holder.is_some() {
                    if let Some((_, team, _, _, _, _, _, _)) = players
                        .iter()
                        .find(|(e, _, _, _, _, _, _, _)| Some(*e) == event_buffer.prev_ball_holder)
                    {
                        let player_id = match team {
                            Team::Left => PlayerId::L,
                            Team::Right => PlayerId::R,
                        };
                        event_buffer.buffer.log(time, GameEvent::Drop { player: player_id });
                    }
                    event_buffer.prev_ball_holder = None;
                }
            }
            BallState::Held(_) => {}
        }
    }
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

/// Check for Start button to restart game (during GameEnded phase)
fn check_restart(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut training_state: ResMut<TrainingState>,
    mut score: ResMut<Score>,
    mut event_buffer: ResMut<TrainingEventBuffer>,
    level_db: Res<LevelDatabase>,
    mut current_level: ResMut<CurrentLevel>,
    mut players: Query<&mut Transform, With<Player>>,
    mut balls: Query<(&mut Transform, &mut BallState, &mut Velocity), (With<Ball>, Without<Player>)>,
) {
    // Only allow restart during GameEnded phase
    if training_state.phase != TrainingPhase::GameEnded {
        return;
    }

    // Check for Start button (keyboard R or gamepad Start)
    let start_pressed = keyboard.just_pressed(KeyCode::KeyR)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::Start));

    if !start_pressed {
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

    // Reset score
    score.left = 0;
    score.right = 0;

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
