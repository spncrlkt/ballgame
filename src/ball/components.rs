//! Ball-related components

use bevy::prelude::*;

/// Marker for ball entities
#[derive(Component)]
pub struct Ball;

/// Ball state - Free, Held, or InFlight
#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub enum BallState {
    #[default]
    Free,
    Held(Entity), // Entity = player holding it
    InFlight {
        shooter: Entity,
        power: f32,
    }, // Who shot it and how hard
}

/// Track overlap for collision effects
#[derive(Component, Default)]
pub struct BallPlayerContact {
    pub overlapping: bool,
}

/// Animation timer for pickup indicator
#[derive(Component, Default)]
pub struct BallPulse {
    pub timer: f32,
}

/// True when ball is rolling on ground
#[derive(Component, Default)]
pub struct BallRolling(pub bool);

/// Timer for post-shot grace period (no friction/player drag)
#[derive(Component, Default)]
pub struct BallShotGrace(pub f32);
