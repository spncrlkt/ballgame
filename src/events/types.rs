//! Event type definitions for the logging system

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

/// Team identifier (Left or Right)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeamId {
    Left,
    Right,
}

impl std::fmt::Display for TeamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeamId::Left => write!(f, "left"),
            TeamId::Right => write!(f, "right"),
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
    pub fn team(&self) -> TeamId {
        match self {
            CharacterId::L0 | CharacterId::L1 => TeamId::Left,
            CharacterId::R0 | CharacterId::R1 => TeamId::Right,
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
        [CharacterId::L0, CharacterId::L1, CharacterId::R0, CharacterId::R1]
    }

    /// Get the two characters on a team
    pub fn team_members(team: TeamId) -> [CharacterId; 2] {
        match team {
            TeamId::Left => [CharacterId::L0, CharacterId::L1],
            TeamId::Right => [CharacterId::R0, CharacterId::R1],
        }
    }

    /// Get the opponents of this character
    pub fn opponents(&self) -> [CharacterId; 2] {
        match self.team() {
            TeamId::Left => [CharacterId::R0, CharacterId::R1],
            TeamId::Right => [CharacterId::L0, CharacterId::L1],
        }
    }

    /// Get the teammate of this character (the other slot on same team)
    pub fn teammate(&self) -> CharacterId {
        match self {
            CharacterId::L0 => CharacterId::L1,
            CharacterId::L1 => CharacterId::L0,
            CharacterId::R0 => CharacterId::R1,
            CharacterId::R1 => CharacterId::R0,
        }
    }

    /// Parse from string (e.g., "L0", "R1")
    pub fn from_str(s: &str) -> Option<CharacterId> {
        match s {
            "L0" => Some(CharacterId::L0),
            "L1" => Some(CharacterId::L1),
            "R0" => Some(CharacterId::R0),
            "R1" => Some(CharacterId::R1),
            // Backward compatibility with old PlayerId format
            "L" => Some(CharacterId::L0),
            "R" => Some(CharacterId::R0),
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

/// Player identifier (Left or Right) - DEPRECATED, use CharacterId
/// Kept for backward compatibility during transition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerId {
    L,
    R,
}

impl PlayerId {
    /// Convert to CharacterId (maps to slot 0 of each team)
    pub fn to_character_id(&self) -> CharacterId {
        match self {
            PlayerId::L => CharacterId::L0,
            PlayerId::R => CharacterId::R0,
        }
    }
}

impl From<PlayerId> for CharacterId {
    fn from(player: PlayerId) -> Self {
        player.to_character_id()
    }
}

/// Source of controller input (for auditability)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ControllerSource {
    /// Human player input (keyboard/gamepad)
    #[default]
    Human,
    /// AI-controlled player
    Ai,
    /// External source (replay, tests, network)
    External,
}

impl std::fmt::Display for ControllerSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControllerSource::Human => write!(f, "H"),
            ControllerSource::Ai => write!(f, "A"),
            ControllerSource::External => write!(f, "X"),
        }
    }
}

impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayerId::L => write!(f, "L"),
            PlayerId::R => write!(f, "R"),
        }
    }
}

/// Game configuration snapshot for analytics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameConfig {
    // Physics
    pub gravity_rise: f32,
    pub gravity_fall: f32,
    pub jump_velocity: f32,
    pub move_speed: f32,
    pub ground_accel: f32,
    pub air_accel: f32,
    // Ball physics
    pub ball_gravity: f32,
    pub ball_bounce: f32,
    pub ball_air_friction: f32,
    pub ball_ground_friction: f32,
    // Shooting
    pub shot_max_power: f32,
    pub shot_max_speed: f32,
    pub shot_charge_time: f32,
    pub shot_max_variance: f32,
    pub shot_min_variance: f32,
    // Steal
    pub steal_range: f32,
    pub steal_success_chance: f32,
    pub steal_cooldown: f32,
    // Active presets (if using preset system)
    pub preset_movement: Option<String>,
    pub preset_ball: Option<String>,
    pub preset_shooting: Option<String>,
    pub preset_composite: Option<String>,
}

/// Data for a single character in a tick frame (2v2 format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterTickData {
    pub id: CharacterId,
    pub pos: (f32, f32),
    pub vel: (f32, f32),
    pub controller: u32, // InputSourceId
}

