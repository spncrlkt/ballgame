//! AI decision system - updates InputState based on game state

use bevy::prelude::*;
use rand::Rng;

use crate::ai::{
    AiGoal, AiNavState, AiProfileDatabase, AiState, InputState, NavAction, NavGraph, find_path,
    find_path_to_shoot, shot_quality::evaluate_shot_quality,
};
use crate::ball::{Ball, BallState};
use crate::constants::*;
use crate::player::{Grounded, HoldingBall, HumanControlled, Player, TargetBasket, Team};
use crate::world::Basket;

/// Update AI navigation paths based on current goals.
/// Runs before ai_decision_update to set up paths that the decision system will execute.
pub fn ai_navigation_update(
    nav_graph: Res<NavGraph>,
    profile_db: Res<AiProfileDatabase>,
    mut ai_query: Query<
        (
            Entity,
            &Transform,
            &Team,
            &AiState,
            &mut AiNavState,
            &TargetBasket,
            Option<&HoldingBall>,
            &Grounded,
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
    // Skip if nav graph not built
    if nav_graph.nodes.is_empty() {
        return;
    }

    for (ai_entity, ai_transform, team, ai_state, mut nav_state, target_basket, _holding, _grounded) in
        &mut ai_query
    {
        // Skip Idle AI
        if ai_state.current_goal == AiGoal::Idle {
            nav_state.clear();
            continue;
        }

        let profile = profile_db.get(ai_state.profile_index);
        let ai_pos = ai_transform.translation.truncate();

        // Get ball position
        let ball_pos = ball_query
            .iter()
            .next()
            .map(|(t, _)| t.translation.truncate());

        // Get target basket position
        let target_basket_pos = basket_query
            .iter()
            .find(|(_, b)| **b == target_basket.0)
            .map(|(t, _)| t.translation.truncate());

        // Calculate defensive position
        let defensive_pos = match *team {
            Team::Left => Vec2::new(-profile.defense_offset, ARENA_FLOOR_Y + 50.0),
            Team::Right => Vec2::new(profile.defense_offset, ARENA_FLOOR_Y + 50.0),
        };

        // Find opponent position
        let opponent_pos = all_players
            .iter()
            .find(|(e, _, _, human)| *e != ai_entity && human.is_some())
            .map(|(_, t, _, _)| t.translation.truncate());

        // Determine navigation target based on goal
        let desired_target: Option<Vec2> = match ai_state.current_goal {
            AiGoal::Idle => None,

            AiGoal::ChaseBall => ball_pos,

            AiGoal::AttackWithBall => {
                // Navigate to a position within shooting range of basket
                if let Some(basket_pos) = target_basket_pos {
                    // Try to find a shooting position (closest platform to basket)
                    if let Some(path_result) =
                        find_path_to_shoot(&nav_graph, ai_pos, basket_pos, profile.shoot_range)
                    {
                        Some(nav_graph.nodes[path_result.goal_node].center)
                    } else {
                        // Fallback: just move toward basket
                        Some(basket_pos)
                    }
                } else {
                    None
                }
            }

            AiGoal::ChargeShot => {
                // Don't navigate while charging - stay put
                None
            }

            AiGoal::AttemptSteal => opponent_pos,

            AiGoal::ReturnToDefense => Some(defensive_pos),
        };

        // Check if we need to update the path
        let needs_path_update = if let Some(target) = desired_target {
            if nav_state.nav_target.is_none() {
                true // No path yet
            } else if let Some(current_target) = nav_state.nav_target {
                // Recalc if target moved significantly
                current_target.distance(target) > NAV_PATH_RECALC_DISTANCE
            } else {
                true
            }
        } else {
            false
        };

        // Check if current path is still valid
        let path_invalid = nav_state.path_complete()
            || (!nav_state.current_path.is_empty() && !nav_state.active);

        // Update path if needed
        if needs_path_update || path_invalid {
            if let Some(target) = desired_target {
                // Check if target requires navigation (different Y level or far away)
                let height_diff = (target.y - ai_pos.y).abs();
                let horizontal_dist = (target.x - ai_pos.x).abs();

                // Use navigation if:
                // 1. Target is significantly higher (need to jump to platform)
                // 2. Target is significantly lower (need to drop)
                // 3. Target is far away and we're not on the same platform
                let needs_navigation = height_diff > PLAYER_SIZE.y * 0.75
                    || (horizontal_dist > profile.position_tolerance
                        && height_diff > NAV_POSITION_TOLERANCE);

                if needs_navigation {
                    if let Some(path_result) = find_path(&nav_graph, ai_pos, target) {
                        nav_state.set_path(path_result.actions, target);
                    } else {
                        // No path found - clear and let simple movement take over
                        nav_state.clear();
                    }
                } else {
                    // Target reachable by simple walking
                    nav_state.clear();
                }
            } else {
                nav_state.clear();
            }
        }
    }
}

/// Update AI input based on decision making.
/// Only processes players WITHOUT HumanControlled marker.
/// Runs in Update schedule after capture_input and ai_navigation_update.
pub fn ai_decision_update(
    time: Res<Time>,
    profile_db: Res<AiProfileDatabase>,
    mut ai_query: Query<
        (
            Entity,
            &Transform,
            &Team,
            &mut InputState,
            &mut AiState,
            &mut AiNavState,
            &TargetBasket,
            Option<&HoldingBall>,
            &Grounded,
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
    for (
        ai_entity,
        ai_transform,
        team,
        mut input,
        mut ai_state,
        mut nav_state,
        target_basket,
        holding,
        grounded,
    ) in &mut ai_query
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

        // Get AI profile for this player
        let profile = profile_db.get(ai_state.profile_index);

        let ai_pos = ai_transform.translation.truncate();

        // Get ball info
        let (ball_transform, ball_state) = match ball_query.iter().next() {
            Some(b) => b,
            None => continue,
        };
        let ball_pos = ball_transform.translation.truncate();

        // Check if AI is holding the ball
        let ai_has_ball = holding.is_some();

        // Check if opponent (any other player) has ball
        let opponent_has_ball = all_players
            .iter()
            .filter(|(e, _, _, _)| *e != ai_entity)
            .any(|(_, _, h, _)| h.is_some());

        // Find opponent position (for defense/steal decisions)
        let opponent_pos = all_players
            .iter()
            .find(|(e, _, _, _)| *e != ai_entity)
            .map(|(_, t, _, _)| t.translation.truncate());

        // Determine the target basket position based on team
        let target_basket_type = target_basket.0;
        let target_basket_pos = basket_query
            .iter()
            .find(|(_, b)| **b == target_basket_type)
            .map(|(t, _)| t.translation.truncate())
            .unwrap_or(Vec2::new(-600.0, 0.0));

        // Defensive position (near own basket based on team, offset from profile)
        let defensive_pos = match *team {
            Team::Left => Vec2::new(-profile.defense_offset, ARENA_FLOOR_Y + 50.0),
            Team::Right => Vec2::new(profile.defense_offset, ARENA_FLOOR_Y + 50.0),
        };

        // Decide current goal (using profile values)
        let new_goal = if ai_has_ball {
            let horizontal_distance = (ai_pos.x - target_basket_pos.x).abs();
            let basket_is_elevated = target_basket_pos.y > ai_pos.y + PLAYER_SIZE.y;

            // For elevated baskets, use horizontal distance for range check
            // (vertical distance doesn't matter - we're shooting UP)
            // For baskets at same level, use full 2D distance
            let effective_distance = if basket_is_elevated {
                horizontal_distance
            } else {
                ai_pos.distance(target_basket_pos)
            };

            // Evaluate shot quality based on position (heatmap-derived)
            let shot_quality = evaluate_shot_quality(ai_pos, target_basket_pos);
            let quality_acceptable = shot_quality >= profile.min_shot_quality;

            // Shoot if within range OR if we've reached our nav target (best position)
            let at_nav_target = nav_state
                .nav_target
                .map(|t| ai_pos.distance(t) < NAV_POSITION_TOLERANCE * 2.0)
                .unwrap_or(false);
            let nav_complete = nav_state.path_complete() || !nav_state.active;

            // Position-based conditions to consider shooting
            let in_shoot_range = effective_distance < profile.shoot_range;
            let reached_target = at_nav_target && nav_complete;

            // Only shoot if shot quality is acceptable AND position conditions are met
            // This prevents shooting from bad positions even if we're "in range"
            if quality_acceptable && (in_shoot_range || reached_target) {
                AiGoal::ChargeShot
            } else {
                AiGoal::AttackWithBall
            }
        } else if opponent_has_ball {
            if let Some(opp_pos) = opponent_pos {
                let distance_to_opponent = ai_pos.distance(opp_pos);
                if distance_to_opponent < profile.steal_range {
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
            // DON'T clear navigation on goal change - only clear when destination changes
            // (handled by ai_navigation_update based on NAV_PATH_RECALC_DISTANCE)
            // Reset jump shot state when goal changes
            ai_state.jump_shot_active = false;
            ai_state.jump_shot_timer = 0.0;
            if new_goal == AiGoal::ChargeShot {
                let mut rng = rand::thread_rng();
                ai_state.shot_charge_target = rng.gen_range(profile.charge_min..profile.charge_max);
            }
        }

        // Reset inputs each frame (will be set below)
        input.move_x = 0.0;
        input.jump_held = false;
        // Don't reset jump_buffer_timer - it counts down
        // Don't reset throw_released - it's consumed by throw system

        // Check if navigation is controlling movement
        let nav_controlling = nav_state.active && !nav_state.path_complete();

        if nav_controlling {
            // Execute navigation actions
            execute_nav_action(&mut input, &mut nav_state, ai_pos, grounded.0, &time);
            // Auto-clear navigation when path completes
            nav_state.update_completion();
        } else {
            // Execute behavior based on goal (simple movement fallback)
            match ai_state.current_goal {
                AiGoal::Idle => {
                    // Handled above with early continue, but needed for exhaustive match
                    unreachable!();
                }

                AiGoal::ChaseBall => {
                    // Move toward ball
                    let dx = ball_pos.x - ai_pos.x;
                    if dx.abs() > profile.position_tolerance {
                        input.move_x = dx.signum();
                    }

                    // Jump if ball is above us and we're close horizontally
                    let dy = ball_pos.y - ai_pos.y;
                    if dy > PLAYER_SIZE.y && dx.abs() < BALL_PICKUP_RADIUS * 2.0 {
                        input.jump_buffer_timer = JUMP_BUFFER_TIME;
                        input.jump_held = true;
                    }

                    // Try to pick up ball when close
                    let distance_to_ball = ai_pos.distance(ball_pos);
                    input.pickup_pressed = distance_to_ball < BALL_PICKUP_RADIUS
                        && matches!(ball_state, BallState::Free);

                    input.throw_held = false;
                }

                AiGoal::AttackWithBall => {
                    // Move toward target basket (simple horizontal movement)
                    let dx = target_basket_pos.x - ai_pos.x;
                    if dx.abs() > profile.position_tolerance {
                        input.move_x = dx.signum();
                    }

                    input.pickup_pressed = false;
                    input.throw_held = false;
                }

                AiGoal::ChargeShot => {
                    input.pickup_pressed = false;

                    // Check if we should do a jump shot (basket is above us)
                    let height_to_basket = target_basket_pos.y - ai_pos.y;
                    let should_jump_shot = height_to_basket > PLAYER_SIZE.y;

                    if should_jump_shot && grounded.0 && !ai_state.jump_shot_active {
                        // Start jump shot sequence - jump first
                        ai_state.jump_shot_active = true;
                        ai_state.jump_shot_timer = 0.0;
                        input.jump_buffer_timer = JUMP_BUFFER_TIME;
                        input.jump_held = true;
                        // Move toward basket while jumping
                        let dx = target_basket_pos.x - ai_pos.x;
                        if dx.abs() > profile.position_tolerance {
                            input.move_x = dx.signum() * 0.5; // Gentle movement
                        }
                    } else if ai_state.jump_shot_active {
                        // Jump shot in progress
                        ai_state.jump_shot_timer += time.delta_secs();

                        // Hold jump briefly for height
                        if ai_state.jump_shot_timer < 0.15 {
                            input.jump_held = true;
                        } else {
                            input.jump_held = false;
                        }

                        // Start charging after initial jump (around peak)
                        if ai_state.jump_shot_timer > 0.1 {
                            if !input.throw_held && !input.throw_released {
                                input.throw_held = true;
                                ai_state.shot_charge_target = rand::thread_rng()
                                    .gen_range(profile.charge_min..profile.charge_max)
                                    .min(0.4); // Shorter charge for jump shots
                            } else if input.throw_held {
                                ai_state.shot_charge_target -= time.delta_secs();
                                if ai_state.shot_charge_target <= 0.0 {
                                    input.throw_held = false;
                                    input.throw_released = true;
                                    ai_state.jump_shot_active = false;
                                }
                            }
                        }

                        // Drift toward basket while airborne
                        let dx = target_basket_pos.x - ai_pos.x;
                        input.move_x = dx.signum() * 0.3;

                        // Reset if landed without shooting (failed attempt)
                        if grounded.0 && ai_state.jump_shot_timer > 0.3 {
                            ai_state.jump_shot_active = false;
                        }
                    } else {
                        // Normal ground shot
                        input.move_x = 0.0;

                        if !input.throw_held && !input.throw_released {
                            input.throw_held = true;
                            ai_state.shot_charge_target =
                                rand::thread_rng().gen_range(profile.charge_min..profile.charge_max);
                        } else if input.throw_held {
                            ai_state.shot_charge_target -= time.delta_secs();
                            if ai_state.shot_charge_target <= 0.0 {
                                input.throw_held = false;
                                input.throw_released = true;
                            }
                        }
                    }
                }

                AiGoal::AttemptSteal => {
                    // Move toward opponent
                    if let Some(opp_pos) = opponent_pos {
                        let dx = opp_pos.x - ai_pos.x;
                        if dx.abs() > profile.position_tolerance {
                            input.move_x = dx.signum();
                        }

                        // Attempt steal when close
                        let distance = ai_pos.distance(opp_pos);
                        input.pickup_pressed = distance < STEAL_RANGE;
                    }

                    input.throw_held = false;
                }

                AiGoal::ReturnToDefense => {
                    // Move toward defensive position
                    let dx = defensive_pos.x - ai_pos.x;
                    if dx.abs() > profile.position_tolerance {
                        input.move_x = dx.signum();
                    }

                    // Jump up to platform if needed (simple case)
                    let dy = defensive_pos.y - ai_pos.y;
                    if dy > PLAYER_SIZE.y * 0.5 && grounded.0 {
                        input.jump_buffer_timer = JUMP_BUFFER_TIME;
                        input.jump_held = true;
                    }

                    input.pickup_pressed = false;
                    input.throw_held = false;
                }
            }
        }

        // Always allow pickup when near a free ball
        let distance_to_ball = ai_pos.distance(ball_pos);
        if distance_to_ball < BALL_PICKUP_RADIUS && matches!(ball_state, BallState::Free) {
            input.pickup_pressed = true;
        }

        // Decay jump buffer timer
        input.jump_buffer_timer = (input.jump_buffer_timer - time.delta_secs()).max(0.0);
    }
}

/// Execute the current navigation action
fn execute_nav_action(
    input: &mut InputState,
    nav_state: &mut AiNavState,
    ai_pos: Vec2,
    grounded: bool,
    time: &Time,
) {
    let Some(action) = nav_state.current_action().cloned() else {
        return;
    };

    match action {
        NavAction::WalkTo { x } => {
            let dx = x - ai_pos.x;
            if dx.abs() > NAV_POSITION_TOLERANCE {
                input.move_x = dx.signum();
            } else {
                // Reached destination, advance to next action
                nav_state.advance();
            }
        }

        NavAction::JumpAt { x, hold_duration } => {
            if !nav_state.action_started {
                // Walk to jump point first
                let dx = x - ai_pos.x;
                if dx.abs() > NAV_JUMP_TOLERANCE {
                    input.move_x = dx.signum();
                } else if grounded {
                    // At jump point and on ground - start jump
                    nav_state.action_started = true;
                    nav_state.jump_timer = 0.0;
                    input.jump_buffer_timer = JUMP_BUFFER_TIME;
                    input.jump_held = true;
                }
            } else {
                // Jumping - hold for duration
                nav_state.jump_timer += time.delta_secs();
                let target_hold_time = hold_duration * SHOT_CHARGE_TIME; // Scale to actual time

                if nav_state.jump_timer < target_hold_time.min(0.3) {
                    // Still holding jump
                    input.jump_held = true;
                } else {
                    // Release and continue moving toward landing
                    input.jump_held = false;

                    // Check if we've landed
                    if grounded && nav_state.jump_timer > 0.1 {
                        nav_state.advance();
                    } else {
                        // Keep moving toward landing point while in air
                        if let Some(NavAction::WalkTo { x: land_x }) =
                            nav_state.current_path.get(nav_state.path_index + 1)
                        {
                            let dx = land_x - ai_pos.x;
                            input.move_x = dx.signum();
                        }
                    }
                }
            }
        }

        NavAction::DropFrom { x } => {
            if !nav_state.action_started {
                // Walk to drop point
                let dx = x - ai_pos.x;
                if dx.abs() > NAV_JUMP_TOLERANCE {
                    input.move_x = dx.signum();
                } else {
                    // At edge - start drop
                    nav_state.action_started = true;
                    // Continue walking off
                    input.move_x = if x > ai_pos.x { 1.0 } else { -1.0 };
                }
            } else {
                // Falling - check if landed
                if grounded {
                    nav_state.advance();
                }
            }
        }

        NavAction::WalkOffEdge { direction } => {
            if !nav_state.action_started {
                nav_state.action_started = true;
            }

            // Keep walking in direction
            input.move_x = direction;

            // Check if we've fallen (no longer grounded) then landed
            if grounded && nav_state.jump_timer > 0.1 {
                nav_state.advance();
            } else if !grounded {
                // Falling
                nav_state.jump_timer += time.delta_secs();
            }
        }
    }
}
