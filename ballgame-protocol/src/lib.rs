//! Ballgame Protocol - Shared types for client-server communication
//!
//! This crate contains all network protocol types without any Bevy dependencies,
//! enabling AI clients and other tools to communicate with the game server.

pub mod events;
pub mod game_state;
pub mod handshake;
pub mod input;
pub mod messages;
pub mod version;

pub use events::GameEvent;
pub use game_state::{AgentSnapshot, AiStateView, BallSnapshot, BallStateKind, GameStateSnapshot, Score};
pub use handshake::{ClientType, GameConfig};
pub use input::AgentInput;
pub use messages::{ClientMessage, ClientPayload, LobbySnapshot, ServerMessage, ServerPayload, SlotInfo, SlotState};
pub use version::{PROTOCOL_VERSION, is_compatible};

// Re-export fundamental types
pub use game_state::{Basket, CharacterId, Team, Vec2};
