//! Physics tweak panel UI components and systems

use bevy::prelude::*;

use crate::constants::*;

/// Runtime-adjustable physics values for tweaking gameplay feel
#[derive(Resource)]
pub struct PhysicsTweaks {
    pub gravity_rise: f32,
    pub gravity_fall: f32,
    pub jump_velocity: f32,
    pub move_speed: f32,
    pub ground_accel: f32,
    pub ground_decel: f32,
    pub air_accel: f32,
    pub air_decel: f32,
    pub ball_gravity: f32,
    pub ball_bounce: f32,
    pub ball_air_friction: f32,
    pub ball_roll_friction: f32,
    pub shot_max_power: f32,
    pub shot_charge_time: f32,
    pub selected_index: usize, // Which value is currently selected for adjustment
    pub panel_visible: bool,
}

impl Default for PhysicsTweaks {
    fn default() -> Self {
        Self {
            gravity_rise: GRAVITY_RISE,
            gravity_fall: GRAVITY_FALL,
            jump_velocity: JUMP_VELOCITY,
            move_speed: MOVE_SPEED,
            ground_accel: GROUND_ACCEL,
            ground_decel: GROUND_DECEL,
            air_accel: AIR_ACCEL,
            air_decel: AIR_DECEL,
            ball_gravity: BALL_GRAVITY,
            ball_bounce: BALL_BOUNCE,
            ball_air_friction: BALL_AIR_FRICTION,
            ball_roll_friction: BALL_ROLL_FRICTION,
            shot_max_power: SHOT_MAX_POWER,
            shot_charge_time: SHOT_CHARGE_TIME,
            selected_index: 0,
            panel_visible: false,
        }
    }
}

impl PhysicsTweaks {
    pub const LABELS: [&'static str; 14] = [
        "Gravity Rise",
        "Gravity Fall",
        "Jump Velocity",
        "Move Speed",
        "Ground Accel",
        "Ground Decel",
        "Air Accel",
        "Air Decel",
        "Ball Gravity",
        "Ball Bounce",
        "Ball Air Friction",
        "Ball Roll Friction",
        "Shot Max Power",
        "Shot Charge Time",
    ];

    pub fn get_value(&self, index: usize) -> f32 {
        match index {
            0 => self.gravity_rise,
            1 => self.gravity_fall,
            2 => self.jump_velocity,
            3 => self.move_speed,
            4 => self.ground_accel,
            5 => self.ground_decel,
            6 => self.air_accel,
            7 => self.air_decel,
            8 => self.ball_gravity,
            9 => self.ball_bounce,
            10 => self.ball_air_friction,
            11 => self.ball_roll_friction,
            12 => self.shot_max_power,
            13 => self.shot_charge_time,
            _ => 0.0,
        }
    }

    pub fn get_default_value(index: usize) -> f32 {
        match index {
            0 => GRAVITY_RISE,
            1 => GRAVITY_FALL,
            2 => JUMP_VELOCITY,
            3 => MOVE_SPEED,
            4 => GROUND_ACCEL,
            5 => GROUND_DECEL,
            6 => AIR_ACCEL,
            7 => AIR_DECEL,
            8 => BALL_GRAVITY,
            9 => BALL_BOUNCE,
            10 => BALL_AIR_FRICTION,
            11 => BALL_ROLL_FRICTION,
            12 => SHOT_MAX_POWER,
            13 => SHOT_CHARGE_TIME,
            _ => 0.0,
        }
    }

    pub fn set_value(&mut self, index: usize, value: f32) {
        match index {
            0 => self.gravity_rise = value,
            1 => self.gravity_fall = value,
            2 => self.jump_velocity = value,
            3 => self.move_speed = value,
            4 => self.ground_accel = value,
            5 => self.ground_decel = value,
            6 => self.air_accel = value,
            7 => self.air_decel = value,
            8 => self.ball_gravity = value,
            9 => self.ball_bounce = value,
            10 => self.ball_air_friction = value,
            11 => self.ball_roll_friction = value,
            12 => self.shot_max_power = value,
            13 => self.shot_charge_time = value,
            _ => {}
        }
    }

