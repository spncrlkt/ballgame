//! AI module - AI decision making and input generation

mod decision;
mod profiles;

pub use decision::*;
pub use profiles::*;

use bevy::prelude::*;

use crate::input::PlayerInput;
use crate::player::{HumanControlled, Player};

/// Per-entity input buffer used by physics systems.
/// All players have this component - human input is copied here, AI writes directly.
/// This unifies input handling so physics systems read from one source.
#[derive(Component, Default)]
pub struct InputState {
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
    /// Index into AiProfileDatabase for this AI's personality
    pub profile_index: usize,
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

/// Copy human PlayerInput into the human-controlled player's InputState.
/// This unifies input handling - all systems just read from InputState.
/// Consumable flags (pickup_pressed, throw_released) are moved, not copied.
/// Runs early in Update, after capture_input.
pub fn copy_human_input(
    mut human_input: ResMut<PlayerInput>,
    mut human_query: Query<&mut InputState, (With<Player>, With<HumanControlled>)>,
) {
    let Ok(mut input_state) = human_query.single_mut() else {
        return;
    };

    // Continuous inputs (overwrite each frame)
    input_state.move_x = human_input.move_x;
    input_state.jump_held = human_input.jump_held;
    input_state.throw_held = human_input.throw_held;

    // Jump buffer timer: copy from PlayerInput to InputState
    // The timer decrements in capture_input (Update) and gets consumed in apply_input (FixedUpdate)
    // We always copy the latest value - if FixedUpdate consumed it, input_state.timer will be 0
    // and won't trigger another jump until a new press sets human_input.timer again
    input_state.jump_buffer_timer = human_input.jump_buffer_timer;

    // Consumable flags (move to InputState, clear from PlayerInput)
    if human_input.pickup_pressed {
        input_state.pickup_pressed = true;
        human_input.pickup_pressed = false;
    }
    if human_input.throw_released {
        input_state.throw_released = true;
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
    mut input_states: Query<&mut InputState>,
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

    // Reset both players' InputState to prevent stale input
    for mut input_state in &mut input_states {
        *input_state = InputState::default();
    }
}
