use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use rand::Rng;
use std::fs;

// Visual constants
const BACKGROUND_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const FLOOR_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);
const PLATFORM_COLOR: Color = Color::srgb(0.5, 0.3, 0.3);
const PLAYER_COLOR: Color = Color::srgb(0.3, 0.3, 0.7);
const BALL_COLOR: Color = Color::srgb(0.9, 0.5, 0.1); // Orange basketball-ish

// Size constants
const PLAYER_SIZE: Vec2 = Vec2::new(32.0, 64.0);
const BALL_SIZE: Vec2 = Vec2::new(24.0, 24.0);
const CHARGE_GAUGE_WIDTH: f32 = 8.0;
const CHARGE_GAUGE_HEIGHT: f32 = PLAYER_SIZE.y; // Same height as player

// Physics constants
const GRAVITY_RISE: f32 = 980.0; // Gravity while rising
const GRAVITY_FALL: f32 = 1400.0; // Gravity while falling (fast fall)
const JUMP_VELOCITY: f32 = 650.0; // Full jump height (hold button)
const JUMP_CUT_MULTIPLIER: f32 = 0.4; // Velocity multiplier when releasing jump early
const MOVE_SPEED: f32 = 300.0;
const GROUND_ACCEL: f32 = 2400.0; // Ground acceleration (pixels/sec²) - snappy start
const GROUND_DECEL: f32 = 1800.0; // Ground deceleration - slight slide when stopping
const AIR_ACCEL: f32 = 1500.0; // Air acceleration - committed but adjustable jumps
const AIR_DECEL: f32 = 900.0; // Air deceleration - momentum preserved in air
const COLLISION_EPSILON: f32 = 0.5; // Skin width for collision detection

// Game feel constants
const COYOTE_TIME: f32 = 0.1; // Seconds after leaving ground you can still jump
const JUMP_BUFFER_TIME: f32 = 0.1; // Seconds before landing that jump input is remembered
const STICK_DEADZONE: f32 = 0.25; // Analog stick deadzone to prevent rebound direction changes

// Ball physics
const BALL_GRAVITY: f32 = 800.0;
const BALL_BOUNCE: f32 = 0.7; // Coefficient of restitution (0 = no bounce, 1 = perfect bounce)
const BALL_AIR_FRICTION: f32 = 0.95; // Horizontal velocity retained after 1 second in air (low drag)
const BALL_GROUND_FRICTION: f32 = 0.6; // Horizontal velocity retained per bounce
const BALL_ROLL_FRICTION: f32 = 0.6; // Horizontal velocity retained after 1 second while rolling
const BALL_BOUNCE_HEIGHT_MULT: f32 = 1.7; // Ball must bounce this × its height to keep bouncing, else rolls
const BALL_PICKUP_RADIUS: f32 = 50.0; // How close player must be to pick up ball
const BALL_FREE_SPEED: f32 = 200.0; // Ball becomes Free when speed drops below this (2x pickup radius speed)

// Shooting - heights: tap=2x player height (128), full=6x player height (384)
// Using h = v_y²/(2g), v_y = sqrt(2*g*h): tap needs v_y≈452, full needs v_y≈784
const SHOT_MAX_POWER: f32 = 900.0; // Maximum horizontal velocity (fallback for extreme shots)
const SHOT_MAX_SPEED: f32 = 800.0; // Maximum total ball speed (caps velocity magnitude)
const SHOT_CHARGE_TIME: f32 = 1.6; // Seconds to reach full charge
const SHOT_MAX_VARIANCE: f32 = 0.50; // Variance at zero charge (50%)
const SHOT_MIN_VARIANCE: f32 = 0.02; // Variance at full charge (2%)
const SHOT_AIR_VARIANCE_PENALTY: f32 = 0.10; // Additional variance when airborne (10%)
const SHOT_MOVE_VARIANCE_PENALTY: f32 = 0.10; // Additional variance at full horizontal speed (10%)
const SHOT_DISTANCE_VARIANCE_FACTOR: f32 = 0.0003; // Variance per unit of distance (30% at 1000 units)
const SHOT_GRACE_PERIOD: f32 = 0.1; // Post-shot grace period (no friction/player drag)

// Ball-player collision
const BALL_PLAYER_DRAG_X: f32 = 0.7; // Horizontal velocity multiplier when ball hits player
const BALL_PLAYER_DRAG_Y: f32 = 0.4; // Vertical velocity multiplier (higher friction than X)
const BALL_KICK_STRENGTH: f32 = 100.0; // How much velocity player imparts to stationary ball
const BALL_KICK_THRESHOLD: f32 = 30.0; // Ball speed below this counts as "stationary"

// Steal contest
const STEAL_RANGE: f32 = 60.0; // How close to initiate steal
const STEAL_CONTEST_DURATION: f32 = 0.4; // Seconds to mash buttons
const STEAL_DEFENDER_ADVANTAGE: u32 = 2; // Defender starts with this many "presses"

// Arena dimensions (16:9 aspect ratio friendly)
const ARENA_WIDTH: f32 = 1600.0;
const ARENA_HEIGHT: f32 = 900.0;
const ARENA_FLOOR_Y: f32 = -ARENA_HEIGHT / 2.0 + 20.0; // Floor near bottom

// Baskets
const BASKET_COLOR: Color = Color::srgb(0.8, 0.2, 0.2); // Red
const BASKET_SIZE: Vec2 = Vec2::new(60.0, 80.0);
const LEFT_BASKET_X: f32 = -ARENA_WIDTH / 2.0 + 120.0;
const RIGHT_BASKET_X: f32 = ARENA_WIDTH / 2.0 - 120.0;

// Spawn
const PLAYER_SPAWN: Vec3 = Vec3::new(-200.0, ARENA_FLOOR_Y + 100.0, 0.0);
const BALL_SPAWN: Vec3 = Vec3::new(0.0, ARENA_FLOOR_Y + 50.0, 2.0); // Center, z=2 to render in front

// Level file path
const LEVELS_FILE: &str = "assets/levels.txt";

/// Move a value toward a target by a maximum delta
fn move_toward(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

fn main() {
    // Load level database from file
    let level_db = LevelDatabase::load_from_file(LEVELS_FILE);

    App::new()
        .add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin::default()))
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
        .add_systems(Update, (capture_input, respawn_player, toggle_debug, update_debug_text, update_score_level_text, animate_pickable_ball, animate_score_flash, update_charge_gauge, update_target_marker, toggle_tweak_panel, update_tweak_panel))
        .add_systems(
            FixedUpdate,
            (
                apply_input,
                cycle_target,
                apply_gravity,
                ball_gravity,
                apply_velocity,
                check_collisions,
                ball_collisions,
                ball_state_update,
                ball_player_collision,
                ball_follow_holder,
                pickup_ball,
                steal_contest_update,
                update_shot_charge,
                throw_ball,
                check_scoring,
            )
                .chain(),
        )
        .run();
}

// Resources

#[derive(Resource, Default)]
struct PlayerInput {
    move_x: f32,
    jump_buffer_timer: f32,    // Time remaining in jump buffer
    jump_held: bool,           // Is jump button currently held
    pickup_pressed: bool,      // West button - pick up ball
    throw_held: bool,          // R shoulder - charging throw
    throw_released: bool,      // R shoulder released - execute throw
    cycle_target_pressed: bool, // L shoulder - cycle target basket
}

#[derive(Resource)]
struct DebugSettings {
    visible: bool,
}

impl Default for DebugSettings {
    fn default() -> Self {
        Self { visible: true }
    }
}

#[derive(Resource, Default)]
struct StealContest {
    active: bool,
    attacker: Option<Entity>,
    defender: Option<Entity>,
    attacker_presses: u32,
    defender_presses: u32,
    timer: f32,
}

#[derive(Resource, Default)]
struct Score {
    left: u32,  // Team scoring in LEFT basket
    right: u32, // Team scoring in RIGHT basket
}

#[derive(Resource)]
struct CurrentLevel(u32);

impl Default for CurrentLevel {
    fn default() -> Self {
        Self(1)
    }
}

/// Information about the last shot taken (for debug display)
#[derive(Resource, Default)]
struct LastShotInfo {
    power: f32,
    arc: f32,
    angle_degrees: f32,
    speed: f32,
    base_variance: f32,
    air_penalty: f32,
    move_penalty: f32,
    distance_penalty: f32,
    arc_penalty: f32,
    total_variance: f32,
    target: Option<Basket>,
}

/// Runtime-adjustable physics values for tweaking gameplay feel
#[derive(Resource)]
struct PhysicsTweaks {
    gravity_rise: f32,
    gravity_fall: f32,
    jump_velocity: f32,
    move_speed: f32,
    ground_accel: f32,
    ground_decel: f32,
    air_accel: f32,
    air_decel: f32,
    ball_gravity: f32,
    ball_bounce: f32,
    ball_air_friction: f32,
    ball_roll_friction: f32,
    shot_max_power: f32,
    shot_charge_time: f32,
    selected_index: usize, // Which value is currently selected for adjustment
    panel_visible: bool,
}

