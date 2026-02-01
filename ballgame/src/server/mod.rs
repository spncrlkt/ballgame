//! Game server module for multiplayer support
//!
//! Provides WebSocket-based server for accepting AI and human clients.

pub mod bridge;
pub mod broadcast;
pub mod listener;
pub mod session;
pub mod slots;
pub mod snapshot;
pub mod systems;
pub mod tournament;

pub use bridge::ServerBridge;
pub use broadcast::Broadcaster;
pub use listener::GameServer;
pub use session::Session;
pub use slots::{Slot, SlotId, SlotManager};
pub use snapshot::create_game_snapshot;
pub use systems::{broadcast_state_system, read_remote_inputs, server_mode_active};
pub use tournament::{check_tournament_end, TournamentConfig};
