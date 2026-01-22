//! Scoring module - score tracking and check_scoring system

use bevy::prelude::*;

use crate::ball::{Ball, BallState, Velocity};
use crate::constants::*;
use crate::player::{HoldingBall, Player};
use crate::ui::ScoreFlash;
use crate::world::Basket;

/// Score resource tracking left/right team scores
#[derive(Resource, Default)]
pub struct Score {
    pub left: u32,  // Team scoring in LEFT basket
    pub right: u32, // Team scoring in RIGHT basket
}

/// Current level number (1-indexed)
#[derive(Resource)]
pub struct CurrentLevel(pub u32);

impl Default for CurrentLevel {
    fn default() -> Self {
        Self(1)
    }
}

/// Check if ball entered a basket and award points
pub fn check_scoring(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut ball_query: Query<(&mut Transform, &mut Velocity, &mut BallState, &Sprite), With<Ball>>,
    basket_query: Query<(Entity, &Transform, &Basket, &Sprite), Without<Ball>>,
    player_query: Query<(Entity, &Sprite), With<Player>>,
) {
    for (mut ball_transform, mut ball_velocity, mut ball_state, _ball_sprite) in &mut ball_query {
        let ball_pos = ball_transform.translation.truncate();
        let is_held = matches!(*ball_state, BallState::Held(_));

        for (basket_entity, basket_transform, basket, basket_sprite) in &basket_query {
            let basket_size = basket_sprite.custom_size.unwrap_or(BASKET_SIZE);
            let basket_pos = basket_transform.translation.truncate();
            let basket_half = basket_size / 2.0;

            // Check if ball center is inside basket
            let in_basket = ball_pos.x > basket_pos.x - basket_half.x
                && ball_pos.x < basket_pos.x + basket_half.x
                && ball_pos.y > basket_pos.y - basket_half.y
                && ball_pos.y < basket_pos.y + basket_half.y;

            if in_basket {
                // Determine points: 2 for carry-in, 1 for throw
                let points = if is_held { 2 } else { 1 };

                match basket {
                    Basket::Left => score.left += points,
                    Basket::Right => score.right += points,
                }

                // Flash the basket (gold/yellow for carry-in, white for throw)
                let flash_color = if is_held {
                    Color::srgb(1.0, 0.85, 0.0) // Gold for 2-point carry
                } else {
                    Color::srgb(1.0, 1.0, 1.0) // White for 1-point throw
                };
                commands.entity(basket_entity).insert(ScoreFlash {
                    timer: 0.6,
                    flash_color,
                    original_color: BASKET_COLOR,
                });

                // If held, also flash the player who scored
                if let BallState::Held(holder) = *ball_state {
                    if let Ok((player_entity, _player_sprite)) = player_query.get(holder) {
                        commands.entity(player_entity).insert(ScoreFlash {
                            timer: 0.6,
                            flash_color,
                            original_color: PLAYER_COLOR,
                        });
                        // Remove HoldingBall from the player
                        commands.entity(player_entity).remove::<HoldingBall>();
                    }
                }

                // Reset ball to center
                ball_transform.translation = BALL_SPAWN;
                ball_velocity.0 = Vec2::ZERO;
                *ball_state = BallState::Free;

                info!(
                    "SCORE {}pts! Left: {} Right: {}",
                    points, score.left, score.right
                );
            }
        }
    }
}
