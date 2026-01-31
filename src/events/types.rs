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
    /// Goal scored
    Goal {
        character: CharacterId,
        score_left: u32,
        score_right: u32,
    },

    // === Ball Events ===
    /// Ball picked up
    Pickup { character: CharacterId },
    /// Ball dropped/lost without shot
    Drop { character: CharacterId },
    /// Shot started (charge began)
    ShotStart {
        character: CharacterId,
        pos: (f32, f32),
        quality: f32,
    },
    /// Shot released
    ShotRelease {
        character: CharacterId,
        charge: f32,
        angle: f32,
        power: f32,
    },
    /// Pass initiated (basic event)
    Pass {
        from: CharacterId,
        to: CharacterId,
    },
    /// Pass completed successfully
    PassCompleted {
        passer: CharacterId,
        receiver: CharacterId,
    },
    /// Pass intercepted by opponent
    PassIntercepted {
        passer: CharacterId,
        interceptor: CharacterId,
    },
    /// Pass missed (not caught)
    PassMissed {
        passer: CharacterId,
        target: CharacterId,
    },

    // === Turbo Events ===
    /// Turbo activated (speed boost started)
    TurboActivated { character: CharacterId },
    /// Turbo deactivated (speed boost ended)
    TurboDeactivated {
        character: CharacterId,
        remaining_gauge: f32,
    },

    // === Block Events ===
    /// Block activated (defensive stance started)
    BlockActivated { character: CharacterId },
    /// Block deactivated (defensive stance ended)
    BlockDeactivated { character: CharacterId },
    /// Block intercepted a ball (pass or shot)
    BlockIntercepted {
        blocker: CharacterId,
        ball_state: char, // P=Pass, S=Shot
    },

    // === Steal Events ===
    /// Steal attempted
    StealAttempt { attacker: CharacterId },
    /// Steal succeeded
    StealSuccess { attacker: CharacterId },
    /// Steal failed
    StealFail { attacker: CharacterId },
    /// Steal attempted but out of range
    StealOutOfRange { attacker: CharacterId },

    // === Movement Events ===
    /// Player jumped
    Jump { character: CharacterId },
    /// Player landed
    Land { character: CharacterId },

    // === AI State Events ===
    /// AI goal changed
    AiGoal { character: CharacterId, goal: String },
    /// AI navigation path started
    NavStart {
        character: CharacterId,
        target: (f32, f32),
    },
    /// AI navigation completed
    NavComplete { character: CharacterId },

    // === Input Events (for replay/analysis) ===
    /// Input state snapshot (periodic, every N frames)
    Input {
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
    Tick {
        frame: u64,
        characters: Vec<CharacterTickData>,
        ball_pos: (f32, f32),
        ball_vel: (f32, f32),
        ball_state: char, // F=Free, H=Held, I=InFlight
    },

    // === Controller Events (event bus) ===
    /// Controller input from any source (human, AI, external)
    ControllerInput {
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
    /// Controller assigned to a character
    ControllerAssign {
        character: CharacterId,
        source_id: u32,
        descriptor: String, // "keyboard", "gamepad:Xbox", "ai:Aggressive"
    },
    /// Controller swapped between characters
    ControllerSwap {
        character: CharacterId,
        old_source: u32,
        new_source: u32,
    },

    // === State Reset Events (event bus) ===
    /// Reset AI state for a character
    ResetAiState { character: CharacterId },
    /// Reset scores to 0-0
    ResetScores,
    /// Reset ball to spawn position
    ResetBall,
    /// Level changed
    LevelChange { level_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_characterid_from_str_all_variants() {
        assert_eq!(CharacterId::from_str("L0"), Some(CharacterId::L0));
        assert_eq!(CharacterId::from_str("L1"), Some(CharacterId::L1));
        assert_eq!(CharacterId::from_str("R0"), Some(CharacterId::R0));
        assert_eq!(CharacterId::from_str("R1"), Some(CharacterId::R1));
        // Invalid strings
        assert_eq!(CharacterId::from_str("X"), None);
        assert_eq!(CharacterId::from_str(""), None);
        assert_eq!(CharacterId::from_str("L2"), None);
        assert_eq!(CharacterId::from_str("L"), None); // Legacy format no longer supported
        assert_eq!(CharacterId::from_str("R"), None);
    }

    #[test]
    fn test_characterid_team_and_slot() {
        // Team assignments
        assert_eq!(CharacterId::L0.team(), TeamId::Left);
        assert_eq!(CharacterId::L1.team(), TeamId::Left);
        assert_eq!(CharacterId::R0.team(), TeamId::Right);
        assert_eq!(CharacterId::R1.team(), TeamId::Right);
        // Slot assignments
        assert_eq!(CharacterId::L0.slot(), 0);
        assert_eq!(CharacterId::L1.slot(), 1);
        assert_eq!(CharacterId::R0.slot(), 0);
        assert_eq!(CharacterId::R1.slot(), 1);
    }

    #[test]
    fn test_characterid_teammate() {
        assert_eq!(CharacterId::L0.teammate(), CharacterId::L1);
        assert_eq!(CharacterId::L1.teammate(), CharacterId::L0);
        assert_eq!(CharacterId::R0.teammate(), CharacterId::R1);
        assert_eq!(CharacterId::R1.teammate(), CharacterId::R0);
    }

    #[test]
    fn test_characterid_opponents() {
        assert_eq!(CharacterId::L0.opponents(), [CharacterId::R0, CharacterId::R1]);
        assert_eq!(CharacterId::L1.opponents(), [CharacterId::R0, CharacterId::R1]);
        assert_eq!(CharacterId::R0.opponents(), [CharacterId::L0, CharacterId::L1]);
        assert_eq!(CharacterId::R1.opponents(), [CharacterId::L0, CharacterId::L1]);
    }

    #[test]
    fn test_characterid_all() {
        let all = CharacterId::all();
        assert_eq!(all.len(), 4);
        assert!(all.contains(&CharacterId::L0));
        assert!(all.contains(&CharacterId::L1));
        assert!(all.contains(&CharacterId::R0));
        assert!(all.contains(&CharacterId::R1));
    }

    #[test]
    fn test_characterid_team_members() {
        assert_eq!(
            CharacterId::team_members(TeamId::Left),
            [CharacterId::L0, CharacterId::L1]
        );
        assert_eq!(
            CharacterId::team_members(TeamId::Right),
            [CharacterId::R0, CharacterId::R1]
        );
    }

    #[test]
    fn test_characterid_display() {
        assert_eq!(CharacterId::L0.to_string(), "L0");
        assert_eq!(CharacterId::L1.to_string(), "L1");
        assert_eq!(CharacterId::R0.to_string(), "R0");
        assert_eq!(CharacterId::R1.to_string(), "R1");
    }

    #[test]
    fn test_teamid_display() {
        assert_eq!(TeamId::Left.to_string(), "left");
        assert_eq!(TeamId::Right.to_string(), "right");
    }

    #[test]
    fn test_controller_source_display() {
        assert_eq!(ControllerSource::Human.to_string(), "H");
        assert_eq!(ControllerSource::Ai.to_string(), "A");
        assert_eq!(ControllerSource::External.to_string(), "X");
    }
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
            GameEvent::Pickup { .. } => "PU",
            GameEvent::Drop { .. } => "DR",
            GameEvent::ShotStart { .. } => "SS",
            GameEvent::ShotRelease { .. } => "SR",
            GameEvent::Pass { .. } => "PA",
            GameEvent::StealAttempt { .. } => "SA",
            GameEvent::StealSuccess { .. } => "S+",
            GameEvent::StealFail { .. } => "S-",
            GameEvent::StealOutOfRange { .. } => "SO",
            GameEvent::Jump { .. } => "J",
            GameEvent::Land { .. } => "LD",
            GameEvent::AiGoal { .. } => "AG",
            GameEvent::NavStart { .. } => "NS",
            GameEvent::NavComplete { .. } => "NC",
            GameEvent::Input { .. } => "I",
            GameEvent::Tick { .. } => "T",
            GameEvent::ControllerInput { .. } => "CI",
            GameEvent::ControllerAssign { .. } => "CA",
            GameEvent::ControllerSwap { .. } => "CS",
            GameEvent::ResetAiState { .. } => "RA",
            GameEvent::ResetScores => "RS",
            GameEvent::ResetBall => "RB",
            GameEvent::LevelChange { .. } => "LC",
            GameEvent::PassCompleted { .. } => "PC",
            GameEvent::PassIntercepted { .. } => "PI",
            GameEvent::PassMissed { .. } => "PM",
            GameEvent::TurboActivated { .. } => "TA",
            GameEvent::TurboDeactivated { .. } => "TD",
            GameEvent::BlockActivated { .. } => "BA",
            GameEvent::BlockDeactivated { .. } => "BD",
            GameEvent::BlockIntercepted { .. } => "BI",
        }
    }
}
