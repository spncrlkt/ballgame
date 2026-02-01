//! Handshake types for client-server connection

use serde::{Deserialize, Serialize};

/// Type of client connecting to the server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientType {
    /// Human player (keyboard/gamepad)
    Human,
    /// AI client with version identifier
    Ai { version: String },
    /// Spectator (view only, no control)
    Spectator,
}

impl ClientType {
    /// Create an AI client type with the given version
    pub fn ai(version: &str) -> Self {
        ClientType::Ai {
            version: version.to_string(),
        }
    }

    /// Check if this is an AI client
    pub fn is_ai(&self) -> bool {
        matches!(self, ClientType::Ai { .. })
    }

    /// Check if this is a human client
    pub fn is_human(&self) -> bool {
        matches!(self, ClientType::Human)
    }

    /// Check if this is a spectator
    pub fn is_spectator(&self) -> bool {
        matches!(self, ClientType::Spectator)
    }

    /// Get AI version if this is an AI client
    pub fn ai_version(&self) -> Option<&str> {
        match self {
            ClientType::Ai { version } => Some(version),
            _ => None,
        }
    }
}

impl std::fmt::Display for ClientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientType::Human => write!(f, "Human"),
            ClientType::Ai { version } => write!(f, "AI-{}", version),
            ClientType::Spectator => write!(f, "Spectator"),
        }
    }
}

/// Game configuration sent to clients during handshake
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameConfig {
    // Arena dimensions
    pub arena_width: f32,
    pub arena_height: f32,
    pub arena_floor_y: f32,

    // Player physics
    pub gravity_rise: f32,
    pub gravity_fall: f32,
    pub jump_velocity: f32,
    pub move_speed: f32,
    pub player_width: f32,
    pub player_height: f32,

    // Ball physics
    pub ball_gravity: f32,
    pub ball_bounce: f32,
    pub ball_size: f32,

    // Shooting
    pub shot_max_power: f32,
    pub shot_max_speed: f32,
    pub shot_charge_time: f32,

    // Steal
    pub steal_range: f32,
    pub steal_cooldown: f32,

    // Turbo
    pub turbo_speed_mult: f32,
    pub turbo_max_gauge: f32,

    // Level info
    pub level_id: String,
    pub level_name: String,
}

impl GameConfig {
    /// Create a default game config with standard values
    pub fn default_config() -> Self {
        Self {
            arena_width: 1920.0,
            arena_height: 1080.0,
            arena_floor_y: -540.0 + 10.0, // ARENA_HEIGHT/2 + FLOOR_THICKNESS

            gravity_rise: 2500.0,
            gravity_fall: 3500.0,
            jump_velocity: 850.0,
            move_speed: 450.0,
            player_width: 40.0,
            player_height: 60.0,

            ball_gravity: 1500.0,
            ball_bounce: 0.6,
            ball_size: 30.0,

            shot_max_power: 1.0,
            shot_max_speed: 1200.0,
            shot_charge_time: 1.0,

            steal_range: 60.0,
            steal_cooldown: 0.5,

            turbo_speed_mult: 1.5,
            turbo_max_gauge: 2.0,

            level_id: String::new(),
            level_name: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_type() {
        let human = ClientType::Human;
        let ai = ClientType::ai("v1");
        let spectator = ClientType::Spectator;

        assert!(human.is_human());
        assert!(!human.is_ai());

        assert!(ai.is_ai());
        assert_eq!(ai.ai_version(), Some("v1"));

        assert!(spectator.is_spectator());
    }

    #[test]
    fn test_game_config_serialization() {
        let config = GameConfig::default_config();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: GameConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.arena_width, config.arena_width);
    }
}
