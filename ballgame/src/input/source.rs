//! Input source tracking for multi-controller support
//!
//! Tracks input sources (gamepads, AI) and their assignments to characters.
//! Note: Keyboard is no longer supported as a gameplay controller input.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for an input source
pub type InputSourceId = u32;

/// Special ID for keyboard input source (always 0)
/// DEPRECATED: Keyboard is no longer supported as a gameplay controller.
/// This constant is kept for compatibility but should not be used for new code.
pub const KEYBOARD_SOURCE_ID: InputSourceId = 0;

/// Starting ID for AI sources (1000+)
pub const AI_SOURCE_ID_START: InputSourceId = 1000;

/// Starting ID for gamepad sources (1-999)
pub const GAMEPAD_SOURCE_ID_START: InputSourceId = 1;

/// Type of input source
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputSourceType {
    /// Gamepad input with device info
    Gamepad {
        /// Bevy's gamepad entity index
        device_index: u32,
        /// Gamepad name if available
        name: String,
    },
    /// AI-controlled with profile name
    Ai {
        /// AI profile name
        profile: String,
    },
}

impl InputSourceType {
    /// Check if this is a human-controlled source (gamepad)
    pub fn is_human(&self) -> bool {
        matches!(self, InputSourceType::Gamepad { .. })
    }

    /// Check if this is an AI source
    pub fn is_ai(&self) -> bool {
        matches!(self, InputSourceType::Ai { .. })
    }
}

impl std::fmt::Display for InputSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputSourceType::Gamepad { name, .. } => write!(f, "gamepad:{}", name),
            InputSourceType::Ai { profile } => write!(f, "ai:{}", profile),
        }
    }
}

/// Information about a registered input source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputSource {
    /// Unique identifier for this source
    pub id: InputSourceId,
    /// Type and details of the source
    pub source_type: InputSourceType,
}

impl InputSource {
    /// Create a new gamepad input source
    pub fn gamepad(id: InputSourceId, device_index: u32, name: String) -> Self {
        Self {
            id,
            source_type: InputSourceType::Gamepad { device_index, name },
        }
    }

    /// Create a new AI input source
    pub fn ai(id: InputSourceId, profile: String) -> Self {
        Self {
            id,
            source_type: InputSourceType::Ai { profile },
        }
    }

    /// Check if this is a human-controlled source
    pub fn is_human(&self) -> bool {
        self.source_type.is_human()
    }

    /// Get a descriptor string for logging
    pub fn descriptor(&self) -> String {
        self.source_type.to_string()
    }
}

/// Raw input state captured from a single source
#[derive(Debug, Clone, Default)]
pub struct RawInput {
    /// Horizontal movement (-1.0 to 1.0)
    pub move_x: f32,
    /// Jump buffer timer (set on press, counts down)
    pub jump_buffer_timer: f32,
    /// Is jump button currently held
    pub jump_held: bool,
    /// Pickup/steal button pressed (buffered)
    pub pickup_pressed: bool,
    /// Throw button currently held (for charging)
    pub throw_held: bool,
    /// Throw button was released (buffered for throw execution)
    pub throw_released: bool,
    /// Pass button pressed (new for 2v2)
    pub pass_pressed: bool,
    /// Block button pressed (buffered)
    pub block_pressed: bool,
    /// Turbo button held (continuous)
    pub turbo_held: bool,
}

impl RawInput {
    /// Reset all buffered (consumable) flags
    pub fn clear_buffers(&mut self) {
        self.pickup_pressed = false;
        self.throw_released = false;
        self.pass_pressed = false;
        self.block_pressed = false;
    }
}

/// Tracks information about connected gamepads
#[derive(Debug, Clone)]
pub struct GamepadInfo {
    /// Our assigned source ID
    pub source_id: InputSourceId,
    /// Bevy's gamepad entity
    pub entity: Entity,
    /// Gamepad name
    pub name: String,
}

