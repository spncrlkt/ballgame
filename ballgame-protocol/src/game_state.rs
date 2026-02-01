//! Game state types for network snapshots
//!
//! These types are Bevy-free versions of the game's core types,
//! suitable for serialization over the network.

use serde::{Deserialize, Serialize};

/// 2D vector (Bevy-free)
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance(&self, other: Vec2) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            Self::ZERO
        }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

/// Team identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Team {
    Left,
    Right,
}

impl Team {
    /// Get the opposing team
    pub fn opponent(&self) -> Team {
        match self {
            Team::Left => Team::Right,
            Team::Right => Team::Left,
        }
    }
}

/// Character identifier for 2v2 (Team + Slot)
/// L0/L1 = Left team, R0/R1 = Right team
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CharacterId {
    L0,
    L1,
    R0,
    R1,
}

impl CharacterId {
    /// Get the team this character belongs to
    pub fn team(&self) -> Team {
        match self {
            CharacterId::L0 | CharacterId::L1 => Team::Left,
            CharacterId::R0 | CharacterId::R1 => Team::Right,
        }
    }

    /// Get the slot number within the team (0 or 1)
    pub fn slot(&self) -> u8 {
        match self {
            CharacterId::L0 | CharacterId::R0 => 0,
            CharacterId::L1 | CharacterId::R1 => 1,
        }
    }

    /// Get all character IDs
    pub fn all() -> [CharacterId; 4] {
        [
            CharacterId::L0,
            CharacterId::L1,
            CharacterId::R0,
            CharacterId::R1,
        ]
    }

    /// Get the teammate of this character
    pub fn teammate(&self) -> CharacterId {
        match self {
            CharacterId::L0 => CharacterId::L1,
            CharacterId::L1 => CharacterId::L0,
            CharacterId::R0 => CharacterId::R1,
            CharacterId::R1 => CharacterId::R0,
        }
    }

    /// Get the opponents of this character
    pub fn opponents(&self) -> [CharacterId; 2] {
        match self.team() {
            Team::Left => [CharacterId::R0, CharacterId::R1],
            Team::Right => [CharacterId::L0, CharacterId::L1],
        }
    }

    /// Parse from string (e.g., "L0", "R1")
    pub fn from_str(s: &str) -> Option<CharacterId> {
        match s {
            "L0" => Some(CharacterId::L0),
            "L1" => Some(CharacterId::L1),
            "R0" => Some(CharacterId::R0),
            "R1" => Some(CharacterId::R1),
            _ => None,
        }
    }

    /// Convert to slot index (0-3)
    pub fn to_slot_index(&self) -> u8 {
        match self {
            CharacterId::L0 => 0,
            CharacterId::L1 => 1,
            CharacterId::R0 => 2,
            CharacterId::R1 => 3,
        }
    }

    /// Create from slot index (0-3)
    pub fn from_slot_index(index: u8) -> Option<CharacterId> {
        match index {
            0 => Some(CharacterId::L0),
            1 => Some(CharacterId::L1),
            2 => Some(CharacterId::R0),
            3 => Some(CharacterId::R1),
            _ => None,
        }
    }
}

impl std::fmt::Display for CharacterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CharacterId::L0 => write!(f, "L0"),
            CharacterId::L1 => write!(f, "L1"),
            CharacterId::R0 => write!(f, "R0"),
            CharacterId::R1 => write!(f, "R1"),
        }
    }
}

/// Which basket (for targeting)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Basket {
    Left,
    Right,
}

/// Game score
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Score {
    pub left: u32,
    pub right: u32,
}

impl Score {
    pub fn new(left: u32, right: u32) -> Self {
        Self { left, right }
    }
}

/// Ball state for network
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BallStateKind {
    /// Ball is free on the ground or in air
    Free,
    /// Ball is held by a character
    Held { holder: CharacterId },
    /// Ball is in flight after a shot
    InFlight { shooter: CharacterId, power: f32 },
    /// Ball is in flight as a pass
    PassInFlight {
        passer: CharacterId,
        target: CharacterId,
    },
}