/// All game events that can be logged
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    // === Session Events ===
    /// Session started (generated once per game launch)
    SessionStart {
        session_id: String, // UUID v4
        timestamp: String,  // ISO 8601
    },
    /// Game configuration snapshot (logged after session start)
    Config(GameConfig),

    // === Match Events ===
    /// Match started
    MatchStart {
        level: u32,
        level_name: String,
        left_profile: String,
        right_profile: String,
        seed: u64,
    },
    /// Match ended
    MatchEnd {
        score_left: u32,
        score_right: u32,
        duration: f32,
    },

    // === Scoring Events ===
    /// Goal scored (legacy format with PlayerId)
    Goal {
        player: PlayerId,
        score_left: u32,
        score_right: u32,
    },
    /// Goal scored (2v2 format with CharacterId)
    Goal2 {
        character: CharacterId,
        score_left: u32,
        score_right: u32,
    },

    // === Ball Events ===
    /// Ball picked up
    Pickup { player: PlayerId },
    /// Ball picked up (2v2)
    Pickup2 { character: CharacterId },
    /// Ball dropped/lost without shot
    Drop { player: PlayerId },
    /// Ball dropped (2v2)
    Drop2 { character: CharacterId },
    /// Shot started (charge began)
    ShotStart {
        player: PlayerId,
        pos: (f32, f32),
        quality: f32,
    },
    /// Shot started (2v2)
    ShotStart2 {
        character: CharacterId,
        pos: (f32, f32),
        quality: f32,
    },
    /// Shot released
    ShotRelease {
        player: PlayerId,
        charge: f32,
        angle: f32,
        power: f32,
    },
    /// Shot released (2v2)
    ShotRelease2 {
        character: CharacterId,
        charge: f32,
        angle: f32,
        power: f32,
    },
    /// Pass initiated (new for 2v2)
    Pass {
        from: CharacterId,
        to: CharacterId,
    },

    // === Steal Events ===
    /// Steal attempted
    StealAttempt { attacker: PlayerId },
    /// Steal attempted (2v2)
    StealAttempt2 { attacker: CharacterId },
    /// Steal succeeded
    StealSuccess { attacker: PlayerId },
    /// Steal succeeded (2v2)
    StealSuccess2 { attacker: CharacterId },
    /// Steal failed
    StealFail { attacker: PlayerId },
    /// Steal failed (2v2)
    StealFail2 { attacker: CharacterId },
    /// Steal attempted but out of range
    StealOutOfRange { attacker: PlayerId },
    /// Steal out of range (2v2)
    StealOutOfRange2 { attacker: CharacterId },

    // === Movement Events ===
    /// Player jumped
    Jump { player: PlayerId },
    /// Player jumped (2v2)
    Jump2 { character: CharacterId },
    /// Player landed
    Land { player: PlayerId },
    /// Player landed (2v2)
    Land2 { character: CharacterId },

    // === AI State Events ===
    /// AI goal changed
    AiGoal { player: PlayerId, goal: String },
    /// AI goal changed (2v2)
    AiGoal2 { character: CharacterId, goal: String },
    /// AI navigation path started
    NavStart {
        player: PlayerId,
        target: (f32, f32),
    },
    /// AI navigation started (2v2)
    NavStart2 {
        character: CharacterId,
        target: (f32, f32),
    },
    /// AI navigation completed
    NavComplete { player: PlayerId },
    /// AI navigation completed (2v2)
    NavComplete2 { character: CharacterId },

    // === Input Events (for replay/analysis) ===
    /// Input state snapshot (periodic, every N frames)
    Input {
        player: PlayerId,
        move_x: f32,
        jump: bool,
        throw: bool,
        pickup: bool,
    },
    /// Input state snapshot (2v2)
    Input2 {
        character: CharacterId,
        source: u32, // InputSourceId
        move_x: f32,
        jump: bool,
        throw: bool,
        pickup: bool,
        pass: bool,
    },

    // === Debug/Tick Events ===
    /// Frame tick with positions and velocities (sampled at 50ms / 20 Hz)
    /// Legacy 1v1 format
    Tick {
        frame: u64,
        left_pos: (f32, f32),
        left_vel: (f32, f32),
        right_pos: (f32, f32),
        right_vel: (f32, f32),
        ball_pos: (f32, f32),
        ball_vel: (f32, f32),
        ball_state: char, // F=Free, H=Held, I=InFlight
    },
    /// Frame tick (2v2 format) - variable number of characters
    Tick2 {
        frame: u64,
        characters: Vec<CharacterTickData>,
        ball_pos: (f32, f32),
        ball_vel: (f32, f32),
        ball_state: char,
    },

    // === Controller Events (event bus) ===
    /// Controller input from any source (human, AI, external)
    ControllerInput {
        player: PlayerId,
        source: ControllerSource,
        move_x: f32,
        jump: bool,
        jump_pressed: bool,
        throw: bool,
        throw_released: bool,
        pickup: bool,
    },
    /// Controller input (2v2 format)
    ControllerInput2 {
        character: CharacterId,
        source_id: u32, // InputSourceId
        move_x: f32,
        jump: bool,
        jump_pressed: bool,
        throw: bool,
        throw_released: bool,
        pickup: bool,
        pass: bool,
    },
    /// Control transferred between players
    ControlSwap {
        from_player: Option<PlayerId>,
        to_player: Option<PlayerId>,
    },
    /// Controller assigned to a character (2v2)
    ControllerAssign {
        character: CharacterId,
        source_id: u32,
        descriptor: String, // "keyboard", "gamepad:Xbox", "ai:Aggressive"
    },
    /// Controller swapped between characters (2v2)
    ControllerSwap2 {
        character: CharacterId,
        old_source: u32,
        new_source: u32,
    },

    // === State Reset Events (event bus) ===
    /// Reset AI state for a player
    ResetAiState { player: PlayerId },
    /// Reset AI state (2v2)
    ResetAiState2 { character: CharacterId },
    /// Reset scores to 0-0
    ResetScores,
    /// Reset ball to spawn position
    ResetBall,
    /// Level changed
    LevelChange { level_id: String },
}

