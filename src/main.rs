//! Ballgame - A 2v2 ball sport game built with Bevy
//!
//! Main entry point: app setup and system registration.

use ballgame::{
    AiGoal, AiInput, AiState, Ball, BallPlayerContact, BallPulse, BallRolling, BallShotGrace,
    BallSpin, BallState, BallStyle, BallTextures, ChargeGaugeBackground, ChargeGaugeFill,
    ChargingShot, CoyoteTimer, CurrentLevel, CurrentPalette, CycleIndicator, CycleSelection,
    DebugSettings, DebugText, Facing, Grounded, HumanControlled, JumpState, LastShotInfo,
    LevelDatabase, PaletteDatabase, PhysicsTweaks, Player, PlayerInput, Score, ScoreLevelText,
    StealContest, StyleTextures, TargetBasket, Team, TweakPanel, TweakRow, Velocity, ViewportScale,
    ai, ball, constants::*, helpers::*, input, levels, player, scoring, shooting, steal, ui, world,
    PALETTES_FILE,
};
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use std::collections::HashMap;
use std::fs;
use world::{Basket, BasketRim, Collider, Platform};

/// Path to ball options file
const BALL_OPTIONS_FILE: &str = "assets/ball_options.txt";

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
    // Load level database from file
    let level_db = LevelDatabase::load_from_file(LEVELS_FILE);

    // Load palette database (creates default file if missing)
    let palette_db = PaletteDatabase::load_or_create(PALETTES_FILE);

    // Get initial background color from first palette
    let initial_bg = palette_db
        .get(0)
        .map(|p| p.background)
        .unwrap_or(DEFAULT_BACKGROUND_COLOR);

    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    // Use first viewport preset for initial size
                    // Set scale_factor_override to 1.0 for consistent behavior on HiDPI displays
                    resolution: bevy::window::WindowResolution::new(
                        VIEWPORT_PRESETS[0].0 as u32,
                        VIEWPORT_PRESETS[0].1 as u32,
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
        .insert_resource(level_db)
        .init_resource::<PlayerInput>()
        .init_resource::<DebugSettings>()
        .init_resource::<StealContest>()
        .init_resource::<Score>()
        .init_resource::<CurrentLevel>()
        .init_resource::<CurrentPalette>()
        .init_resource::<PhysicsTweaks>()
        .init_resource::<LastShotInfo>()
        .init_resource::<ViewportScale>()
        .init_resource::<CycleSelection>()
        .add_systems(Startup, setup)
        // Input systems must run in order: capture -> copy -> swap -> AI
        .add_systems(
            Update,
            (
                input::capture_input,
                ai::copy_human_input,
                ai::swap_control,
                ai::ai_decision_update,
            )
                .chain(),
        )
        // Core Update systems
        .add_systems(
            Update,
            (
                player::respawn_player,
                ui::toggle_debug,
                levels::reload_levels,
                ui::update_debug_text,
                ui::update_score_level_text,
                ui::animate_pickable_ball,
                ui::animate_score_flash,
                ui::update_charge_gauge,
            ),
        )
        // UI panel and cycle systems
        .add_systems(
            Update,
            (
                ui::toggle_tweak_panel,
                ui::update_tweak_panel,
                ui::cycle_viewport,
                ui::unified_cycle_system,
            ),
        )
        // Cycle indicator and palette application
        .add_systems(
            Update,
            (ui::update_cycle_indicator, ui::apply_palette_colors),
        )
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
                steal::steal_contest_update,
                shooting::update_shot_charge,
                shooting::throw_ball,
                scoring::check_scoring,
            )
                .chain(),
        )
        .run();
}

