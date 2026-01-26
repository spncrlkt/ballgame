//! AI module - AI decision making and input generation

pub mod capabilities;
pub mod decision;
pub mod navigation;
pub mod pathfinding;
mod profiles;
pub mod shot_quality;
pub mod world_model;

pub use capabilities::AiCapabilities;
pub use decision::*;
pub use navigation::{
    AiNavState, EdgeType, NavAction, NavEdge, NavGraph, NavNode,
    mark_nav_dirty_on_level_change, rebuild_nav_graph,
};
pub use pathfinding::{PathResult, find_path, find_path_to_shoot};
pub use profiles::*;
pub use shot_quality::{evaluate_shot_quality, SHOT_QUALITY_ACCEPTABLE, SHOT_QUALITY_GOOD};
pub use world_model::{extract_platform_data, extract_platforms_from_nav, PlatformBounds};

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
    /// Target position for navigation (set by goal system, consumed by nav system)
    pub nav_target: Option<bevy::prelude::Vec2>,
    /// Whether AI is performing a jump shot
    pub jump_shot_active: bool,
    /// Timer for jump shot (tracks jump phase)
    pub jump_shot_timer: f32,
    /// Last position for stuck detection
    pub last_position: Option<bevy::prelude::Vec2>,
    /// Timer for how long AI has been stuck (not moving while trying to)
    pub stuck_timer: f32,
    /// Elapsed time at last defensive goal switch (for hysteresis)
    pub last_defense_switch: f32,
    /// Timer for steal reaction delay (simulates human reaction time)
    pub steal_reaction_timer: f32,
    /// Whether AI was in steal range last frame (for reset detection)
    pub was_in_steal_range: bool,
    /// Cooldown timer for button presses (simulates human mashing speed)
    pub button_press_cooldown: f32,
    /// Commitment timer for steal attempts - prevents premature exit from AttemptSteal
    pub steal_commit_timer: f32,
    /// Time in seconds the AI has been holding the ball (for desperation shots)
    pub ball_hold_time: f32,
    /// Position at start of stuck detection window (for cumulative movement check)
    pub stuck_window_start: Option<bevy::prelude::Vec2>,
    /// Timer for stuck detection window (resets when movement detected)
    pub stuck_window_timer: f32,
    /// Timer for stuck reversal - when > 0, override movement direction
    pub stuck_reverse_timer: f32,
    /// The reversed direction to use when stuck_reverse_timer > 0
    pub stuck_reverse_direction: f32,
}

/// Goals the AI can pursue
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum AiGoal {
    /// Debug mode - stand still, do nothing
    Idle,
    /// Move toward free ball and pick it up
    #[default]
    ChaseBall,
    /// Move toward basket with ball
    AttackWithBall,
    /// Charging a shot at the basket
    ChargeShot,
    /// Attempting to steal from opponent
    AttemptSteal,
    /// Chase ball carrier, position on shot line (uses navigation for platforms)
    InterceptDefense,
    /// Close-range: stay on opponent, attempt steals
    PressureDefense,
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
