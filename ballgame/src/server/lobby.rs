//! Server lobby state and systems
//!
//! Manages the pre-match lobby screen in server mode where
//! the host can configure settings and wait for clients.

use bevy::prelude::*;

/// Menu row types in the lobby
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LobbyRow {
    /// Slot configuration (0-3)
    #[default]
    Slot0,
    Slot1,
    Slot2,
    Slot3,
    /// Level selection
    Level,
    /// Score limit toggle
    ScoreLimit,
    /// Time limit toggle
    TimeLimit,
    /// Start game button
    StartGame,
}

impl LobbyRow {
    /// Get the next row (wraps around)
    pub fn next(&self) -> Self {
        match self {
            LobbyRow::Slot0 => LobbyRow::Slot1,
            LobbyRow::Slot1 => LobbyRow::Slot2,
            LobbyRow::Slot2 => LobbyRow::Slot3,
            LobbyRow::Slot3 => LobbyRow::Level,
            LobbyRow::Level => LobbyRow::ScoreLimit,
            LobbyRow::ScoreLimit => LobbyRow::TimeLimit,
            LobbyRow::TimeLimit => LobbyRow::StartGame,
            LobbyRow::StartGame => LobbyRow::Slot0,
        }
    }

    /// Get the previous row (wraps around)
    pub fn prev(&self) -> Self {
        match self {
            LobbyRow::Slot0 => LobbyRow::StartGame,
            LobbyRow::Slot1 => LobbyRow::Slot0,
            LobbyRow::Slot2 => LobbyRow::Slot1,
            LobbyRow::Slot3 => LobbyRow::Slot2,
            LobbyRow::Level => LobbyRow::Slot3,
            LobbyRow::ScoreLimit => LobbyRow::Level,
            LobbyRow::TimeLimit => LobbyRow::ScoreLimit,
            LobbyRow::StartGame => LobbyRow::TimeLimit,
        }
    }

    /// Check if this row is a slot row
    pub fn is_slot(&self) -> bool {
        matches!(
            self,
            LobbyRow::Slot0 | LobbyRow::Slot1 | LobbyRow::Slot2 | LobbyRow::Slot3
        )
    }

    /// Get slot index if this is a slot row
    pub fn slot_index(&self) -> Option<u8> {
        match self {
            LobbyRow::Slot0 => Some(0),
            LobbyRow::Slot1 => Some(1),
            LobbyRow::Slot2 => Some(2),
            LobbyRow::Slot3 => Some(3),
            _ => None,
        }
    }
}

/// Lobby state resource
#[derive(Resource)]
pub struct LobbyState {
    /// Whether the lobby is currently active
    pub active: bool,
    /// Currently selected row
    pub selected_row: LobbyRow,
    /// Whether the game can be started (always true for host)
    pub can_start: bool,
    /// Pulse animation timer for selected elements
    pub pulse_timer: f32,
    /// Timer for broadcasting lobby state to clients
    pub broadcast_timer: f32,
}

impl Default for LobbyState {
    fn default() -> Self {
        Self {
            active: false,
            selected_row: LobbyRow::StartGame,
            can_start: true,
            pulse_timer: 0.0,
            broadcast_timer: 0.0,
        }
    }
}

impl LobbyState {
    /// Create a new lobby state with lobby active
    pub fn new_active() -> Self {
        Self {
            active: true,
            ..Default::default()
        }
    }
}

/// Run condition: lobby is active
pub fn in_lobby(lobby: Option<Res<LobbyState>>) -> bool {
    lobby.map(|l| l.active).unwrap_or(false)
}

/// Run condition: lobby is NOT active (or doesn't exist)
pub fn not_in_lobby(lobby: Option<Res<LobbyState>>) -> bool {
    lobby.map(|l| !l.active).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lobby_row_navigation() {
        let mut row = LobbyRow::Slot0;
        row = row.next();
        assert_eq!(row, LobbyRow::Slot1);
        row = row.next();
        assert_eq!(row, LobbyRow::Slot2);
        row = row.prev();
        assert_eq!(row, LobbyRow::Slot1);
    }

    #[test]
    fn test_lobby_row_wrap() {
        assert_eq!(LobbyRow::StartGame.next(), LobbyRow::Slot0);
        assert_eq!(LobbyRow::Slot0.prev(), LobbyRow::StartGame);
    }

    #[test]
    fn test_slot_index() {
        assert_eq!(LobbyRow::Slot0.slot_index(), Some(0));
        assert_eq!(LobbyRow::Slot3.slot_index(), Some(3));
        assert_eq!(LobbyRow::Level.slot_index(), None);
    }
}