impl Default for PhysicsTweaks {
    fn default() -> Self {
        Self {
            gravity_rise: GRAVITY_RISE,
            gravity_fall: GRAVITY_FALL,
            jump_velocity: JUMP_VELOCITY,
            move_speed: MOVE_SPEED,
            ground_accel: GROUND_ACCEL,
            ground_decel: GROUND_DECEL,
            air_accel: AIR_ACCEL,
            air_decel: AIR_DECEL,
            ball_gravity: BALL_GRAVITY,
            ball_bounce: BALL_BOUNCE,
            ball_air_friction: BALL_AIR_FRICTION,
            ball_roll_friction: BALL_ROLL_FRICTION,
            shot_max_power: SHOT_MAX_POWER,
            shot_charge_time: SHOT_CHARGE_TIME,
            selected_index: 0,
            panel_visible: false,
        }
    }
}

impl PhysicsTweaks {
    const LABELS: [&'static str; 14] = [
        "Gravity Rise",
        "Gravity Fall",
        "Jump Velocity",
        "Move Speed",
        "Ground Accel",
        "Ground Decel",
        "Air Accel",
        "Air Decel",
        "Ball Gravity",
        "Ball Bounce",
        "Ball Air Friction",
        "Ball Roll Friction",
        "Shot Max Power",
        "Shot Charge Time",
    ];

    fn get_value(&self, index: usize) -> f32 {
        match index {
            0 => self.gravity_rise,
            1 => self.gravity_fall,
            2 => self.jump_velocity,
            3 => self.move_speed,
            4 => self.ground_accel,
            5 => self.ground_decel,
            6 => self.air_accel,
            7 => self.air_decel,
            8 => self.ball_gravity,
            9 => self.ball_bounce,
            10 => self.ball_air_friction,
            11 => self.ball_roll_friction,
            12 => self.shot_max_power,
            13 => self.shot_charge_time,
            _ => 0.0,
        }
    }

    fn get_default_value(index: usize) -> f32 {
        match index {
            0 => GRAVITY_RISE,
            1 => GRAVITY_FALL,
            2 => JUMP_VELOCITY,
            3 => MOVE_SPEED,
            4 => GROUND_ACCEL,
            5 => GROUND_DECEL,
            6 => AIR_ACCEL,
            7 => AIR_DECEL,
            8 => BALL_GRAVITY,
            9 => BALL_BOUNCE,
            10 => BALL_AIR_FRICTION,
            11 => BALL_ROLL_FRICTION,
            12 => SHOT_MAX_POWER,
            13 => SHOT_CHARGE_TIME,
            _ => 0.0,
        }
    }

    fn set_value(&mut self, index: usize, value: f32) {
        match index {
            0 => self.gravity_rise = value,
            1 => self.gravity_fall = value,
            2 => self.jump_velocity = value,
            3 => self.move_speed = value,
            4 => self.ground_accel = value,
            5 => self.ground_decel = value,
            6 => self.air_accel = value,
            7 => self.air_decel = value,
            8 => self.ball_gravity = value,
            9 => self.ball_bounce = value,
            10 => self.ball_air_friction = value,
            11 => self.ball_roll_friction = value,
            12 => self.shot_max_power = value,
            13 => self.shot_charge_time = value,
            _ => {}
        }
    }

    fn is_modified(&self, index: usize) -> bool {
        let current = self.get_value(index);
        let default = Self::get_default_value(index);
        (current - default).abs() > 0.001
    }

    fn reset_value(&mut self, index: usize) {
        self.set_value(index, Self::get_default_value(index));
    }

    fn reset_all(&mut self) {
        for i in 0..Self::LABELS.len() {
            self.reset_value(i);
        }
    }

    fn get_step(&self, index: usize) -> f32 {
        // Step size is ~10% of default value
        let default = Self::get_default_value(index);
        (default * 0.1).max(0.01) // At least 0.01 for small values
    }
}

/// Platform definition in level data
#[derive(Clone, Debug)]
enum PlatformDef {
    Mirror { x: f32, y: f32, width: f32 },  // Spawns at -x and +x
    Center { y: f32, width: f32 },           // Spawns at x=0
}

/// Single level definition
#[derive(Clone, Debug)]
struct LevelData {
    name: String,
    basket_height: f32,
    platforms: Vec<PlatformDef>,
}

/// Database of all loaded levels
#[derive(Resource)]
struct LevelDatabase {
    levels: Vec<LevelData>,
}

impl Default for LevelDatabase {
    fn default() -> Self {
        Self { levels: Vec::new() }
    }
}

impl LevelDatabase {
    /// Load levels from file, returns default hardcoded levels on error
    fn load_from_file(path: &str) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => Self::parse(&content),
            Err(e) => {
                warn!("Failed to load levels from {}: {}, using defaults", path, e);
                Self::default_levels()
            }
        }
    }

    /// Parse level data from string
    fn parse(content: &str) -> Self {
        let mut levels = Vec::new();
        let mut current_level: Option<LevelData> = None;

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(name) = line.strip_prefix("level:") {
                // Save previous level if exists
                if let Some(level) = current_level.take() {
                    levels.push(level);
                }
                // Start new level
                current_level = Some(LevelData {
                    name: name.trim().to_string(),
                    basket_height: 400.0, // default
                    platforms: Vec::new(),
                });
            } else if let Some(height_str) = line.strip_prefix("basket:") {
                if let Some(level) = &mut current_level {
                    if let Ok(height) = height_str.trim().parse::<f32>() {
                        level.basket_height = height;
                    }
                }
            } else if let Some(params) = line.strip_prefix("mirror:") {
                if let Some(level) = &mut current_level {
                    let parts: Vec<&str> = params.trim().split_whitespace().collect();
                    if parts.len() >= 3 {
                        if let (Ok(x), Ok(y), Ok(w)) = (
                            parts[0].parse::<f32>(),
                            parts[1].parse::<f32>(),
                            parts[2].parse::<f32>(),
                        ) {
                            level.platforms.push(PlatformDef::Mirror { x, y, width: w });
                        }
                    }
                }
            } else if let Some(params) = line.strip_prefix("center:") {
                if let Some(level) = &mut current_level {
                    let parts: Vec<&str> = params.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let (Ok(y), Ok(w)) = (
                            parts[0].parse::<f32>(),
                            parts[1].parse::<f32>(),
                        ) {
                            level.platforms.push(PlatformDef::Center { y, width: w });
                        }
                    }
                }
            }
        }

        // Don't forget the last level
        if let Some(level) = current_level {
            levels.push(level);
        }

        if levels.is_empty() {
            warn!("No levels parsed, using defaults");
            return Self::default_levels();
        }

        info!("Loaded {} levels from file", levels.len());
        Self { levels }
    }

    /// Hardcoded fallback levels
    fn default_levels() -> Self {
        Self {
            levels: vec![
                LevelData {
                    name: "Simple".to_string(),
                    basket_height: 350.0,
                    platforms: vec![PlatformDef::Mirror { x: 400.0, y: 150.0, width: 200.0 }],
                },
                LevelData {
                    name: "Default".to_string(),
                    basket_height: 400.0,
                    platforms: vec![
                        PlatformDef::Mirror { x: 400.0, y: 150.0, width: 180.0 },
                        PlatformDef::Center { y: 280.0, width: 200.0 },
                    ],
                },
            ],
        }
    }

    /// Save all levels to file
    fn get(&self, index: usize) -> Option<&LevelData> {
        self.levels.get(index)
    }

    fn len(&self) -> usize {
        self.levels.len()
    }
}

// Components

#[derive(Component)]
struct Player;

#[derive(Component, Default)]
struct Velocity(Vec2);

#[derive(Component)]
struct Grounded(bool);

#[derive(Component, Default)]
struct CoyoteTimer(f32); // Time remaining where jump is still allowed after leaving ground

#[derive(Component, Default)]
struct JumpState {
    is_jumping: bool, // Currently in a jump (for variable height)
}

#[derive(Component, Default)]
struct Collider;

#[derive(Component)]
#[require(Collider)]
struct Platform;

#[derive(Component)]
struct LevelPlatform; // Marks platforms that belong to current level (despawned on level change)

// Player state
#[derive(Component)]
struct Facing(f32); // -1.0 = left, 1.0 = right

impl Default for Facing {
    fn default() -> Self {
        Self(1.0) // Default facing right
    }
}

#[derive(Component)]
struct HoldingBall(Entity); // Reference to held ball

#[derive(Component, Default)]
struct ChargingShot {
    charge_time: f32, // How long throw button has been held
}

#[derive(Component)]
struct TargetBasket(Basket); // Which basket player is aiming at

