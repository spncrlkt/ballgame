//! Character assignment system for lobby
//!
//! Tracks which input sources (gamepads, remote clients, AI profiles)
//! are assigned to which characters (L0, L1, R0, R1).

use bevy::prelude::*;

use crate::events::CharacterId;
use crate::input::InputSourceId;

/// Truncate a name to fit in the UI, adding "..." if needed
fn truncate_name(name: &str, max_len: usize) -> String {
    if name.chars().count() <= max_len {
        name.to_string()
    } else {
        let truncated: String = name.chars().take(max_len - 2).collect();
        format!("{}..", truncated)
    }
}

/// Assignment state for a single character
#[derive(Debug, Clone)]
pub enum CharacterAssignment {
    /// Character is not assigned to any input source
    Unassigned,
    /// Character is controlled by a local input source (keyboard or gamepad)
    Local {
        /// Source ID (KEYBOARD_SOURCE_ID or gamepad source ID)
        source_id: InputSourceId,
        /// Display name for the source (e.g., "Keyboard", "DualSense #1")
        source_name: String,
    },
    /// Character is controlled by a remote client
    Remote {
        /// Unique client ID
        client_id: u64,
        /// Client display name
        client_name: String,
    },
    /// Character is controlled by server-side AI
    ServerAi {
        /// AI profile name
        profile_name: String,
    },
}

impl Default for CharacterAssignment {
    fn default() -> Self {
        CharacterAssignment::Unassigned
    }
}

impl CharacterAssignment {
    /// Check if this assignment is from a local input source
    pub fn is_local(&self) -> bool {
        matches!(self, CharacterAssignment::Local { .. })
    }

    /// Check if this assignment is from a remote client
    pub fn is_remote(&self) -> bool {
        matches!(self, CharacterAssignment::Remote { .. })
    }

    /// Check if this assignment is AI-controlled
    pub fn is_ai(&self) -> bool {
        matches!(self, CharacterAssignment::ServerAi { .. })
    }

    /// Check if this character slot is unassigned
    pub fn is_unassigned(&self) -> bool {
        matches!(self, CharacterAssignment::Unassigned)
    }

    /// Get display name for UI (truncated to fit)
    pub fn display_name(&self) -> String {
        match self {
            CharacterAssignment::Unassigned => "EMPTY".to_string(),
            CharacterAssignment::Local { source_name, .. } => truncate_name(source_name, 14),
            CharacterAssignment::Remote { client_name, .. } => truncate_name(client_name, 14),
            CharacterAssignment::ServerAi { profile_name } => format!("AI: {}", truncate_name(profile_name, 10)),
        }
    }

    /// Get the source ID if this is a local assignment
    pub fn local_source_id(&self) -> Option<InputSourceId> {
        match self {
            CharacterAssignment::Local { source_id, .. } => Some(*source_id),
            _ => None,
        }
    }

    /// Get the client ID if this is a remote assignment
    pub fn remote_client_id(&self) -> Option<u64> {
        match self {
            CharacterAssignment::Remote { client_id, .. } => Some(*client_id),
            _ => None,
        }
    }
}

/// Resource tracking character assignments for all 4 characters
#[derive(Resource)]
pub struct CharacterAssignments {
    /// Assignments indexed by character (L0=0, L1=1, R0=2, R1=3)
    pub assignments: [CharacterAssignment; 4],
}

impl Default for CharacterAssignments {
    fn default() -> Self {
        // All characters start unassigned - must use gamepads to control
        Self::all_unassigned()
    }
}

impl CharacterAssignments {
    /// Create with all slots unassigned
    pub fn all_unassigned() -> Self {
        Self {
            assignments: [
                CharacterAssignment::Unassigned,
                CharacterAssignment::Unassigned,
                CharacterAssignment::Unassigned,
                CharacterAssignment::Unassigned,
            ],
        }
    }

    /// Get assignment for a character
    pub fn get(&self, character: CharacterId) -> &CharacterAssignment {
        &self.assignments[character.to_slot_index() as usize]
    }

    /// Get mutable assignment for a character
    pub fn get_mut(&mut self, character: CharacterId) -> &mut CharacterAssignment {
        &mut self.assignments[character.to_slot_index() as usize]
    }

    /// Assign a local source to a character
    pub fn assign_local(&mut self, character: CharacterId, source_id: InputSourceId, source_name: String) {
        // First, unassign this source from any other character
        for assignment in &mut self.assignments {
            if let CharacterAssignment::Local { source_id: sid, .. } = assignment {
                if *sid == source_id {
                    *assignment = CharacterAssignment::Unassigned;
                }
            }
        }
        // Now assign to the target character
        self.assignments[character.to_slot_index() as usize] = CharacterAssignment::Local {
            source_id,
            source_name,
        };
    }

