//! Ball-player interaction systems

use bevy::prelude::*;

use crate::ai::AiInput;
use crate::ball::components::*;
use crate::constants::*;
use crate::player::{Facing, HoldingBall, Player, Velocity};
use crate::shooting::ChargingShot;
use crate::steal::StealContest;

/// Handle ball-player collision physics
pub fn ball_player_collision(
    mut ball_query: Query<
        (
            &Transform,
            &mut Velocity,
            &mut BallPlayerContact,
            &BallState,
            &Sprite,
            &mut BallRolling,
            &BallShotGrace,
        ),
        With<Ball>,
    >,
    mut player_query: Query<(&Transform, &mut Velocity, &Sprite), (With<Player>, Without<Ball>)>,
) {
    for (
        ball_transform,
        mut ball_velocity,
        mut contact,
        ball_state,
        ball_sprite,
        mut rolling,
        grace,
    ) in &mut ball_query
    {
        // Skip held balls
        if matches!(ball_state, BallState::Held(_)) {
            contact.overlapping = false;
            continue;
        }

        let ball_size = ball_sprite.custom_size.unwrap_or(BALL_SIZE);
        let ball_half = ball_size / 2.0;
        let ball_pos = ball_transform.translation.truncate();

        let mut is_overlapping = false;

        for (player_transform, mut player_velocity, player_sprite) in &mut player_query {
            let player_size = player_sprite.custom_size.unwrap_or(PLAYER_SIZE);
            let player_half = player_size / 2.0;
            let player_pos = player_transform.translation.truncate();

            let diff = ball_pos - player_pos;
            let overlap_x = ball_half.x + player_half.x - diff.x.abs();
            let overlap_y = ball_half.y + player_half.y - diff.y.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                is_overlapping = true;

                // Only apply effects on first frame of contact, and skip if in grace period
                if !contact.overlapping && grace.0 <= 0.0 {
                    let ball_speed = ball_velocity.0.length();
                    let player_speed = player_velocity.0.length();

                    // Both slow down when passing through each other
                    // Ball has higher Y friction (gravity effect) than X
                    ball_velocity.0.x *= BALL_PLAYER_DRAG_X;
                    ball_velocity.0.y *= BALL_PLAYER_DRAG_Y;
                    player_velocity.0 *= BALL_PLAYER_DRAG_X;

                    // If ball is slow/stationary and player is moving, kick the ball
                    if ball_speed < BALL_KICK_THRESHOLD && player_speed > BALL_KICK_THRESHOLD {
                        let kick_dir = if player_velocity.0.x > 0.0 { 1.0 } else { -1.0 };
                        ball_velocity.0.x += kick_dir * BALL_KICK_STRENGTH;
                        ball_velocity.0.y += BALL_KICK_STRENGTH * 0.3; // Small upward nudge
                        rolling.0 = false; // Ball is kicked into the air
                    }
                }
            }
        }

        contact.overlapping = is_overlapping;
    }
}

/// Make ball follow holder
pub fn ball_follow_holder(
    mut ball_query: Query<(&mut Transform, &BallState), With<Ball>>,
    player_query: Query<(&Transform, &Facing), (With<Player>, Without<Ball>)>,
) {
    for (mut ball_transform, state) in &mut ball_query {
        if let BallState::Held(holder_entity) = state {
            if let Ok((player_transform, facing)) = player_query.get(*holder_entity) {
                // Position ball inside player, on facing side, at middle height
                ball_transform.translation.x =
                    player_transform.translation.x + facing.0 * (PLAYER_SIZE.x / 4.0);
                ball_transform.translation.y = player_transform.translation.y; // Center height
            }
        }
    }
}

/// Handle ball pickup.
/// All players read from their AiInput component.
pub fn pickup_ball(
    mut commands: Commands,
    mut steal_contest: ResMut<StealContest>,
    mut non_holding_players: Query<
        (Entity, &Transform, &mut ChargingShot, &mut AiInput),
        (With<Player>, Without<HoldingBall>),
    >,
    mut holding_players: Query<(Entity, &Transform, &HoldingBall, &mut AiInput), With<Player>>,
    mut ball_query: Query<(Entity, &Transform, &mut BallState), With<Ball>>,
) {
    // If steal contest is active, count presses from both players
    if steal_contest.active {
        // Count presses for attacker
        for (entity, _, _, mut input) in &mut non_holding_players {
            if input.pickup_pressed {
                input.pickup_pressed = false;
                if steal_contest.attacker == Some(entity) {
                    steal_contest.attacker_presses += 1;
                }
            }
        }

        // Count presses for defender
        for (entity, _, _, mut input) in &mut holding_players {
            if input.pickup_pressed {
                input.pickup_pressed = false;
                if steal_contest.defender == Some(entity) {
                    steal_contest.defender_presses += 1;
                }
            }
        }

        return; // Don't allow pickup during contest
    }

    // Check each non-holding player for pickup/steal attempts
    for (player_entity, player_transform, mut charging, mut input) in &mut non_holding_players {
        if !input.pickup_pressed {
            continue;
        }

        // Consume the input
        input.pickup_pressed = false;

        let player_pos = player_transform.translation.truncate();

        // First, try to pick up a free ball
        let mut picked_up = false;
        for (ball_entity, ball_transform, mut ball_state) in &mut ball_query {
            if *ball_state != BallState::Free {
                continue;
            }

            let distance = player_pos.distance(ball_transform.translation.truncate());

            if distance < BALL_PICKUP_RADIUS {
                *ball_state = BallState::Held(player_entity);
                commands
                    .entity(player_entity)
                    .insert(HoldingBall(ball_entity));
                // Reset charge so it starts fresh (even if throw button is held)
                charging.charge_time = 0.0;
                picked_up = true;
                break;
            }
        }

        if picked_up {
            return; // Done - picked up ball
        }

        // If no free ball nearby, check for steal opportunity
        for (defender_entity, defender_transform, _holding, _) in &holding_players {
            let distance = player_pos.distance(defender_transform.translation.truncate());

            if distance < STEAL_RANGE {
                // Initiate steal contest
                steal_contest.active = true;
                steal_contest.attacker = Some(player_entity);
                steal_contest.defender = Some(defender_entity);
                steal_contest.attacker_presses = 1; // Count the initiating press
                steal_contest.defender_presses = STEAL_DEFENDER_ADVANTAGE;
                steal_contest.timer = STEAL_CONTEST_DURATION;
                return;
            }
        }
    }
}

