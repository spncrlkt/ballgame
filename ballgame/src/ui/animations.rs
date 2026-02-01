//! Animation systems for score flash and ball pulse

use bevy::prelude::*;

use crate::ball::{Ball, BallPulse, BallState};
use crate::constants::{BALL_PICKUP_RADIUS, BALL_SIZE};
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
/// With texture, sprite.color tints the texture (white = normal, other colors = tinted)
pub fn animate_pickable_ball(
    time: Res<Time>,
    players: Query<(&Transform, Option<&HoldingBall>), With<Player>>,
    mut ball_query: Query<(&Transform, &BallState, &mut Sprite, &mut BallPulse), With<Ball>>,
) {
    for (ball_transform, ball_state, mut sprite, mut pulse) in &mut ball_query {
        // Only pulse if ball is Free
        if *ball_state != BallState::Free {
            // Reset to normal when not free (white = no tint)
            sprite.custom_size = Some(BALL_SIZE);
            sprite.color = Color::WHITE;
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
            pulse.timer += time.delta_secs();
            let t = pulse.timer * 5.0 * std::f32::consts::TAU;
            let pulse_factor = (t.cos() + 1.0) / 2.0; // 0 to 1

            // Size pulse: 100% to 103%
            let scale = 1.0 + 0.03 * pulse_factor;
            sprite.custom_size = Some(BALL_SIZE * scale);

            // Gold tint pulse (white → gold → white)
            // Gold tint reduces green and blue to create warm golden glow
            sprite.color = Color::srgb(
                1.0,
                1.0 - 0.15 * pulse_factor, // slightly reduce green
                1.0 - 0.4 * pulse_factor,  // reduce blue more → gold tint
            );
        } else {
            // Reset to normal (white = no tint)
            sprite.custom_size = Some(BALL_SIZE);
            sprite.color = Color::WHITE;
            pulse.timer = 0.0;
        }
    }
}