    /// Assign a remote client to a character
    pub fn assign_remote(&mut self, character: CharacterId, client_id: u64, client_name: String) {
        // First, unassign this client from any other character
        for assignment in &mut self.assignments {
            if let CharacterAssignment::Remote { client_id: cid, .. } = assignment {
                if *cid == client_id {
                    *assignment = CharacterAssignment::Unassigned;
                }
            }
        }
        // Now assign to the target character
        self.assignments[character.to_slot_index() as usize] = CharacterAssignment::Remote {
            client_id,
            client_name,
        };
    }

    /// Assign server AI to a character
    pub fn assign_ai(&mut self, character: CharacterId, profile_name: String) {
        self.assignments[character.to_slot_index() as usize] = CharacterAssignment::ServerAi {
            profile_name,
        };
    }

    /// Unassign a character
    pub fn unassign(&mut self, character: CharacterId) {
        self.assignments[character.to_slot_index() as usize] = CharacterAssignment::Unassigned;
    }

    /// Find which character a local source is assigned to
    pub fn find_by_local_source(&self, source_id: InputSourceId) -> Option<CharacterId> {
        for (i, assignment) in self.assignments.iter().enumerate() {
            if let CharacterAssignment::Local { source_id: sid, .. } = assignment {
                if *sid == source_id {
                    return CharacterId::from_slot_index(i as u8);
                }
            }
        }
        None
    }

    /// Find which character a remote client is assigned to
    pub fn find_by_client_id(&self, client_id: u64) -> Option<CharacterId> {
        for (i, assignment) in self.assignments.iter().enumerate() {
            if let CharacterAssignment::Remote { client_id: cid, .. } = assignment {
                if *cid == client_id {
                    return CharacterId::from_slot_index(i as u8);
                }
            }
        }
        None
    }

    /// Fill unassigned characters with AI using the given profile
    pub fn fill_with_ai(&mut self, profile_name: &str) {
        for assignment in &mut self.assignments {
            if matches!(assignment, CharacterAssignment::Unassigned) {
                *assignment = CharacterAssignment::ServerAi {
                    profile_name: profile_name.to_string(),
                };
            }
        }
    }

    /// Clear all AI assignments back to unassigned
    pub fn clear_ai(&mut self) {
        for assignment in &mut self.assignments {
            if matches!(assignment, CharacterAssignment::ServerAi { .. }) {
                *assignment = CharacterAssignment::Unassigned;
            }
        }
    }
}

/// Type of connected input
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectedInputType {
    /// Connected gamepad
    Gamepad {
        /// Our assigned source ID
        source_id: InputSourceId,
    },
    /// Remote client (human or AI)
    RemoteClient {
        /// Unique client ID
        client_id: u64,
    },
}

/// Connection health status for remote clients
///
/// Tracks consecutive missed polls to provide graceful status changes:
/// - 0-2 misses: Good (normal display)
/// - 3-5 misses: Failing (red display)
/// - 6+ misses: Disconnected (remove from list)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConnectionHealth {
    /// Number of consecutive missed polls (resets to 0 when seen)
    pub miss_count: u8,
}

impl ConnectionHealth {
    /// Threshold for showing warning (red) color
    const WARNING_THRESHOLD: u8 = 3;
    /// Threshold for removing from list
    const DISCONNECT_THRESHOLD: u8 = 6;

    /// Check if connection should show as failing (red)
    pub fn is_failing(&self) -> bool {
        self.miss_count >= Self::WARNING_THRESHOLD
    }

    /// Check if connection should be removed
    pub fn should_disconnect(&self) -> bool {
        self.miss_count >= Self::DISCONNECT_THRESHOLD
    }

    /// Record a successful poll (client was seen)
    pub fn mark_seen(&mut self) {
        self.miss_count = 0;
    }

    /// Record a missed poll (client was not seen)
    pub fn mark_missed(&mut self) {
        self.miss_count = self.miss_count.saturating_add(1);
    }
}

/// Information about a connected input source
#[derive(Debug, Clone)]
pub struct ConnectedInput {
    /// Type of input
    pub input_type: ConnectedInputType,
    /// Display name for UI
    pub display_name: String,
    /// Which character this input is assigned to (if any)
    pub assigned_to: Option<CharacterId>,
    /// Connection health (only relevant for remote clients)
    pub health: ConnectionHealth,
}

impl ConnectedInput {
    /// Create a gamepad input entry
    pub fn gamepad(source_id: InputSourceId, name: String) -> Self {
        Self {
            input_type: ConnectedInputType::Gamepad { source_id },
            display_name: name,
            assigned_to: None,
            health: ConnectionHealth::default(),
        }
    }

