//! Agent input types for network transmission

use serde::{Deserialize, Serialize};

/// Input from an agent (player or AI) for a single tick
///
/// This is the network-serializable version of the game's InputState.
/// All values represent the desired actions for a single game tick.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentInput {
    /// Horizontal movement (-1.0 to 1.0)
    pub move_x: f32,

    /// Jump button pressed this tick
    pub jump_pressed: bool,

    /// Jump button held
    pub jump_held: bool,

    /// Pickup/steal/pass button pressed (context-dependent action)
    pub action_pressed: bool,

    /// Shoot button held (for charging)
    pub shoot_held: bool,

    /// Shoot button released (triggers shot)
    pub shoot_released: bool,

    /// Turbo button held (speed boost)
    pub turbo_held: bool,

    /// Block button pressed
    pub block_pressed: bool,

    /// Pass button pressed
    pub pass_pressed: bool,
}

impl AgentInput {
    /// Create a new empty input (no buttons pressed)
    pub fn new() -> Self {
        Self::default()
    }

    /// Create input for moving in a direction
    pub fn with_movement(move_x: f32) -> Self {
        Self {
            move_x,
            ..Default::default()
        }
    }

    /// Add jump to this input
    pub fn with_jump(mut self) -> Self {
        self.jump_pressed = true;
        self.jump_held = true;
        self
    }

    /// Add action (pickup/steal) to this input
    pub fn with_action(mut self) -> Self {
        self.action_pressed = true;
        self
    }

    /// Add shoot charge to this input
    pub fn with_shoot_held(mut self) -> Self {
        self.shoot_held = true;
        self
    }

    /// Add shoot release to this input
    pub fn with_shoot_release(mut self) -> Self {
        self.shoot_released = true;
        self
    }

    /// Add turbo to this input
    pub fn with_turbo(mut self) -> Self {
        self.turbo_held = true;
        self
    }

    /// Add block to this input
    pub fn with_block(mut self) -> Self {
        self.block_pressed = true;
        self
    }

    /// Add pass to this input
    pub fn with_pass(mut self) -> Self {
        self.pass_pressed = true;
        self
    }

    /// Check if any button is pressed
    pub fn has_input(&self) -> bool {
        self.move_x.abs() > 0.1
            || self.jump_pressed
            || self.jump_held
            || self.action_pressed
            || self.shoot_held
            || self.shoot_released
            || self.turbo_held
            || self.block_pressed
            || self.pass_pressed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_builder() {
        let input = AgentInput::with_movement(1.0).with_jump().with_turbo();

        assert_eq!(input.move_x, 1.0);
        assert!(input.jump_pressed);
        assert!(input.turbo_held);
        assert!(!input.action_pressed);
    }

    #[test]
    fn test_has_input() {
        assert!(!AgentInput::default().has_input());
        assert!(AgentInput::with_movement(1.0).has_input());
        assert!(AgentInput::default().with_jump().has_input());
    }

    #[test]
    fn test_serialization() {
        let input = AgentInput::with_movement(-0.5).with_action();
        let json = serde_json::to_string(&input).unwrap();
        let parsed: AgentInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.move_x, input.move_x);
        assert_eq!(parsed.action_pressed, input.action_pressed);
    }
}