impl Default for TargetBasket {
    fn default() -> Self {
        Self(Basket::Right) // Default targeting right basket
    }
}

#[derive(Component)]
struct TargetMarker; // White marker shown in targeted basket

// Ball components
#[derive(Component)]
struct Ball;

#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
enum BallState {
    #[default]
    Free,
    Held(Entity),                              // Entity = player holding it
    InFlight { shooter: Entity, power: f32 },  // Who shot it and how hard
}

#[derive(Component, Default)]
struct BallPlayerContact {
    overlapping: bool, // Track if currently overlapping to apply effects once on entry
}

#[derive(Component, Default)]
struct BallPulse {
    timer: f32, // Animation timer for pickup indicator
}

#[derive(Component, Default)]
struct BallRolling(bool); // True when ball is rolling on ground

#[derive(Component, Default)]
struct BallShotGrace(f32); // Timer for post-shot grace period (no friction/player drag)

#[derive(Component, Clone, Copy, PartialEq)]
enum Basket {
    Left,
    Right,
}

#[derive(Component)]
struct DebugText;

#[derive(Component)]
struct ScoreLevelText;

#[derive(Component)]
struct ChargeGaugeBackground;

#[derive(Component)]
struct ChargeGaugeFill;

#[derive(Component)]
struct TweakPanel;

#[derive(Component)]
struct TweakRow(usize); // Index of this row's parameter

#[derive(Component)]
struct ScoreFlash {
    timer: f32,            // Time remaining in flash
    flash_color: Color,    // Color to flash to
    original_color: Color, // Color to restore after flash
}

// Systems

fn setup(mut commands: Commands, level_db: Res<LevelDatabase>) {
    // Camera - orthographic, shows entire arena
    // Using scale to zoom out and show full arena (default is 1.0 = 1 pixel per unit)
    // With 1600x900 arena, we need to scale so it fits in typical window sizes
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 0.0, 0.0),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.5, // Zoom out to show more of the arena
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Player - spawns above the floor
    let player_entity = commands
        .spawn((
            Sprite::from_color(PLAYER_COLOR, PLAYER_SIZE),
            Transform::from_translation(PLAYER_SPAWN),
            Player,
            Velocity::default(),
            Grounded(false),
            CoyoteTimer::default(),
            JumpState::default(),
            Facing::default(),
            ChargingShot::default(),
            TargetBasket::default(),
            Collider,
        ))
        .id();

    // Target marker - white indicator shown in targeted basket
    commands.spawn((
        Sprite::from_color(Color::WHITE, Vec2::new(20.0, 20.0)),
        Transform::from_xyz(0.0, 0.0, 0.5), // Position updated by update_target_marker
        TargetMarker,
    ));

    // Charge gauge - inside player, opposite side of ball
    // Start on left side (default facing is right, so ball is right, gauge is left)
    let gauge_x = -PLAYER_SIZE.x / 4.0;

    // Background (black bar, always visible, centered vertically on player)
    let gauge_bg = commands
        .spawn((
            Sprite::from_color(Color::BLACK, Vec2::new(CHARGE_GAUGE_WIDTH, CHARGE_GAUGE_HEIGHT)),
            Transform::from_xyz(gauge_x, 0.0, 0.5),
            ChargeGaugeBackground,
        ))
        .id();
    commands.entity(player_entity).add_child(gauge_bg);

    // Fill (green->red, scales with charge) - starts invisible
    let gauge_fill = commands
        .spawn((
            Sprite::from_color(Color::srgb(0.0, 0.8, 0.0), Vec2::new(CHARGE_GAUGE_WIDTH - 2.0, CHARGE_GAUGE_HEIGHT - 2.0)),
            Transform::from_xyz(gauge_x, 0.0, 0.6).with_scale(Vec3::new(1.0, 0.0, 1.0)),
            ChargeGaugeFill,
        ))
        .id();
    commands.entity(player_entity).add_child(gauge_fill);

    // Ball
    commands.spawn((
        Sprite::from_color(BALL_COLOR, BALL_SIZE),
        Transform::from_translation(BALL_SPAWN),
        Ball,
        BallState::default(),
        Velocity::default(),
        BallPlayerContact::default(),
        BallPulse::default(),
        BallRolling::default(),
        BallShotGrace::default(),
    ));

    // Arena floor (spans most of the arena width)
    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, Vec2::new(ARENA_WIDTH - 100.0, 40.0)),
        Transform::from_xyz(0.0, ARENA_FLOOR_Y, 0.0),
        Platform,
    ));

    // Left wall (extends high above viewport)
    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, Vec2::new(40.0, 5000.0)),
        Transform::from_xyz(-ARENA_WIDTH / 2.0 + 20.0, 2000.0, 0.0),
        Platform,
    ));

    // Right wall (extends high above viewport)
    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, Vec2::new(40.0, 5000.0)),
        Transform::from_xyz(ARENA_WIDTH / 2.0 - 20.0, 2000.0, 0.0),
        Platform,
    ));

    // Spawn level 1 platforms
    spawn_level_platforms(&mut commands, &level_db, 0);

    // Baskets (goals) - height varies per level
    let basket_y = level_db.get(0).map(|l| ARENA_FLOOR_Y + l.basket_height).unwrap_or(ARENA_FLOOR_Y + 400.0);
    commands.spawn((
        Sprite::from_color(BASKET_COLOR, BASKET_SIZE),
        Transform::from_xyz(LEFT_BASKET_X, basket_y, -0.1), // Slightly behind
        Basket::Left,
    ));
    commands.spawn((
        Sprite::from_color(BASKET_COLOR, BASKET_SIZE),
        Transform::from_xyz(RIGHT_BASKET_X, basket_y, -0.1),
        Basket::Right,
    ));

    // Score/Level display - top center
    commands.spawn((
        Text::new("Score"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::BLACK),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        ScoreLevelText,
    ));

    // Debug UI - bottom center (shot info)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::BLACK),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
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
                TextColor(Color::WHITE),
            ));
            parent.spawn((
                Text::new("Up/Down: select | Left/Right: +/-10%"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));
            parent.spawn((
                Text::new("R: reset selected | Shift+R: reset all"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
            ));

            // Create a row for each tweakable parameter
            for i in 0..PhysicsTweaks::LABELS.len() {
                parent.spawn((
                    Text::new(format!("{}: ---", PhysicsTweaks::LABELS[i])),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    TweakRow(i),
                ));
            }
        });
}

/// Runs in Update to capture input state before it's cleared
fn capture_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut input: ResMut<PlayerInput>,
    tweaks: Res<PhysicsTweaks>,
    time: Res<Time>,
) {
    // Don't capture game input when tweak panel is open (uses arrow keys)
    if tweaks.panel_visible {
        return;
    }
    // Horizontal movement (continuous - overwrite each frame)
    let mut move_x = 0.0;

    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        move_x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        move_x += 1.0;
    }

    for gamepad in &gamepads {
        if let Some(stick_x) = gamepad.get(GamepadAxis::LeftStickX) {
            if stick_x.abs() > STICK_DEADZONE {
                move_x += stick_x;
            }
        }
    }

    input.move_x = move_x.clamp(-1.0, 1.0);

    // Jump button state
    let jump_pressed = keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::KeyW)
        || keyboard.just_pressed(KeyCode::ArrowUp)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::South));

    input.jump_held = keyboard.pressed(KeyCode::Space)
        || keyboard.pressed(KeyCode::KeyW)
        || keyboard.pressed(KeyCode::ArrowUp)
        || gamepads.iter().any(|gp| gp.pressed(GamepadButton::South));

    // Jump buffering - reset timer on press, count down otherwise
    if jump_pressed {
        input.jump_buffer_timer = JUMP_BUFFER_TIME;
    } else {
        input.jump_buffer_timer = (input.jump_buffer_timer - time.delta_secs()).max(0.0);
    }

    // Pickup (West button / E key) - accumulate until consumed
    if keyboard.just_pressed(KeyCode::KeyE)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::West))
    {
        input.pickup_pressed = true;
    }

    // Throw (R shoulder / F key)
    let throw_held_now = keyboard.pressed(KeyCode::KeyF)
        || gamepads.iter().any(|gp| gp.pressed(GamepadButton::RightTrigger));

    // Accumulate throw_released until consumed (like jump buffering)
    if input.throw_held && !throw_held_now {
        input.throw_released = true;
    }
    input.throw_held = throw_held_now;

    // Cycle target (L shoulder / Q key) - accumulate until consumed
    if keyboard.just_pressed(KeyCode::KeyQ)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::LeftTrigger2))
    {
        input.cycle_target_pressed = true;
    }
}