    /// Create a remote client input entry
    pub fn remote(client_id: u64, name: String) -> Self {
        Self {
            input_type: ConnectedInputType::RemoteClient { client_id },
            display_name: name,
            assigned_to: None,
            health: ConnectionHealth::default(),
        }
    }

    /// Get source ID if this is a gamepad
    pub fn source_id(&self) -> Option<InputSourceId> {
        match self.input_type {
            ConnectedInputType::Gamepad { source_id } => Some(source_id),
            ConnectedInputType::RemoteClient { .. } => None,
        }
    }

    /// Get client ID if this is a remote client
    pub fn client_id(&self) -> Option<u64> {
        match self.input_type {
            ConnectedInputType::RemoteClient { client_id } => Some(client_id),
            _ => None,
        }
    }
}

/// How often to poll remote client connections in lobby (seconds)
const REMOTE_POLL_INTERVAL: f32 = 2.0;

/// Resource tracking all currently connected inputs
///
/// This is used by the lobby UI to show available input sources
/// in the left panel. Updated by the update_connected_inputs system.
#[derive(Resource, Default)]
pub struct ConnectedInputs {
    /// List of connected inputs
    pub inputs: Vec<ConnectedInput>,
    /// Timer for polling remote connections (only in lobby)
    pub remote_poll_timer: f32,
}

impl ConnectedInputs {
    /// Find input by source ID
    pub fn find_by_source_id(&self, source_id: InputSourceId) -> Option<&ConnectedInput> {
        self.inputs.iter().find(|i| i.source_id() == Some(source_id))
    }

    /// Find input by client ID
    pub fn find_by_client_id(&self, client_id: u64) -> Option<&ConnectedInput> {
        self.inputs.iter().find(|i| i.client_id() == Some(client_id))
    }

    /// Update from gamepad registry and assignments
    pub fn update_from_registry(
        &mut self,
        registry: &crate::input::GamepadRegistry,
        assignments: &CharacterAssignments,
    ) {
        // Remove only gamepad inputs (preserve remote clients and their health state)
        self.inputs.retain(|i| !matches!(i.input_type, ConnectedInputType::Gamepad { .. }));

        // Add all connected gamepads
        for info in registry.gamepads.values() {
            let mut gp = ConnectedInput::gamepad(info.source_id, info.name.clone());
            gp.assigned_to = assignments.find_by_local_source(info.source_id);
            self.inputs.push(gp);
        }

        // Sort: gamepads first (by source ID), then remote clients
        self.inputs.sort_by_key(|i| match i.input_type {
            ConnectedInputType::Gamepad { source_id } => (0, source_id as u64),
            ConnectedInputType::RemoteClient { client_id } => (1, client_id),
        });
    }

    /// Add or update a remote client
    pub fn add_remote_client(
        &mut self,
        client_id: u64,
        name: String,
        assigned_to: Option<CharacterId>,
    ) {
        // Check if client already exists
        for input in &mut self.inputs {
            if let ConnectedInputType::RemoteClient { client_id: cid } = input.input_type {
                if cid == client_id {
                    input.display_name = name;
                    input.assigned_to = assigned_to;
                    return;
                }
            }
        }

        // Add new client
        let mut input = ConnectedInput::remote(client_id, name);
        input.assigned_to = assigned_to;
        self.inputs.push(input);
    }

    /// Remove a remote client
    pub fn remove_remote_client(&mut self, client_id: u64) {
        self.inputs.retain(|i| {
            if let ConnectedInputType::RemoteClient { client_id: cid } = i.input_type {
                cid != client_id
            } else {
                true
            }
        });
    }

    /// Update assignment status for all inputs
    pub fn sync_assignments(&mut self, assignments: &CharacterAssignments) {
        for input in &mut self.inputs {
            match &input.input_type {
                ConnectedInputType::Gamepad { source_id } => {
                    input.assigned_to = assignments.find_by_local_source(*source_id);
                }
                ConnectedInputType::RemoteClient { client_id } => {
                    input.assigned_to = assignments.find_by_client_id(*client_id);
                }
            }
        }
    }
}

/// System to update ConnectedInputs from GamepadRegistry
///
/// Run this in Update to keep the connected inputs list in sync
/// with gamepad connections.
pub fn update_connected_inputs(
    registry: Res<crate::input::GamepadRegistry>,
    assignments: Res<CharacterAssignments>,
    mut connected: ResMut<ConnectedInputs>,
) {
    // Rebuild the local inputs list (keyboard + gamepads)
    // Remote clients are handled separately by sync_remote_clients_to_connected
    connected.update_from_registry(&registry, &assignments);
}