/// Setup the game world
fn setup(
    mut commands: Commands,
    level_db: Res<LevelDatabase>,
    palette_db: Res<PaletteDatabase>,
    asset_server: Res<AssetServer>,
) {
    // Camera - orthographic, shows entire arena
    // Scale adjusts based on window size to always show full arena
    let initial_camera_scale = ARENA_WIDTH / VIEWPORT_PRESETS[0].0;
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 0.0, 0.0),
        Projection::Orthographic(OrthographicProjection {
            scale: initial_camera_scale,
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Get initial palette colors
    let initial_palette = palette_db.get(0).expect("No palettes loaded");

    // Left team player - spawns on left side, starts human-controlled
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
            ChargingShot::default(),
            TargetBasket(Basket::Right), // Left team scores in right basket
            Collider,
            Team::Left,
            HumanControlled, // Starts as human-controlled
            AiInput::default(),
            AiState::default(),
        ))
        .id();

    // Check if this is a debug level early (for AI goal)
    let is_debug_level_for_ai = level_db.get(0).map(|l| l.debug).unwrap_or(false);

    // Right team player - spawns on right side, starts AI-controlled
    let right_player = commands
        .spawn((
            Sprite::from_color(initial_palette.right, PLAYER_SIZE),
            Transform::from_translation(PLAYER_SPAWN_RIGHT),
            Player,
            Velocity::default(),
            Grounded(false),
            CoyoteTimer::default(),
            JumpState::default(),
            Facing(-1.0), // Faces left
            ChargingShot::default(),
            TargetBasket(Basket::Left), // Right team scores in left basket
            Collider,
            Team::Right,
            AiInput::default(),
            AiState {
                // On debug level, AI stands still (Idle); otherwise normal AI
                current_goal: if is_debug_level_for_ai {
                    AiGoal::Idle
                } else {
                    AiGoal::default()
                },
                ..default()
            },
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

    // Load ball style names from config file
    let style_names = load_ball_style_names();
    let num_palettes = palette_db.len();

    // Load ball textures for all styles dynamically
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

    // Check if this is a debug level (spawns all ball styles, AI idle)
    let is_debug_level = level_db.get(0).map(|l| l.debug).unwrap_or(false);

    // Initial palette index is 0
    let initial_palette_idx = 0;

    if is_debug_level {
        // Debug level: spawn ALL ball styles dynamically
        let num_styles = style_names.len();
        let total_width = 1200.0; // Spread balls across this width
        let spacing = if num_styles > 1 {
            total_width / (num_styles - 1) as f32
        } else {
            0.0
        };
        let start_x = -total_width / 2.0;

        for (i, style_name) in style_names.iter().enumerate() {
            let x = start_x + (i as f32 * spacing);
            if let Some(textures) = ball_textures.get(style_name) {
                commands.spawn((
                    Sprite {
                        image: textures.textures[initial_palette_idx].clone(),
                        custom_size: Some(BALL_SIZE),
                        ..default()
                    },
                    Transform::from_xyz(x, ARENA_FLOOR_Y + BALL_SIZE.y / 2.0 + 20.0, 2.0),
                    Ball,
                    BallState::default(),
                    Velocity::default(),
                    BallPlayerContact::default(),
                    BallPulse::default(),
                    BallRolling::default(),
                    BallShotGrace::default(),
                    BallSpin::default(),
                    BallStyle::new(style_name),
                ));
            }
        }
    } else {
        // Normal levels: spawn single ball with first style
        let default_style = ball_textures.default_style().cloned().unwrap_or_else(|| "wedges".to_string());
        if let Some(textures) = ball_textures.get(&default_style) {
            commands.spawn((
                Sprite {
                    image: textures.textures[initial_palette_idx].clone(),
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
                BallStyle::new(&default_style),
            ));
        }
    }

    // Arena floor (spans between walls)
    commands.spawn((
        Sprite::from_color(
            initial_palette.platforms,
            Vec2::new(ARENA_WIDTH - WALL_THICKNESS * 2.0, 40.0),
        ),
        Transform::from_xyz(0.0, ARENA_FLOOR_Y, 0.0),
        Platform,
    ));

    // Left wall (flush with arena edge)
    commands.spawn((
        Sprite::from_color(initial_palette.platforms, Vec2::new(WALL_THICKNESS, 5000.0)),
        Transform::from_xyz(-ARENA_WIDTH / 2.0 + WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
    ));

    // Right wall (flush with arena edge)
    commands.spawn((
        Sprite::from_color(initial_palette.platforms, Vec2::new(WALL_THICKNESS, 5000.0)),
        Transform::from_xyz(ARENA_WIDTH / 2.0 - WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
    ));

    // Spawn level 1 platforms
    levels::spawn_level_platforms(&mut commands, &level_db, 0, initial_palette.platforms);

    // Baskets (goals) - height and X position vary per level
    let initial_level = level_db.get(0);
    let basket_y = initial_level
        .map(|l| ARENA_FLOOR_Y + l.basket_height)
        .unwrap_or(ARENA_FLOOR_Y + 400.0);
    let basket_push_in = initial_level
        .map(|l| l.basket_push_in)
        .unwrap_or(BASKET_PUSH_IN);
    let (left_basket_x, right_basket_x) = basket_x_from_offset(basket_push_in);

    // Rim dimensions (RIM_THICKNESS is now a top-level constant)
    let rim_outer_height = BASKET_SIZE.y * 0.5; // 50% - wall side
    let rim_inner_height = BASKET_SIZE.y * 0.1; // 10% - center side
    let rim_outer_y = -BASKET_SIZE.y / 2.0 + rim_outer_height / 2.0; // Positioned at bottom
    let rim_inner_y = -BASKET_SIZE.y / 2.0 + rim_inner_height / 2.0; // Positioned at bottom
    let rim_bottom_width = BASKET_SIZE.x + RIM_THICKNESS; // Basket width + one rim thickness (side rims half-in)

    // Left basket (left team's home) with contrasting rims (right team dark)
    commands
        .spawn((
            Sprite::from_color(initial_palette.left, BASKET_SIZE),
            Transform::from_xyz(left_basket_x, basket_y, -0.1), // Slightly behind
            Basket::Left,
        ))
        .with_children(|parent| {
            // Left rim (outer - wall side, 50%) - center at basket edge
            parent.spawn((
                Sprite::from_color(initial_palette.right_rim, Vec2::new(RIM_THICKNESS, rim_outer_height)),
                Transform::from_xyz(-BASKET_SIZE.x / 2.0, rim_outer_y, 0.1),
                Platform,
                BasketRim,
            ));
            // Right rim (inner - center side, 10%) - center at basket edge
            parent.spawn((
                Sprite::from_color(initial_palette.right_rim, Vec2::new(RIM_THICKNESS, rim_inner_height)),
                Transform::from_xyz(BASKET_SIZE.x / 2.0, rim_inner_y, 0.1),
                Platform,
                BasketRim,
            ));
            // Bottom rim - center at basket bottom edge
            parent.spawn((
                Sprite::from_color(initial_palette.right_rim, Vec2::new(rim_bottom_width, RIM_THICKNESS)),
                Transform::from_xyz(0.0, -BASKET_SIZE.y / 2.0, 0.1),
                Platform,
                BasketRim,
            ));
        });

    // Right basket (right team's home) with contrasting rims (left team dark)
    commands
        .spawn((
            Sprite::from_color(initial_palette.right, BASKET_SIZE),
            Transform::from_xyz(right_basket_x, basket_y, -0.1),
            Basket::Right,
        ))
        .with_children(|parent| {
            // Left rim (inner - center side, 10%) - center at basket edge
            parent.spawn((
                Sprite::from_color(initial_palette.left_rim, Vec2::new(RIM_THICKNESS, rim_inner_height)),
                Transform::from_xyz(-BASKET_SIZE.x / 2.0, rim_inner_y, 0.1),
                Platform,
                BasketRim,
            ));
            // Right rim (outer - wall side, 50%) - center at basket edge
            parent.spawn((
                Sprite::from_color(initial_palette.left_rim, Vec2::new(RIM_THICKNESS, rim_outer_height)),
                Transform::from_xyz(BASKET_SIZE.x / 2.0, rim_outer_y, 0.1),
                Platform,
                BasketRim,
            ));
            // Bottom rim - center at basket bottom edge
            parent.spawn((
                Sprite::from_color(initial_palette.left_rim, Vec2::new(rim_bottom_width, RIM_THICKNESS)),
                Transform::from_xyz(0.0, -BASKET_SIZE.y / 2.0, 0.1),
                Platform,
                BasketRim,
            ));
        });

    // Corner ramps - angled walls in bottom corners
    let initial_level = level_db.get(0);
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

    // Cycle indicator - shows current cycle target when using controller (D-pad Down + RT/LT)
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        TextColor(TEXT_ACCENT),
        Transform::from_xyz(0.0, ARENA_HEIGHT / 2.0 - 60.0, 1.0),
        Visibility::Hidden,
        CycleIndicator,
    ));

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
}