fn respawn_player(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut commands: Commands,
    level_db: Res<LevelDatabase>,
    mut current_level: ResMut<CurrentLevel>,
    mut player: Query<(Entity, &mut Transform, &mut Velocity, Option<&HoldingBall>), With<Player>>,
    mut ball: Query<(&mut Transform, &mut Velocity, &mut BallState, &mut BallRolling), (With<Ball>, Without<Player>)>,
    level_platforms: Query<Entity, With<LevelPlatform>>,
    mut baskets: Query<&mut Transform, (With<Basket>, Without<Player>, Without<Ball>)>,
) {
    let respawn_pressed = keyboard.just_pressed(KeyCode::KeyR)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::Start));

    if respawn_pressed {
        // Reset player
        if let Ok((player_entity, mut p_transform, mut p_velocity, holding)) = player.single_mut() {
            p_transform.translation = PLAYER_SPAWN;
            p_velocity.0 = Vec2::ZERO;

            // Drop ball if holding
            if holding.is_some() {
                commands.entity(player_entity).remove::<HoldingBall>();
            }
        }

        // Reset ball
        if let Ok((mut b_transform, mut b_velocity, mut b_state, mut b_rolling)) = ball.single_mut() {
            b_transform.translation = BALL_SPAWN;
            b_velocity.0 = Vec2::ZERO;
            *b_state = BallState::Free;
            b_rolling.0 = false;
        }

        // Cycle to next level (0-indexed internally)
        let num_levels = level_db.len();
        current_level.0 = (current_level.0 % num_levels as u32) + 1;
        let level_index = (current_level.0 - 1) as usize;

        // Despawn old level platforms
        for entity in &level_platforms {
            commands.entity(entity).despawn();
        }

        // Spawn new level platforms
        spawn_level_platforms(&mut commands, &level_db, level_index);

        // Update basket heights for new level
        if let Some(level) = level_db.get(level_index) {
            let basket_y = ARENA_FLOOR_Y + level.basket_height;
            for mut basket_transform in &mut baskets {
                basket_transform.translation.y = basket_y;
            }
        }
    }
}

/// Runs in FixedUpdate to apply captured input to physics
fn apply_input(
    mut input: ResMut<PlayerInput>,
    tweaks: Res<PhysicsTweaks>,
    mut player: Query<
        (&mut Velocity, &mut CoyoteTimer, &mut JumpState, &mut Facing, &Grounded),
        With<Player>,
    >,
    time: Res<Time>,
) {
    let Ok((mut velocity, mut coyote, mut jump_state, mut facing, grounded)) =
        player.single_mut()
    else {
        return;
    };

    // Acceleration-based horizontal movement
    let target_speed = input.move_x * tweaks.move_speed;
    let current_speed = velocity.0.x;

    // Determine if accelerating (toward input) or decelerating (stopping/reversing)
    let has_input = input.move_x.abs() > STICK_DEADZONE;
    let same_direction = target_speed.signum() == current_speed.signum() || current_speed.abs() < 1.0;
    let is_accelerating = has_input && same_direction;

    // Select appropriate acceleration rate based on ground state and direction
    let rate = if grounded.0 {
        if is_accelerating { tweaks.ground_accel } else { tweaks.ground_decel }
    } else {
        if is_accelerating { tweaks.air_accel } else { tweaks.air_decel }
    };

    velocity.0.x = move_toward(current_speed, target_speed, rate * time.delta_secs());

    // Update facing direction based on input (not velocity, so turning feels responsive)
    if input.move_x > STICK_DEADZONE {
        facing.0 = 1.0;
    } else if input.move_x < -STICK_DEADZONE {
        facing.0 = -1.0;
    }

    // Update coyote timer
    if grounded.0 {
        coyote.0 = COYOTE_TIME;
        jump_state.is_jumping = false; // Reset jump state when grounded
    } else {
        coyote.0 = (coyote.0 - time.delta_secs()).max(0.0);
    }

    // Can jump if grounded OR within coyote time
    let can_jump = grounded.0 || coyote.0 > 0.0;

    // Jump if we have buffered input and can jump
    if input.jump_buffer_timer > 0.0 && can_jump {
        velocity.0.y = tweaks.jump_velocity;
        input.jump_buffer_timer = 0.0; // Consume the buffered jump
        coyote.0 = 0.0; // Consume coyote time so we can't double jump
        jump_state.is_jumping = true; // Mark that we're in a jump
    }

    // Variable jump height: cut velocity if button released while rising
    // Check: in a jump + rising + button NOT held = cut velocity
    if jump_state.is_jumping && velocity.0.y > 0.0 && !input.jump_held {
        velocity.0.y *= JUMP_CUT_MULTIPLIER;
        jump_state.is_jumping = false; // Only cut once per jump
    }
}

fn apply_gravity(
    tweaks: Res<PhysicsTweaks>,
    mut query: Query<(&mut Velocity, &Grounded)>,
    time: Res<Time>,
) {
    for (mut velocity, grounded) in &mut query {
        if !grounded.0 {
            // Fast fall: use higher gravity when falling than rising
            let gravity = if velocity.0.y > 0.0 {
                tweaks.gravity_rise
            } else {
                tweaks.gravity_fall
            };
            velocity.0.y -= gravity * time.delta_secs();
        }
    }
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * time.delta_secs();
        transform.translation.y += velocity.0.y * time.delta_secs();
    }
}

fn check_collisions(
    mut player_query: Query<(&mut Transform, &mut Velocity, &mut Grounded, &Sprite), With<Player>>,
    platform_query: Query<(&Transform, &Sprite), (With<Platform>, Without<Player>)>,
) {
    let Ok((mut player_transform, mut player_velocity, mut grounded, player_sprite)) =
        player_query.single_mut()
    else {
        return;
    };

    let player_size = player_sprite.custom_size.unwrap_or(PLAYER_SIZE);
    let player_half = player_size / 2.0;

    // Assume not grounded until we find a floor beneath us
    grounded.0 = false;

    for (platform_transform, platform_sprite) in &platform_query {
        let platform_size = platform_sprite.custom_size.unwrap_or(Vec2::new(100.0, 20.0));
        let platform_half = platform_size / 2.0;

        let player_pos = player_transform.translation.truncate();
        let platform_pos = platform_transform.translation.truncate();

        // Calculate overlap
        let diff = player_pos - platform_pos;
        let overlap_x = player_half.x + platform_half.x - diff.x.abs();
        let overlap_y = player_half.y + platform_half.y - diff.y.abs();

        // No collision
        if overlap_x <= 0.0 || overlap_y <= 0.0 {
            continue;
        }

        // Resolve collision along the smallest overlap axis
        if overlap_y < overlap_x {
            // Vertical collision
            if diff.y > 0.0 {
                // Player is above - land on platform
                // Position slightly inside (EPSILON) so next frame still detects collision
                player_transform.translation.y =
                    platform_pos.y + platform_half.y + player_half.y - COLLISION_EPSILON;
                if player_velocity.0.y <= 0.0 {
                    player_velocity.0.y = 0.0;
                    grounded.0 = true;
                }
            } else {
                // Player hit ceiling
                player_transform.translation.y =
                    platform_pos.y - platform_half.y - player_half.y + COLLISION_EPSILON;
                if player_velocity.0.y > 0.0 {
                    player_velocity.0.y = 0.0;
                }
            }
        } else {
            // Horizontal collision - push player out
            if diff.x > 0.0 {
                player_transform.translation.x =
                    platform_pos.x + platform_half.x + player_half.x - COLLISION_EPSILON;
            } else {
                player_transform.translation.x =
                    platform_pos.x - platform_half.x - player_half.x + COLLISION_EPSILON;
            }
            // Don't zero horizontal velocity - let player slide along walls
        }
    }
}

fn ball_gravity(
    tweaks: Res<PhysicsTweaks>,
    mut query: Query<(&mut Velocity, &BallState, &BallRolling, &mut BallShotGrace), With<Ball>>,
    time: Res<Time>,
) {
    for (mut velocity, state, rolling, mut grace) in &mut query {
        // Decrement grace timer
        if grace.0 > 0.0 {
            grace.0 = (grace.0 - time.delta_secs()).max(0.0);
        }

        match state {
            BallState::Free | BallState::InFlight { .. } => {
                if rolling.0 {
                    // Rolling on ground - no gravity, apply rolling friction (skip if grace active)
                    velocity.0.y = 0.0;
                    if grace.0 <= 0.0 {
                        velocity.0.x *= tweaks.ball_roll_friction.powf(time.delta_secs());
                    }
                } else {
                    // In air - apply gravity, apply air friction only if no grace
                    velocity.0.y -= tweaks.ball_gravity * time.delta_secs();
                    if grace.0 <= 0.0 {
                        velocity.0.x *= tweaks.ball_air_friction.powf(time.delta_secs());
                    }
                }
            }
            BallState::Held(_) => {
                // Ball follows player, no gravity
                velocity.0 = Vec2::ZERO;
            }
        }
    }
}

