//! Ball-related components

use bevy::prelude::*;

/// Marker for ball entities
#[derive(Component)]
pub struct Ball;

/// Visual style of a ball (each has 3 possession textures)
#[derive(Component, Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub enum BallStyleType {
    Stripe,
    Wedges,
    #[default]
    Dot,
    Half,
    Ring,
    Solid,
}

impl BallStyleType {
    /// All ball styles in order (matches debug level left-to-right)
    pub const ALL: [BallStyleType; 6] = [
        BallStyleType::Stripe,
        BallStyleType::Wedges,
        BallStyleType::Dot,
        BallStyleType::Half,
        BallStyleType::Ring,
        BallStyleType::Solid,
    ];

    /// Name for display
    pub fn name(&self) -> &'static str {
        match self {
            BallStyleType::Stripe => "stripe",
            BallStyleType::Wedges => "wedges",
            BallStyleType::Dot => "dot",
            BallStyleType::Half => "half",
            BallStyleType::Ring => "ring",
            BallStyleType::Solid => "solid",
        }
    }
}

/// Textures for a single ball style (neutral, left, right)
#[derive(Clone)]
pub struct StyleTextures {
    pub neutral: Handle<Image>,
    pub left: Handle<Image>,
    pub right: Handle<Image>,
}

/// Holds handles to all ball textures for dynamic swapping based on possession
#[derive(Resource, Clone)]
pub struct BallTextures {
    pub stripe: StyleTextures,
    pub wedges: StyleTextures,
    pub dot: StyleTextures,
    pub half: StyleTextures,
    pub ring: StyleTextures,
    pub solid: StyleTextures,
}

impl BallTextures {
    /// Get textures for a specific style
    pub fn get(&self, style: BallStyleType) -> &StyleTextures {
        match style {
            BallStyleType::Stripe => &self.stripe,
            BallStyleType::Wedges => &self.wedges,
            BallStyleType::Dot => &self.dot,
            BallStyleType::Half => &self.half,
            BallStyleType::Ring => &self.ring,
            BallStyleType::Solid => &self.solid,
        }
    }
}

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

/// Tracks ball's angular velocity (radians per second)
#[derive(Component, Default)]
pub struct BallSpin(pub f32);
