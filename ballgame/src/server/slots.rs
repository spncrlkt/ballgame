//! Player slot management
//!
//! Manages the assignment of players to game slots (0-3).

use std::sync::Arc;
use tokio::sync::RwLock;

use ballgame_protocol::{AgentInput, CharacterId, SlotState, handshake::ClientType};
use crate::input::InputSourceId;

/// Display information for a slot (used by lobby UI)
#[derive(Debug, Clone)]
pub enum SlotDisplay {
    /// Slot is empty
    Empty,
    /// Local player (host)
    Local,
    /// Remote client connected
    Remote {
        /// Client display name
        name: String,
    },
    /// Server-controlled AI
    ServerAi {
        /// AI profile name
        profile: String,
    },
}

impl SlotDisplay {
    /// Convert to protocol SlotState
    pub fn to_slot_state(&self) -> SlotState {
        match self {
            SlotDisplay::Empty => SlotState::Empty,
            SlotDisplay::Local => SlotState::Local,
            SlotDisplay::Remote { .. } => SlotState::Remote,
            SlotDisplay::ServerAi { .. } => SlotState::ServerAi,
        }
    }

    /// Get client name if this is a remote slot
    pub fn client_name(&self) -> Option<&str> {
        match self {
            SlotDisplay::Remote { name } => Some(name),
            _ => None,
        }
    }

    /// Get AI profile if this is a ServerAi slot
    pub fn ai_profile(&self) -> Option<&str> {
        match self {
            SlotDisplay::ServerAi { profile } => Some(profile),
            _ => None,
        }
    }
}

/// Slot identifier (0-3)
pub type SlotId = u8;

/// Maximum number of player slots
pub const MAX_SLOTS: usize = 4;

/// State of a single player slot
#[derive(Debug, Clone)]
pub enum Slot {
    /// Slot is empty (no player assigned)
    Empty,

    /// Local player (keyboard/gamepad input handled by server)
    Local {
        /// Which input source controls this slot (keyboard or specific gamepad)
        source_id: InputSourceId,
        /// Display name for the source
        source_name: String,
        /// Last input from local player
        input: AgentInput,
    },

    /// Remote client connected via WebSocket
    Remote {
        /// Unique client ID
        client_id: u64,
        /// Type of client (human, AI version, etc.)
        client_type: ClientType,
        /// Client display name
        client_name: String,
        /// Last received input from this client
        last_input: AgentInput,
        /// Last tick this client acknowledged
        last_ack_tick: u64,
    },

    /// Server-side AI (fallback when no client connected)
    ServerAi {
        /// AI profile ID
        profile_id: String,
    },
}

impl Slot {
    /// Check if slot is empty
    pub fn is_empty(&self) -> bool {
        matches!(self, Slot::Empty)
    }

    /// Check if slot has a remote client
    pub fn is_remote(&self) -> bool {
        matches!(self, Slot::Remote { .. })
    }

    /// Check if slot is local
    pub fn is_local(&self) -> bool {
        matches!(self, Slot::Local { .. })
    }

    /// Get the current input for this slot
    pub fn get_input(&self) -> AgentInput {
        match self {
            Slot::Empty => AgentInput::default(),
            Slot::Local { input, .. } => input.clone(),
            Slot::Remote { last_input, .. } => last_input.clone(),
            Slot::ServerAi { .. } => AgentInput::default(), // AI decides separately
        }
    }

    /// Get the client type if this is a remote slot
    pub fn client_type(&self) -> Option<&ClientType> {
        match self {
            Slot::Remote { client_type, .. } => Some(client_type),
            _ => None,
        }
    }

    /// Get the source ID if this is a local slot
    pub fn source_id(&self) -> Option<InputSourceId> {
        match self {
            Slot::Local { source_id, .. } => Some(*source_id),
            _ => None,
        }
    }
}

/// A client waiting to be assigned to a slot
#[derive(Debug, Clone)]
pub struct WaitingClient {
    pub client_id: u64,
    pub client_type: ClientType,
    pub client_name: String,
    /// Slot assigned by server operator (None = still waiting)
    pub assigned_slot: Option<SlotId>,
}

/// Manages all player slots
pub struct SlotManager {
    slots: Arc<RwLock<[Slot; MAX_SLOTS]>>,
    next_client_id: Arc<RwLock<u64>>,
    /// Clients waiting to be assigned to a slot
    waiting_clients: Arc<RwLock<Vec<WaitingClient>>>,
}

