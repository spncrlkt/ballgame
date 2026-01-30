//! Controller-to-character mapping for multi-player support
//!
//! Tracks which input source (keyboard, gamepad, AI) controls which character.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::events::CharacterId;
use super::source::InputSourceId;

/// Path to persistent controller mapping file
pub const CONTROLLER_MAPPING_FILE: &str = "config/controller_mapping.json";

/// Mapping from characters to their controlling input sources
#[derive(Resource, Default, Debug, Clone)]
pub struct ControllerMapping {
    /// Map from character to their assigned input source
    assignments: HashMap<CharacterId, InputSourceId>,
    /// Reverse lookup: source to character
    source_to_character: HashMap<InputSourceId, CharacterId>,
}

impl ControllerMapping {
    pub fn new() -> Self {
        Self::default()
    }

    /// Assign an input source to control a character
    /// Returns the previous source ID if the character was already assigned
    pub fn assign(&mut self, character: CharacterId, source_id: InputSourceId) -> Option<InputSourceId> {
        // Remove any existing assignment for this source
        if let Some(old_char) = self.source_to_character.remove(&source_id) {
            self.assignments.remove(&old_char);
        }

        // Remove any existing assignment for this character
        let old_source = self.assignments.remove(&character);
        if let Some(old) = old_source {
            self.source_to_character.remove(&old);
        }

        // Create new assignment
        self.assignments.insert(character, source_id);
        self.source_to_character.insert(source_id, character);

        old_source
    }

    /// Remove assignment for a character
    pub fn unassign_character(&mut self, character: CharacterId) -> Option<InputSourceId> {
        if let Some(source_id) = self.assignments.remove(&character) {
            self.source_to_character.remove(&source_id);
            Some(source_id)
        } else {
            None
        }
    }

    /// Remove assignment for an input source
    pub fn unassign_source(&mut self, source_id: InputSourceId) -> Option<CharacterId> {
        if let Some(character) = self.source_to_character.remove(&source_id) {
            self.assignments.remove(&character);
            Some(character)
        } else {
            None
        }
    }

    /// Get the input source controlling a character
    pub fn get_source(&self, character: CharacterId) -> Option<InputSourceId> {
        self.assignments.get(&character).copied()
    }

    /// Get the character controlled by an input source
    pub fn get_character(&self, source_id: InputSourceId) -> Option<CharacterId> {
        self.source_to_character.get(&source_id).copied()
    }

    /// Check if a character has an assigned controller
    pub fn is_assigned(&self, character: CharacterId) -> bool {
        self.assignments.contains_key(&character)
    }

    /// Get all assigned characters
    pub fn assigned_characters(&self) -> impl Iterator<Item = &CharacterId> {
        self.assignments.keys()
    }

    /// Get all assignments as (character, source) pairs
    pub fn all_assignments(&self) -> impl Iterator<Item = (&CharacterId, &InputSourceId)> {
        self.assignments.iter()
    }

    /// Get unassigned characters from a list
    pub fn unassigned_from(&self, characters: &[CharacterId]) -> Vec<CharacterId> {
        characters
            .iter()
            .filter(|c| !self.is_assigned(**c))
            .copied()
            .collect()
    }

    /// Clear all assignments
    pub fn clear(&mut self) {
        self.assignments.clear();
        self.source_to_character.clear();
    }
}

/// Persistent mapping that can be saved to disk
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistentMapping {
    /// Last known gamepad -> character assignments (by gamepad name)
    pub gamepad_assignments: HashMap<String, String>,
    /// Whether keyboard was assigned to a character
    pub keyboard_character: Option<String>,
}

