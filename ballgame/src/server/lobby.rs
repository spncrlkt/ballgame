//! Server lobby state and systems
//!
//! Manages the pre-match lobby screen in server mode where
//! the host can configure settings and wait for clients.

use bevy::prelude::*;

use crate::events::CharacterId;

/// Menu row types in the lobby
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LobbyRow {
    /// Character assignment (L0, L1, R0, R1)
    #[default]
    CharacterL0,
    CharacterL1,
    CharacterR0,
    CharacterR1,
    /// Level selection
    Level,
    /// Score limit toggle
    ScoreLimit,
    /// Time limit toggle
    TimeLimit,
    /// Start game button
    StartGame,
}

/// Source picker option type
#[derive(Debug, Clone, PartialEq)]
pub enum SourceOption {
    /// Unassigned / clear assignment
    Unassigned,
    /// Gamepad input source
    Gamepad {
        source_id: u32,
        name: String,
    },
    /// Remote client
    Remote {
        client_id: u64,
        name: String,
    },
    /// AI profile
    Ai {
        profile_name: String,
    },
}

impl LobbyRow {
    /// Get the next row (wraps around)
    pub fn next(&self) -> Self {
        match self {
            LobbyRow::CharacterL0 => LobbyRow::CharacterL1,
            LobbyRow::CharacterL1 => LobbyRow::CharacterR0,
            LobbyRow::CharacterR0 => LobbyRow::CharacterR1,
            LobbyRow::CharacterR1 => LobbyRow::Level,
            LobbyRow::Level => LobbyRow::ScoreLimit,
            LobbyRow::ScoreLimit => LobbyRow::TimeLimit,
            LobbyRow::TimeLimit => LobbyRow::StartGame,
            LobbyRow::StartGame => LobbyRow::CharacterL0,
        }
    }

    /// Get the previous row (wraps around)
    pub fn prev(&self) -> Self {
        match self {
            LobbyRow::CharacterL0 => LobbyRow::StartGame,
            LobbyRow::CharacterL1 => LobbyRow::CharacterL0,
            LobbyRow::CharacterR0 => LobbyRow::CharacterL1,
            LobbyRow::CharacterR1 => LobbyRow::CharacterR0,
            LobbyRow::Level => LobbyRow::CharacterR1,
            LobbyRow::ScoreLimit => LobbyRow::Level,
            LobbyRow::TimeLimit => LobbyRow::ScoreLimit,
            LobbyRow::StartGame => LobbyRow::TimeLimit,
        }
    }

    /// Check if this row is a character row
    pub fn is_character(&self) -> bool {
        matches!(
            self,
            LobbyRow::CharacterL0 | LobbyRow::CharacterL1 | LobbyRow::CharacterR0 | LobbyRow::CharacterR1
        )
    }

    /// Get character ID if this is a character row
    pub fn character_id(&self) -> Option<CharacterId> {
        match self {
            LobbyRow::CharacterL0 => Some(CharacterId::L0),
            LobbyRow::CharacterL1 => Some(CharacterId::L1),
            LobbyRow::CharacterR0 => Some(CharacterId::R0),
            LobbyRow::CharacterR1 => Some(CharacterId::R1),
            _ => None,
        }
    }

    /// Get slot index if this is a character row (for backwards compatibility)
    pub fn slot_index(&self) -> Option<u8> {
        self.character_id().map(|c| c.to_slot_index())
    }

    /// Create from character ID
    pub fn from_character(character: CharacterId) -> Self {
        match character {
            CharacterId::L0 => LobbyRow::CharacterL0,
            CharacterId::L1 => LobbyRow::CharacterL1,
            CharacterId::R0 => LobbyRow::CharacterR0,
            CharacterId::R1 => LobbyRow::CharacterR1,
        }
    }
}

/// Source picker overlay state
#[derive(Debug, Clone, Default)]
pub struct SourcePickerState {
    /// Whether the picker is currently open
    pub open: bool,
    /// Skip selection on the frame the picker opens (same button press)
    pub just_opened: bool,
    /// Which character we're assigning (when open)
    pub target_character: Option<CharacterId>,
    /// Currently selected option index
    pub selected_index: usize,
    /// Available options
    pub options: Vec<SourceOption>,
}

impl SourcePickerState {
    /// Open the picker for a specific character
    pub fn open_for(&mut self, character: CharacterId, options: Vec<SourceOption>) {
        self.open = true;
        self.just_opened = true;
        self.target_character = Some(character);
        self.selected_index = 0;
        self.options = options;
    }

    /// Close the picker
    pub fn close(&mut self) {
        self.open = false;
        self.just_opened = false;
        self.target_character = None;
        self.selected_index = 0;
        self.options.clear();
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if !self.options.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.options.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if !self.options.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.options.len();
        }
    }

    /// Get the currently selected option
    pub fn selected_option(&self) -> Option<&SourceOption> {
        self.options.get(self.selected_index)
    }

    /// Get the currently selected option (mutable)
    pub fn selected_option_mut(&mut self) -> Option<&mut SourceOption> {
        self.options.get_mut(self.selected_index)
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
    /// Source picker overlay state
    pub source_picker: SourcePickerState,
}

impl Default for LobbyState {
    fn default() -> Self {
        Self {
            active: false,
            selected_row: LobbyRow::StartGame,
            can_start: true,
            pulse_timer: 0.0,
            broadcast_timer: 0.0,
            source_picker: SourcePickerState::default(),
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

    /// Check if currently in picker mode
    pub fn in_picker_mode(&self) -> bool {
        self.source_picker.open
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
        let mut row = LobbyRow::CharacterL0;
        row = row.next();
        assert_eq!(row, LobbyRow::CharacterL1);
        row = row.next();
        assert_eq!(row, LobbyRow::CharacterR0);
        row = row.prev();
        assert_eq!(row, LobbyRow::CharacterL1);
    }

    #[test]
    fn test_lobby_row_wrap() {
        assert_eq!(LobbyRow::StartGame.next(), LobbyRow::CharacterL0);
        assert_eq!(LobbyRow::CharacterL0.prev(), LobbyRow::StartGame);
    }

    #[test]
    fn test_character_id() {
        assert_eq!(LobbyRow::CharacterL0.character_id(), Some(CharacterId::L0));
        assert_eq!(LobbyRow::CharacterR1.character_id(), Some(CharacterId::R1));
        assert_eq!(LobbyRow::Level.character_id(), None);
    }

    #[test]
    fn test_slot_index() {
        assert_eq!(LobbyRow::CharacterL0.slot_index(), Some(0));
        assert_eq!(LobbyRow::CharacterR1.slot_index(), Some(3));
        assert_eq!(LobbyRow::Level.slot_index(), None);
    }
}
