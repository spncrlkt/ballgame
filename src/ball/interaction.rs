//! Ball-player interaction systems

use bevy::prelude::*;
use rand::Rng;

use crate::ai::{decision::defender_in_shot_path, InputState};
use crate::ball::components::*;
use crate::constants::*;
use crate::player::{Facing, HoldingBall, Player, Velocity};
use crate::shooting::ChargingShot;
use crate::steal::{StealContest, StealCooldown};

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
    mut player_query: Query<(Entity, &Transform, &mut Velocity, &Sprite), (With<Player>, Without<Ball>)>,
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

        // Get shooter entity if ball is in flight
        let shooter_entity = match ball_state {
            BallState::InFlight { shooter, .. } => Some(*shooter),
            _ => None,
        };

        for (player_entity, player_transform, mut player_velocity, player_sprite) in &mut player_query {
            let player_size = player_sprite.custom_size.unwrap_or(PLAYER_SIZE);
            let player_half = player_size / 2.0;
            let player_pos = player_transform.translation.truncate();

            let diff = ball_pos - player_pos;
            let overlap_x = ball_half.x + player_half.x - diff.x.abs();
            let overlap_y = ball_half.y + player_half.y - diff.y.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                is_overlapping = true;

                // Check if this is a defender blocking a shot
                // If the ball is in flight and this player is NOT the shooter,
                // check if they're in the shot path and reduce grace accordingly
                let is_blocking_defender = shooter_entity
                    .map(|shooter| shooter != player_entity)
                    .unwrap_or(false)
                    && defender_in_shot_path(
                        ball_pos,
                        ball_velocity.0,
                        player_pos,
                        PLAYER_SIZE.x * 1.5, // Blocking radius
                    );

                // Calculate effective grace - reduce if defender is blocking
                let effective_grace = if is_blocking_defender {
                    grace.0 * DEFENSE_GRACE_REDUCTION
                } else {
                    grace.0
                };

                // Only apply effects on first frame of contact, and skip if in grace period
                if !contact.overlapping && effective_grace <= 0.0 {
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

/// Handle ball pickup and instant steal attempts.
/// All players read from their InputState component.
pub fn pickup_ball(
    mut commands: Commands,
    mut steal_contest: ResMut<StealContest>,
    mut non_holding_players: Query<
        (
            Entity,
            &Transform,
            &mut ChargingShot,
            &mut InputState,
            &mut StealCooldown,
        ),
        (With<Player>, Without<HoldingBall>),
    >,
    mut holding_players: Query<
        (
            Entity,
            &Transform,
            &HoldingBall,
            &ChargingShot,
            &mut Velocity,
            &mut StealCooldown,
        ),
        With<Player>,
    >,
    mut ball_query: Query<(Entity, &Transform, &mut BallState), With<Ball>>,
) {
    // Check each non-holding player for pickup/steal attempts
    for (player_entity, player_transform, mut charging, mut input, mut cooldown) in
        &mut non_holding_players
    {
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

        // Skip steal attempts if on cooldown
        if cooldown.0 > 0.0 {
            continue;
        }

        // If no free ball nearby, check for steal opportunity
        for (
            defender_entity,
            defender_transform,
            holding,
            defender_charging,
            mut defender_velocity,
            mut defender_cooldown,
        ) in &mut holding_players
        {
            let distance = player_pos.distance(defender_transform.translation.truncate());

            if distance < STEAL_RANGE {
                // Instant steal attempt - calculate success chance
                let mut success_chance = STEAL_SUCCESS_CHANCE;

                // Bonus if defender is charging a shot
                if defender_charging.charge_time > 0.0 {
                    success_chance += STEAL_CHARGING_BONUS;
                }

                // Roll for success
                let mut rng = rand::thread_rng();
                let roll: f32 = rng.gen_range(0.0..1.0);

                if roll < success_chance {
                    // Steal succeeded! Transfer ball
                    let ball_entity = holding.0;
                    if let Ok((_, _, mut ball_state)) = ball_query.get_mut(ball_entity) {
                        *ball_state = BallState::Held(player_entity);
                        commands.entity(defender_entity).remove::<HoldingBall>();
                        commands
                            .entity(player_entity)
                            .insert(HoldingBall(ball_entity));

                        // Apply pushback to defender (away from attacker)
                        let pushback_dir = if defender_transform.translation.x >= player_pos.x {
                            1.0
                        } else {
                            -1.0
                        };
                        defender_velocity.0.x += pushback_dir * STEAL_PUSHBACK_STRENGTH;
                        defender_velocity.0.y += STEAL_PUSHBACK_STRENGTH * 0.3; // Small upward nudge

                        // Apply no-stealback cooldown to victim
                        defender_cooldown.0 = STEAL_VICTIM_COOLDOWN;
                    }
                } else {
                    // Steal failed - set fail flash
                    steal_contest.last_attempt_failed = true;
                    steal_contest.fail_flash_timer = STEAL_FAIL_FLASH_DURATION;
                    steal_contest.fail_flash_entity = Some(player_entity);
                }

                // Apply cooldown to attacker regardless of success
                cooldown.0 = STEAL_COOLDOWN;
                return;
            }
        }
    }
}