/// System to sync remote clients from SlotManager to ConnectedInputs
///
/// Run this in Update (server mode only) to keep remote clients visible in lobby.
/// Uses graceful polling: checks every 2 seconds, marks failing after miss,
/// only disconnects after 3 consecutive missed polls.
pub fn sync_remote_clients_to_connected(
    bridge: Res<crate::server::ServerBridge>,
    assignments: Res<CharacterAssignments>,
    mut connected: ResMut<ConnectedInputs>,
    time: Res<bevy::prelude::Time>,
    lobby_state: Option<Res<crate::server::LobbyState>>,
) {
    // In lobby, use graceful polling (every 2 seconds)
    // Outside lobby, sync immediately for responsiveness
    let in_lobby = lobby_state.map(|l| l.active).unwrap_or(false);

    if in_lobby {
        connected.remote_poll_timer += time.delta_secs();
        if connected.remote_poll_timer < REMOTE_POLL_INTERVAL {
            return;
        }
        connected.remote_poll_timer = 0.0;
    }

    // Get the slot snapshot
    let slots = bridge.runtime.block_on(bridge.slots.snapshot());

    // Build set of currently connected client IDs
    let current_client_ids: Vec<u64> = slots
        .iter()
        .filter_map(|slot| {
            if let crate::server::Slot::Remote { client_id, .. } = slot {
                Some(*client_id)
            } else {
                None
            }
        })
        .collect();

    if in_lobby {
        // Graceful handling: track misses, show warning at 3, remove at 6
        connected.inputs.retain_mut(|input| {
            if let ConnectedInputType::RemoteClient { client_id } = input.input_type {
                if current_client_ids.contains(&client_id) {
                    // Client is present, reset miss count
                    input.health.mark_seen();
                    true
                } else {
                    // Client is missing, increment miss count
                    input.health.mark_missed();
                    // Keep until disconnect threshold reached
                    !input.health.should_disconnect()
                }
            } else {
                true // Keep non-remote inputs
            }
        });
    } else {
        // Outside lobby: immediate removal (game is running)
        connected.inputs.retain(|input| {
            if let ConnectedInputType::RemoteClient { client_id } = input.input_type {
                current_client_ids.contains(&client_id)
            } else {
                true
            }
        });
    }

    // Add/update remote clients from slots
    for slot in slots.iter() {
        if let crate::server::Slot::Remote {
            client_id,
            client_name,
            ..
        } = slot
        {
            let assigned_to = assignments.find_by_client_id(*client_id);
            connected.add_remote_client(*client_id, client_name.clone(), assigned_to);
        }
    }

    // Sync assignment status for all inputs
    connected.sync_assignments(&assignments);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::GAMEPAD_SOURCE_ID_START;

    #[test]
    fn test_character_assignments_default() {
        let assignments = CharacterAssignments::default();
        // All characters start unassigned (no keyboard default)
        assert!(assignments.get(CharacterId::L0).is_unassigned());
        assert!(assignments.get(CharacterId::L1).is_unassigned());
        assert!(assignments.get(CharacterId::R0).is_unassigned());
        assert!(assignments.get(CharacterId::R1).is_unassigned());
    }

    #[test]
    fn test_assign_local_reassigns() {
        let mut assignments = CharacterAssignments::default();

        // Assign gamepad to L0
        let gamepad_id = GAMEPAD_SOURCE_ID_START;
        assignments.assign_local(CharacterId::L0, gamepad_id, "Gamepad 1".to_string());
        assert_eq!(assignments.find_by_local_source(gamepad_id), Some(CharacterId::L0));

        // Reassign gamepad to R0
        assignments.assign_local(CharacterId::R0, gamepad_id, "Gamepad 1".to_string());

        // L0 should now be unassigned
        assert!(assignments.get(CharacterId::L0).is_unassigned());
        // R0 should have the gamepad
        assert_eq!(assignments.find_by_local_source(gamepad_id), Some(CharacterId::R0));
    }

    #[test]
    fn test_fill_with_ai() {
        let mut assignments = CharacterAssignments::default();

        // Assign a gamepad to L0 first
        assignments.assign_local(CharacterId::L0, GAMEPAD_SOURCE_ID_START, "Gamepad 1".to_string());

        assignments.fill_with_ai("Balanced");

        // L0 should still have gamepad (was already assigned)
        assert!(assignments.get(CharacterId::L0).is_local());
        // Others should be AI
        assert!(assignments.get(CharacterId::L1).is_ai());
        assert!(assignments.get(CharacterId::R0).is_ai());
        assert!(assignments.get(CharacterId::R1).is_ai());
    }
}
