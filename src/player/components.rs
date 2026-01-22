//! Player-related components

use bevy::prelude::*;

/// Marker for player entities
#[derive(Component)]
pub struct Player;

/// 2D velocity vector - shared by player and ball
#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

/// Whether player is on ground
#[derive(Component)]
pub struct Grounded(pub bool);

/// Time remaining for coyote jump (seconds after leaving ground you can still jump)
#[derive(Component, Default)]
pub struct CoyoteTimer(pub f32);

/// Tracks if currently in a jump (for variable height)
#[derive(Component, Default)]
pub struct JumpState {
    pub is_jumping: bool,
}

/// Direction player faces (-1.0 = left, 1.0 = right)
/// Used for ball/gauge position only
#[derive(Component)]
pub struct Facing(pub f32);

impl Default for Facing {
    fn default() -> Self {
        Self(1.0) // Default facing right
    }
}

/// Reference to held ball entity
#[derive(Component)]
pub struct HoldingBall(pub Entity);