    pub fn is_modified(&self, index: usize) -> bool {
        let current = self.get_value(index);
        let default = Self::get_default_value(index);
        (current - default).abs() > 0.001
    }

    pub fn reset_value(&mut self, index: usize) {
        self.set_value(index, Self::get_default_value(index));
    }

    pub fn reset_all(&mut self) {
        for i in 0..Self::LABELS.len() {
            self.reset_value(i);
        }
    }

    pub fn get_step(&self, index: usize) -> f32 {
        // Step size is ~10% of default value
        let default = Self::get_default_value(index);
        (default * 0.1).max(0.01) // At least 0.01 for small values
    }
}

/// Tweak panel container component
#[derive(Component)]
pub struct TweakPanel;

/// Tweak row component with index
#[derive(Component)]
pub struct TweakRow(pub usize);

/// Toggle tweak panel visibility and handle input
pub fn toggle_tweak_panel(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut tweaks: ResMut<PhysicsTweaks>,
    mut panel_query: Query<&mut Visibility, With<TweakPanel>>,
) {
    // F1 toggles panel visibility
    if keyboard.just_pressed(KeyCode::F1) {
        tweaks.panel_visible = !tweaks.panel_visible;
        if let Ok(mut visibility) = panel_query.single_mut() {
            *visibility = if tweaks.panel_visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }

    // Only process input when panel is visible
    if !tweaks.panel_visible {
        return;
    }

    let num_params = PhysicsTweaks::LABELS.len();

    // Up/Down to select parameter
    if keyboard.just_pressed(KeyCode::ArrowUp) {
        tweaks.selected_index = (tweaks.selected_index + num_params - 1) % num_params;
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        tweaks.selected_index = (tweaks.selected_index + 1) % num_params;
    }

    // Left/Right to adjust value (10% increments)
    let idx = tweaks.selected_index;
    let step = tweaks.get_step(idx);
    if keyboard.just_pressed(KeyCode::ArrowLeft) {
        let current = tweaks.get_value(idx);
        tweaks.set_value(idx, (current - step).max(0.01));
    }
    if keyboard.just_pressed(KeyCode::ArrowRight) {
        let current = tweaks.get_value(idx);
        tweaks.set_value(idx, current + step);
    }

    // R to reset selected parameter to default
    if keyboard.just_pressed(KeyCode::KeyR) {
        if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
            // Shift+R resets ALL parameters
            tweaks.reset_all();
        } else {
            // R resets just the selected parameter
            tweaks.reset_value(idx);
        }
    }
}

/// Update tweak panel display
pub fn update_tweak_panel(
    tweaks: Res<PhysicsTweaks>,
    mut row_query: Query<(&mut Text, &mut TextColor, &TweakRow)>,
) {
    if !tweaks.panel_visible {
        return;
    }

    for (mut text, mut color, row) in &mut row_query {
        let value = tweaks.get_value(row.0);
        let label = PhysicsTweaks::LABELS[row.0];
        let is_modified = tweaks.is_modified(row.0);

        // Format based on value type (friction shows 2 decimals, others show 0-1)
        let value_str = match row.0 {
            5 | 6 | 7 => format!("{:.2}", value), // Bounce/friction
            10 => format!("{:.1}s", value),       // Charge time
            _ => format!("{:.0}", value),         // Velocities
        };

        text.0 = format!("{}: {}", label, value_str);

        // Color priority: selected (yellow) > modified (red) > default (white)
        if row.0 == tweaks.selected_index {
            color.0 = Color::srgb(1.0, 1.0, 0.0); // Yellow for selected
        } else if is_modified {
            color.0 = Color::srgb(1.0, 0.4, 0.4); // Red for modified
        } else {
            color.0 = Color::WHITE;
        }
    }
}