fn ball_collisions(
    tweaks: Res<PhysicsTweaks>,
    mut ball_query: Query<(&mut Transform, &mut Velocity, &BallState, &Sprite, &mut BallRolling), With<Ball>>,
    platform_query: Query<(&Transform, &Sprite), (With<Platform>, Without<Ball>)>,
) {
    for (mut ball_transform, mut ball_velocity, state, ball_sprite, mut rolling) in &mut ball_query {
        // Skip collision for held balls
        if matches!(state, BallState::Held(_)) {
            continue;
        }

        let ball_size = ball_sprite.custom_size.unwrap_or(BALL_SIZE);
        let ball_half = ball_size / 2.0;

        // Track if ball has ground contact this frame (for rolling detection)
        let was_rolling = rolling.0;
        let mut has_ground_contact = false;

        for (platform_transform, platform_sprite) in &platform_query {
            let platform_size = platform_sprite.custom_size.unwrap_or(Vec2::new(100.0, 20.0));
            let platform_half = platform_size / 2.0;

            let ball_pos = ball_transform.translation.truncate();
            let platform_pos = platform_transform.translation.truncate();

            let diff = ball_pos - platform_pos;
            let overlap_x = ball_half.x + platform_half.x - diff.x.abs();
            let overlap_y = ball_half.y + platform_half.y - diff.y.abs();

            if overlap_x <= 0.0 || overlap_y <= 0.0 {
                continue;
            }

            // Resolve collision with bounce
            if overlap_y < overlap_x {
                // Vertical collision
                if diff.y > 0.0 {
                    // Ball above platform (landed on floor)
                    has_ground_contact = true;
                    // Position slightly into platform so collision is detected next frame
                    ball_transform.translation.y =
                        platform_pos.y + platform_half.y + ball_half.y - COLLISION_EPSILON;
                    if ball_velocity.0.y < 0.0 {
                        // Apply ground friction to horizontal velocity
                        ball_velocity.0.x *= BALL_GROUND_FRICTION;

                        // Calculate post-bounce velocity
                        let post_bounce_vel = ball_velocity.0.y.abs() * tweaks.ball_bounce;
                        // Calculate max height: h = v² / (2g)
                        let max_bounce_height = (post_bounce_vel * post_bounce_vel) / (2.0 * tweaks.ball_gravity);

                        // Only bounce if ball will rise above threshold height
                        if max_bounce_height > ball_size.y * BALL_BOUNCE_HEIGHT_MULT {
                            ball_velocity.0.y = -ball_velocity.0.y * tweaks.ball_bounce;
                            rolling.0 = false; // Ball is bouncing, not rolling
                        } else {
                            // Bounce too small - start rolling
                            ball_velocity.0.y = 0.0;
                            rolling.0 = true;
                        }
                    }
                } else {
                    // Ball below platform (hit ceiling)
                    ball_transform.translation.y =
                        platform_pos.y - platform_half.y - ball_half.y;
                    if ball_velocity.0.y > 0.0 {
                        ball_velocity.0.y = -ball_velocity.0.y * tweaks.ball_bounce;
                    }
                }
            } else {
                // Horizontal collision - bounce off walls
                if diff.x > 0.0 {
                    ball_transform.translation.x =
                        platform_pos.x + platform_half.x + ball_half.x;
                } else {
                    ball_transform.translation.x =
                        platform_pos.x - platform_half.x - ball_half.x;
                }
                ball_velocity.0.x = -ball_velocity.0.x * tweaks.ball_bounce;
            }
        }

        // If ball was rolling but lost ground contact, start falling
        if was_rolling && !has_ground_contact {
            rolling.0 = false;
        }
    }
}

fn ball_state_update(mut ball_query: Query<(&Velocity, &mut BallState), With<Ball>>) {
    for (velocity, mut state) in &mut ball_query {
        // InFlight balls become Free when they slow down enough
        if matches!(*state, BallState::InFlight { .. }) {
            let speed = velocity.0.length();
            if speed < BALL_FREE_SPEED {
                *state = BallState::Free;
            }
        }
    }
}

fn ball_player_collision(
    mut ball_query: Query<
        (&Transform, &mut Velocity, &mut BallPlayerContact, &BallState, &Sprite, &mut BallRolling, &BallShotGrace),
        With<Ball>,
    >,
    mut player_query: Query<(&Transform, &mut Velocity, &Sprite), (With<Player>, Without<Ball>)>,
) {
    for (ball_transform, mut ball_velocity, mut contact, ball_state, ball_sprite, mut rolling, grace) in
        &mut ball_query
    {
        // Skip held balls
        if matches!(ball_state, BallState::Held(_)) {
            contact.overlapping = false;
            continue;
        }

        let ball_size = ball_sprite.custom_size.unwrap_or(BALL_SIZE);
        let ball_half = ball_size / 2.0;
        let ball_pos = ball_transform.translation.truncate();

        let mut is_overlapping = false;

        for (player_transform, mut player_velocity, player_sprite) in &mut player_query {
            let player_size = player_sprite.custom_size.unwrap_or(PLAYER_SIZE);
            let player_half = player_size / 2.0;
            let player_pos = player_transform.translation.truncate();

            let diff = ball_pos - player_pos;
            let overlap_x = ball_half.x + player_half.x - diff.x.abs();
            let overlap_y = ball_half.y + player_half.y - diff.y.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                is_overlapping = true;

                // Only apply effects on first frame of contact, and skip if in grace period
                if !contact.overlapping && grace.0 <= 0.0 {
                    let ball_speed = ball_velocity.0.length();
                    let player_speed = player_velocity.0.length();

                    // Both slow down when passing through each other
                    // Ball has higher Y friction (gravity effect) than X
                    ball_velocity.0.x *= BALL_PLAYER_DRAG_X;
                    ball_velocity.0.y *= BALL_PLAYER_DRAG_Y;
                    player_velocity.0 *= BALL_PLAYER_DRAG_X;

                    // If ball is slow/stationary and player is moving, kick the ball
                    if ball_speed < BALL_KICK_THRESHOLD && player_speed > BALL_KICK_THRESHOLD {
                        let kick_dir = if player_velocity.0.x > 0.0 { 1.0 } else { -1.0 };
                        ball_velocity.0.x += kick_dir * BALL_KICK_STRENGTH;
                        ball_velocity.0.y += BALL_KICK_STRENGTH * 0.3; // Small upward nudge
                        rolling.0 = false; // Ball is kicked into the air
                    }
                }
            }
        }

        contact.overlapping = is_overlapping;
    }
}

fn ball_follow_holder(
    mut ball_query: Query<(&mut Transform, &BallState), With<Ball>>,
    player_query: Query<(&Transform, &Facing), (With<Player>, Without<Ball>)>,
) {
    for (mut ball_transform, state) in &mut ball_query {
        if let BallState::Held(holder_entity) = state {
            if let Ok((player_transform, facing)) = player_query.get(*holder_entity) {
                // Position ball inside player, on facing side, at middle height
                ball_transform.translation.x =
                    player_transform.translation.x + facing.0 * (PLAYER_SIZE.x / 4.0);
                ball_transform.translation.y = player_transform.translation.y; // Center height
            }
        }
    }
}

fn pickup_ball(
    mut input: ResMut<PlayerInput>,
    mut commands: Commands,
    mut steal_contest: ResMut<StealContest>,
    mut non_holding_players: Query<(Entity, &Transform, &mut ChargingShot), (With<Player>, Without<HoldingBall>)>,
    holding_players: Query<(Entity, &Transform, &HoldingBall), With<Player>>,
    mut ball_query: Query<(Entity, &Transform, &mut BallState), With<Ball>>,
) {
    // If steal contest is active, count presses
    if steal_contest.active {
        if input.pickup_pressed {
            // For now, attribute press to attacker (single player)
            // In multiplayer, check which player pressed
            steal_contest.attacker_presses += 1;
            input.pickup_pressed = false; // Consume input
        }
        return; // Don't allow pickup during contest
    }

    if !input.pickup_pressed {
        return;
    }

    // Consume the input immediately
    input.pickup_pressed = false;

    // Check each non-holding player
    for (player_entity, player_transform, mut charging) in &mut non_holding_players {
        let player_pos = player_transform.translation.truncate();

        // First, try to pick up a free ball
        for (ball_entity, ball_transform, mut ball_state) in &mut ball_query {
            if *ball_state != BallState::Free {
                continue;
            }

            let distance = player_pos.distance(ball_transform.translation.truncate());

            if distance < BALL_PICKUP_RADIUS {
                *ball_state = BallState::Held(player_entity);
                commands.entity(player_entity).insert(HoldingBall(ball_entity));
                // Reset charge so it starts fresh (even if throw button is held)
                charging.charge_time = 0.0;
                return; // Done - picked up ball
            }
        }

        // If no free ball nearby, check for steal opportunity
        for (defender_entity, defender_transform, _holding) in &holding_players {
            let distance = player_pos.distance(defender_transform.translation.truncate());

            if distance < STEAL_RANGE {
                // Initiate steal contest
                steal_contest.active = true;
                steal_contest.attacker = Some(player_entity);
                steal_contest.defender = Some(defender_entity);
                steal_contest.attacker_presses = 1; // Count the initiating press
                steal_contest.defender_presses = STEAL_DEFENDER_ADVANTAGE;
                steal_contest.timer = STEAL_CONTEST_DURATION;
                return;
            }
        }
    }
}

