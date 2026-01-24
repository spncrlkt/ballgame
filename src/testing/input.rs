//! Scripted input injection for tests

use bevy::prelude::*;
use std::collections::HashMap;

use super::parser::{FrameInput, InputSnapshot};

/// Component to track test entity IDs
#[derive(Component, Debug, Clone)]
pub struct TestEntityId(pub String);

/// Resource holding scripted inputs for a test
#[derive(Resource, Default)]
pub struct ScriptedInputs {
    /// Map of frame -> (entity_id -> input_snapshot)
    pub frames: HashMap<u64, HashMap<String, InputSnapshot>>,
    /// Current input state per entity (persists between frames)
    pub current_state: HashMap<String, CurrentInputState>,
    /// Current frame number
    pub current_frame: u64,
    /// Maximum frame to run
    pub max_frame: u64,
}

/// Current input state for an entity (with persistence)
#[derive(Debug, Clone, Default)]
pub struct CurrentInputState {
    pub move_x: f32,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub pickup_pressed: bool,
    pub throw_held: bool,
}

impl ScriptedInputs {
    /// Create from parsed frame inputs
    pub fn from_inputs(inputs: &[FrameInput]) -> Self {
        let mut frames: HashMap<u64, HashMap<String, InputSnapshot>> = HashMap::new();
        let mut max_frame = 0u64;

        for fi in inputs {
            max_frame = max_frame.max(fi.frame);
            frames.insert(fi.frame, fi.inputs.clone());
        }

        Self {
            frames,
            current_state: HashMap::new(),
            current_frame: 0,
            max_frame,
        }
    }

    /// Set max frame (for state assertions)
    pub fn set_max_frame(&mut self, frame: u64) {
        self.max_frame = self.max_frame.max(frame);
    }

    /// Advance to next frame and return inputs for entities
    pub fn advance_frame(&mut self) -> HashMap<String, CurrentInputState> {
        // Check if there are new inputs for this frame
        if let Some(frame_inputs) = self.frames.get(&self.current_frame) {
            for (entity_id, snapshot) in frame_inputs {
                let state = self.current_state.entry(entity_id.clone()).or_default();

                // Update state from snapshot (only update fields that are explicitly set)
                state.move_x = snapshot.effective_move_x();

                if let Some(jump) = snapshot.jump {
                    // Jump is a press - only true for one frame
                    state.jump_pressed = jump;
                    state.jump_held = jump;
                } else {
                    state.jump_pressed = false;
                }

                if let Some(pickup) = snapshot.pickup {
                    state.pickup_pressed = pickup;
                } else {
                    state.pickup_pressed = false;
                }

                if let Some(throw) = snapshot.throw_held {
                    state.throw_held = throw;
                }
            }
        } else {
            // Clear single-frame inputs
            for state in self.current_state.values_mut() {
                state.jump_pressed = false;
                state.pickup_pressed = false;
            }
        }

        self.current_frame += 1;
        self.current_state.clone()
    }

    /// Check if simulation should continue
    pub fn should_continue(&self) -> bool {
        self.current_frame <= self.max_frame
    }
}
