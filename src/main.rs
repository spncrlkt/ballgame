//! Ballgame - A 2v2 ball sport game built with Bevy
//!
//! Main entry point: app setup and system registration.

use ballgame::ui::spawn_steal_indicators;
use ballgame::{
    AiCapabilities, AiGoal, AiNavState, AiProfileDatabase, AiState, Ball, BallPlayerContact,
    BallPulse, BallRolling, BallShotGrace, BallSpin, BallState, BallStyle, BallTextures,
    ChargeGaugeBackground, ChargeGaugeFill, ChargingShot, ConfigWatcher, CoyoteTimer, CurrentLevel,
    CurrentPalette, CurrentPresets, CurrentSettings, CycleIndicator, CycleSelection,
    DebugLogConfig, DebugSettings, DebugText, DisplayBallWave, EventBus, Facing, Grounded,
    HumanControlTarget, HumanControlled, InputState, JumpState, LastShotInfo, LevelChangeTracker,
    LevelDatabase, MatchCountdown, NavGraph, PALETTES_FILE, PRESETS_FILE, PaletteDatabase,
    PhysicsTweaks, Player, PlayerId, PlayerInput, PresetDatabase, Score, ScoreLevelText,
    SnapshotConfig, SnapshotTriggerState, StealContest, StealCooldown, StealTracker, StyleTextures,
    TargetBasket, Team, TweakPanel, TweakPanelState, TweakRow, Velocity, ViewportScale, ai,
    apply_preset_to_tweaks, ball, config_watcher, constants::*, countdown, display_ball_wave,
    emit_level_change_events, input, levels, player, replay, save_settings_system, scoring,
    shooting, snapshot, spawn_countdown_text, steal, tuning, ui, update_event_bus_time, world,
};
use bevy::{camera::ScalingMode, diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use world::{Basket, Collider};

/// Path to ball options file
const BALL_OPTIONS_FILE: &str = "config/ball_options.txt";
const DEFAULT_REPLAY_DB: &str = "db/training.db";
const DEFAULT_REPLAY_TIMEOUT_SECS: f32 = 5.0;

/// Parse ball_options.txt to get list of style names
fn load_ball_style_names() -> Vec<String> {
    let content = fs::read_to_string(BALL_OPTIONS_FILE).unwrap_or_else(|e| {
        warn!("Could not read ball options file: {}, using defaults", e);
        return String::new();
    });

    let mut styles = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix("style:") {
            styles.push(name.trim().to_string());
        }
    }

    if styles.is_empty() {
        // Fallback defaults
        styles = vec!["wedges".to_string(), "half".to_string()];
    }

    info!("Loaded {} ball styles: {:?}", styles.len(), styles);
    styles
}

