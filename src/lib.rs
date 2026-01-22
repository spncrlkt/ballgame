//! Shared constants and utilities for ballgame
//!
//! This module is used by both the main game and tools like the heatmap generator.

// =============================================================================
// ARENA DIMENSIONS
// =============================================================================

pub const ARENA_WIDTH: f32 = 1600.0;
pub const ARENA_HEIGHT: f32 = 900.0;
pub const ARENA_FLOOR_Y: f32 = -ARENA_HEIGHT / 2.0 + 20.0;

// =============================================================================
// BASKETS
// =============================================================================

pub const BASKET_SIZE_X: f32 = 60.0;
pub const BASKET_SIZE_Y: f32 = 80.0;
pub const LEFT_BASKET_X: f32 = -ARENA_WIDTH / 2.0 + 140.0;
pub const RIGHT_BASKET_X: f32 = ARENA_WIDTH / 2.0 - 140.0;
pub const RIM_THICKNESS: f32 = 10.0;

// =============================================================================
// BALL PHYSICS
// =============================================================================

pub const BALL_SIZE: f32 = 24.0;
pub const BALL_GRAVITY: f32 = 800.0;
pub const BALL_BOUNCE: f32 = 0.7;
pub const BALL_AIR_FRICTION: f32 = 0.95;
pub const BALL_GROUND_FRICTION: f32 = 0.6;

// =============================================================================
// SHOOTING
// =============================================================================

pub const SHOT_MAX_POWER: f32 = 900.0;
pub const SHOT_MAX_SPEED: f32 = 800.0;
pub const SHOT_CHARGE_TIME: f32 = 1.6;
pub const SHOT_MAX_VARIANCE: f32 = 0.50;
pub const SHOT_MIN_VARIANCE: f32 = 0.02;
pub const SHOT_AIR_VARIANCE_PENALTY: f32 = 0.10;
pub const SHOT_MOVE_VARIANCE_PENALTY: f32 = 0.10;
pub const SHOT_DISTANCE_VARIANCE: f32 = 0.00025;
pub const SHOT_QUICK_THRESHOLD: f32 = 0.4;
pub const SHOT_DEFAULT_ANGLE: f32 = 60.0;
pub const SHOT_GRACE_PERIOD: f32 = 0.1;

// =============================================================================
// CORNER STEPS
// =============================================================================

pub const CORNER_STEP_TOTAL_HEIGHT: f32 = 320.0;
pub const CORNER_STEP_TOTAL_WIDTH: f32 = 170.0;
pub const CORNER_STEP_COUNT: usize = 12;
pub const CORNER_STEP_THICKNESS: f32 = 20.0;

// =============================================================================
// TRAJECTORY CALCULATION
// =============================================================================

/// Shot trajectory result containing angle, required speed, and distance variance
#[derive(Debug, Clone, Copy)]
pub struct ShotTrajectory {
    /// Absolute angle in radians (0=right, π/2=up, π=left)
    pub angle: f32,
    /// Exact speed needed to hit target at this angle
    pub required_speed: f32,
    /// Variance penalty from distance
    pub distance_variance: f32,
}

/// Calculate shot trajectory to hit target.
/// Returns the angle and exact speed needed to hit the target.
/// Uses a fixed elevation angle (60°) and calculates the required speed.
pub fn calculate_shot_trajectory(
    shooter_x: f32,
    shooter_y: f32,
    target_x: f32,
    target_y: f32,
    gravity: f32,
) -> Option<ShotTrajectory> {
    let tx = target_x - shooter_x; // Positive = target is right, negative = left
    let ty = target_y - shooter_y; // Positive = target is above, negative = below
    let dx = tx.abs(); // Horizontal distance (always positive)
    let distance = (tx * tx + ty * ty).sqrt();

    // Variance penalty based on distance (longer shots are less accurate)
    let distance_variance = distance * SHOT_DISTANCE_VARIANCE;

    // Directly under/over target
    if dx < 1.0 {
        let required_speed = if ty > 0.0 {
            // Need enough speed to reach height ty against gravity
            // v² = 2*g*h → v = sqrt(2*g*h)
            (2.0 * gravity * ty).sqrt()
        } else {
            SHOT_MAX_SPEED * 0.3 // Minimal speed for dropping down
        };
        return Some(ShotTrajectory {
            angle: if ty > 0.0 {
                std::f32::consts::FRAC_PI_2
            } else {
                -std::f32::consts::FRAC_PI_2
            },
            required_speed,
            distance_variance,
        });
    }

    // Choose a preferred elevation angle (60° gives nice arc)
    let elevation = 60.0_f32.to_radians();

    // Calculate required speed: v² = g*dx² / (2*cos²(θ)*(dx*tan(θ) - dy))
    let cos_e = elevation.cos();
    let tan_e = elevation.tan();
    let denominator = 2.0 * cos_e * cos_e * (dx * tan_e - ty);

    // If denominator <= 0, angle is too low to reach target height
    // Try a higher angle
    let (final_elevation, required_speed) = if denominator <= 0.0 {
        // Need steeper angle - try 75°
        let steep = 75.0_f32.to_radians();
        let cos_s = steep.cos();
        let tan_s = steep.tan();
        let denom2 = 2.0 * cos_s * cos_s * (dx * tan_s - ty);
        if denom2 <= 0.0 {
            // Even 75° can't reach - use near-vertical
            let very_steep = 85.0_f32.to_radians();
            let cos_vs = very_steep.cos();
            let tan_vs = very_steep.tan();
            let denom3 = 2.0 * cos_vs * cos_vs * (dx * tan_vs - ty);
            (very_steep, (gravity * dx * dx / denom3).sqrt())
        } else {
            (steep, (gravity * dx * dx / denom2).sqrt())
        }
    } else {
        (elevation, (gravity * dx * dx / denominator).sqrt())
    };

    // Convert elevation to absolute angle based on target direction
    let angle = if tx >= 0.0 {
        final_elevation
    } else {
        std::f32::consts::PI - final_elevation
    };

    Some(ShotTrajectory {
        angle,
        required_speed,
        distance_variance,
    })
}
