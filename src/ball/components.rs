//! Ball-related components

use bevy::prelude::*;
use std::collections::HashMap;

/// Marker for ball entities
#[derive(Component)]
pub struct Ball;

/// Ball style name - stored as a string to be fully dynamic
#[derive(Component, Clone, Default, Debug, PartialEq, Eq, Hash)]
pub struct BallStyle(pub String);

impl BallStyle {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}

/// Textures for a single ball style (one per palette)
#[derive(Clone, Default)]
pub struct StyleTextures {
    pub textures: Vec<Handle<Image>>,
}

/// Holds handles to all ball textures for all styles and palettes
/// Dynamically loaded based on ball_options.txt
#[derive(Resource, Clone, Default)]
pub struct BallTextures {
    /// Map from style name to its textures
    pub styles: HashMap<String, StyleTextures>,
    /// Ordered list of style names (for cycling through styles)
    pub style_order: Vec<String>,
}

impl BallTextures {
    /// Get textures for a specific style by name
    pub fn get(&self, style: &str) -> Option<&StyleTextures> {
        self.styles.get(style)
    }

    /// Get the first style name (default)
    pub fn default_style(&self) -> Option<&String> {
        self.style_order.first()
    }

    /// Get style at index (for cycling)
    pub fn style_at(&self, index: usize) -> Option<&String> {
        self.style_order.get(index)
    }

    /// Number of available styles
    pub fn len(&self) -> usize {
        self.style_order.len()
    }

    /// Check if there are no styles
    pub fn is_empty(&self) -> bool {
        self.style_order.is_empty()
    }

    /// Get index of a style by name
    pub fn index_of(&self, style: &str) -> Option<usize> {
        self.style_order.iter().position(|s| s == style)
    }

    /// Get next style (wrapping)
    pub fn next_style(&self, current: &str) -> &str {
        let idx = self.index_of(current).unwrap_or(0);
        let next_idx = (idx + 1) % self.style_order.len();
        &self.style_order[next_idx]
    }

    /// Get previous style (wrapping)
    pub fn prev_style(&self, current: &str) -> &str {
        let idx = self.index_of(current).unwrap_or(0);
        let prev_idx = if idx == 0 {
            self.style_order.len() - 1
        } else {
            idx - 1
        };
        &self.style_order[prev_idx]
    }
}

/// Current color palette index (0 to palette_count-1)
/// Default is the first palette (Aurora)
#[derive(Resource)]
pub struct CurrentPalette(pub usize);

impl Default for CurrentPalette {
    fn default() -> Self {
        Self(0) // First palette (Aurora)
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