fn main() {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let screenshot_and_quit = args.iter().any(|a| a == "--screenshot-and-quit");

    // Check for --level <name> override (accepts level name, looked up at runtime)
    let level_name_override = args
        .iter()
        .position(|a| a == "--level")
        .and_then(|i| args.get(i + 1).cloned());

    // Check for --viewport <width> <height> override
    let viewport_override = args.iter().position(|a| a == "--viewport").and_then(|i| {
        let width = args.get(i + 1).and_then(|s| s.parse::<f32>().ok())?;
        let height = args.get(i + 2).and_then(|s| s.parse::<f32>().ok())?;
        Some((width, height))
    });

    // Check for --palette <index> override
    let palette_override = args
        .iter()
        .position(|a| a == "--palette")
        .and_then(|i| args.get(i + 1).and_then(|s| s.parse::<usize>().ok()));

    // Check for --freeze-countdown flag
    let freeze_countdown = args.iter().any(|a| a == "--freeze-countdown");

    // Check for replay mode: --replay-db <match_id>
    let replay_db_match_id = args
        .iter()
        .position(|a| a == "--replay-db")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<i64>().ok());

    // Check for replay timeout: --replay-timeout <secs>
    let replay_timeout_secs = args.iter().position(|a| a == "--replay-timeout").map(|i| {
        args.get(i + 1)
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(DEFAULT_REPLAY_TIMEOUT_SECS)
    });

    // Load persistent settings (uses defaults if file doesn't exist)
    let current_settings = CurrentSettings::default();

    // Save settings on first run to ensure file exists
    if let Err(e) = current_settings.settings.save() {
        warn!("Failed to save initial settings: {}", e);
    }

    // Load level database from file (needed for level name lookup)
    let level_db = LevelDatabase::load_from_file(LEVELS_FILE);

    // Resolve level: CLI name override -> saved settings -> first level
    // Supports: level ID (16-char hex), level name, or level number (backward compat)
    let resolve_level_id = |input: &str| -> Option<String> {
        // Empty string = use first level
        if input.is_empty() {
            return level_db.all().first().map(|l| l.id.clone());
        }
        // Try as level ID first (16-char hex)
        if input.len() == 16 && input.chars().all(|c| c.is_ascii_hexdigit()) {
            if level_db.get_by_id(input).is_some() {
                return Some(input.to_string());
            }
        }
        // Try as level number (backward compatibility)
        if let Ok(num) = input.parse::<usize>() {
            if num > 0 {
                return level_db.get(num - 1).map(|l| l.id.clone());
            }
        }
        // Try as level name
        level_db.get_by_name(input).map(|l| l.id.clone())
    };

    // Resolve CLI override first, then settings, then default to first level
    let loaded_level_id = level_name_override
        .as_ref()
        .and_then(|s| resolve_level_id(s))
        .or_else(|| resolve_level_id(&current_settings.settings.level))
        .unwrap_or_else(|| {
            level_db
                .all()
                .first()
                .map(|l| l.id.clone())
                .unwrap_or_default()
        });

    // Extract values from loaded settings for resource initialization
    let loaded_viewport_index = current_settings.settings.viewport_index;
    let loaded_palette_index = palette_override.unwrap_or(current_settings.settings.palette_index);
    let loaded_active_direction = current_settings.settings.active_direction.clone();
    let loaded_down_option = current_settings.settings.down_option.clone();
    let loaded_right_option = current_settings.settings.right_option.clone();

    // Check if initial level is a regression level (for countdown freezing)
    let is_regression_level = level_db
        .get_by_id(&loaded_level_id)
        .map(|l| l.regression)
        .unwrap_or(false);

    // Check if countdown should be frozen (regression level or explicit flag)
    let should_freeze_countdown = is_regression_level || freeze_countdown;

    // Load palette database (creates default file if missing)
    let palette_db = PaletteDatabase::load_or_create(PALETTES_FILE);

    // Load preset database
    let preset_db = PresetDatabase::load_from_file(PRESETS_FILE);

    // Get initial background color from first palette
    let initial_bg = palette_db
        .get(0)
        .map(|p| p.background)
        .unwrap_or(DEFAULT_BACKGROUND_COLOR);

    // Use viewport override or loaded preset (clamped to valid range)
    let (viewport_width, viewport_height) = if let Some((w, h)) = viewport_override {
        (w, h)
    } else {
        let viewport_index = loaded_viewport_index.min(VIEWPORT_PRESETS.len() - 1);
        let (w, h, _) = VIEWPORT_PRESETS[viewport_index];
        (w, h)
    };

    let args: Vec<String> = std::env::args().collect();
    let debug_config = DebugLogConfig::load_with_args(&args);
    debug_config.apply_env();

    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    // Use loaded viewport preset for initial size
                    // Set scale_factor_override to 1.0 for consistent behavior on HiDPI displays
                    resolution: bevy::window::WindowResolution::new(
                        viewport_width as u32,
                        viewport_height as u32,
                    )
                    .with_scale_factor_override(1.0),
                    title: "Ballgame".into(),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .insert_resource(ClearColor(initial_bg))
        .insert_resource(palette_db)
        .insert_resource(preset_db)
        .insert_resource(level_db)
        .insert_resource(current_settings)
        .init_resource::<PlayerInput>()
        .init_resource::<DebugSettings>()
        .init_resource::<StealContest>()
        .init_resource::<StealTracker>()
        .init_resource::<Score>()
        .insert_resource(CurrentLevel(loaded_level_id))
        .insert_resource(CurrentPalette(loaded_palette_index))
        .insert_resource(debug_config)
        .init_resource::<PhysicsTweaks>()
        .init_resource::<TweakPanelState>()
        .init_resource::<LastShotInfo>()
        .insert_resource(ViewportScale {
            preset_index: loaded_viewport_index,
        })
        .insert_resource(CycleSelection {
            active_direction: ui::CycleDirection::from_str(&loaded_active_direction),
            down_option: ui::DownOption::from_str(&loaded_down_option),
            right_option: ui::RightOption::from_str(&loaded_right_option),
            ai_player_index: 0,
            menu_enabled: false,
        })
        .init_resource::<ConfigWatcher>()
        .init_resource::<AiProfileDatabase>()
        .init_resource::<CurrentPresets>()
        .init_resource::<NavGraph>()
        .init_resource::<AiCapabilities>()
        .init_resource::<ai::HeatmapBundle>()
        // Event bus for cross-module communication
        .insert_resource(EventBus::new())
        // Human control target (initialized in setup based on settings)
        .init_resource::<HumanControlTarget>()
        // Level change tracker for event emission
        .init_resource::<LevelChangeTracker>()
        .insert_resource(SnapshotConfig {
            // Only enable screenshots when running via screenshot script
            enabled: screenshot_and_quit,
            exit_after_startup: screenshot_and_quit,
            ..default()
        })
        .init_resource::<SnapshotTriggerState>()
        .init_resource::<DisplayBallWave>()
        // Initialize countdown (frozen if regression level or --freeze-countdown flag)
        .insert_resource(if should_freeze_countdown {
            let mut countdown = MatchCountdown::default();
            countdown.start_frozen();
            countdown
        } else {
            MatchCountdown::default()
        })
        // Replay mode resources
        .insert_resource(if let Some(match_id) = replay_db_match_id {
            replay::ReplayMode::new_db(match_id)
        } else {
            replay::ReplayMode::default()
        })
        .insert_resource(ReplayTimeout {
            remaining_secs: replay_timeout_secs.unwrap_or(0.0),
            active: replay_timeout_secs.is_some(),
        })
        .init_resource::<replay::ReplayState>()
        // Startup system - use normal setup only when NOT in replay mode
        .add_systems(Startup, tuning::load_global_tuning_system)
        .add_systems(Startup, setup.run_if(replay::not_replay_active))
        // =========== NORMAL GAME SYSTEMS (disabled in replay mode) ===========
        // Countdown system - always runs to update timer and text
        .add_systems(
            Update,
            countdown::update_countdown.run_if(replay::not_replay_active),
        )
        // Event bus time update (runs every frame for timestamping)
        .add_systems(
            Update,
            update_event_bus_time.run_if(replay::not_replay_active),
        )
        // Input systems must run in order: capture -> copy -> swap -> nav graph -> nav -> AI
        // Only runs when NOT in countdown and NOT in replay mode
        .add_systems(
            Update,
            (
                input::capture_input,
                ai::copy_human_input,
                ai::swap_control,
                ai::mark_nav_dirty_on_level_change,
                ai::load_heatmaps_on_level_change,
                ai::rebuild_nav_graph,
                ai::ai_navigation_update,
                ai::ai_decision_update,
            )
                .chain()
                .run_if(replay::not_replay_active.and(countdown::not_in_countdown)),
        )
        // Settings reset (double-click Start) - must run before respawn
        .add_systems(
            Update,
            player::check_settings_reset.run_if(replay::not_replay_active),
        )
        // Core Update systems - split to avoid tuple issues with respawn_player
        .add_systems(
            Update,
            player::respawn_player.run_if(replay::not_replay_active),
        )
        // Emit level change events for auditability (runs after systems that change level)
        .add_systems(
            Update,
            emit_level_change_events.run_if(replay::not_replay_active),
        )
        // Countdown trigger on level change (only in manual game mode)
        .add_systems(
            Update,
            countdown::trigger_countdown_on_level_change.run_if(replay::not_replay_active),
        )
        .add_systems(
            Update,
            (ui::toggle_debug, config_watcher::check_config_changes)
                .run_if(replay::not_replay_active),
        )
        .add_systems(
            Update,
            (ui::update_debug_text, ui::update_score_level_text).run_if(replay::not_replay_active),
        )
        .add_systems(
            Update,
            (
                ui::animate_pickable_ball,
                ui::animate_score_flash,
                ui::update_charge_gauge,
                ui::update_steal_indicators,
                display_ball_wave,
                player::manage_debug_display,
            )
                .run_if(replay::not_replay_active),
        )
        // UI panel and cycle systems
        .add_systems(
            Update,
            (
                ui::toggle_tweak_panel,
                ui::update_tweak_panel,
                ui::cycle_viewport,
                ui::unified_cycle_system,
            )
                .run_if(replay::not_replay_active),
        )
        // Cycle indicator, palette application, and preset application
        .add_systems(
            Update,
            (
                ui::update_cycle_indicator,
                ui::apply_palette_colors,
                apply_preset_to_tweaks,
            )
                .run_if(replay::not_replay_active),
        )
        // Snapshot system - captures game state on events
        .add_systems(
            Update,
            (
                snapshot::snapshot_trigger_system,
                snapshot::toggle_snapshot_system,
                snapshot::toggle_screenshot_capture,
                snapshot::manual_snapshot,
            )
                .run_if(replay::not_replay_active),
        )
        // Settings persistence - save when dirty
        .add_systems(
            Update,
            save_settings_system.run_if(replay::not_replay_active),
        )
        .add_systems(Update, replay_timeout.run_if(replay::replay_active))
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
                .run_if(replay::not_replay_active.and(countdown::not_in_countdown)),
        )
        // =========== REPLAY MODE SYSTEMS ===========
        // Replay startup - load file, setup camera
        .add_systems(Startup, replay_load_file.run_if(replay::replay_active))
        // Replay setup - spawn game world (runs after load, needs ReplayData)
        .add_systems(
            Startup,
            (replay::replay_setup, replay::setup_replay_ui)
                .run_if(replay::replay_active)
                .after(replay_load_file),
        )
        // Replay update systems
        .add_systems(
            Update,
            (
                replay::replay_playback,
                replay::replay_input_handler,
                replay::update_replay_ui,
            )
                .chain()
                .run_if(replay::replay_active),
        )
        .run();
}

