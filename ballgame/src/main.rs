//! Ballgame - A 2v2 ball sport game built with Bevy
//!
//! Main entry point: app setup and system registration.

use ballgame::ui::spawn_steal_indicators;
use ballgame::{
    AiCapabilities, AiProfileDatabase, Ball, BallPlayerContact,
    BallPulse, BallRolling, BallShotGrace, BallSpin, BallState, BallStyle, BallTextures,
    CharacterId, ConfigWatcher, CountdownEndTracker, CurrentLevel, CurrentPalette, CurrentPresets,
    CurrentSettings, CycleSelection, DebugLogConfig, DebugSettings,
    DisplayBallWave, EventBus, GameMode, LastShotInfo, LevelChangeTracker, LevelDatabase,
    MatchCountdown, NavGraph, PALETTES_FILE, PRESETS_FILE, PaletteDatabase, PhysicsTweaks,
    PlayerInput, PresetDatabase, Score, ScoreLevelText, SnapshotConfig, SnapshotTriggerState,
    StealContest, StealTracker, StyleTextures, Velocity,
    ViewportScale, ai, apply_preset_to_tweaks, ball, config_watcher, constants::*, countdown,
    display_ball_wave, emit_level_change_events, color_for_character, initial_facing, input,
    levels, player, replay, save_settings_system, scoring, server, shooting, snapshot,
    spawn_charge_gauge, spawn_characters_for_mode, spawn_countdown_text, steal, tuning, ui,
    update_event_bus_time, world,
};
use bevy::{camera::ScalingMode, diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Path to ball options file
const BALL_OPTIONS_FILE: &str = "config/ball_options.txt";
const DEFAULT_REPLAY_TIMEOUT_SECS: f32 = 5.0;

/// Get the default replay database path (finds the most recent training database)
fn default_replay_db() -> Option<String> {
    ballgame::db_paths::find_latest(ballgame::db_paths::DbType::Training)
}

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

    // Server mode flags
    let server_mode = args.iter().any(|a| a == "--server");
    let server_port = args
        .iter()
        .position(|a| a == "--port")
        .and_then(|i| args.get(i + 1).and_then(|s| s.parse::<u16>().ok()))
        .unwrap_or(9000);
    let local_slot = args
        .iter()
        .position(|a| a == "--local-slot")
        .and_then(|i| args.get(i + 1).and_then(|s| s.parse::<u8>().ok()));

    // Tournament mode flags
    let tournament_mode = args.iter().any(|a| a == "--tournament");
    let score_limit = args
        .iter()
        .position(|a| a == "--score-limit")
        .and_then(|i| args.get(i + 1).and_then(|s| s.parse::<u32>().ok()));
    let time_limit_secs = args
        .iter()
        .position(|a| a == "--time-limit")
        .and_then(|i| args.get(i + 1).and_then(|s| s.parse::<f32>().ok()));

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

    let mut app = App::new();

    app.add_plugins((
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
    ));

    // Resources
    app.insert_resource(ClearColor(initial_bg));
    app.insert_resource(palette_db);
    app.insert_resource(preset_db);
    app.insert_resource(level_db);
    app.insert_resource(current_settings);
    app.init_resource::<PlayerInput>();
    app.init_resource::<DebugSettings>();
    app.init_resource::<StealContest>();
    app.init_resource::<StealTracker>();
    app.init_resource::<Score>();
    app.init_resource::<scoring::GamePaused>();
    app.init_resource::<scoring::RestartRequested>();
    app.init_resource::<ui::PauseMenuState>();
    app.insert_resource(CurrentLevel(loaded_level_id));
    app.insert_resource(CurrentPalette(loaded_palette_index));
    app.insert_resource(debug_config);
    app.init_resource::<PhysicsTweaks>();
    app.init_resource::<LastShotInfo>();
    app.init_resource::<ui::DebugMenuState>();
    app.insert_resource(ViewportScale {
        preset_index: loaded_viewport_index,
    });
    app.insert_resource(CycleSelection {
        active_direction: ui::CycleDirection::from_str(&loaded_active_direction),
        down_option: ui::DownOption::from_str(&loaded_down_option),
        right_option: ui::RightOption::from_str(&loaded_right_option),
        ai_player_index: 0,
        menu_enabled: false,
    });
    app.init_resource::<ConfigWatcher>();
    app.init_resource::<AiProfileDatabase>();
    app.init_resource::<CurrentPresets>();
    app.init_resource::<NavGraph>();
    app.init_resource::<AiCapabilities>();
    app.init_resource::<ai::HeatmapBundle>();

    // Event bus for cross-module communication
    app.insert_resource(EventBus::new());

    // Level change tracker for event emission
    app.init_resource::<LevelChangeTracker>();

    app.insert_resource(SnapshotConfig {
        // Only enable screenshots when running via screenshot script
        enabled: screenshot_and_quit,
        exit_after_startup: screenshot_and_quit,
        ..default()
    });
    app.init_resource::<SnapshotTriggerState>();
    app.init_resource::<DisplayBallWave>();

    // Initialize countdown (frozen if regression level or --freeze-countdown flag)
    app.insert_resource(if should_freeze_countdown {
        let mut countdown = MatchCountdown::default();
        countdown.start_frozen();
        countdown
    } else {
        MatchCountdown::default()
    });

    // Countdown end tracker for jump ball velocity
    app.init_resource::<CountdownEndTracker>();

    // Replay mode resources
    app.insert_resource(if let Some(match_id) = replay_db_match_id {
        replay::ReplayMode::new_db(match_id)
    } else {
        replay::ReplayMode::default()
    });
    app.insert_resource(ReplayTimeout {
        remaining_secs: replay_timeout_secs.unwrap_or(0.0),
        active: replay_timeout_secs.is_some(),
    });
    app.init_resource::<replay::ReplayState>();

    // Startup systems - use normal setup only when NOT in replay mode
    app.add_systems(Startup, tuning::load_global_tuning_system);
    app.add_systems(Startup, setup.run_if(replay::not_replay_active));
    app.add_systems(Startup, ui::spawn_pause_overlay);
    app.add_systems(Startup, ui::spawn_debug_menu.run_if(replay::not_replay_active));

    // =========== NORMAL GAME SYSTEMS (disabled in replay mode) ===========
    // Countdown system - always runs to update timer and text
    app.add_systems(
        Update,
        (
            countdown::update_countdown,
            countdown::apply_jump_ball_velocity,
        )
            .chain()
            .run_if(replay::not_replay_active),
    );

    // Event bus time update (runs every frame for timestamping)
    app.add_systems(
        Update,
        update_event_bus_time.run_if(replay::not_replay_active),
    );

    // Input systems must run in order: capture -> copy -> swap -> nav graph -> nav -> AI
    // Only runs when NOT in countdown and NOT in replay mode
    app.add_systems(
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
            .run_if(
                replay::not_replay_active
                    .and(countdown::not_in_countdown)
                    .and(scoring::not_paused),
            ),
    );

    // Pause toggle (Start button)
    app.add_systems(
        Update,
        player::check_pause_toggle.run_if(replay::not_replay_active),
    );

    // Quit game (Escape or Select button)
    app.add_systems(Update, player::check_quit);

    // Core Update systems - split to avoid tuple issues with respawn_player
    app.add_systems(
        Update,
        player::respawn_player.run_if(replay::not_replay_active),
    );

    // Emit level change events for auditability (runs after systems that change level)
    app.add_systems(
        Update,
        emit_level_change_events.run_if(replay::not_replay_active),
    );

    // Countdown trigger on level change (only in manual game mode)
    app.add_systems(
        Update,
        countdown::trigger_countdown_on_level_change.run_if(replay::not_replay_active),
    );

    app.add_systems(
        Update,
        (ui::toggle_debug, config_watcher::check_config_changes)
            .run_if(replay::not_replay_active),
    );

    app.add_systems(
        Update,
        (
            ui::update_score_level_text,
            ui::spawn_character_indicators,
            ui::update_character_indicators,
            ui::update_indicator_colors,
        )
            .run_if(replay::not_replay_active),
    );

    app.add_systems(
        Update,
        (
            ui::animate_pickable_ball,
            ui::animate_score_flash,
            ui::update_charge_gauge,
            ui::update_steal_indicators,
            ui::update_pause_overlay,
            ui::pause_menu_navigation,
            ui::pause_menu_confirm,
            display_ball_wave,
            player::manage_debug_display,
        )
            .run_if(replay::not_replay_active),
    );

    // Debug menu systems
    app.add_systems(Update, ui::toggle_debug_menu);
    app.add_systems(Update, ui::debug_menu_navigation);
    app.add_systems(Update, ui::debug_menu_apply_cycle);
    app.add_systems(Update, ui::debug_menu_character_cycle);
    app.add_systems(Update, ui::debug_menu_ability_cycle);
    app.add_systems(Update, ui::update_debug_menu_display);

    // Palette application and preset application
    app.add_systems(
        Update,
        (ui::apply_palette_colors, apply_preset_to_tweaks).run_if(replay::not_replay_active),
    );

    // Snapshot system - captures game state on events
    app.add_systems(
        Update,
        (
            snapshot::snapshot_trigger_system,
            snapshot::toggle_snapshot_system,
            snapshot::toggle_screenshot_capture,
            snapshot::manual_snapshot,
        )
            .run_if(replay::not_replay_active),
    );

    // Settings persistence - save when dirty
    app.add_systems(
        Update,
        save_settings_system.run_if(replay::not_replay_active),
    );

    app.add_systems(Update, replay_timeout.run_if(replay::replay_active));

    // Physics systems in FixedUpdate
    app.add_systems(
        FixedUpdate,
        // Split into nested chains to avoid Bevy's tuple size limit
        (
            (
                player::apply_input,
                player::apply_gravity,
                player::turbo_update,
                player::block_update,
                ball::ball_gravity,
                ball::ball_spin,
                ball::apply_velocity,
                player::check_collisions,
                player::player_player_collision,
                ball::ball_collisions,
            )
                .chain(),
            (
                ball::ball_state_update,
                ball::pass_state_update,
                ball::ball_player_collision,
                ball::block_intercept,
                ball::pass_completion,
                ball::ball_follow_holder,
                ball::pickup_ball,
                steal::steal_cooldown_update,
                shooting::update_shot_charge,
                shooting::throw_ball,
                ball::handle_pass,
                scoring::check_scoring,
            )
                .chain(),
        )
            .chain()
            .run_if(
                replay::not_replay_active
                    .and(countdown::not_in_countdown)
                    .and(scoring::not_paused),
            ),
    );

    // =========== REPLAY MODE SYSTEMS ===========
    // Replay startup - load file, setup camera
    app.add_systems(Startup, replay_load_file.run_if(replay::replay_active));

    // Replay setup - spawn game world (runs after load, needs ReplayData)
    app.add_systems(
        Startup,
        (replay::replay_setup, replay::setup_replay_ui)
            .run_if(replay::replay_active)
            .after(replay_load_file),
    );

    // Replay update systems
    app.add_systems(
        Update,
        (
            replay::replay_playback,
            replay::replay_input_handler,
            replay::update_replay_ui,
        )
            .chain()
            .run_if(replay::replay_active),
    );

    // =========== SERVER MODE (optional) ===========
    if server_mode {
        info!("Starting in server mode on port {}", server_port);

        // Add server bridge resource (starts the WebSocket server)
        app.insert_resource(server::ServerBridge::new(server_port, local_slot));

        // Add tournament config if tournament mode is enabled
        let tournament_config = if tournament_mode || score_limit.is_some() || time_limit_secs.is_some() {
            server::TournamentConfig::new(score_limit, time_limit_secs)
        } else {
            server::TournamentConfig::default()
        };
        app.insert_resource(tournament_config);

        // Add server systems
        app.add_systems(
            Update,
            server::read_remote_inputs
                .before(ai::ai_decision_update)
                .run_if(replay::not_replay_active.and(server::server_mode_active)),
        );

        app.add_systems(
            FixedUpdate,
            server::broadcast_state_system
                .after(scoring::check_scoring)
                .run_if(replay::not_replay_active.and(server::server_mode_active)),
        );

        app.add_systems(
            Update,
            server::check_tournament_end
                .run_if(replay::not_replay_active.and(server::server_mode_active)),
        );
    }

    app.run();
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

    // Check if this is a debug or regression level early (for AI goal)
    let is_special_level = level_data.map(|l| l.debug || l.regression).unwrap_or(false);

    // Determine which character is human-controlled (if any)
    let human_controlled = if left_is_human {
        Some(CharacterId::L0)
    } else {
        None
    };

    // Spawn characters using the helper function (2v2 mode)
    let spawned_characters = spawn_characters_for_mode(
        &mut commands,
        GameMode::TwoVsTwo,
        &initial_palette,
        &left_ai_profile_id,
        &right_ai_profile_id,
        human_controlled,
        is_special_level,
        &profile_db,
    );

    // Spawn charge gauges and steal indicators for all spawned characters
    for (character_id, entity) in &spawned_characters {
        let facing = initial_facing(*character_id);
        spawn_charge_gauge(&mut commands, *entity, facing);
        spawn_steal_indicators(&mut commands, *entity, facing);
    }

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

    // Calculate ball spawn position from level config (if set) or use default
    let ball_spawn_pos = level_data
        .and_then(|l| l.ball_start)
        .map(|pos| Vec3::new(pos.x, ARENA_FLOOR_Y + pos.y, BALL_SPAWN.z))
        .unwrap_or(BALL_SPAWN);

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
                Transform::from_translation(ball_spawn_pos),
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
                Transform::from_translation(ball_spawn_pos),
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
    // Get slot 1 colors (darker variants) for basket stripes
    let left_color2 = color_for_character(CharacterId::L1, &initial_palette);
    let right_color2 = color_for_character(CharacterId::R1, &initial_palette);
    world::spawn_baskets(
        &mut commands,
        basket_y,
        basket_push_in,
        initial_palette.left,
        left_color2,
        initial_palette.right,
        right_color2,
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

    // Countdown text (3-2-1 before match starts)
    spawn_countdown_text(&mut commands);
}

/// Setup system for replay mode - loads replay data
fn replay_load_file(mut commands: Commands, replay_mode: Res<replay::ReplayMode>) {
    let replay_result = if let Some(match_id) = replay_mode.match_id {
        // Find the most recent training database, or fall back to legacy path
        let db_path = default_replay_db()
            .unwrap_or_else(|| ballgame::db_paths::default_path(ballgame::db_paths::DbType::Training));
        replay::load_replay_from_db(Path::new(&db_path), match_id)
            .map_err(|e| format!("Failed to load replay from DB match {} ({}): {}", match_id, db_path, e))
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
