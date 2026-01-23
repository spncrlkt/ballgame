//! AI module - AI decision making and input generation

mod decision;
mod profiles;

pub use decision::*;
pub use profiles::*;

use bevy::prelude::*;

use crate::input::PlayerInput;
use crate::player::{HumanControlled, Player, Team};

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
/// Cycles through: Left player → Right player → Observer (both AI) → Left player
pub fn swap_control(
    mut commands: Commands,
    mut input: ResMut<PlayerInput>,
    players: Query<(Entity, &Team), With<Player>>,
    human_query: Query<(Entity, &Team), (With<Player>, With<HumanControlled>)>,
    mut input_states: Query<&mut InputState>,
) {
    if !input.swap_pressed {
        return;
    }
    input.swap_pressed = false;

    // Find left and right players
    let mut left_player = None;
    let mut right_player = None;
    for (entity, team) in &players {
        match team {
            Team::Left => left_player = Some(entity),
            Team::Right => right_player = Some(entity),
        }
    }

    let (Some(left), Some(right)) = (left_player, right_player) else {
        return;
    };

    // Determine current state and cycle to next
    // Left → Right → Observer → Left
    match human_query.iter().next() {
        Some((_, Team::Left)) => {
            // Currently controlling left, switch to right
            commands.entity(left).remove::<HumanControlled>();
            commands.entity(right).insert(HumanControlled);
            info!("Control: Right player");
        }
        Some((_, Team::Right)) => {
            // Currently controlling right, switch to observer mode
            commands.entity(right).remove::<HumanControlled>();
            info!("Control: Observer (both AI)");
        }
        None => {
            // Observer mode, switch to left
            commands.entity(left).insert(HumanControlled);
            info!("Control: Left player");
        }
    }

    // Reset both players' InputState to prevent stale input
    for mut input_state in &mut input_states {
        *input_state = InputState::default();
    }
}