/// Setup the game world
fn setup(
    mut commands: Commands,
    level_db: Res<LevelDatabase>,
    palette_db: Res<PaletteDatabase>,
    asset_server: Res<AssetServer>,
    current_palette: Res<CurrentPalette>,
    current_level: Res<CurrentLevel>,
    current_settings: Res<CurrentSettings>,
    profile_db: Res<AiProfileDatabase>,
    mut human_target: ResMut<HumanControlTarget>,
) {
    // Camera - orthographic, shows entire arena
    // FixedVertical ensures the full arena height is always visible regardless of window size
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

    // Get palette colors from loaded settings (clamped to valid range)
    let palette_index = current_palette.0.min(palette_db.len().saturating_sub(1));
    let initial_palette = palette_db.get(palette_index).expect("No palettes loaded");

    // Get level data from current level ID
    let level_data = level_db.get_by_id(&current_level.0);

    // Load AI profile IDs for players (use name lookup, fall back to first profile)
    let left_ai_profile_id = current_settings
        .settings
        .left_ai_profile
        .as_ref()
        .and_then(|name| profile_db.get_by_name(name))
        .map(|p| p.id.clone())
        .unwrap_or_else(|| profile_db.default_profile().id.clone());
    let right_ai_profile_id = profile_db
        .get_by_name(&current_settings.settings.right_ai_profile)
        .map(|p| p.id.clone())
        .unwrap_or_else(|| profile_db.default_profile().id.clone());

    // Determine if left player is human or AI based on settings
    let left_is_human = current_settings.settings.left_ai_profile.is_none();

    // Initialize HumanControlTarget based on whether left player is human-controlled
    human_target.0 = if left_is_human {
        Some(PlayerId::L)
    } else {
        None // Observer mode or all AI
    };

    // Check if this is a debug or regression level early (for AI goal)
    let is_special_level = level_data.map(|l| l.debug || l.regression).unwrap_or(false);

    // Left team player - spawns on left side
    let left_player = commands
        .spawn((
            Sprite::from_color(initial_palette.left, PLAYER_SIZE),
            Transform::from_translation(PLAYER_SPAWN_LEFT),
            (
                Player,
                Velocity::default(),
                Grounded(false),
                CoyoteTimer::default(),
            ),
            (
                JumpState::default(),
                Facing::default(),
                ChargingShot::default(),
            ),
            TargetBasket(Basket::Right), // Left team scores in right basket
            Collider,
            Team::Left,
            (
                InputState::default(),
                AiState {
                    current_goal: if is_special_level {
                        AiGoal::Idle
                    } else {
                        AiGoal::default()
                    },
                    profile_id: left_ai_profile_id,
                    ..default()
                },
                AiNavState::default(),
                StealCooldown::default(),
            ),
        ))
        .id();

    // Conditionally add HumanControlled marker to left player
    if left_is_human {
        commands.entity(left_player).insert(HumanControlled);
    }

    // Right team player - spawns on right side, starts AI-controlled
    let right_player = commands
        .spawn((
            Sprite::from_color(initial_palette.right, PLAYER_SIZE),
            Transform::from_translation(PLAYER_SPAWN_RIGHT),
            (
                Player,
                Velocity::default(),
                Grounded(false),
                CoyoteTimer::default(),
            ),
            (JumpState::default(), Facing(-1.0), ChargingShot::default()),
            TargetBasket(Basket::Left), // Right team scores in left basket
            Collider,
            Team::Right,
            (
                InputState::default(),
                AiState {
                    // On debug level, AI stands still (Idle); otherwise normal AI
                    current_goal: if is_special_level {
                        AiGoal::Idle
                    } else {
                        AiGoal::default()
                    },
                    profile_id: right_ai_profile_id,
                    ..default()
                },
                AiNavState::default(),
                StealCooldown::default(),
            ),
        ))
        .id();

    // Charge gauge - inside player, opposite side of ball
    // Start on left side (default facing is right, so ball is right, gauge is left)
    let gauge_x = -PLAYER_SIZE.x / 4.0;

    // Background (black bar, always visible, centered vertically on player)
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

    // Fill (green->red, scales with charge) - starts invisible
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

    // Charge gauge for right player (faces left, so gauge is on right side)
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

    // Steal indicators for both players
    spawn_steal_indicators(&mut commands, left_player, 1.0); // Left player faces right
    spawn_steal_indicators(&mut commands, right_player, -1.0); // Right player faces left

    // Load ball style names from config file
    let style_names = load_ball_style_names();
    let num_palettes = palette_db.len();

    // Load ball textures for all styles dynamically
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

    // Check if this is a debug level (spawns all ball styles, AI idle)
    let is_debug_level = level_data.map(|l| l.debug).unwrap_or(false);

    if is_debug_level {
        // Debug level: spawn ALL ball styles on shelf platforms with labels
        player::spawn_debug_display(&mut commands, &ball_textures, palette_index);

        // Spawn one random playable ball on the floor
        let random_idx = rand::random::<usize>() % style_names.len();
        let random_style = &style_names[random_idx];
        if let Some(textures) = ball_textures.get(random_style) {
            commands.spawn((
                Sprite {
                    image: textures.textures[palette_index].clone(),
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
                BallStyle::new(random_style),
            ));
        }
    } else {
        // Normal levels: spawn single ball with loaded style (or default if not found)
        let loaded_style = &current_settings.settings.ball_style;
        let ball_style_name = if ball_textures.get(loaded_style).is_some() {
            loaded_style.clone()
        } else {
            ball_textures
                .default_style()
                .cloned()
                .unwrap_or_else(|| "wedges".to_string())
        };
        if let Some(textures) = ball_textures.get(&ball_style_name) {
            commands.spawn((
                Sprite {
                    image: textures.textures[palette_index].clone(),
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
    }

    // Arena floor and walls (shared spawning functions)
    world::spawn_floor(&mut commands, initial_palette.platforms);
    world::spawn_walls(&mut commands, initial_palette.platforms);

    // Spawn level platforms for the loaded level
    levels::spawn_level_platforms(
        &mut commands,
        &level_db,
        &current_level.0,
        initial_palette.platforms,
    );

    // Baskets with rims (shared spawning function)
    let initial_level = level_data;
    let basket_y = initial_level
        .map(|l| ARENA_FLOOR_Y + l.basket_height)
        .unwrap_or(ARENA_FLOOR_Y + 400.0);
    let basket_push_in = initial_level
        .map(|l| l.basket_push_in)
        .unwrap_or(BASKET_PUSH_IN);
    world::spawn_baskets(
        &mut commands,
        basket_y,
        basket_push_in,
        initial_palette.left,
        initial_palette.right,
        initial_palette.left_rim,
        initial_palette.right_rim,
    );

    // Corner ramps - angled walls in bottom corners (reuse initial_level from earlier)
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

    // Score/Level display - world space, above arena
    commands.spawn((
        Text2d::new("Score"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(TEXT_PRIMARY),
        Transform::from_xyz(0.0, ARENA_HEIGHT / 2.0 - 30.0, 1.0),
        ScoreLevelText,
    ));

    // Debug UI - world space, centered on floor
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(TEXT_PRIMARY),
        Transform::from_xyz(0.0, ARENA_FLOOR_Y + 10.0, 1.0),
        DebugText,
    ));

    // Cycle indicator - 4 separate lines for individual styling
    // Each line can have different font size when selected
    // Position: well inside visible area (camera uses FixedVertical, so horizontal extent varies)
    let cycle_base_x = -ARENA_WIDTH / 2.0 + WALL_THICKNESS + 120.0;
    let cycle_base_y = ARENA_HEIGHT / 2.0 - 30.0;
    let cycle_line_spacing = 22.0;

    for i in 0..4 {
        commands.spawn((
            Text2d::new(""),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextLayout::new_with_justify(Justify::Left),
            TextColor(TEXT_ACCENT),
            Transform::from_xyz(
                cycle_base_x,
                cycle_base_y - (i as f32 * cycle_line_spacing),
                1.0,
            ),
            CycleIndicator(i),
        ));
    }

    // Physics Tweak Panel (hidden by default, toggle with F1)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
            Visibility::Hidden,
            TweakPanel,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("Physics Tweaks (F1 to close)"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(TEXT_PRIMARY),
            ));
            parent.spawn((
                Text::new("Up/Down: select | Left/Right: +/-10%"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(TEXT_SECONDARY),
            ));
            parent.spawn((
                Text::new("R: reset selected | Shift+R: reset all"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(TEXT_SECONDARY),
            ));

            // Create a row for each tweakable parameter
            for i in 0..PhysicsTweaks::LABELS.len() {
                parent.spawn((
                    Text::new(format!("{}: ---", PhysicsTweaks::LABELS[i])),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(TEXT_PRIMARY),
                    TweakRow(i),
                ));
            }
        });

    // Countdown text (3-2-1 before match starts)
    spawn_countdown_text(&mut commands);
}

/// Setup system for replay mode - loads replay data
fn replay_load_file(mut commands: Commands, replay_mode: Res<replay::ReplayMode>) {
    let replay_result = if let Some(match_id) = replay_mode.match_id {
        replay::load_replay_from_db(Path::new(DEFAULT_REPLAY_DB), match_id)
            .map_err(|e| format!("Failed to load replay from DB match {}: {}", match_id, e))
    } else {
        Err("Replay mode active but no match ID specified".to_string())
    };

    match replay_result {
        Ok(replay_data) => {
            info!(
                "Loaded replay: {} ticks, {} events",
                replay_data.ticks.len(),
                replay_data.events.len()
            );
            commands.insert_resource(replay_data);
        }
        Err(e) => {
            error!("{}", e);
            commands.insert_resource(replay::ReplayData::default());
        }
    }

    // Camera - orthographic, shows entire arena
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 0.0, 0.0),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: bevy::camera::ScalingMode::FixedVertical {
                viewport_height: ARENA_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

#[derive(Resource)]
struct ReplayTimeout {
    remaining_secs: f32,
    active: bool,
}

fn replay_timeout(
    mut timeout: ResMut<ReplayTimeout>,
    time: Res<Time>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if !timeout.active {
        return;
    }

    if timeout.remaining_secs <= 0.0 {
        app_exit.write(AppExit::Success);
        return;
    }

    timeout.remaining_secs -= time.delta_secs();
    if timeout.remaining_secs <= 0.0 {
        app_exit.write(AppExit::Success);
    }
}