fn steal_contest_update(
    mut commands: Commands,
    mut steal_contest: ResMut<StealContest>,
    time: Res<Time>,
    mut ball_query: Query<&mut BallState, With<Ball>>,
    holding_query: Query<&HoldingBall>,
) {
    if !steal_contest.active {
        return;
    }

    steal_contest.timer -= time.delta_secs();

    // TODO: In multiplayer, check defender's button presses here
    // For now, defender gets occasional "presses" to simulate resistance
    if steal_contest.timer > 0.0 && steal_contest.timer % 0.1 < time.delta_secs() {
        steal_contest.defender_presses += 1;
    }

    if steal_contest.timer <= 0.0 {
        // Contest ended - resolve it
        let attacker = steal_contest.attacker.unwrap();
        let defender = steal_contest.defender.unwrap();

        if steal_contest.attacker_presses > steal_contest.defender_presses {
            // Attacker wins - steal the ball
            if let Ok(holding) = holding_query.get(defender) {
                let ball_entity = holding.0;
                if let Ok(mut ball_state) = ball_query.get_mut(ball_entity) {
                    *ball_state = BallState::Held(attacker);
                    commands.entity(defender).remove::<HoldingBall>();
                    commands.entity(attacker).insert(HoldingBall(ball_entity));
                }
            }
        }
        // If defender wins or tie, they keep the ball (no action needed)

        // Reset contest
        *steal_contest = StealContest::default();
    }
}

fn update_shot_charge(
    input: Res<PlayerInput>,
    time: Res<Time>,
    mut player_query: Query<&mut ChargingShot, With<Player>>,
) {
    for mut charging in &mut player_query {
        if input.throw_held {
            charging.charge_time += time.delta_secs();
        }
        // Don't reset here - let throw_ball reset after using the charge
        // Otherwise charge resets to 0 before throw_ball can read it
    }
}

fn cycle_target(
    mut input: ResMut<PlayerInput>,
    mut player_query: Query<&mut TargetBasket, With<Player>>,
    baskets: Query<&Basket>,
) {
    if !input.cycle_target_pressed {
        return;
    }
    input.cycle_target_pressed = false; // Consume input

    // Collect unique basket types available
    let mut has_left = false;
    let mut has_right = false;
    for basket in &baskets {
        match basket {
            Basket::Left => has_left = true,
            Basket::Right => has_right = true,
        }
    }

    // Cycle to next available target
    for mut target in &mut player_query {
        target.0 = match target.0 {
            Basket::Left => {
                if has_right { Basket::Right } else { Basket::Left }
            }
            Basket::Right => {
                if has_left { Basket::Left } else { Basket::Right }
            }
        };
    }
}

fn update_target_marker(
    player_query: Query<(&Transform, &TargetBasket), With<Player>>,
    baskets: Query<(&Transform, &Basket), Without<TargetMarker>>,
    mut marker_query: Query<&mut Transform, (With<TargetMarker>, Without<Player>)>,
) {
    let Ok((player_transform, target)) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation.truncate();

    // Find the closest basket matching the target type
    let target_pos = baskets
        .iter()
        .filter(|(_, basket)| **basket == target.0)
        .min_by(|(a, _), (b, _)| {
            let dist_a = player_pos.distance_squared(a.translation.truncate());
            let dist_b = player_pos.distance_squared(b.translation.truncate());
            dist_a.partial_cmp(&dist_b).unwrap()
        })
        .map(|(transform, _)| transform.translation);

    let Some(basket_pos) = target_pos else {
        return;
    };

    // Move marker to target basket
    for mut marker_transform in &mut marker_query {
        marker_transform.translation.x = basket_pos.x;
        marker_transform.translation.y = basket_pos.y;
    }
}

/// Shot trajectory result containing angle, speed, and variance penalties
struct ShotTrajectory {
    angle: f32,           // Absolute angle in radians (0=right, π/2=up, π=left)
    speed: f32,           // Ball speed (usually max, but can vary for special cases)
    distance_penalty: f32, // Variance penalty for distance
}

/// Calculate shot trajectory to hit target at max speed.
/// Returns the angle needed to hit target, always using full power.
fn calculate_shot_trajectory(
    shooter_pos: Vec2,
    target_pos: Vec2,
    gravity: f32,
    max_speed: f32,
) -> Option<ShotTrajectory> {
    let delta = target_pos - shooter_pos;
    let tx = delta.x; // Positive = target is right, negative = left
    let ty = delta.y; // Positive = target is above, negative = below
    let dx = tx.abs(); // Horizontal distance (always positive)

    // Directly under/over target - shoot straight up or down
    if dx < 1.0 {
        if ty > 0.0 {
            // Target above - shoot straight up at full speed
            return Some(ShotTrajectory {
                angle: std::f32::consts::FRAC_PI_2, // 90° straight up
                speed: max_speed,
                distance_penalty: 0.0,
            });
        } else {
            // Target below - shoot straight down at half speed
            return Some(ShotTrajectory {
                angle: -std::f32::consts::FRAC_PI_2, // -90° straight down
                speed: max_speed * 0.5,
                distance_penalty: 0.0,
            });
        }
    }

    // Calculate angle to hit target at max_speed
    // Quadratic: k*u² - dx*u + (ty + k) = 0, where u = tan(θ), k = g*dx²/(2v²)
    let v2 = max_speed * max_speed;
    let k = gravity * dx * dx / (2.0 * v2);
    let discriminant = dx * dx - 4.0 * k * (ty + k);

    if discriminant < 0.0 {
        return None; // Target out of range at max speed
    }

    let sqrt_d = discriminant.sqrt();
    let u_high = (dx + sqrt_d) / (2.0 * k); // High trajectory (lob)
    let elevation = u_high.atan();

    // Convert elevation to absolute angle based on target direction
    let angle = if tx >= 0.0 {
        elevation
    } else {
        std::f32::consts::PI - elevation
    };

    // Distance penalty for variance
    let distance = delta.length();
    let distance_penalty = distance * SHOT_DISTANCE_VARIANCE_FACTOR;

    Some(ShotTrajectory {
        angle,
        speed: max_speed,
        distance_penalty,
    })
}

