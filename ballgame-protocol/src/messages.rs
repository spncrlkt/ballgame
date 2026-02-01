//! Client-server message types

use serde::{Deserialize, Serialize};

use crate::events::GameEvent;
use crate::game_state::{CharacterId, GameStateSnapshot, Team};
use crate::handshake::{ClientType, GameConfig};
use crate::input::AgentInput;
use crate::version::PROTOCOL_VERSION;

/// State of a slot in the lobby
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotState {
    /// Slot is empty (no player assigned)
    Empty,
    /// Local player (host keyboard/gamepad)
    Local,
    /// Remote client connected
    Remote,
    /// Server-controlled AI
    ServerAi,
}

/// Information about a single slot in the lobby
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotInfo {
    /// Slot ID (0-3)
    pub slot_id: u8,
    /// Current state of the slot
    pub state: SlotState,
    /// Client display name (if Remote)
    pub client_name: Option<String>,
    /// AI profile name (if ServerAi or configurable empty slot)
    pub ai_profile: Option<String>,
}

impl Default for SlotInfo {
    fn default() -> Self {
        Self {
            slot_id: 0,
            state: SlotState::Empty,
            client_name: None,
            ai_profile: None,
        }
    }
}

/// Snapshot of lobby state for broadcasting to clients
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LobbySnapshot {
    /// State of all 4 slots
    pub slots: [SlotInfo; 4],
    /// Current level ID
    pub level_id: String,
    /// Score limit (None = unlimited)
    pub score_limit: Option<u32>,
    /// Time limit in seconds (None = unlimited)
    pub time_limit_secs: Option<f32>,
}

/// Server to client message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMessage {
    /// Sequence number for ordering/acknowledgment
    pub seq: u64,
    /// Current authoritative game tick
    pub tick: u64,
    /// Message payload
    pub payload: ServerPayload,
}

impl ServerMessage {
    /// Create a new server message
    pub fn new(seq: u64, tick: u64, payload: ServerPayload) -> Self {
        Self { seq, tick, payload }
    }
}

/// Server to client message payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerPayload {
    /// Handshake response - connection accepted
    Welcome {
        /// Protocol version for compatibility check
        protocol_version: u32,
        /// Assigned player slot (0-3)
        assigned_slot: u8,
        /// Server tick rate in Hz
        tick_rate_hz: u8,
        /// Game configuration
        game_config: GameConfig,
    },

    /// Handshake rejection - connection refused
    Rejected {
        /// Reason for rejection
        reason: String,
    },

    /// Full game state snapshot (sent every tick)
    State(GameStateSnapshot),

    /// Game event notification
    Event(GameEvent),

    /// Match lifecycle - match starting
    MatchStart {
        /// Level ID
        level_id: String,
    },

    /// Match lifecycle - match ended
    MatchEnd {
        /// Winning team (None if tie)
        winner: Option<Team>,
    },

    /// Slot assignment changed
    SlotAssigned {
        /// Character assigned to this client
        character: CharacterId,
    },

    /// Ping for latency measurement
    Ping {
        /// Server timestamp when ping was sent
        server_time: f64,
    },

    /// Server shutting down
    Shutdown {
        /// Reason for shutdown
        reason: String,
    },

    /// Lobby state update (sent while in lobby)
    LobbyUpdate(LobbySnapshot),

    /// Match is starting (sent when host starts the game)
    MatchStarting {
        /// Level ID for the match
        level_id: String,
        /// Countdown duration in seconds
        countdown_secs: f32,
    },
}

impl ServerPayload {
    /// Create a welcome message with default tick rate
    pub fn welcome(assigned_slot: u8, game_config: GameConfig) -> Self {
        ServerPayload::Welcome {
            protocol_version: PROTOCOL_VERSION,
            assigned_slot,
            tick_rate_hz: 60,
            game_config,
        }
    }

    /// Create a rejection message
    pub fn rejected(reason: &str) -> Self {
        ServerPayload::Rejected {
            reason: reason.to_string(),
        }
    }
}

/// Client to server message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMessage {
    /// Sequence number for ordering
    pub seq: u64,
    /// Target tick this input applies to
    pub target_tick: u64,
    /// Last tick client has processed (for acknowledgment)
    pub ack_tick: u64,
    /// Message payload
    pub payload: ClientPayload,
}

impl ClientMessage {
    /// Create a new client message
    pub fn new(seq: u64, target_tick: u64, ack_tick: u64, payload: ClientPayload) -> Self {
        Self {
            seq,
            target_tick,
            ack_tick,
            payload,
        }
    }

    /// Create an input message
    pub fn input(seq: u64, target_tick: u64, ack_tick: u64, input: AgentInput) -> Self {
        Self::new(seq, target_tick, ack_tick, ClientPayload::Input(input))
    }
}

/// Client to server message payloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientPayload {
    /// Handshake request
    Hello {
        /// Protocol version for compatibility check
        protocol_version: u32,
        /// Client display name
        client_name: String,
        /// Type of client (human, AI, spectator)
        client_type: ClientType,
    },

    /// Input for this tick
    Input(AgentInput),

    /// Pong response to server ping
    Pong {
        /// Server timestamp from the ping
        server_time: f64,
        /// Client timestamp when responding
        client_time: f64,
    },

    /// Graceful disconnect
    Goodbye,
}

impl ClientPayload {
    /// Create a hello message for a human player
    pub fn hello_human(name: &str) -> Self {
        ClientPayload::Hello {
            protocol_version: PROTOCOL_VERSION,
            client_name: name.to_string(),
            client_type: ClientType::Human,
        }
    }

    /// Create a hello message for an AI client
    pub fn hello_ai(name: &str, version: &str) -> Self {
        ClientPayload::Hello {
            protocol_version: PROTOCOL_VERSION,
            client_name: name.to_string(),
            client_type: ClientType::ai(version),
        }
    }

    /// Create a hello message for a spectator
    pub fn hello_spectator(name: &str) -> Self {
        ClientPayload::Hello {
            protocol_version: PROTOCOL_VERSION,
            client_name: name.to_string(),
            client_type: ClientType::Spectator,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_message_serialization() {
        let msg = ServerMessage::new(
            1,
            100,
            ServerPayload::welcome(0, GameConfig::default_config()),
        );
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ServerMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.seq, 1);
        assert_eq!(parsed.tick, 100);
    }

    #[test]
    fn test_client_message_serialization() {
        let msg = ClientMessage::input(1, 101, 100, AgentInput::with_movement(1.0).with_jump());
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.seq, 1);
        assert_eq!(parsed.target_tick, 101);
    }

    #[test]
    fn test_hello_messages() {
        let human = ClientPayload::hello_human("Player1");
        let ai = ClientPayload::hello_ai("Bot", "v1");
        let spectator = ClientPayload::hello_spectator("Viewer");

        match human {
            ClientPayload::Hello { client_type, .. } => assert!(client_type.is_human()),
            _ => panic!("Expected Hello"),
        }
        match ai {
            ClientPayload::Hello { client_type, .. } => {
                assert!(client_type.is_ai());
                assert_eq!(client_type.ai_version(), Some("v1"));
            }
            _ => panic!("Expected Hello"),
        }
        match spectator {
            ClientPayload::Hello { client_type, .. } => assert!(client_type.is_spectator()),
            _ => panic!("Expected Hello"),
        }
    }
}
