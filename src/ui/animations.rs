//! Animation systems for score flash and ball pulse

use bevy::prelude::*;

use crate::ball::{Ball, BallPulse, BallState};
use crate::constants::*;
use crate::player::{HoldingBall, Player};

/// Score flash animation component
#[derive(Component)]
pub struct ScoreFlash {
    pub timer: f32,            // Time remaining in flash
    pub flash_color: Color,    // Color to flash to
    pub original_color: Color, // Color to restore after flash
}

/// Animate score flash on baskets/players
pub fn animate_score_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut ScoreFlash)>,
) {
    for (entity, mut sprite, mut flash) in &mut query {
        flash.timer -= time.delta_secs();

        if flash.timer <= 0.0 {
            // Flash complete - restore original color
            sprite.color = flash.original_color;
            commands.entity(entity).remove::<ScoreFlash>();
        } else {
            // Fast flicker between flash color and original
            let t = (flash.timer * 25.0).sin(); // ~4 flashes per 0.6 seconds
            let blend = (t + 1.0) / 2.0; // 0 to 1

            // Extract RGB from both colors and interpolate
            let flash_rgba = flash.flash_color.to_srgba();
            let orig_rgba = flash.original_color.to_srgba();

            sprite.color = Color::srgb(
                orig_rgba.red + (flash_rgba.red - orig_rgba.red) * blend,
                orig_rgba.green + (flash_rgba.green - orig_rgba.green) * blend,
                orig_rgba.blue + (flash_rgba.blue - orig_rgba.blue) * blend,
            );
        }
    }
}

/// Animate pickable ball (pulse when near player)
pub fn animate_pickable_ball(
    time: Res<Time>,
    players: Query<(&Transform, Option<&HoldingBall>), With<Player>>,
    mut ball_query: Query<(&Transform, &BallState, &mut Sprite, &mut BallPulse), With<Ball>>,
) {
    for (ball_transform, ball_state, mut sprite, mut pulse) in &mut ball_query {
        // Only pulse if ball is Free
        if *ball_state != BallState::Free {
            // Reset to normal when not free
            sprite.custom_size = Some(BALL_SIZE);
            sprite.color = BALL_COLOR;
            pulse.timer = 0.0;
            continue;
        }

        let ball_pos = ball_transform.translation.truncate();

        // Check if any player without a ball is close enough to pick up
        let mut can_pickup = false;
        for (player_transform, holding) in &players {
            if holding.is_some() {
                continue; // Player already has a ball
            }
            let distance = ball_pos.distance(player_transform.translation.truncate());
            if distance < BALL_PICKUP_RADIUS {
                can_pickup = true;
                break;
            }
        }

        if can_pickup {
            // Animate pulse - 5 cycles per second
            // Pattern: dark -> regular -> light -> regular
            pulse.timer += time.delta_secs();
            let t = pulse.timer * 5.0 * std::f32::consts::TAU;
            let pulse_factor = -(t.cos()); // -1 (dark) -> 0 (regular) -> 1 (light) -> 0 (regular)

            // Size: pulse between 97% and 103% (subtle)
            let scale_factor = 1.0 + 0.03 * pulse_factor;
            sprite.custom_size = Some(BALL_SIZE * scale_factor);

            // Color interpolation: dark orange <-> regular orange <-> light orange-cyan mix
            // Regular orange: (0.9, 0.5, 0.1)
            // Dark orange: (0.5, 0.25, 0.05)
            // Light (orange + cyan-white): (0.95, 0.75, 0.55)
            let (r, g, b) = if pulse_factor < 0.0 {
                // Dark to regular (pulse_factor: -1 to 0)
                let blend = pulse_factor + 1.0; // 0 to 1
                (
                    0.5 + 0.4 * blend,   // 0.5 -> 0.9
                    0.25 + 0.25 * blend, // 0.25 -> 0.5
                    0.05 + 0.05 * blend, // 0.05 -> 0.1
                )
            } else {
                // Regular to light (pulse_factor: 0 to 1)
                let blend = pulse_factor; // 0 to 1
                (
                    0.9 + 0.05 * blend, // 0.9 -> 0.95
                    0.5 + 0.25 * blend, // 0.5 -> 0.75
                    0.1 + 0.45 * blend, // 0.1 -> 0.55
                )
            };
            sprite.color = Color::srgb(r, g, b);
        } else {
            // Reset to normal
            sprite.custom_size = Some(BALL_SIZE);
            sprite.color = BALL_COLOR;
            pulse.timer = 0.0;
        }
    }
}
