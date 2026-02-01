//! Charge gauge UI components and systems

use bevy::prelude::*;

use crate::constants::*;
use crate::player::{Facing, HoldingBall, Player};
use crate::shooting::ChargingShot;
use crate::tuning::PhysicsTweaks;

/// Charge gauge background component
#[derive(Component)]
pub struct ChargeGaugeBackground;

/// Charge gauge fill component
#[derive(Component)]
pub struct ChargeGaugeFill;

/// Update charge gauge display
pub fn update_charge_gauge(
    tweaks: Res<PhysicsTweaks>,
    player_query: Query<(&ChargingShot, &Facing, &Children, Option<&HoldingBall>), With<Player>>,
    mut bg_query: Query<&mut Transform, (With<ChargeGaugeBackground>, Without<ChargeGaugeFill>)>,
    mut fill_query: Query<(&mut Sprite, &mut Transform), With<ChargeGaugeFill>>,
) {
    // Gauge inside player, opposite side of ball
    let fill_height = CHARGE_GAUGE_HEIGHT - 2.0;

    for (charging, facing, children, holding) in &player_query {
        // Gauge is inside player, opposite side of facing (ball is on facing side)
        let gauge_x = -facing.0 * (PLAYER_SIZE.x / 4.0);

        for child in children.iter() {
            // Update background position
            if let Ok(mut bg_transform) = bg_query.get_mut(child) {
                bg_transform.translation.x = gauge_x;
            }

            // Update fill position, scale, and color
            if let Ok((mut sprite, mut transform)) = fill_query.get_mut(child) {
                transform.translation.x = gauge_x;

                let charge_pct = (charging.charge_time / tweaks.shot_charge_time).min(1.0);

                // Only show fill when holding ball and charging
                if holding.is_none() || charging.charge_time < 0.001 {
                    // Not charging - hide the fill (scale to 0)
                    transform.scale.y = 0.0;
                } else {
                    // Charging - show fill scaled by percentage
                    transform.scale.y = charge_pct;

                    // Offset Y so bar grows from bottom
                    // At 0%: bar is at bottom (y = -height/2 + 0)
                    // At 100%: bar is centered (y = 0)
                    let y_offset = -fill_height / 2.0 * (1.0 - charge_pct);
                    transform.translation.y = y_offset;

                    // Color transition: green (0%) -> red (100%)
                    let r = charge_pct * 0.9;
                    let g = (1.0 - charge_pct) * 0.8;
                    sprite.color = Color::srgb(r, g, 0.0);
                }
            }
        }
    }
}
