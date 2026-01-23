//! Ballgame - A 2v2 ball sport game built with Bevy
//!
//! Main entry point: app setup and system registration.

use ballgame::{
    AiGoal, AiInput, AiState, Ball, BallPlayerContact, BallPulse, BallRolling, BallShotGrace,
    BallSpin, BallState, BallStyleType, BallTextures, ChargeGaugeBackground, ChargeGaugeFill,
    ChargingShot, CoyoteTimer, CurrentLevel, DebugSettings, DebugStyleKey, DebugText, Facing,
    Grounded, HumanControlled, JumpState, LastShotInfo, LevelDatabase, PhysicsTweaks, Player,
    PlayerInput, Score, ScoreLevelText, StealContest, StyleTextures, TargetBasket, Team,
    TweakPanel, TweakRow, Velocity, ai, ball, constants::*, helpers::*, input, levels, player,
    scoring, shooting, steal, ui, world,
};
use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use world::{Basket, BasketRim, Collider, Platform};

fn main() {
    // Load level database from file
    let level_db = LevelDatabase::load_from_file(LEVELS_FILE);

    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: (ARENA_WIDTH as u32, ARENA_HEIGHT as u32).into(),
                    title: "Ballgame".into(),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(level_db)
        .init_resource::<PlayerInput>()
        .init_resource::<DebugSettings>()
        .init_resource::<StealContest>()
        .init_resource::<Score>()
        .init_resource::<CurrentLevel>()
        .init_resource::<PhysicsTweaks>()
        .init_resource::<LastShotInfo>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                input::capture_input,
                ai::copy_human_input,
                ai::swap_control,
                ai::ai_decision_update,
                player::respawn_player,
                ui::toggle_debug,
                levels::reload_levels,
                ui::update_debug_text,
                ui::update_score_level_text,
                ui::animate_pickable_ball,
                ui::animate_score_flash,
                ui::update_charge_gauge,
                ui::toggle_tweak_panel,
                ui::update_tweak_panel,
                ui::update_style_key_visibility,
                ball::ball_texture_swap,
            ),
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
fn setup(mut commands: Commands, level_db: Res<LevelDatabase>, asset_server: Res<AssetServer>) {
    // Camera - orthographic, shows entire arena
    // Using scale to zoom out and show full arena (default is 1.0 = 1 pixel per unit)
    // With 1600x900 arena, we need to scale so it fits in typical window sizes
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 0.0, 0.0),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0, // 1:1 mapping: 1 world unit = 1 pixel
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Left team player (blue) - spawns on left side, starts human-controlled
    let left_player = commands
        .spawn((
            Sprite::from_color(TEAM_LEFT_PRIMARY, PLAYER_SIZE),
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

    // Right team player (orange) - spawns on right side, starts AI-controlled
    let _right_player = commands
        .spawn((
            Sprite::from_color(TEAM_RIGHT_PRIMARY, PLAYER_SIZE),
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

    // Load ball textures for all styles (6 styles × 3 states = 18 textures)
    let load_style = |style: &str| -> StyleTextures {
        StyleTextures {
            neutral: asset_server.load(format!("ball_{}_neutral.png", style)),
            left: asset_server.load(format!("ball_{}_left.png", style)),
            right: asset_server.load(format!("ball_{}_right.png", style)),
        }
    };
    let ball_textures = BallTextures {
        stripe: load_style("stripe"),
        wedges: load_style("wedges"),
        dot: load_style("dot"),
        half: load_style("half"),
        ring: load_style("ring"),
        solid: load_style("solid"),
    };
    commands.insert_resource(ball_textures.clone());

    // Check if this is a debug level (spawns all ball styles, AI idle)
    let is_debug_level = level_db.get(0).map(|l| l.debug).unwrap_or(false);

    if is_debug_level {
        // Debug level: spawn 6 balls with different styles
        let x_positions = [-600.0, -360.0, -120.0, 120.0, 360.0, 600.0];

        for (style, x) in BallStyleType::ALL.iter().zip(x_positions.iter()) {
            let textures = ball_textures.get(*style);
            commands.spawn((
                Sprite {
                    image: textures.neutral.clone(),
                    custom_size: Some(BALL_SIZE),
                    ..default()
                },
                Transform::from_xyz(*x, ARENA_FLOOR_Y + BALL_SIZE.y / 2.0 + 20.0, 2.0),
                Ball,
                BallState::default(),
                Velocity::default(),
                BallPlayerContact::default(),
                BallPulse::default(),
                BallRolling::default(),
                BallShotGrace::default(),
                BallSpin::default(),
                *style, // BallStyleType component
            ));
        }
    } else {
        // Normal levels: spawn single ball with default style (dot)
        let default_textures = ball_textures.get(BallStyleType::default());
        commands.spawn((
            Sprite {
                image: default_textures.neutral.clone(),
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
            BallStyleType::default(),
        ));
    }

    // Arena floor (spans between walls)
    commands.spawn((
        Sprite::from_color(
            FLOOR_COLOR,
            Vec2::new(ARENA_WIDTH - WALL_THICKNESS * 2.0, 40.0),
        ),
        Transform::from_xyz(0.0, ARENA_FLOOR_Y, 0.0),
        Platform,
    ));

    // Left wall (flush with arena edge)
    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, Vec2::new(WALL_THICKNESS, 5000.0)),
        Transform::from_xyz(-ARENA_WIDTH / 2.0 + WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
    ));

    // Right wall (flush with arena edge)
    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, Vec2::new(WALL_THICKNESS, 5000.0)),
        Transform::from_xyz(ARENA_WIDTH / 2.0 - WALL_THICKNESS / 2.0, 2000.0, 0.0),
        Platform,
    ));

    // Spawn level 1 platforms
    levels::spawn_level_platforms(&mut commands, &level_db, 0);

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

    // Left basket (turquoise - left team's home) with contrasting rims (burnt clay)
    commands
        .spawn((
            Sprite::from_color(TEAM_LEFT_PRIMARY, BASKET_SIZE),
            Transform::from_xyz(left_basket_x, basket_y, -0.1), // Slightly behind
            Basket::Left,
        ))
        .with_children(|parent| {
            // Left rim (outer - wall side, 50%) - center at basket edge
            parent.spawn((
                Sprite::from_color(TEAM_RIGHT_DARK, Vec2::new(RIM_THICKNESS, rim_outer_height)),
                Transform::from_xyz(-BASKET_SIZE.x / 2.0, rim_outer_y, 0.1),
                Platform,
                BasketRim,
            ));
            // Right rim (inner - center side, 10%) - center at basket edge
            parent.spawn((
                Sprite::from_color(TEAM_RIGHT_DARK, Vec2::new(RIM_THICKNESS, rim_inner_height)),
                Transform::from_xyz(BASKET_SIZE.x / 2.0, rim_inner_y, 0.1),
                Platform,
                BasketRim,
            ));
            // Bottom rim - center at basket bottom edge
            parent.spawn((
                Sprite::from_color(TEAM_RIGHT_DARK, Vec2::new(rim_bottom_width, RIM_THICKNESS)),
                Transform::from_xyz(0.0, -BASKET_SIZE.y / 2.0, 0.1),
                Platform,
                BasketRim,
            ));
        });

    // Right basket (terracotta - right team's home) with contrasting rims (deep teal)
    commands
        .spawn((
            Sprite::from_color(TEAM_RIGHT_PRIMARY, BASKET_SIZE),
            Transform::from_xyz(right_basket_x, basket_y, -0.1),
            Basket::Right,
        ))
        .with_children(|parent| {
            // Left rim (inner - center side, 10%) - center at basket edge
            parent.spawn((
                Sprite::from_color(TEAM_LEFT_DARK, Vec2::new(RIM_THICKNESS, rim_inner_height)),
                Transform::from_xyz(-BASKET_SIZE.x / 2.0, rim_inner_y, 0.1),
                Platform,
                BasketRim,
            ));
            // Right rim (outer - wall side, 50%) - center at basket edge
            parent.spawn((
                Sprite::from_color(TEAM_LEFT_DARK, Vec2::new(RIM_THICKNESS, rim_outer_height)),
                Transform::from_xyz(BASKET_SIZE.x / 2.0, rim_outer_y, 0.1),
                Platform,
                BasketRim,
            ));
            // Bottom rim - center at basket bottom edge
            parent.spawn((
                Sprite::from_color(TEAM_LEFT_DARK, Vec2::new(rim_bottom_width, RIM_THICKNESS)),
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

    // Debug Style Key (visible only on Debug level)
    // Shows numbered list of ball styles on left side of screen
    let style_key_visible = if is_debug_level {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.85)),
            style_key_visible,
            DebugStyleKey,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Ball Styles"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(TEXT_PRIMARY),
            ));
            // List each style with number (1-6 = left to right on floor)
            for (i, style) in BallStyleType::ALL.iter().enumerate() {
                parent.spawn((
                    Text::new(format!("{}. {}", i + 1, style.name())),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(TEXT_SECONDARY),
                ));
            }
        });
}
