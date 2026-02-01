//! Input module - PlayerInput resource and capture_input system

pub mod mapping;
pub mod source;

pub use mapping::{
    ControllerMapping, GameMode, PersistentMapping, CONTROLLER_MAPPING_FILE,
};
pub use source::{
    GamepadInfo, GamepadRegistry, InputBuffers, InputSource, InputSourceId, InputSourceType,
    RawInput, AI_SOURCE_ID_START, GAMEPAD_SOURCE_ID_START, KEYBOARD_SOURCE_ID,
};

use bevy::prelude::*;

use crate::constants::*;
use crate::events::{CharacterId, EventBus, GameEvent};
use crate::player::{Character, HumanControlled, Player, Team};
use crate::scoring::GamePaused;
use crate::ui::DebugMenuState;

/// Possession context for modal input mapping
/// Determines which actions are available based on ball possession
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PossessionContext {
    /// Player is holding the ball
    HoldingBall,
    /// Teammate has the ball
    TeammateHasBall,
    /// Opponent has the ball
    OpponentHasBall,
    /// Free ball is nearby (within pickup range)
    FreeBallNearby,
    /// No special context (ball far away, no one has it)
    #[default]
    NoPossession,
}

impl PossessionContext {
    /// Check if player should use "has ball" actions (pass, shoot)
    pub fn has_ball(&self) -> bool {
        matches!(self, PossessionContext::HoldingBall)
    }

    /// Check if player can attempt pickup
    pub fn can_pickup(&self) -> bool {
        matches!(self, PossessionContext::FreeBallNearby)
    }

    /// Check if player can attempt steal
    pub fn can_steal(&self) -> bool {
        matches!(self, PossessionContext::OpponentHasBall)
    }

    /// Check if player should use defensive actions (block)
    pub fn is_defensive(&self) -> bool {
        matches!(
            self,
            PossessionContext::OpponentHasBall
                | PossessionContext::TeammateHasBall
                | PossessionContext::NoPossession
        )
    }
}

/// Buffered input state for the human-controlled player
#[derive(Resource, Default)]
pub struct PlayerInput {
    pub move_x: f32,
    pub jump_buffer_timer: f32,      // Time remaining in jump buffer
    pub jump_held: bool,             // Is jump button currently held
    pub pickup_pressed: bool,        // West button - pick up ball (context: free ball)
    pub throw_held: bool,            // R shoulder - charging throw (context: holding ball)
    pub throw_released: bool,        // R shoulder released - execute throw
    pub swap_pressed: bool,          // L shoulder / Q key - swap which player you control
    pub advance_level_pressed: bool, // L shoulder / Q key - advance to next level (Reachability)
    pub pass_pressed: bool,          // L shoulder - pass to teammate (context: holding ball)
    pub block_pressed: bool,         // R shoulder - block (context: not holding ball)
    pub turbo_held: bool,            // West button held - turbo speed boost
    pub restart_level_pressed: bool, // D-pad Down - restart current level (training)
    pub next_level_pressed: bool,    // D-pad Left - advance to next level (training)
}

/// Runs in Update to capture input state before it's cleared.
/// Also emits ControllerInput events to the EventBus for auditability.
pub fn capture_input(
    _keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut input: ResMut<PlayerInput>,
    debug_menu: Res<DebugMenuState>,
    game_paused: Res<GamePaused>,
    time: Res<Time>,
    mut event_bus: ResMut<EventBus>,
    human_query: Query<(Option<&Character>, &Team), (With<Player>, With<HumanControlled>)>,
) {
    // Don't capture game input when debug menu is open or game is paused
    if debug_menu.open || game_paused.0 {
        return;
    }
    // Horizontal movement (continuous - overwrite each frame)
    let mut move_x = 0.0;

    for gamepad in &gamepads {
        if let Some(stick_x) = gamepad.get(GamepadAxis::LeftStickX) {
            if stick_x.abs() > STICK_DEADZONE {
                move_x += stick_x;
            }
        }
    }

    input.move_x = move_x.clamp(-1.0, 1.0);

    // Jump button state
    let jump_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::South));

    input.jump_held = gamepads.iter().any(|gp| gp.pressed(GamepadButton::South));

    // Jump buffering - reset timer on press, count down otherwise
    if jump_pressed {
        input.jump_buffer_timer = JUMP_BUFFER_TIME;
    } else {
        input.jump_buffer_timer = (input.jump_buffer_timer - time.delta_secs()).max(0.0);
    }

    // Track pickup presses this frame for event emission
    // (pickup_pressed is accumulated from LB and RB)
    let mut pickup_just_pressed = false;

    // Throw (R shoulder)
    let throw_held_now = gamepads
        .iter()
        .any(|gp| gp.pressed(GamepadButton::RightTrigger));

    // Accumulate throw_released until consumed (like jump buffering)
    let throw_just_released = input.throw_held && !throw_held_now;
    if throw_just_released {
        input.throw_released = true;
    }
    input.throw_held = throw_held_now;

    // Pass/Pickup/Steal (L shoulder) - modal based on context
    // - Holding ball: triggers pass
    // - Near free ball: triggers pickup
    // - Otherwise: triggers steal attempt
    // Also triggers advance_level for Reachability protocol
    if gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::LeftTrigger))
    {
        input.pass_pressed = true;
        input.pickup_pressed = true; // Both LB and RB can pickup
        input.advance_level_pressed = true;
        pickup_just_pressed = true;
    }

    // Turbo (West button held) - speed boost
    // Continuous state, not buffered
    input.turbo_held = gamepads.iter().any(|gp| gp.pressed(GamepadButton::West));

    // Block/Pickup (RB when not throwing) - modal
    // - Not holding ball + near free ball: pickup
    // - Not holding ball + defending: block
    // Note: RB is shared with throw, modal logic determines which action to take
    let block_just_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::RightTrigger));
    if block_just_pressed {
        input.block_pressed = true;
        input.pickup_pressed = true; // RB also triggers pickup
        pickup_just_pressed = true;
    }

    // D-pad controls for training mode
    // D-pad Down: Restart level
    if gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadDown))
    {
        input.restart_level_pressed = true;
    }

    // D-pad Left: Next level
    if gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadLeft))
    {
        input.next_level_pressed = true;
    }

    // D-pad Right: Cycle character (training mode)
    if gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadRight))
    {
        input.swap_pressed = true;
    }

    // Emit ControllerInput event to EventBus for auditability
    // Only emit if there's a human-controlled player
    if let Ok((character_opt, team)) = human_query.single() {
        // Get CharacterId from Character component, or derive from Team
        let character = character_opt
            .map(|c| c.0)
            .unwrap_or_else(|| match team {
                Team::Left => CharacterId::L0,
                Team::Right => CharacterId::R0,
            });

        event_bus.emit(GameEvent::ControllerInput {
            character,
            source_id: crate::input::KEYBOARD_SOURCE_ID,
            move_x: input.move_x,
            jump: input.jump_held,
            jump_pressed,
            throw: input.throw_held,
            throw_released: throw_just_released,
            pickup: pickup_just_pressed,
            pass: false, // Pass detection handled separately
        });
    }
}