impl SlotManager {
    /// Create a new slot manager with all slots empty
    pub fn new() -> Self {
        Self {
            slots: Arc::new(RwLock::new([
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
            ])),
            next_client_id: Arc::new(RwLock::new(1)),
            waiting_clients: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a slot manager with a local player in the specified slot
    pub fn with_local_slot(local_slot: SlotId) -> Self {
        use crate::input::KEYBOARD_SOURCE_ID;
        // Build slots array synchronously for initialization
        let mut slots = [
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
        ];
        if (local_slot as usize) < MAX_SLOTS {
            slots[local_slot as usize] = Slot::Local {
                source_id: KEYBOARD_SOURCE_ID,
                source_name: "Keyboard".to_string(),
                input: AgentInput::default(),
            };
        }
        Self {
            slots: Arc::new(RwLock::new(slots)),
            next_client_id: Arc::new(RwLock::new(1)),
            waiting_clients: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a slot manager with a specific source in the specified slot
    pub fn with_local_source(local_slot: SlotId, source_id: InputSourceId, source_name: String) -> Self {
        let mut slots = [
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
            Slot::Empty,
        ];
        if (local_slot as usize) < MAX_SLOTS {
            slots[local_slot as usize] = Slot::Local {
                source_id,
                source_name,
                input: AgentInput::default(),
            };
        }
        Self {
            slots: Arc::new(RwLock::new(slots)),
            next_client_id: Arc::new(RwLock::new(1)),
            waiting_clients: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a remote client without assigning a slot (client waits in lobby)
    /// Returns the client ID
    pub async fn register_waiting(
        &self,
        client_type: ClientType,
        client_name: String,
    ) -> u64 {
        let mut next_id = self.next_client_id.write().await;
        let client_id = *next_id;
        *next_id += 1;

        let mut waiting = self.waiting_clients.write().await;
        waiting.push(WaitingClient {
            client_id,
            client_type,
            client_name,
            assigned_slot: None,
        });

        client_id
    }

    /// Check if a waiting client has been assigned a slot
    /// Returns the assigned slot if assigned, None otherwise
    pub async fn check_waiting_assignment(&self, client_id: u64) -> Option<SlotId> {
        let waiting = self.waiting_clients.read().await;
        waiting
            .iter()
            .find(|c| c.client_id == client_id)
            .and_then(|c| c.assigned_slot)
    }

    /// Assign a waiting client to a specific slot
    /// Returns true if successful, false if slot not available or client not found
    pub async fn assign_waiting_to_slot(&self, client_id: u64, slot_id: SlotId) -> bool {
        if (slot_id as usize) >= MAX_SLOTS {
            return false;
        }

        let mut slots = self.slots.write().await;
        let mut waiting = self.waiting_clients.write().await;

        // Check if slot is available (empty or ServerAi)
        if !matches!(slots[slot_id as usize], Slot::Empty | Slot::ServerAi { .. }) {
            return false;
        }

        // Find and update the waiting client
        if let Some(client) = waiting.iter_mut().find(|c| c.client_id == client_id) {
            // Move client from waiting to slot
            slots[slot_id as usize] = Slot::Remote {
                client_id,
                client_type: client.client_type.clone(),
                client_name: client.client_name.clone(),
                last_input: AgentInput::default(),
                last_ack_tick: 0,
            };
            client.assigned_slot = Some(slot_id);
            true
        } else {
            false
        }
    }

    /// Get list of waiting clients (for lobby display)
    pub async fn get_waiting_clients(&self) -> Vec<WaitingClient> {
        self.waiting_clients.read().await.clone()
    }

    /// Remove a waiting client (on disconnect)
    pub async fn remove_waiting(&self, client_id: u64) {
        let mut waiting = self.waiting_clients.write().await;
        waiting.retain(|c| c.client_id != client_id);
    }

    /// Find an empty slot and assign a remote client to it
    /// Returns the assigned slot ID and client ID, or None if no slots available
    pub async fn assign_remote(
        &self,
        client_type: ClientType,
        client_name: String,
    ) -> Option<(SlotId, u64)> {
        let mut slots = self.slots.write().await;
        let mut next_id = self.next_client_id.write().await;

        // Find first empty slot
        for (i, slot) in slots.iter_mut().enumerate() {
            if slot.is_empty() {
                let client_id = *next_id;
                *next_id += 1;

                *slot = Slot::Remote {
                    client_id,
                    client_type,
                    client_name,
                    last_input: AgentInput::default(),
                    last_ack_tick: 0,
                };

                return Some((i as SlotId, client_id));
            }
        }

        None
    }

    /// Release a slot (client disconnected)
    pub async fn release(&self, slot_id: SlotId) {
        if (slot_id as usize) < MAX_SLOTS {
            let mut slots = self.slots.write().await;
            slots[slot_id as usize] = Slot::Empty;
        }
    }

    /// Update input for a slot
    pub async fn set_input(&self, slot_id: SlotId, input: AgentInput, ack_tick: u64) {
        if (slot_id as usize) < MAX_SLOTS {
            let mut slots = self.slots.write().await;
            match &mut slots[slot_id as usize] {
                Slot::Local { input: i, .. } => {
                    *i = input;
                }
                Slot::Remote {
                    last_input,
                    last_ack_tick,
                    ..
                } => {
                    *last_input = input;
                    *last_ack_tick = ack_tick;
                }
                _ => {}
            }
        }
    }

    /// Update local input (for keyboard/gamepad)
    pub async fn set_local_input(&self, slot_id: SlotId, input: AgentInput) {
        if (slot_id as usize) < MAX_SLOTS {
            let mut slots = self.slots.write().await;
            if let Slot::Local { input: ref mut i, .. } = slots[slot_id as usize] {
                *i = input;
            }
        }
    }

    /// Assign a local source to a slot
    pub async fn assign_local(&self, slot_id: SlotId, source_id: InputSourceId, source_name: String) {
        if (slot_id as usize) < MAX_SLOTS {
            let mut slots = self.slots.write().await;
            // Only assign if slot is empty or already local
            if matches!(slots[slot_id as usize], Slot::Empty | Slot::Local { .. }) {
                slots[slot_id as usize] = Slot::Local {
                    source_id,
                    source_name,
                    input: AgentInput::default(),
                };
            }
        }
    }

    /// Find slot by source ID
    pub async fn find_by_source_id(&self, source_id: InputSourceId) -> Option<SlotId> {
        let slots = self.slots.read().await;
        for (i, slot) in slots.iter().enumerate() {
            if let Slot::Local { source_id: sid, .. } = slot {
                if *sid == source_id {
                    return Some(i as SlotId);
                }
            }
        }
        None
    }

    /// Get all current inputs (for game tick)
    pub async fn collect_inputs(&self) -> [AgentInput; MAX_SLOTS] {
        let slots = self.slots.read().await;
        [
            slots[0].get_input(),
            slots[1].get_input(),
            slots[2].get_input(),
            slots[3].get_input(),
        ]
    }

    /// Get a snapshot of all slots
    pub async fn snapshot(&self) -> [Slot; MAX_SLOTS] {
        self.slots.read().await.clone()
    }

    /// Get character ID for a slot
    pub fn slot_to_character(slot_id: SlotId) -> Option<CharacterId> {
        CharacterId::from_slot_index(slot_id)
    }

    /// Get slot ID for a character
    pub fn character_to_slot(character: CharacterId) -> SlotId {
        character.to_slot_index()
    }

    /// Check if a specific slot is occupied
    pub async fn is_slot_occupied(&self, slot_id: SlotId) -> bool {
        if (slot_id as usize) >= MAX_SLOTS {
            return false;
        }
        let slots = self.slots.read().await;
        !slots[slot_id as usize].is_empty()
    }

    /// Count occupied slots
    pub async fn occupied_count(&self) -> usize {
        let slots = self.slots.read().await;
        slots.iter().filter(|s| !s.is_empty()).count()
    }

    /// Get display info for a slot (for lobby UI)
    pub async fn get_slot_display(&self, slot_id: SlotId) -> SlotDisplay {
        if (slot_id as usize) >= MAX_SLOTS {
            return SlotDisplay::Empty;
        }
        let slots = self.slots.read().await;
        match &slots[slot_id as usize] {
            Slot::Empty => SlotDisplay::Empty,
            Slot::Local { .. } => SlotDisplay::Local,
            Slot::Remote { client_name, .. } => SlotDisplay::Remote {
                name: client_name.clone(),
            },
            Slot::ServerAi { profile_id } => SlotDisplay::ServerAi {
                profile: profile_id.clone(),
            },
        }
    }

    /// Get display info for all slots
    pub async fn get_all_slot_displays(&self) -> [SlotDisplay; MAX_SLOTS] {
        let slots = self.slots.read().await;
        [
            Self::slot_to_display(&slots[0]),
            Self::slot_to_display(&slots[1]),
            Self::slot_to_display(&slots[2]),
            Self::slot_to_display(&slots[3]),
        ]
    }

    /// Convert a Slot to SlotDisplay
    fn slot_to_display(slot: &Slot) -> SlotDisplay {
        match slot {
            Slot::Empty => SlotDisplay::Empty,
            Slot::Local { .. } => SlotDisplay::Local,
            Slot::Remote { client_name, .. } => SlotDisplay::Remote {
                name: client_name.clone(),
            },
            Slot::ServerAi { profile_id } => SlotDisplay::ServerAi {
                profile: profile_id.clone(),
            },
        }
    }

    /// Set AI profile for an empty or ServerAi slot
    pub async fn set_ai_profile(&self, slot_id: SlotId, profile: String) {
        if (slot_id as usize) >= MAX_SLOTS {
            return;
        }
        let mut slots = self.slots.write().await;
        match &mut slots[slot_id as usize] {
            Slot::Empty => {
                slots[slot_id as usize] = Slot::ServerAi { profile_id: profile };
            }
            Slot::ServerAi { profile_id } => {
                *profile_id = profile;
            }
            _ => {
                // Can't change AI profile for Local or Remote slots
            }
        }
    }

    /// Fill empty slots with ServerAi using the given default profile
    pub async fn fill_empty_with_ai(&self, default_profile: &str) {
        let mut slots = self.slots.write().await;
        for slot in slots.iter_mut() {
            if matches!(slot, Slot::Empty) {
                *slot = Slot::ServerAi {
                    profile_id: default_profile.to_string(),
                };
            }
        }
    }

    /// Clear ServerAi slots back to Empty (for returning to lobby)
    pub async fn clear_server_ai_slots(&self) {
        let mut slots = self.slots.write().await;
        for slot in slots.iter_mut() {
            if matches!(slot, Slot::ServerAi { .. }) {
                *slot = Slot::Empty;
            }
        }
    }

    /// Find slot by client ID
    pub async fn find_by_client_id(&self, client_id: u64) -> Option<SlotId> {
        let slots = self.slots.read().await;
        for (i, slot) in slots.iter().enumerate() {
            if let Slot::Remote { client_id: cid, .. } = slot {
                if *cid == client_id {
                    return Some(i as SlotId);
                }
            }
        }
        None
    }

    /// Reassign a remote client to a different slot
    /// Returns the old slot ID if successful, or None if client not found or target slot not available
    pub async fn reassign_remote(&self, client_id: u64, new_slot_id: SlotId) -> Option<SlotId> {
        if (new_slot_id as usize) >= MAX_SLOTS {
            return None;
        }

        let mut slots = self.slots.write().await;

        // Find current slot for this client
        let mut old_slot_id = None;
        let mut client_data = None;

        for (i, slot) in slots.iter_mut().enumerate() {
            if let Slot::Remote { client_id: cid, .. } = slot {
                if *cid == client_id {
                    old_slot_id = Some(i as SlotId);
                    // Take the client data
                    client_data = Some(std::mem::replace(slot, Slot::Empty));
                    break;
                }
            }
        }

        let old_slot = old_slot_id?;
        let data = client_data?;

        // Check if target slot is available (empty or ServerAi)
        if !matches!(slots[new_slot_id as usize], Slot::Empty | Slot::ServerAi { .. }) {
            // Target slot occupied by Local or another Remote - restore original
            slots[old_slot as usize] = data;
            return None;
        }

        // Move client to new slot
        slots[new_slot_id as usize] = data;

        Some(old_slot)
    }

    /// Get client ID for a slot (if it's a Remote slot)
    pub async fn get_client_id(&self, slot_id: SlotId) -> Option<u64> {
        if (slot_id as usize) >= MAX_SLOTS {
            return None;
        }
        let slots = self.slots.read().await;
        if let Slot::Remote { client_id, .. } = &slots[slot_id as usize] {
            Some(*client_id)
        } else {
            None
        }
    }
}

impl Default for SlotManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_slot_assignment() {
        let manager = SlotManager::new();

        // Assign first client
        let result = manager
            .assign_remote(ClientType::ai("v1"), "Bot1".to_string())
            .await;
        assert!(result.is_some());
        let (slot1, client1) = result.unwrap();
        assert_eq!(slot1, 0);
        assert_eq!(client1, 1);

        // Assign second client
        let result = manager
            .assign_remote(ClientType::Human, "Player".to_string())
            .await;
        assert!(result.is_some());
        let (slot2, client2) = result.unwrap();
        assert_eq!(slot2, 1);
        assert_eq!(client2, 2);

        // Release first slot
        manager.release(slot1).await;

        // New client should get slot 0 back
        let result = manager
            .assign_remote(ClientType::ai("v2"), "Bot2".to_string())
            .await;
        assert!(result.is_some());
        let (slot3, _) = result.unwrap();
        assert_eq!(slot3, 0);
    }

    #[tokio::test]
    async fn test_input_collection() {
        let manager = SlotManager::with_local_slot(0);

        // Set local input
        manager
            .set_local_input(0, AgentInput::with_movement(1.0))
            .await;

        let inputs = manager.collect_inputs().await;
        assert_eq!(inputs[0].move_x, 1.0);
        assert_eq!(inputs[1].move_x, 0.0); // Empty slot
    }
}
