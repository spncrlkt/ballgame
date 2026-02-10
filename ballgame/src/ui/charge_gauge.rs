//! Charge gauge UI components and systems

use bevy::prelude::*;

use crate::constants::*;
use crate::player::{HoldingBall, Player};
use crate::shooting::ChargingShot;
use crate::tuning::PhysicsTweaks;

/// Charge gauge outline component (black border)
#[derive(Component)]
pub struct ChargeGaugeOutline;

/// Charge gauge background component
#[derive(Component)]
pub struct ChargeGaugeBackground;

/// Charge gauge fill component
#[derive(Component)]
pub struct ChargeGaugeFill;

/// Update charge gauge display
pub fn update_charge_gauge(
    tweaks: Res<PhysicsTweaks>,
    player_query: Query<(&ChargingShot, &Children, Option<&HoldingBall>), With<Player>>,
    mut fill_query: Query<(&mut Sprite, &mut Transform), With<ChargeGaugeFill>>,
) {
    // Horizontal bar fill width
    let fill_width = CHARGE_GAUGE_WIDTH - 2.0;

    for (charging, children, holding) in &player_query {
        for child in children.iter() {
            // Update fill scale and color
            if let Ok((mut sprite, mut transform)) = fill_query.get_mut(child) {
                let charge_pct = (charging.charge_time / tweaks.shot_charge_time).min(1.0);

                // Only show fill when holding ball and charging
                if holding.is_none() || charging.charge_time < 0.001 {
                    // Not charging - hide the fill (scale to 0)
                    transform.scale.x = 0.0;
                } else {
                    // Charging - show fill scaled by percentage
                    transform.scale.x = charge_pct;

                    // Offset X so bar grows from left
                    // At 0%: bar is at left edge
                    // At 100%: bar is centered
                    let x_offset = -fill_width / 2.0 * (1.0 - charge_pct);
                    transform.translation.x = x_offset;

                    // Color transition: green (0%) -> red (100%)
                    let r = charge_pct * 0.9;
                    let g = (1.0 - charge_pct) * 0.8;
                    sprite.color = Color::srgb(r, g, 0.0);
                }
            }
        }
    }
}
