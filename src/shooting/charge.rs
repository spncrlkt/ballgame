//! Shot charging components and systems

use bevy::prelude::*;

use crate::ai::AiInput;
use crate::player::Player;

/// Charge time accumulator
#[derive(Component, Default)]
pub struct ChargingShot {
    pub charge_time: f32, // How long throw button has been held
}

/// Information about the last shot taken (for debug display)
#[derive(Resource, Default)]
pub struct LastShotInfo {
    pub angle_degrees: f32,
    pub speed: f32,
    pub base_variance: f32,
    pub air_penalty: f32,
    pub move_penalty: f32,
    pub distance_variance: f32,
    pub required_speed: f32,
    pub total_variance: f32,
    pub target: Option<crate::world::Basket>,
}

/// Update shot charge while throw button is held.
/// All players read from their AiInput component.
pub fn update_shot_charge(
    time: Res<Time>,
    mut player_query: Query<(&mut ChargingShot, &AiInput), With<Player>>,
) {
    for (mut charging, input) in &mut player_query {
        if input.throw_held {
            charging.charge_time += time.delta_secs();
        }
        // Don't reset here - let throw_ball reset after using the charge
        // Otherwise charge resets to 0 before throw_ball can read it
    }
}
