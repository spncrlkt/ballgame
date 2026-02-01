//! Player-related components

use bevy::prelude::*;

use crate::events::CharacterId;
use crate::input::InputSourceId;
use crate::world::Basket;

/// Buff for a character - each character picks ONE buff
/// that provides a specific gameplay advantage.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Buff {
    /// +15% movement speed (default)
    #[default]
    Speed,
    /// +50% turbo gauge capacity, +25% refill rate
    Turbo,
    /// -30% shot variance
    Accuracy,
    /// +15% steal success chance
    Steal,
    /// +10% jump velocity
    Jump,
    /// +20% steal resistance
    Defense,
    /// -30% cooldowns (steal, block)
    Recovery,
}

impl Buff {
    /// All buff variants in display order
    pub const ALL: [Buff; 7] = [
        Buff::Speed,
        Buff::Turbo,
        Buff::Accuracy,
        Buff::Steal,
        Buff::Jump,
        Buff::Defense,
        Buff::Recovery,
    ];

    /// Get the next buff in the cycle
    pub fn next(&self) -> Buff {
        let idx = Self::ALL.iter().position(|a| a == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    /// Get the previous buff in the cycle
    pub fn prev(&self) -> Buff {
        let idx = Self::ALL.iter().position(|a| a == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }

    /// Get display name for this buff
    pub fn name(&self) -> &'static str {
        match self {
            Buff::Speed => "Speed",
            Buff::Turbo => "Turbo",
            Buff::Accuracy => "Accuracy",
            Buff::Steal => "Steal",
            Buff::Jump => "Jump",
            Buff::Defense => "Defense",
            Buff::Recovery => "Recovery",
        }
    }

    /// Parse buff from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Buff> {
        match s.to_lowercase().as_str() {
            "speed" => Some(Buff::Speed),
            "turbo" => Some(Buff::Turbo),
            "accuracy" => Some(Buff::Accuracy),
            "steal" => Some(Buff::Steal),
            "jump" => Some(Buff::Jump),
            "defense" => Some(Buff::Defense),
            "recovery" => Some(Buff::Recovery),
            _ => None,
        }
    }
}

/// Marker for player entities
#[derive(Component)]
pub struct Player;

/// Identifies which character this entity represents (L0, L1, R0, R1)
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Character(pub CharacterId);

impl Character {
    pub fn id(&self) -> CharacterId {
        self.0
    }
}

/// Identifies which input source controls this character
/// For human players: keyboard (0) or gamepad ID
/// For AI players: AI source ID (1000+)
#[derive(Component, Debug, Clone, Copy)]
pub struct ControlledBy(pub InputSourceId);

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

/// Which basket a player is aiming at (set once based on Team at spawn)
#[derive(Component)]
pub struct TargetBasket(pub Basket);

impl Default for TargetBasket {
    fn default() -> Self {
        Self(Basket::Right) // Default targeting right basket
    }
}

/// Turbo gauge for speed boost mechanic
/// Drains while turbo is held, refills when released
#[derive(Component)]
pub struct TurboGauge {
    /// Current gauge value (0.0 - 1.0)
    pub current: f32,
    /// Maximum gauge value
    pub max: f32,
    /// Drain rate per second while active
    pub drain_rate: f32,
    /// Refill rate per second when inactive
    pub refill_rate: f32,
}

impl Default for TurboGauge {
    fn default() -> Self {
        use crate::constants::*;
        Self {
            current: TURBO_MAX_GAUGE,
            max: TURBO_MAX_GAUGE,
            drain_rate: TURBO_DRAIN_RATE,
            refill_rate: TURBO_REFILL_RATE,
        }
    }
}

impl TurboGauge {
    /// Check if turbo can be used (has gauge remaining)
    pub fn can_use(&self) -> bool {
        self.current > 0.0
    }

    /// Drain the gauge by the given delta time
    /// Returns true if gauge was drained (still had charge)
    pub fn drain(&mut self, dt: f32) -> bool {
        if self.current > 0.0 {
            self.current = (self.current - self.drain_rate * dt).max(0.0);
            true
        } else {
            false
        }
    }

    /// Refill the gauge by the given delta time
    pub fn refill(&mut self, dt: f32) {
        self.current = (self.current + self.refill_rate * dt).min(self.max);
    }
}

/// Block state for defensive interception mechanic
/// When active, creates a larger hitbox that can intercept passes/shots
#[derive(Component, Default)]
pub struct BlockState {
    /// Whether block is currently active
    pub active: bool,
    /// Remaining duration of active block
    pub active_timer: f32,
    /// Remaining cooldown before can block again
    pub cooldown: f32,
}

impl BlockState {
    /// Check if can initiate a new block
    pub fn can_block(&self) -> bool {
        !self.active && self.cooldown <= 0.0
    }

    /// Start a block
    pub fn start_block(&mut self, duration: f32) {
        self.active = true;
        self.active_timer = duration;
    }

    /// Update block timers
    /// Returns true if block just ended this frame
    pub fn update(&mut self, dt: f32, cooldown_duration: f32) -> bool {
        let mut just_ended = false;

        if self.active {
            self.active_timer -= dt;
            if self.active_timer <= 0.0 {
                self.active = false;
                self.active_timer = 0.0;
                self.cooldown = cooldown_duration;
                just_ended = true;
            }
        }

        if self.cooldown > 0.0 {
            self.cooldown = (self.cooldown - dt).max(0.0);
        }

        just_ended
    }
}
