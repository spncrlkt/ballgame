//! Ball-related components

use bevy::prelude::*;

/// Marker for ball entities
#[derive(Component)]
pub struct Ball;

/// Number of color palettes available
pub const NUM_PALETTES: usize = 10;

/// Visual style of a ball (each has textures for all palettes)
#[derive(Component, Clone, Copy, Default, Debug, PartialEq, Eq, Hash)]
pub enum BallStyleType {
    #[default]
    Stripe,
    Wedges,
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

/// Textures for a single ball style (one per palette)
#[derive(Clone)]
pub struct StyleTextures {
    pub textures: [Handle<Image>; NUM_PALETTES],
}

/// Holds handles to all ball textures for all styles and palettes
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

/// Current color palette index (0-9)
#[derive(Resource, Default)]
pub struct CurrentPalette(pub usize);

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