/// Registry of all connected gamepads and their source IDs
#[derive(Resource, Default)]
pub struct GamepadRegistry {
    /// Map from Bevy gamepad entity to our GamepadInfo
    pub gamepads: HashMap<Entity, GamepadInfo>,
    /// Next available source ID for gamepads
    next_gamepad_id: InputSourceId,
    /// Next available source ID for AI
    next_ai_id: InputSourceId,
}

impl GamepadRegistry {
    pub fn new() -> Self {
        Self {
            gamepads: HashMap::new(),
            next_gamepad_id: GAMEPAD_SOURCE_ID_START,
            next_ai_id: AI_SOURCE_ID_START,
        }
    }

    /// Register a new gamepad and return its source ID
    pub fn register_gamepad(&mut self, entity: Entity, name: String) -> InputSourceId {
        if let Some(info) = self.gamepads.get(&entity) {
            return info.source_id;
        }

        let source_id = self.next_gamepad_id;
        self.next_gamepad_id += 1;

        self.gamepads.insert(
            entity,
            GamepadInfo {
                source_id,
                entity,
                name,
            },
        );

        source_id
    }

    /// Unregister a gamepad
    pub fn unregister_gamepad(&mut self, entity: Entity) -> Option<GamepadInfo> {
        self.gamepads.remove(&entity)
    }

    /// Get source ID for a gamepad entity
    pub fn get_source_id(&self, entity: Entity) -> Option<InputSourceId> {
        self.gamepads.get(&entity).map(|info| info.source_id)
    }

    /// Get gamepad info by source ID
    pub fn get_by_source_id(&self, source_id: InputSourceId) -> Option<&GamepadInfo> {
        self.gamepads.values().find(|info| info.source_id == source_id)
    }

    /// Allocate a new AI source ID
    pub fn allocate_ai_source_id(&mut self) -> InputSourceId {
        let id = self.next_ai_id;
        self.next_ai_id += 1;
        id
    }

    /// Get all registered gamepad source IDs
    pub fn all_gamepad_source_ids(&self) -> Vec<InputSourceId> {
        self.gamepads.values().map(|info| info.source_id).collect()
    }
}

/// Buffer holding raw input from all sources
#[derive(Resource, Default)]
pub struct InputBuffers {
    /// Map from source ID to raw input state
    pub buffers: HashMap<InputSourceId, RawInput>,
}

impl InputBuffers {
    /// Get or create a buffer for a source
    pub fn get_or_create(&mut self, source_id: InputSourceId) -> &mut RawInput {
        self.buffers.entry(source_id).or_default()
    }

    /// Get a buffer for a source (immutable)
    pub fn get(&self, source_id: InputSourceId) -> Option<&RawInput> {
        self.buffers.get(&source_id)
    }

    /// Clear all buffers
    pub fn clear_all(&mut self) {
        for buffer in self.buffers.values_mut() {
            buffer.clear_buffers();
        }
    }
}

/// System to track gamepad connections and update the registry
pub fn update_gamepad_registry(
    mut registry: ResMut<GamepadRegistry>,
    mut connection_events: bevy::prelude::MessageReader<
        bevy::input::gamepad::GamepadConnectionEvent,
    >,
) {
    for event in connection_events.read() {
        match &event.connection {
            bevy::input::gamepad::GamepadConnection::Connected { name, .. } => {
                let source_id = registry.register_gamepad(event.gamepad, name.clone());
                info!(
                    "Gamepad connected: {} (entity: {:?}, source_id: {})",
                    name, event.gamepad, source_id
                );
            }
            bevy::input::gamepad::GamepadConnection::Disconnected => {
                if let Some(info) = registry.unregister_gamepad(event.gamepad) {
                    info!(
                        "Gamepad disconnected: {} (entity: {:?}, source_id: {})",
                        info.name, event.gamepad, info.source_id
                    );
                }
            }
        }
    }
}

