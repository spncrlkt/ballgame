//! Preset application system - copies preset values into PhysicsTweaks

use bevy::prelude::*;

use crate::presets::PresetDatabase;
use crate::tuning::PhysicsTweaks;

/// Tracks which preset is selected for each category
#[derive(Resource, Default)]
pub struct CurrentPresets {
    pub movement: usize,
    pub ball: usize,
    pub shooting: usize,
    pub composite: usize,
    /// Set to true to trigger a preset apply
    pub apply_pending: bool,
}

impl CurrentPresets {
    /// Mark that presets need to be applied to PhysicsTweaks
    pub fn mark_apply(&mut self) {
        self.apply_pending = true;
    }
}

/// Apply selected presets to PhysicsTweaks when marked
pub fn apply_preset_to_tweaks(
    preset_db: Res<PresetDatabase>,
    mut current: ResMut<CurrentPresets>,
    mut tweaks: ResMut<PhysicsTweaks>,
) {
    if !current.apply_pending {
        return;
    }
    current.apply_pending = false;

    // Apply movement preset
    if let Some(movement) = preset_db.get_movement(current.movement) {
        tweaks.move_speed = movement.move_speed;
        tweaks.ground_accel = movement.ground_accel;
        tweaks.ground_decel = movement.ground_decel;
        tweaks.air_accel = movement.air_accel;
        tweaks.air_decel = movement.air_decel;
        tweaks.jump_velocity = movement.jump_velocity;
        tweaks.gravity_rise = movement.gravity_rise;
        tweaks.gravity_fall = movement.gravity_fall;
    }

    // Apply ball preset
    if let Some(ball) = preset_db.get_ball(current.ball) {
        tweaks.ball_gravity = ball.ball_gravity;
        tweaks.ball_bounce = ball.ball_bounce;
        tweaks.ball_air_friction = ball.ball_air_friction;
        tweaks.ball_roll_friction = ball.ball_roll_friction;
    }

    // Apply shooting preset
    if let Some(shooting) = preset_db.get_shooting(current.shooting) {
        tweaks.shot_charge_time = shooting.shot_charge_time;
        tweaks.shot_max_power = shooting.shot_max_power;
        // Accuracy/cadence fields
        tweaks.shot_max_variance = shooting.shot_max_variance;
        tweaks.shot_min_variance = shooting.shot_min_variance;
        tweaks.shot_air_variance_penalty = shooting.shot_air_variance_penalty;
        tweaks.shot_move_variance_penalty = shooting.shot_move_variance_penalty;
        tweaks.shot_quick_threshold = shooting.shot_quick_threshold;
        tweaks.quick_power_multiplier = shooting.quick_power_multiplier;
        tweaks.quick_power_threshold = shooting.quick_power_threshold;
        tweaks.speed_randomness_min = shooting.speed_randomness_min;
        tweaks.speed_randomness_max = shooting.speed_randomness_max;
        tweaks.shot_distance_variance = shooting.shot_distance_variance;
    }

    info!(
        "Applied presets: movement={}, ball={}, shooting={}",
        preset_db
            .get_movement(current.movement)
            .map(|p| p.name.as_str())
            .unwrap_or("?"),
        preset_db
            .get_ball(current.ball)
            .map(|p| p.name.as_str())
            .unwrap_or("?"),
        preset_db
            .get_shooting(current.shooting)
            .map(|p| p.name.as_str())
            .unwrap_or("?"),
    );
}

/// Apply a composite preset (sets all category indices and triggers apply)
pub fn apply_composite_preset(
    current: &mut CurrentPresets,
    preset_db: &PresetDatabase,
    composite_index: usize,
) {
    if let Some(composite) = preset_db.get_composite(composite_index) {
        // Find indices for each category by name
        for (i, m) in preset_db.movement.iter().enumerate() {
            if m.name == composite.movement {
                current.movement = i;
                break;
            }
        }
        for (i, b) in preset_db.ball.iter().enumerate() {
            if b.name == composite.ball {
                current.ball = i;
                break;
            }
        }
        for (i, s) in preset_db.shooting.iter().enumerate() {
            if s.name == composite.shooting {
                current.shooting = i;
                break;
            }
        }
        current.composite = composite_index;
        current.mark_apply();
    }
}
