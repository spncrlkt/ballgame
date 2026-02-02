//! Game server module for multiplayer support
//!
//! Provides WebSocket-based server for accepting AI and human clients.

pub mod assignment;
pub mod bridge;
pub mod broadcast;
pub mod event_logger;
pub mod listener;
pub mod lobby;
pub mod session;
pub mod slots;
pub mod snapshot;
pub mod systems;
pub mod tournament;

pub use assignment::{
    CharacterAssignment, CharacterAssignments, ConnectedInput, ConnectedInputs,
    ConnectedInputType, ConnectionHealth, update_connected_inputs, sync_remote_clients_to_connected,
    sync_assignments_to_ai_state,
};
pub use bridge::ServerBridge;
pub use broadcast::Broadcaster;
pub use listener::GameServer;
pub use lobby::{in_lobby, not_in_lobby, LobbyRow, LobbyState, SourceOption, SourcePickerState};
pub use session::Session;
pub use slots::{Slot, SlotDisplay, SlotId, SlotManager};
pub use snapshot::create_game_snapshot;
pub use systems::{broadcast_lobby_state, broadcast_state_system, read_remote_inputs, server_mode_active};
pub use tournament::{check_tournament_end, update_match_timer, TournamentConfig};
pub use event_logger::{ServerEventLogger, ServerLoggingState, flush_server_events, track_match_logging};