fn throw_ball(
    mut input: ResMut<PlayerInput>,
    tweaks: Res<PhysicsTweaks>,
    mut commands: Commands,
    mut shot_info: ResMut<LastShotInfo>,
    mut player_query: Query<
        (Entity, &Transform, &Velocity, &TargetBasket, &Grounded, &mut ChargingShot, Option<&HoldingBall>),
        With<Player>,
    >,
    mut ball_query: Query<
        (&mut Velocity, &mut BallState, &mut BallRolling, &mut BallShotGrace),
        (With<Ball>, Without<Player>),
    >,
    basket_query: Query<(&Transform, &Basket), Without<Player>>,
) {
    if !input.throw_released {
        return;
    }

    // Consume the throw_released flag immediately
    input.throw_released = false;

    let Ok((player_entity, player_transform, player_velocity, target, grounded, mut charging, holding)) =
        player_query.single_mut()
    else {
        return;
    };

    let Some(holding_ball) = holding else {
        // Not holding a ball - reset charge since they released the button
        charging.charge_time = 0.0;
        return;
    };

    let Ok((mut ball_velocity, mut ball_state, mut rolling, mut grace)) =
        ball_query.get_mut(holding_ball.0)
    else {
        return;
    };

    // Ball is being thrown - no longer rolling, start grace period
    rolling.0 = false;
    grace.0 = SHOT_GRACE_PERIOD;

    // Calculate charge percentage (0.0 to 1.0)
    let charge_pct = (charging.charge_time / tweaks.shot_charge_time).min(1.0);

    let mut rng = rand::thread_rng();
    let player_pos = player_transform.translation.truncate();

    // Find closest basket matching the target type
    let target_basket_pos = basket_query
        .iter()
        .filter(|(_, basket)| **basket == target.0)
        .min_by(|(a, _), (b, _)| {
            let dist_a = player_pos.distance_squared(a.translation.truncate());
            let dist_b = player_pos.distance_squared(b.translation.truncate());
            dist_a.partial_cmp(&dist_b).unwrap()
        })
        .map(|(transform, _)| transform.translation.truncate());

    // Calculate optimal trajectory to basket
    let trajectory = if let Some(basket_pos) = target_basket_pos {
        calculate_shot_trajectory(player_pos, basket_pos, BALL_GRAVITY, SHOT_MAX_SPEED)
    } else {
        None
    };

    // Base variance from charge level: 50% at 0 charge → 2% at full charge
    let base_variance = SHOT_MAX_VARIANCE - (SHOT_MAX_VARIANCE - SHOT_MIN_VARIANCE) * charge_pct;
    let mut variance = base_variance;

    // Air shot penalty: +10% variance when airborne
    let air_penalty = if !grounded.0 { SHOT_AIR_VARIANCE_PENALTY } else { 0.0 };
    variance += air_penalty;

    // Horizontal movement penalty: 0-10% variance based on horizontal speed
    let move_penalty = (player_velocity.0.x.abs() / MOVE_SPEED).min(1.0) * SHOT_MOVE_VARIANCE_PENALTY;
    variance += move_penalty;

    // Get base angle and speed from trajectory, add penalties to variance
    let (base_angle, base_speed, distance_penalty) = if let Some(traj) = &trajectory {
        variance += traj.distance_penalty;
        (traj.angle, traj.speed, traj.distance_penalty)
    } else {
        // Fallback for impossible trajectories - 45° toward target or right
        let fallback_angle = if let Some(basket_pos) = target_basket_pos {
            if basket_pos.x >= player_pos.x {
                std::f32::consts::FRAC_PI_4 // 45° right
            } else {
                std::f32::consts::PI - std::f32::consts::FRAC_PI_4 // 135° left
            }
        } else {
            std::f32::consts::FRAC_PI_4 // Default: 45° right
        };
        (fallback_angle, SHOT_MAX_SPEED, 0.0)
    };

    // Apply variance to angle (max ±30° at full variance)
    let max_angle_variance = 30.0_f32.to_radians();
    let angle_variance = rng.gen_range(-variance..variance) * max_angle_variance;
    let final_angle = base_angle + angle_variance;
    let final_speed = base_speed;

    // Convert angle + speed to velocity (simple and direct!)
    // Angle is absolute: 0=right, π/2=up, π=left
    let vx = final_speed * final_angle.cos();
    let vy = final_speed * final_angle.sin();

    // Set ball velocity
    ball_velocity.0.x = vx;
    ball_velocity.0.y = vy;

    *ball_state = BallState::InFlight {
        shooter: player_entity,
        power: final_speed,
    };

    // Record shot info for debug display
    let angle_degrees = final_angle.to_degrees();
    *shot_info = LastShotInfo {
        power: final_speed,
        arc: final_angle.tan(), // For legacy display compatibility
        angle_degrees,
        speed: final_speed,
        base_variance,
        air_penalty,
        move_penalty,
        distance_penalty,
        arc_penalty: 0.0, // No longer used
        total_variance: variance,
        target: Some(target.0),
    };

    // Reset charge and release ball
    charging.charge_time = 0.0;
    commands.entity(player_entity).remove::<HoldingBall>();
}

fn check_scoring(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut ball_query: Query<(&mut Transform, &mut Velocity, &mut BallState, &Sprite), With<Ball>>,
    basket_query: Query<(Entity, &Transform, &Basket, &Sprite), Without<Ball>>,
    player_query: Query<(Entity, &Sprite), With<Player>>,
) {
    for (mut ball_transform, mut ball_velocity, mut ball_state, _ball_sprite) in &mut ball_query {
        let ball_pos = ball_transform.translation.truncate();
        let is_held = matches!(*ball_state, BallState::Held(_));

        for (basket_entity, basket_transform, basket, basket_sprite) in &basket_query {
            let basket_size = basket_sprite.custom_size.unwrap_or(BASKET_SIZE);
            let basket_pos = basket_transform.translation.truncate();
            let basket_half = basket_size / 2.0;

            // Check if ball center is inside basket
            let in_basket = ball_pos.x > basket_pos.x - basket_half.x
                && ball_pos.x < basket_pos.x + basket_half.x
                && ball_pos.y > basket_pos.y - basket_half.y
                && ball_pos.y < basket_pos.y + basket_half.y;

            if in_basket {
                // Determine points: 2 for carry-in, 1 for throw
                let points = if is_held { 2 } else { 1 };

                match basket {
                    Basket::Left => score.left += points,
                    Basket::Right => score.right += points,
                }

                // Flash the basket (gold/yellow for carry-in, white for throw)
                let flash_color = if is_held {
                    Color::srgb(1.0, 0.85, 0.0) // Gold for 2-point carry
                } else {
                    Color::srgb(1.0, 1.0, 1.0) // White for 1-point throw
                };
                commands.entity(basket_entity).insert(ScoreFlash {
                    timer: 0.6,
                    flash_color,
                    original_color: BASKET_COLOR,
                });

                // If held, also flash the player who scored
                if let BallState::Held(holder) = *ball_state {
                    if let Ok((player_entity, _player_sprite)) = player_query.get(holder) {
                        commands.entity(player_entity).insert(ScoreFlash {
                            timer: 0.6,
                            flash_color,
                            original_color: PLAYER_COLOR,
                        });
                        // Remove HoldingBall from the player
                        commands.entity(player_entity).remove::<HoldingBall>();
                    }
                }

                // Reset ball to center
                ball_transform.translation = BALL_SPAWN;
                ball_velocity.0 = Vec2::ZERO;
                *ball_state = BallState::Free;

                info!("SCORE {}pts! Left: {} Right: {}", points, score.left, score.right);
            }
        }
    }
}

fn animate_score_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut ScoreFlash)>,
) {
    for (entity, mut sprite, mut flash) in &mut query {
        flash.timer -= time.delta_secs();

        if flash.timer <= 0.0 {
            // Flash complete - restore original color
            sprite.color = flash.original_color;
            commands.entity(entity).remove::<ScoreFlash>();
        } else {
            // Fast flicker between flash color and original
            let t = (flash.timer * 25.0).sin(); // ~4 flashes per 0.6 seconds
            let blend = (t + 1.0) / 2.0; // 0 to 1

            // Extract RGB from both colors and interpolate
            let flash_rgba = flash.flash_color.to_srgba();
            let orig_rgba = flash.original_color.to_srgba();

            sprite.color = Color::srgb(
                orig_rgba.red + (flash_rgba.red - orig_rgba.red) * blend,
                orig_rgba.green + (flash_rgba.green - orig_rgba.green) * blend,
                orig_rgba.blue + (flash_rgba.blue - orig_rgba.blue) * blend,
            );
        }
    }
}