/// Deadzone for analog sticks
const STICK_DEADZONE: f32 = 0.15;

/// Jump buffer time in seconds
const JUMP_BUFFER_TIME: f32 = 0.1;

/// System to capture per-source input into InputBuffers
///
/// This captures input from each connected gamepad independently,
/// storing it in the InputBuffers resource indexed by source ID.
pub fn capture_per_source_input(
    gamepads: Query<(Entity, &Gamepad)>,
    registry: Res<GamepadRegistry>,
    mut buffers: ResMut<InputBuffers>,
    time: Res<Time>,
) {
    // Capture each gamepad's input independently
    for (entity, gamepad) in &gamepads {
        if let Some(source_id) = registry.get_source_id(entity) {
            let buffer = buffers.get_or_create(source_id);
            capture_gamepad_input(gamepad, buffer, time.delta_secs());
        }
    }
}

/// Capture gamepad input into a RawInput buffer
fn capture_gamepad_input(
    gamepad: &Gamepad,
    buffer: &mut RawInput,
    delta_secs: f32,
) {
    // Movement
    let mut move_x = 0.0;
    if let Some(stick_x) = gamepad.get(GamepadAxis::LeftStickX) {
        if stick_x.abs() > STICK_DEADZONE {
            move_x = stick_x;
        }
    }
    buffer.move_x = move_x;

    // Jump (South button - A on Xbox, X on PlayStation)
    let jump_pressed = gamepad.just_pressed(GamepadButton::South);
    buffer.jump_held = gamepad.pressed(GamepadButton::South);
    if jump_pressed {
        buffer.jump_buffer_timer = JUMP_BUFFER_TIME;
    } else {
        buffer.jump_buffer_timer = (buffer.jump_buffer_timer - delta_secs).max(0.0);
    }

    // Throw (RB / Right Trigger)
    let throw_held_now = gamepad.pressed(GamepadButton::RightTrigger);
    let throw_just_released = buffer.throw_held && !throw_held_now;
    if throw_just_released {
        buffer.throw_released = true;
    }
    buffer.throw_held = throw_held_now;

    // Pass/Pickup/Steal (LB / Left Trigger)
    if gamepad.just_pressed(GamepadButton::LeftTrigger) {
        buffer.pass_pressed = true;
        buffer.pickup_pressed = true;
    }

    // Pickup also from RB press
    if gamepad.just_pressed(GamepadButton::RightTrigger) {
        buffer.pickup_pressed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_source_types() {
        let gamepad = InputSource::gamepad(1, 0, "Xbox Controller".to_string());
        assert!(gamepad.is_human());
        assert_eq!(gamepad.descriptor(), "gamepad:Xbox Controller");

        let ai = InputSource::ai(1000, "Aggressive".to_string());
        assert!(!ai.is_human());
        assert_eq!(ai.descriptor(), "ai:Aggressive");
    }

    #[test]
    fn test_gamepad_registry() {
        let mut registry = GamepadRegistry::new();

        // Simulate entity IDs (in real Bevy these come from world.spawn())
        // from_raw_u32 returns Option<Entity> in recent Bevy versions
        let entity1 = Entity::from_raw_u32(1).expect("valid entity");
        let entity2 = Entity::from_raw_u32(2).expect("valid entity");

        let id1 = registry.register_gamepad(entity1, "Controller 1".to_string());
        let id2 = registry.register_gamepad(entity2, "Controller 2".to_string());

        assert_eq!(id1, GAMEPAD_SOURCE_ID_START);
        assert_eq!(id2, GAMEPAD_SOURCE_ID_START + 1);

        // Re-registering same entity returns same ID
        let id1_again = registry.register_gamepad(entity1, "Controller 1".to_string());
        assert_eq!(id1, id1_again);

        assert_eq!(registry.get_source_id(entity1), Some(id1));
        assert_eq!(registry.get_source_id(entity2), Some(id2));
    }
}
