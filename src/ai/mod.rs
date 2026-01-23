//! AI module - AI decision making and input generation

mod decision;

pub use decision::*;

use bevy::prelude::*;

use crate::input::PlayerInput;
use crate::player::{HumanControlled, Player};

/// Per-entity input state for AI-controlled players.
/// Both players have this component since either can be AI-controlled.
/// Mirrors PlayerInput structure for consistent physics behavior.
#[derive(Component, Default)]
pub struct AiInput {
    pub move_x: f32,
    pub jump_buffer_timer: f32,
    pub jump_held: bool,
    pub pickup_pressed: bool,
    pub throw_held: bool,
    pub throw_released: bool,
}

/// AI state machine tracking current goal and parameters
#[derive(Component, Default)]
pub struct AiState {
    pub current_goal: AiGoal,
    pub shot_charge_target: f32,
}

/// Goals the AI can pursue
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum AiGoal {
    /// Debug mode - stand still, do nothing
    Idle,
    /// Move toward free ball and pick it up
    #[default]
    ChaseBall,
    /// Return to defensive position when opponent has ball
    ReturnToDefense,
    /// Move toward basket with ball
    AttackWithBall,
    /// Charging a shot at the basket
    ChargeShot,
    /// Attempting to steal from opponent
    AttemptSteal,
}

/// Copy human PlayerInput into the human-controlled player's AiInput.
/// This unifies input handling - all systems just read from AiInput.
/// Consumable flags (pickup_pressed, throw_released) are moved, not copied.
/// Runs early in Update, after capture_input.
pub fn copy_human_input(
    mut human_input: ResMut<PlayerInput>,
    mut human_query: Query<&mut AiInput, (With<Player>, With<HumanControlled>)>,
) {
    let Ok(mut ai_input) = human_query.single_mut() else {
        return;
    };

    // Continuous inputs (overwrite each frame)
    ai_input.move_x = human_input.move_x;
    ai_input.jump_held = human_input.jump_held;
    ai_input.throw_held = human_input.throw_held;

    // Jump buffer timer: copy from PlayerInput to AiInput
    // The timer decrements in capture_input (Update) and gets consumed in apply_input (FixedUpdate)
    // We always copy the latest value - if FixedUpdate consumed it, ai_input.timer will be 0
    // and won't trigger another jump until a new press sets human_input.timer again
    ai_input.jump_buffer_timer = human_input.jump_buffer_timer;

    // Consumable flags (move to AiInput, clear from PlayerInput)
    if human_input.pickup_pressed {
        ai_input.pickup_pressed = true;
        human_input.pickup_pressed = false;
    }
    if human_input.throw_released {
        ai_input.throw_released = true;
        human_input.throw_released = false;
    }
}

/// Swap which player the human controls (Q key / L bumper).
/// Moves the HumanControlled marker to the other player.
/// AI controls whichever player doesn't have HumanControlled.
pub fn swap_control(
    mut commands: Commands,
    mut input: ResMut<PlayerInput>,
    human_query: Query<Entity, (With<Player>, With<HumanControlled>)>,
    other_query: Query<Entity, (With<Player>, Without<HumanControlled>)>,
    mut ai_inputs: Query<&mut AiInput>,
) {
    if !input.swap_pressed {
        return;
    }
    input.swap_pressed = false;

    // Find current human-controlled player
    let Ok(current_human) = human_query.single() else {
        return;
    };

    // Find the other player
    let Ok(other_player) = other_query.single() else {
        return;
    };

    // Swap: remove HumanControlled from current, add to other
    commands.entity(current_human).remove::<HumanControlled>();
    commands.entity(other_player).insert(HumanControlled);

    // Reset both players' AiInput to prevent stale input
    for mut ai_input in &mut ai_inputs {
        *ai_input = AiInput::default();
    }
}
