//! Minimal gamepad debug tool - prints all controller events
//!
//! Usage: cargo run --bin gamepad_debug

use bevy::prelude::*;

fn main() {
    println!("=== Gamepad Debug Tool ===");
    println!("Press any button or move any stick on your controller.");
    println!("Press Escape to quit.\n");

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Gamepad Debug".to_string(),
                resolution: bevy::window::WindowResolution::from((400u32, 200u32)),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Update, (debug_gamepad_connections, debug_gamepad_input, check_exit))
        .run();
}

fn debug_gamepad_connections(
    mut connection_events: bevy::prelude::MessageReader<bevy::input::gamepad::GamepadConnectionEvent>,
) {
    for event in connection_events.read() {
        match &event.connection {
            bevy::input::gamepad::GamepadConnection::Connected { name, vendor_id, product_id } => {
                println!("[CONNECTED] Gamepad: {:?}", event.gamepad);
                println!("  Name: {}", name);
                println!("  Vendor ID: {:?}", vendor_id);
                println!("  Product ID: {:?}", product_id);
                println!();
            }
            bevy::input::gamepad::GamepadConnection::Disconnected => {
                println!("[DISCONNECTED] Gamepad: {:?}", event.gamepad);
            }
        }
    }
}

fn debug_gamepad_input(
    gamepads: Query<(Entity, &Gamepad)>,
) {
    for (_entity, gamepad) in &gamepads {
        // Check all buttons
        let buttons = [
            ("South (A/X)", GamepadButton::South),
            ("East (B/O)", GamepadButton::East),
            ("West (X/□)", GamepadButton::West),
            ("North (Y/△)", GamepadButton::North),
            ("LeftTrigger (LB)", GamepadButton::LeftTrigger),
            ("RightTrigger (RB)", GamepadButton::RightTrigger),
            ("LeftTrigger2 (LT)", GamepadButton::LeftTrigger2),
            ("RightTrigger2 (RT)", GamepadButton::RightTrigger2),
            ("Start", GamepadButton::Start),
            ("Select", GamepadButton::Select),
            ("LeftThumb (L3)", GamepadButton::LeftThumb),
            ("RightThumb (R3)", GamepadButton::RightThumb),
            ("DPadUp", GamepadButton::DPadUp),
            ("DPadDown", GamepadButton::DPadDown),
            ("DPadLeft", GamepadButton::DPadLeft),
            ("DPadRight", GamepadButton::DPadRight),
        ];

        for (name, button) in buttons {
            if gamepad.just_pressed(button) {
                println!("[BUTTON PRESSED]  {}", name);
            }
            if gamepad.just_released(button) {
                println!("[BUTTON RELEASED] {}", name);
            }
        }

        // Check all axes
        let axes = [
            ("LeftStickX", GamepadAxis::LeftStickX),
            ("LeftStickY", GamepadAxis::LeftStickY),
            ("RightStickX", GamepadAxis::RightStickX),
            ("RightStickY", GamepadAxis::RightStickY),
        ];

        for (name, axis) in axes {
            if let Some(value) = gamepad.get(axis) {
                if value.abs() > 0.15 {
                    println!("[AXIS] {}: {:.3}", name, value);
                }
            }
        }
    }
}

fn check_exit(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: bevy::prelude::MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        println!("\nExiting...");
        exit.write(AppExit::Success);
    }
}