fn toggle_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<DebugSettings>,
    mut text_query: Query<&mut Visibility, With<DebugText>>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        settings.visible = !settings.visible;
        if let Ok(mut visibility) = text_query.single_mut() {
            *visibility = if settings.visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn animate_pickable_ball(
    time: Res<Time>,
    players: Query<(&Transform, Option<&HoldingBall>), With<Player>>,
    mut ball_query: Query<(&Transform, &BallState, &mut Sprite, &mut BallPulse), With<Ball>>,
) {
    for (ball_transform, ball_state, mut sprite, mut pulse) in &mut ball_query {
        // Only pulse if ball is Free
        if *ball_state != BallState::Free {
            // Reset to normal when not free
            sprite.custom_size = Some(BALL_SIZE);
            sprite.color = BALL_COLOR;
            pulse.timer = 0.0;
            continue;
        }

        let ball_pos = ball_transform.translation.truncate();

        // Check if any player without a ball is close enough to pick up
        let mut can_pickup = false;
        for (player_transform, holding) in &players {
            if holding.is_some() {
                continue; // Player already has a ball
            }
            let distance = ball_pos.distance(player_transform.translation.truncate());
            if distance < BALL_PICKUP_RADIUS {
                can_pickup = true;
                break;
            }
        }

        if can_pickup {
            // Animate pulse - 5 cycles per second
            // Pattern: dark -> regular -> light -> regular
            pulse.timer += time.delta_secs();
            let t = pulse.timer * 5.0 * std::f32::consts::TAU;
            let pulse_factor = -(t.cos()); // -1 (dark) -> 0 (regular) -> 1 (light) -> 0 (regular)

            // Size: pulse between 97% and 103% (subtle)
            let scale_factor = 1.0 + 0.03 * pulse_factor;
            sprite.custom_size = Some(BALL_SIZE * scale_factor);

            // Color interpolation: dark orange <-> regular orange <-> light orange-cyan mix
            // Regular orange: (0.9, 0.5, 0.1)
            // Dark orange: (0.5, 0.25, 0.05)
            // Light (orange + cyan-white): (0.95, 0.75, 0.55)
            let (r, g, b) = if pulse_factor < 0.0 {
                // Dark to regular (pulse_factor: -1 to 0)
                let blend = pulse_factor + 1.0; // 0 to 1
                (
                    0.5 + 0.4 * blend,   // 0.5 -> 0.9
                    0.25 + 0.25 * blend, // 0.25 -> 0.5
                    0.05 + 0.05 * blend, // 0.05 -> 0.1
                )
            } else {
                // Regular to light (pulse_factor: 0 to 1)
                let blend = pulse_factor; // 0 to 1
                (
                    0.9 + 0.05 * blend,  // 0.9 -> 0.95
                    0.5 + 0.25 * blend,  // 0.5 -> 0.75
                    0.1 + 0.45 * blend,  // 0.1 -> 0.55
                )
            };
            sprite.color = Color::srgb(r, g, b);
        } else {
            // Reset to normal
            sprite.custom_size = Some(BALL_SIZE);
            sprite.color = BALL_COLOR;
            pulse.timer = 0.0;
        }
    }
}

fn update_charge_gauge(
    tweaks: Res<PhysicsTweaks>,
    player_query: Query<(&ChargingShot, &Facing, &Children, Option<&HoldingBall>), With<Player>>,
    mut bg_query: Query<&mut Transform, (With<ChargeGaugeBackground>, Without<ChargeGaugeFill>)>,
    mut fill_query: Query<(&mut Sprite, &mut Transform), With<ChargeGaugeFill>>,
) {
    // Gauge inside player, opposite side of ball
    let fill_height = CHARGE_GAUGE_HEIGHT - 2.0;

    for (charging, facing, children, holding) in &player_query {
        // Gauge is inside player, opposite side of facing (ball is on facing side)
        let gauge_x = -facing.0 * (PLAYER_SIZE.x / 4.0);

        for child in children.iter() {
            // Update background position
            if let Ok(mut bg_transform) = bg_query.get_mut(child) {
                bg_transform.translation.x = gauge_x;
            }

            // Update fill position, scale, and color
            if let Ok((mut sprite, mut transform)) = fill_query.get_mut(child) {
                transform.translation.x = gauge_x;

                let charge_pct = (charging.charge_time / tweaks.shot_charge_time).min(1.0);

                // Only show fill when holding ball and charging
                if holding.is_none() || charging.charge_time < 0.001 {
                    // Not charging - hide the fill (scale to 0)
                    transform.scale.y = 0.0;
                } else {
                    // Charging - show fill scaled by percentage
                    transform.scale.y = charge_pct;

                    // Offset Y so bar grows from bottom
                    // At 0%: bar is at bottom (y = -height/2 + 0)
                    // At 100%: bar is centered (y = 0)
                    let y_offset = -fill_height / 2.0 * (1.0 - charge_pct);
                    transform.translation.y = y_offset;

                    // Color transition: green (0%) -> red (100%)
                    let r = charge_pct * 0.9;
                    let g = (1.0 - charge_pct) * 0.8;
                    sprite.color = Color::srgb(r, g, 0.0);
                }
            }
        }
    }
}

/// Helper to spawn a platform mirrored on both sides (symmetric)
fn spawn_mirrored_platform(commands: &mut Commands, x: f32, y: f32, width: f32) {
    // Left side
    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(width, 20.0)),
        Transform::from_xyz(-x, y, 0.0),
        Platform,
        LevelPlatform,
    ));
    // Right side (mirrored)
    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(width, 20.0)),
        Transform::from_xyz(x, y, 0.0),
        Platform,
        LevelPlatform,
    ));
}

/// Helper to spawn a centered platform
fn spawn_center_platform(commands: &mut Commands, y: f32, width: f32) {
    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(width, 20.0)),
        Transform::from_xyz(0.0, y, 0.0),
        Platform,
        LevelPlatform,
    ));
}

fn spawn_level_platforms(commands: &mut Commands, level_db: &LevelDatabase, level_index: usize) {
    let Some(level) = level_db.get(level_index) else {
        warn!("Level {} not found, spawning empty", level_index);
        return;
    };

    for platform in &level.platforms {
        match platform {
            PlatformDef::Mirror { x, y, width } => {
                spawn_mirrored_platform(commands, *x, ARENA_FLOOR_Y + y, *width);
            }
            PlatformDef::Center { y, width } => {
                spawn_center_platform(commands, ARENA_FLOOR_Y + y, *width);
            }
        }
    }
}

fn update_debug_text(
    debug_settings: Res<DebugSettings>,
    shot_info: Res<LastShotInfo>,
    steal_contest: Res<StealContest>,
    mut text_query: Query<&mut Text, With<DebugText>>,
) {
    if !debug_settings.visible {
        return;
    }

    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let steal_str = if steal_contest.active {
        format!(
            " | Steal: A:{} D:{} ({:.1}s)",
            steal_contest.attacker_presses,
            steal_contest.defender_presses,
            steal_contest.timer
        )
    } else {
        String::new()
    };

    // Show last shot info
    if shot_info.target.is_some() {
        let target_str = match shot_info.target {
            Some(Basket::Left) => "Left",
            Some(Basket::Right) => "Right",
            None => "?",
        };
        text.0 = format!(
            "Last Shot: {:.0}° {:.0}u/s | Power:{:.0} Arc:{:.2} | Variance: base {:.0}% + air {:.0}% + move {:.0}% + dist {:.0}% + arc {:.0}% = {:.0}% | Target: {}{}",
            shot_info.angle_degrees,
            shot_info.speed,
            shot_info.power,
            shot_info.arc,
            shot_info.base_variance * 100.0,
            shot_info.air_penalty * 100.0,
            shot_info.move_penalty * 100.0,
            shot_info.distance_penalty * 100.0,
            shot_info.arc_penalty * 100.0,
            shot_info.total_variance * 100.0,
            target_str,
            steal_str,
        );
    } else {
        text.0 = format!("No shots yet{}", steal_str);
    }
}

fn update_score_level_text(
    score: Res<Score>,
    current_level: Res<CurrentLevel>,
    level_db: Res<LevelDatabase>,
    mut text_query: Query<&mut Text, With<ScoreLevelText>>,
) {
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let level_index = (current_level.0 - 1) as usize;
    let level_name = level_db.get(level_index).map(|l| l.name.as_str()).unwrap_or("???");
    let num_levels = level_db.len();

    text.0 = format!(
        "Lv {}/{}: {}  |  {} - {}",
        current_level.0,
        num_levels,
        level_name,
        score.left,
        score.right,
    );
}

fn toggle_tweak_panel(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut tweaks: ResMut<PhysicsTweaks>,
    mut panel_query: Query<&mut Visibility, With<TweakPanel>>,
) {
    // F1 toggles panel visibility
    if keyboard.just_pressed(KeyCode::F1) {
        tweaks.panel_visible = !tweaks.panel_visible;
        if let Ok(mut visibility) = panel_query.single_mut() {
            *visibility = if tweaks.panel_visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }

    // Only process input when panel is visible
    if !tweaks.panel_visible {
        return;
    }

    let num_params = PhysicsTweaks::LABELS.len();

    // Up/Down to select parameter
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        tweaks.selected_index = (tweaks.selected_index + num_params - 1) % num_params;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        tweaks.selected_index = (tweaks.selected_index + 1) % num_params;
    }

    // Left/Right to adjust value (10% increments)
    let idx = tweaks.selected_index;
    let step = tweaks.get_step(idx);
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        let current = tweaks.get_value(idx);
        tweaks.set_value(idx, (current - step).max(0.01));
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        let current = tweaks.get_value(idx);
        tweaks.set_value(idx, current + step);
    }

    // R to reset selected parameter to default
    if keyboard.just_pressed(KeyCode::KeyR) {
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            // Shift+R resets ALL parameters
            tweaks.reset_all();
        } else {
            // R resets just the selected parameter
            tweaks.reset_value(idx);
        }
    }
}

fn update_tweak_panel(
    tweaks: Res<PhysicsTweaks>,
    mut row_query: Query<(&mut Text, &mut TextColor, &TweakRow)>,
) {
    if !tweaks.panel_visible {
        return;
    }

    for (mut text, mut color, row) in &mut row_query {
        let value = tweaks.get_value(row.0);
        let label = PhysicsTweaks::LABELS[row.0];
        let is_modified = tweaks.is_modified(row.0);

        // Format based on value type (friction shows 2 decimals, others show 0-1)
        let value_str = match row.0 {
            5 | 6 | 7 => format!("{:.2}", value), // Bounce/friction
            10 => format!("{:.1}s", value),        // Charge time
            _ => format!("{:.0}", value),          // Velocities
        };

        text.0 = format!("{}: {}", label, value_str);

        // Color priority: selected (yellow) > modified (red) > default (white)
        if row.0 == tweaks.selected_index {
            color.0 = Color::srgb(1.0, 1.0, 0.0); // Yellow for selected
        } else if is_modified {
            color.0 = Color::srgb(1.0, 0.4, 0.4); // Red for modified
        } else {
            color.0 = Color::WHITE;
        }
    }
}