impl GameEvent {
    /// Get the event type code for compact serialization
    pub fn type_code(&self) -> &'static str {
        match self {
            GameEvent::SessionStart { .. } => "SE",
            GameEvent::Config(_) => "CF",
            GameEvent::MatchStart { .. } => "MS",
            GameEvent::MatchEnd { .. } => "ME",
            GameEvent::Goal { .. } => "G",
            GameEvent::Goal2 { .. } => "G2",
            GameEvent::Pickup { .. } => "PU",
            GameEvent::Pickup2 { .. } => "P2",
            GameEvent::Drop { .. } => "DR",
            GameEvent::Drop2 { .. } => "D2",
            GameEvent::ShotStart { .. } => "SS",
            GameEvent::ShotStart2 { .. } => "S2",
            GameEvent::ShotRelease { .. } => "SR",
            GameEvent::ShotRelease2 { .. } => "R2",
            GameEvent::Pass { .. } => "PA",
            GameEvent::StealAttempt { .. } => "SA",
            GameEvent::StealAttempt2 { .. } => "A2",
            GameEvent::StealSuccess { .. } => "S+",
            GameEvent::StealSuccess2 { .. } => "+2",
            GameEvent::StealFail { .. } => "S-",
            GameEvent::StealFail2 { .. } => "-2",
            GameEvent::StealOutOfRange { .. } => "SO",
            GameEvent::StealOutOfRange2 { .. } => "O2",
            GameEvent::Jump { .. } => "J",
            GameEvent::Jump2 { .. } => "J2",
            GameEvent::Land { .. } => "LD",
            GameEvent::Land2 { .. } => "L2",
            GameEvent::AiGoal { .. } => "AG",
            GameEvent::AiGoal2 { .. } => "AI",
            GameEvent::NavStart { .. } => "NS",
            GameEvent::NavStart2 { .. } => "N2",
            GameEvent::NavComplete { .. } => "NC",
            GameEvent::NavComplete2 { .. } => "C2",
            GameEvent::Input { .. } => "I",
            GameEvent::Input2 { .. } => "I2",
            GameEvent::Tick { .. } => "T",
            GameEvent::Tick2 { .. } => "T2",
            GameEvent::ControllerInput { .. } => "CI",
            GameEvent::ControllerInput2 { .. } => "X2",
            GameEvent::ControlSwap { .. } => "CS",
            GameEvent::ControllerAssign { .. } => "CA",
            GameEvent::ControllerSwap2 { .. } => "W2",
            GameEvent::ResetAiState { .. } => "RA",
            GameEvent::ResetAiState2 { .. } => "Z2",
            GameEvent::ResetScores => "RS",
            GameEvent::ResetBall => "RB",
            GameEvent::LevelChange { .. } => "LC",
        }
    }
}
