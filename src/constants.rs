//! Tunable constants for ballgame
//!
//! All gameplay values are defined here for easy tweaking.

use bevy::prelude::*;

// =============================================================================
// DEFAULT ARENA COLORS (used at startup before palette loads)
// =============================================================================

pub const DEFAULT_BACKGROUND_COLOR: Color = Color::srgb(0.35, 0.32, 0.28);
pub const DEFAULT_FLOOR_COLOR: Color = Color::srgb(0.15, 0.13, 0.12);
pub const DEFAULT_PLATFORM_COLOR: Color = Color::srgb(0.2, 0.18, 0.16);

// =============================================================================
// TEXT/UI COLORS
// =============================================================================

pub const TEXT_PRIMARY: Color = Color::srgb(0.95, 0.9, 0.8); // Bone white/cream
pub const TEXT_SECONDARY: Color = Color::srgb(0.7, 0.65, 0.55); // Aged parchment
pub const TEXT_ACCENT: Color = Color::srgb(0.9, 0.75, 0.4); // Gold/amber

// =============================================================================
// SIZE CONSTANTS
// =============================================================================

pub const PLAYER_SIZE: Vec2 = Vec2::new(32.0, 64.0);
pub const BALL_SIZE: Vec2 = Vec2::new(26.0, 26.0); // 10% larger than original 24x24
pub const CHARGE_GAUGE_WIDTH: f32 = 8.0;
pub const CHARGE_GAUGE_HEIGHT: f32 = PLAYER_SIZE.y; // Same height as player

// =============================================================================
// PHYSICS CONSTANTS
// =============================================================================

pub const GRAVITY_RISE: f32 = 980.0; // Gravity while rising
pub const GRAVITY_FALL: f32 = 1400.0; // Gravity while falling (fast fall)
pub const JUMP_VELOCITY: f32 = 650.0; // Full jump height (hold button)
pub const JUMP_CUT_MULTIPLIER: f32 = 0.4; // Velocity multiplier when releasing jump early
pub const MOVE_SPEED: f32 = 300.0;
pub const GROUND_ACCEL: f32 = 2400.0; // Ground acceleration (pixels/sec²) - snappy start
pub const GROUND_DECEL: f32 = 1800.0; // Ground deceleration - slight slide when stopping
pub const AIR_ACCEL: f32 = 1500.0; // Air acceleration - committed but adjustable jumps
pub const AIR_DECEL: f32 = 900.0; // Air deceleration - momentum preserved in air
pub const COLLISION_EPSILON: f32 = 0.5; // Skin width for collision detection

// =============================================================================
// GAME FEEL CONSTANTS
// =============================================================================

pub const COYOTE_TIME: f32 = 0.1; // Seconds after leaving ground you can still jump
pub const JUMP_BUFFER_TIME: f32 = 0.1; // Seconds before landing that jump input is remembered
pub const STICK_DEADZONE: f32 = 0.25; // Analog stick deadzone to prevent rebound direction changes

// =============================================================================
// BALL PHYSICS
// =============================================================================

pub const BALL_GRAVITY: f32 = 800.0;
pub const BALL_BOUNCE: f32 = 0.7; // Coefficient of restitution (0 = no bounce, 1 = perfect bounce)
pub const BALL_AIR_FRICTION: f32 = 0.95; // Horizontal velocity retained after 1 second in air (low drag)
pub const BALL_GROUND_FRICTION: f32 = 0.6; // Horizontal velocity retained per bounce
pub const BALL_ROLL_FRICTION: f32 = 0.6; // Horizontal velocity retained after 1 second while rolling
pub const BALL_BOUNCE_HEIGHT_MULT: f32 = 1.0; // Ball must bounce this × its height to keep bouncing, else rolls
pub const BALL_PICKUP_RADIUS: f32 = 50.0; // How close player must be to pick up ball
pub const BALL_FREE_SPEED: f32 = 200.0; // Ball becomes Free when speed drops below this (2x pickup radius speed)

// =============================================================================
// BALL SPIN/ROTATION
// =============================================================================

pub const BALL_SPIN_FACTOR: f32 = 0.01; // Spin rate per unit velocity (airborne)
pub const BALL_SPIN_DECAY: f32 = 0.5; // Spin retained per second (airborne)

// =============================================================================
// SHOOTING
// =============================================================================

