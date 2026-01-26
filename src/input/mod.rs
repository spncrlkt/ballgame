//! Input module - PlayerInput resource and capture_input system

use bevy::prelude::*;

use crate::constants::*;
use crate::events::{ControllerSource, EventBus, GameEvent};
use crate::player::HumanControlTarget;
use crate::ui::PhysicsTweaks;

/// Buffered input state for the human-controlled player
#[derive(Resource, Default)]
pub struct PlayerInput {
    pub move_x: f32,
    pub jump_buffer_timer: f32, // Time remaining in jump buffer
    pub jump_held: bool,        // Is jump button currently held
    pub pickup_pressed: bool,   // West button - pick up ball
    pub throw_held: bool,       // R shoulder - charging throw
    pub throw_released: bool,   // R shoulder released - execute throw
    pub swap_pressed: bool,     // L shoulder / Q key - swap which player you control
}

/// Runs in Update to capture input state before it's cleared.
/// Also emits ControllerInput events to the EventBus for auditability.
pub fn capture_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut input: ResMut<PlayerInput>,
    tweaks: Res<PhysicsTweaks>,
    time: Res<Time>,
    mut event_bus: ResMut<EventBus>,
    human_target: Res<HumanControlTarget>,
) {
    // Don't capture game input when tweak panel is open (uses arrow keys)
    if tweaks.panel_visible {
        return;
    }
    // Horizontal movement (continuous - overwrite each frame)
    let mut move_x = 0.0;

    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        move_x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        move_x += 1.0;
    }

    for gamepad in &gamepads {
        if let Some(stick_x) = gamepad.get(GamepadAxis::LeftStickX) {
            if stick_x.abs() > STICK_DEADZONE {
                move_x += stick_x;
            }
        }
    }

    input.move_x = move_x.clamp(-1.0, 1.0);

    // Jump button state
    let jump_pressed = keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::KeyW)
        || keyboard.just_pressed(KeyCode::ArrowUp)
        || gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::South));

    input.jump_held = keyboard.pressed(KeyCode::Space)
        || keyboard.pressed(KeyCode::KeyW)
        || keyboard.pressed(KeyCode::ArrowUp)
        || gamepads.iter().any(|gp| gp.pressed(GamepadButton::South));

    // Jump buffering - reset timer on press, count down otherwise
    if jump_pressed {
        input.jump_buffer_timer = JUMP_BUFFER_TIME;
    } else {
        input.jump_buffer_timer = (input.jump_buffer_timer - time.delta_secs()).max(0.0);
    }

    // Pickup (West button / E key) - accumulate until consumed
    let pickup_just_pressed = keyboard.just_pressed(KeyCode::KeyE)
        || gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::West));
    if pickup_just_pressed {
        input.pickup_pressed = true;
    }

    // Throw (R shoulder / F key)
    let throw_held_now = keyboard.pressed(KeyCode::KeyF)
        || gamepads
            .iter()
            .any(|gp| gp.pressed(GamepadButton::RightTrigger));

    // Accumulate throw_released until consumed (like jump buffering)
    let throw_just_released = input.throw_held && !throw_held_now;
    if throw_just_released {
        input.throw_released = true;
    }
    input.throw_held = throw_held_now;

    // Swap control (L shoulder / Q key) - accumulate until consumed
    if keyboard.just_pressed(KeyCode::KeyQ)
        || gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::LeftTrigger))
    {
        input.swap_pressed = true;
    }

    // Emit ControllerInput event to EventBus for auditability
    // Only emit if there's a human-controlled player
    if let Some(player) = human_target.0 {
        event_bus.emit(GameEvent::ControllerInput {
            player,
            source: ControllerSource::Human,
            move_x: input.move_x,
            jump: input.jump_held,
            jump_pressed,
            throw: input.throw_held,
            throw_released: throw_just_released,
            pickup: pickup_just_pressed,
        });
    }
}
