//! Player-related components

use bevy::prelude::*;

use crate::world::Basket;

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

/// Which team a player belongs to
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    Left,
    Right,
}

/// Marker for the player currently controlled by the human.
/// Only ONE player has this at a time - AI controls the other.
#[derive(Component)]
pub struct HumanControlled;

/// Resource tracking which player (if any) is under human control.
/// None = observer mode (both players AI-controlled).
/// Used by event bus to route controller input events.
#[derive(Resource, Default)]
pub struct HumanControlTarget(pub Option<crate::events::PlayerId>);

/// Which basket a player is aiming at (set once based on Team at spawn)
#[derive(Component)]
pub struct TargetBasket(pub Basket);

impl Default for TargetBasket {
    fn default() -> Self {
        Self(Basket::Right) // Default targeting right basket
    }
}