// Heights: tap=2x player height (128), full=6x player height (384)
// Using h = v_y²/(2g), v_y = sqrt(2*g*h): tap needs v_y≈452, full needs v_y≈784
pub const SHOT_MAX_POWER: f32 = 900.0; // Maximum horizontal velocity (fallback for extreme shots)
pub const SHOT_MAX_SPEED: f32 = 800.0; // Maximum total ball speed (caps velocity magnitude)
pub const SHOT_HARD_CAP: f32 = 2000.0; // Absolute maximum shot speed (alerts if reached)
pub const SHOT_CHARGE_TIME: f32 = 1.6; // Seconds to reach full charge
pub const SHOT_MAX_VARIANCE: f32 = 0.50; // Variance at zero charge (50%)
pub const SHOT_MIN_VARIANCE: f32 = 0.02; // Variance at full charge (2%)
pub const SHOT_AIR_VARIANCE_PENALTY: f32 = 0.10; // Additional variance when airborne (10%)
pub const SHOT_MOVE_VARIANCE_PENALTY: f32 = 0.10; // Additional variance at full horizontal speed (10%)
pub const SHOT_QUICK_THRESHOLD: f32 = 0.4; // Charge below this (400ms) = half power shot
pub const SHOT_DEFAULT_ANGLE: f32 = 60.0; // Default shot angle in degrees
pub const SHOT_GRACE_PERIOD: f32 = 0.1; // Post-shot grace period (no friction/player drag)

// =============================================================================
// BALL-PLAYER COLLISION
// =============================================================================

pub const BALL_PLAYER_DRAG_X: f32 = 0.7; // Horizontal velocity multiplier when ball hits player
pub const BALL_PLAYER_DRAG_Y: f32 = 0.4; // Vertical velocity multiplier (higher friction than X)
pub const BALL_KICK_STRENGTH: f32 = 100.0; // How much velocity player imparts to stationary ball
pub const BALL_KICK_THRESHOLD: f32 = 30.0; // Ball speed below this counts as "stationary"

// =============================================================================
// STEAL CONTEST
// =============================================================================

pub const STEAL_RANGE: f32 = 60.0; // How close to initiate steal
pub const STEAL_CONTEST_DURATION: f32 = 0.4; // Seconds to mash buttons
pub const STEAL_DEFENDER_ADVANTAGE: u32 = 2; // Defender starts with this many "presses"

// =============================================================================
// ARENA DIMENSIONS
// =============================================================================

pub const ARENA_WIDTH: f32 = 1600.0;
pub const ARENA_HEIGHT: f32 = 900.0;
pub const ARENA_FLOOR_Y: f32 = -ARENA_HEIGHT / 2.0; // Floor at bottom edge

// =============================================================================
// BASKETS
// =============================================================================

pub const BASKET_SIZE: Vec2 = Vec2::new(60.0, 80.0);
pub const RIM_THICKNESS: f32 = 10.0;
pub const WALL_THICKNESS: f32 = 20.0; // Walls are 20 wide
pub const BASKET_PUSH_IN: f32 = 156.0; // Default distance from wall inner edge to basket center

// =============================================================================
// CORNER STEPS
// =============================================================================

pub const CORNER_STEP_TOTAL_HEIGHT: f32 = 320.0;
pub const CORNER_STEP_TOTAL_WIDTH: f32 = 200.0;
pub const CORNER_STEP_COUNT: usize = 13;
pub const CORNER_STEP_THICKNESS: f32 = 20.0;
pub const STEP_PUSH_IN: f32 = 0.0; // Distance from wall to where stairs start (top step extends to wall)
pub const STEP_BOUNCE_RETENTION: f32 = 0.92; // Steps keep more velocity than normal bounce
pub const STEP_DEFLECT_ANGLE_MAX: f32 = 35.0; // Max random deflection angle in degrees

// =============================================================================
// SPAWN POSITIONS
// =============================================================================

pub const PLAYER_SPAWN: Vec3 = Vec3::new(-200.0, ARENA_FLOOR_Y + 100.0, 0.0);
pub const PLAYER_SPAWN_LEFT: Vec3 = Vec3::new(-300.0, ARENA_FLOOR_Y + 100.0, 0.0);
pub const PLAYER_SPAWN_RIGHT: Vec3 = Vec3::new(300.0, ARENA_FLOOR_Y + 100.0, 0.0);
pub const BALL_SPAWN: Vec3 = Vec3::new(0.0, ARENA_FLOOR_Y + 50.0, 2.0); // Center, z=2 to render in front

// =============================================================================
// AI TUNING
// =============================================================================

pub const AI_POSITION_TOLERANCE: f32 = 30.0; // How close AI needs to be to target position
pub const AI_SHOOT_RANGE: f32 = 400.0; // Distance from basket to start shooting
pub const AI_CHARGE_TIME_MIN: f32 = 0.5; // Minimum charge time for shots
pub const AI_CHARGE_TIME_MAX: f32 = 1.2; // Maximum charge time for shots
pub const AI_STEAL_RANGE: f32 = 80.0; // Distance to attempt steal

// =============================================================================
// LEVEL FILE
// =============================================================================

pub const LEVELS_FILE: &str = "assets/levels.txt";