impl Default for BallStateKind {
    fn default() -> Self {
        BallStateKind::Free
    }
}

/// Ball snapshot for network transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BallSnapshot {
    pub position: Vec2,
    pub velocity: Vec2,
    pub state: BallStateKind,
    pub style: String,
}

impl Default for BallSnapshot {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            state: BallStateKind::Free,
            style: "wedges".to_string(),
        }
    }
}

/// AI state view for debugging/decision-making
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiStateView {
    /// Current goal name for debugging
    pub current_goal: String,
    /// Time AI has been holding the ball
    pub ball_hold_time: f32,
    /// Steal reaction timer
    pub steal_reaction_timer: f32,
    /// Profile ID
    pub profile_id: String,
}

/// Agent (player) snapshot for network transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    /// Character ID (L0, L1, R0, R1)
    pub character: CharacterId,
    /// Team
    pub team: Team,
    /// Position
    pub position: Vec2,
    /// Velocity
    pub velocity: Vec2,
    /// Whether grounded
    pub grounded: bool,
    /// Whether holding the ball
    pub holding_ball: bool,
    /// Whether charging a shot
    pub charging_shot: bool,
    /// Charge progress (0.0-1.0)
    pub charge_progress: f32,
    /// Direction facing (-1.0 = left, 1.0 = right)
    pub facing: f32,
    /// Target basket
    pub target_basket: Basket,
    /// Turbo gauge (0.0-1.0)
    pub turbo_gauge: f32,
    /// Block active
    pub block_active: bool,
    /// AI state (if AI-controlled)
    pub ai_state: Option<AiStateView>,
}

impl Default for AgentSnapshot {
    fn default() -> Self {
        Self {
            character: CharacterId::L0,
            team: Team::Left,
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
            grounded: true,
            holding_ball: false,
            charging_shot: false,
            charge_progress: 0.0,
            facing: 1.0,
            target_basket: Basket::Right,
            turbo_gauge: 1.0,
            block_active: false,
            ai_state: None,
        }
    }
}

/// Complete game state for client rendering/decision-making
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateSnapshot {
    /// Current game tick
    pub tick: u64,
    /// Game time in seconds
    pub time: f32,
    /// All agents (players)
    pub agents: Vec<AgentSnapshot>,
    /// Ball state
    pub ball: BallSnapshot,
    /// Current score
    pub score: Score,
    /// Current level ID
    pub level_id: String,
    /// Countdown remaining (0 if match active)
    pub countdown: f32,
}

impl Default for GameStateSnapshot {
    fn default() -> Self {
        Self {
            tick: 0,
            time: 0.0,
            agents: Vec::new(),
            ball: BallSnapshot::default(),
            score: Score::default(),
            level_id: String::new(),
            countdown: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec2_operations() {
        let a = Vec2::new(3.0, 4.0);
        let b = Vec2::new(1.0, 2.0);

        assert_eq!(a + b, Vec2::new(4.0, 6.0));
        assert_eq!(a - b, Vec2::new(2.0, 2.0));
        assert_eq!(a * 2.0, Vec2::new(6.0, 8.0));
        assert!((a.length() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_character_id() {
        assert_eq!(CharacterId::L0.team(), Team::Left);
        assert_eq!(CharacterId::R1.team(), Team::Right);
        assert_eq!(CharacterId::L0.teammate(), CharacterId::L1);
        assert_eq!(CharacterId::from_str("L0"), Some(CharacterId::L0));
        assert_eq!(CharacterId::from_slot_index(2), Some(CharacterId::R0));
    }

    #[test]
    fn test_serialization() {
        let snapshot = GameStateSnapshot::default();
        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: GameStateSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.tick, snapshot.tick);
    }
}
