//! AI decision system - updates AiInput based on game state

use bevy::prelude::*;
use rand::Rng;

use crate::ai::{AiGoal, AiInput, AiState};
use crate::ball::{Ball, BallState};
use crate::constants::*;
use crate::player::{HoldingBall, HumanControlled, Player, TargetBasket, Team};
use crate::world::Basket;

/// Update AI input based on decision making.
/// Only processes players WITHOUT HumanControlled marker.
/// Runs in Update schedule after capture_input.
pub fn ai_decision_update(
    time: Res<Time>,
    mut ai_query: Query<
        (
            Entity,
            &Transform,
            &Team,
            &mut AiInput,
            &mut AiState,
            &TargetBasket,
            Option<&HoldingBall>,
        ),
        (With<Player>, Without<HumanControlled>),
    >,
    all_players: Query<
        (
            Entity,
            &Transform,
            Option<&HoldingBall>,
            Option<&HumanControlled>,
        ),
        With<Player>,
    >,
    ball_query: Query<(&Transform, &BallState), With<Ball>>,
    basket_query: Query<(&Transform, &Basket)>,
) {
    for (ai_entity, ai_transform, team, mut input, mut ai_state, target_basket, holding) in
        &mut ai_query
    {
        // Idle goal: do nothing, skip all AI logic
        if ai_state.current_goal == AiGoal::Idle {
            input.move_x = 0.0;
            input.jump_held = false;
            input.pickup_pressed = false;
            input.throw_held = false;
            input.throw_released = false;
            continue;
        }

        let ai_pos = ai_transform.translation.truncate();

        // Get ball info
        let (ball_transform, ball_state) = match ball_query.iter().next() {
            Some(b) => b,
            None => continue,
        };
        let ball_pos = ball_transform.translation.truncate();

        // Check if AI is holding the ball
        let ai_has_ball = holding.is_some();

        // Check if opponent (human-controlled player) has ball
        let opponent_has_ball = all_players
            .iter()
            .filter(|(e, _, _, human)| *e != ai_entity && human.is_some())
            .any(|(_, _, holding, _)| holding.is_some());

        // Find opponent position (for defense/steal decisions)
        let opponent_pos = all_players
            .iter()
            .find(|(e, _, _, human)| *e != ai_entity && human.is_some())
            .map(|(_, t, _, _)| t.translation.truncate());

        // Determine the target basket position based on team
        let target_basket_type = target_basket.0;
        let target_basket_pos = basket_query
            .iter()
            .find(|(_, b)| **b == target_basket_type)
            .map(|(t, _)| t.translation.truncate())
            .unwrap_or(Vec2::new(-600.0, 0.0));

        // Defensive position (near own basket based on team)
        let defensive_pos = match *team {
            Team::Left => Vec2::new(-400.0, ARENA_FLOOR_Y + 50.0),
            Team::Right => Vec2::new(400.0, ARENA_FLOOR_Y + 50.0),
        };

        // Decide current goal
        let new_goal = if ai_has_ball {
            let distance_to_basket = ai_pos.distance(target_basket_pos);
            if distance_to_basket < AI_SHOOT_RANGE {
                AiGoal::ChargeShot
            } else {
                AiGoal::AttackWithBall
            }
        } else if opponent_has_ball {
            if let Some(opp_pos) = opponent_pos {
                let distance_to_opponent = ai_pos.distance(opp_pos);
                if distance_to_opponent < AI_STEAL_RANGE {
                    AiGoal::AttemptSteal
                } else {
                    AiGoal::ReturnToDefense
                }
            } else {
                AiGoal::ReturnToDefense
            }
        } else {
            // Ball is free
            AiGoal::ChaseBall
        };

        // Update goal (and randomize charge target when starting to charge)
        if new_goal != ai_state.current_goal {
            ai_state.current_goal = new_goal;
            if new_goal == AiGoal::ChargeShot {
                let mut rng = rand::thread_rng();
                ai_state.shot_charge_target = rng.gen_range(AI_CHARGE_TIME_MIN..AI_CHARGE_TIME_MAX);
            }
        }

        // Reset inputs each frame (will be set below)
        input.move_x = 0.0;
        // Don't reset jump_buffer_timer - it counts down
        // Don't reset throw_released - it's consumed by throw system

        // Execute behavior based on goal
        match ai_state.current_goal {
            AiGoal::Idle => {
                // Handled above with early continue, but needed for exhaustive match
                unreachable!();
            }

            AiGoal::ChaseBall => {
                // Move toward ball
                let dx = ball_pos.x - ai_pos.x;
                if dx.abs() > AI_POSITION_TOLERANCE {
                    input.move_x = dx.signum();
                }

                // Jump if ball is above us
                let dy = ball_pos.y - ai_pos.y;
                if dy > PLAYER_SIZE.y && dx.abs() < BALL_PICKUP_RADIUS * 2.0 {
                    input.jump_buffer_timer = JUMP_BUFFER_TIME;
                    input.jump_held = true;
                } else {
                    input.jump_held = false;
                }

                // Try to pick up ball when close
                let distance_to_ball = ai_pos.distance(ball_pos);
                input.pickup_pressed =
                    distance_to_ball < BALL_PICKUP_RADIUS && matches!(ball_state, BallState::Free);

                input.throw_held = false;
            }

            AiGoal::AttackWithBall => {
                // Move toward target basket
                let dx = target_basket_pos.x - ai_pos.x;
                if dx.abs() > AI_POSITION_TOLERANCE {
                    input.move_x = dx.signum();
                }

                input.jump_held = false;
                input.pickup_pressed = false;
                input.throw_held = false;
            }

            AiGoal::ChargeShot => {
                // Stop moving, charge shot
                input.move_x = 0.0;
                input.jump_held = false;
                input.pickup_pressed = false;

                // Hold throw button until charge target reached
                // The actual charge time is tracked by ChargingShot component
                // We track a separate timer here for when to release
                if !input.throw_held && !input.throw_released {
                    // Start charging
                    input.throw_held = true;
                    ai_state.shot_charge_target =
                        rand::thread_rng().gen_range(AI_CHARGE_TIME_MIN..AI_CHARGE_TIME_MAX);
                } else if input.throw_held {
                    // Count down charge target
                    ai_state.shot_charge_target -= time.delta_secs();
                    if ai_state.shot_charge_target <= 0.0 {
                        // Release shot
                        input.throw_held = false;
                        input.throw_released = true;
                    }
                }
            }

            AiGoal::AttemptSteal => {
                // Move toward opponent
                if let Some(opp_pos) = opponent_pos {
                    let dx = opp_pos.x - ai_pos.x;
                    if dx.abs() > AI_POSITION_TOLERANCE {
                        input.move_x = dx.signum();
                    }

                    // Attempt steal when close
                    let distance = ai_pos.distance(opp_pos);
                    input.pickup_pressed = distance < STEAL_RANGE;
                }

                input.jump_held = false;
                input.throw_held = false;
            }

            AiGoal::ReturnToDefense => {
                // Move toward defensive position
                let dx = defensive_pos.x - ai_pos.x;
                if dx.abs() > AI_POSITION_TOLERANCE {
                    input.move_x = dx.signum();
                }

                // Jump up to platform if needed
                let dy = defensive_pos.y - ai_pos.y;
                if dy > PLAYER_SIZE.y * 0.5 {
                    input.jump_buffer_timer = JUMP_BUFFER_TIME;
                    input.jump_held = true;
                } else {
                    input.jump_held = false;
                }

                input.pickup_pressed = false;
                input.throw_held = false;
            }
        }

        // Decay jump buffer timer
        input.jump_buffer_timer = (input.jump_buffer_timer - time.delta_secs()).max(0.0);
    }
}