impl PersistentMapping {
    /// Load mapping from file, or return default if file doesn't exist
    pub fn load() -> Self {
        let path = Path::new(CONTROLLER_MAPPING_FILE);
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(mapping) => return mapping,
                        Err(e) => {
                            warn!("Failed to parse controller mapping: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to read controller mapping: {}", e);
                }
            }
        }
        Self::default()
    }

    /// Save mapping to file
    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Path::new(CONTROLLER_MAPPING_FILE);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(path, content)
    }

    /// Record a gamepad assignment by name
    pub fn set_gamepad(&mut self, gamepad_name: &str, character: CharacterId) {
        self.gamepad_assignments.insert(
            gamepad_name.to_string(),
            character.to_string(),
        );
    }

    /// Get the character a gamepad was last assigned to
    pub fn get_gamepad(&self, gamepad_name: &str) -> Option<CharacterId> {
        self.gamepad_assignments
            .get(gamepad_name)
            .and_then(|s| CharacterId::from_str(s))
    }

    /// Record keyboard assignment
    pub fn set_keyboard(&mut self, character: Option<CharacterId>) {
        self.keyboard_character = character.map(|c| c.to_string());
    }

    /// Get keyboard's last assigned character
    pub fn get_keyboard(&self) -> Option<CharacterId> {
        self.keyboard_character
            .as_ref()
            .and_then(|s| CharacterId::from_str(s))
    }
}

/// Game mode determines how many characters are spawned
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    /// 1v1 mode: L0 vs R0 only
    #[default]
    OneVsOne,
    /// 2v2 mode: L0+L1 vs R0+R1
    TwoVsTwo,
}

/// Characters for 1v1 mode (static)
const ONE_V_ONE_CHARACTERS: [CharacterId; 2] = [CharacterId::L0, CharacterId::R0];

/// Characters for 2v2 mode (static)
const TWO_V_TWO_CHARACTERS: [CharacterId; 4] = [
    CharacterId::L0,
    CharacterId::L1,
    CharacterId::R0,
    CharacterId::R1,
];

impl GameMode {
    /// Get the characters that should be spawned for this mode
    pub fn characters(&self) -> &'static [CharacterId] {
        match self {
            GameMode::OneVsOne => &ONE_V_ONE_CHARACTERS,
            GameMode::TwoVsTwo => &TWO_V_TWO_CHARACTERS,
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<GameMode> {
        match s.to_lowercase().as_str() {
            "1v1" | "1vs1" | "onevone" => Some(GameMode::OneVsOne),
            "2v2" | "2vs2" | "twovtwo" => Some(GameMode::TwoVsTwo),
            _ => None,
        }
    }

    /// To string for serialization
    pub fn as_str(&self) -> &'static str {
        match self {
            GameMode::OneVsOne => "1v1",
            GameMode::TwoVsTwo => "2v2",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_mapping() {
        let mut mapping = ControllerMapping::new();

        // Assign keyboard (source 0) to L0
        mapping.assign(CharacterId::L0, 0);
        assert_eq!(mapping.get_source(CharacterId::L0), Some(0));
        assert_eq!(mapping.get_character(0), Some(CharacterId::L0));

        // Assign gamepad (source 1) to R0
        mapping.assign(CharacterId::R0, 1);
        assert_eq!(mapping.get_source(CharacterId::R0), Some(1));

        // Reassigning source 0 to R0 should remove it from L0
        mapping.assign(CharacterId::R0, 0);
        assert_eq!(mapping.get_source(CharacterId::L0), None);
        assert_eq!(mapping.get_source(CharacterId::R0), Some(0));
        assert_eq!(mapping.get_character(0), Some(CharacterId::R0));
    }

    #[test]
    fn test_game_mode() {
        assert_eq!(GameMode::OneVsOne.characters().len(), 2);
        assert_eq!(GameMode::TwoVsTwo.characters().len(), 4);

        assert_eq!(GameMode::from_str("1v1"), Some(GameMode::OneVsOne));
        assert_eq!(GameMode::from_str("2v2"), Some(GameMode::TwoVsTwo));
        assert_eq!(GameMode::from_str("invalid"), None);
    }
}
