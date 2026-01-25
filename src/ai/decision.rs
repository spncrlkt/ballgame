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

/// Calculate the interception position on the line between ball carrier and defender's basket.
/// The defender should position themselves between the opponent and their own basket
/// (the basket the opponent is trying to score on).
fn calculate_interception_position(
    opponent_pos: Vec2,
    basket_pos: Vec2,
    pressure_distance: f32,
    defensive_iq: f32,
) -> Vec2 {
    // Direction from opponent to basket (the shot line)
    let shot_direction = (basket_pos - opponent_pos).normalize_or_zero();

    // Base intercept position: on the shot line, pressure_distance away from opponent
    let base_intercept = opponent_pos + shot_direction * pressure_distance;

    // Apply defensive IQ - higher IQ means staying more precisely on the shot line
    // Lower IQ introduces perpendicular positioning error
    let perpendicular = Vec2::new(-shot_direction.y, shot_direction.x);
    let error_magnitude = (1.0 - defensive_iq) * 40.0;
    let offset = perpendicular * error_magnitude * (0.5 - defensive_iq);

    let unclamped = base_intercept + offset;

    // Clamp to valid arena bounds to prevent AI from targeting unreachable positions
    // Leave margin for wall thickness and corner steps
    let margin = WALL_THICKNESS + CORNER_STEP_TOTAL_WIDTH + PLAYER_SIZE.x;
    let min_x = -ARENA_WIDTH / 2.0 + margin;
    let max_x = ARENA_WIDTH / 2.0 - margin;
    let min_y = ARENA_FLOOR_Y + PLAYER_SIZE.y / 2.0;

    Vec2::new(
        unclamped.x.clamp(min_x, max_x),
        unclamped.y.max(min_y),
    )
}

/// Check if a defender is positioned to block a shot trajectory
pub fn defender_in_shot_path(
    ball_pos: Vec2,
    ball_velocity: Vec2,
    defender_pos: Vec2,
    blocking_radius: f32,
) -> bool {
    // Project defender position onto the shot trajectory
    let shot_dir = ball_velocity.normalize_or_zero();
    if shot_dir.length_squared() < 0.01 {
        return false;
    }

    // Vector from ball to defender
    let to_defender = defender_pos - ball_pos;

    // Project onto shot direction
    let projection_length = to_defender.dot(shot_dir);

    // Defender must be in front of the ball (positive projection)
    if projection_length < 0.0 {
        return false;
    }

    // Calculate perpendicular distance from shot line
    let closest_point = ball_pos + shot_dir * projection_length;
    let perpendicular_dist = defender_pos.distance(closest_point);

    perpendicular_dist < blocking_radius
}

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

        // Find opponent position
        let opponent_pos = all_players
            .iter()
            .find(|(e, _, _, human)| *e != ai_entity && human.is_some())
            .map(|(_, t, _, _)| t.translation.truncate());

        // Get the AI's own basket (the one they're defending)
        // AI defends the opposite basket from what they're targeting
        let own_basket_pos = basket_query
            .iter()
            .find(|(_, b)| **b != target_basket.0)
            .map(|(t, _)| t.translation.truncate());

        // Calculate defensive/interception position based on opponent and own basket
        let intercept_pos = if let (Some(opp_pos), Some(own_basket)) = (opponent_pos, own_basket_pos) {
            // Position on the shot line between opponent and our basket
            calculate_interception_position(
                opp_pos,
                own_basket,
                profile.pressure_distance,
                profile.defensive_iq,
            )
        } else if let Some(opp_pos) = opponent_pos {
            // Fallback: just get close to opponent
            opp_pos
        } else {
            // Fallback defensive position (old logic)
            let defensive_y = ARENA_FLOOR_Y + 50.0;
            match *team {
                Team::Left => Vec2::new(-profile.defense_offset, defensive_y),
                Team::Right => Vec2::new(profile.defense_offset, defensive_y),
            }
        };

        // Determine navigation target based on goal
        let desired_target: Option<Vec2> = match ai_state.current_goal {
            AiGoal::Idle => None,

            AiGoal::ChaseBall => ball_pos,

            AiGoal::AttackWithBall => {
                // Navigate to a position within shooting range of basket
                // Pass min_shot_quality to avoid navigating to positions where shots are low quality
                // (e.g., directly under the basket)
                if let Some(basket_pos) = target_basket_pos {
                    // Try to find a shooting position that meets quality threshold
                    if let Some(path_result) = find_path_to_shoot(
                        &nav_graph,
                        ai_pos,
                        basket_pos,
                        profile.shoot_range,
                        profile.min_shot_quality,
                    ) {
                        Some(nav_graph.nodes[path_result.goal_node].center)
                    } else {
                        // Fallback: find best elevated platform instead of going under basket
                        // This prevents AI from getting stuck under the basket
                        if let Some(elevated_idx) =
                            nav_graph.find_elevated_platform(basket_pos, profile.min_shot_quality)
                        {
                            Some(nav_graph.nodes[elevated_idx].center)
                        } else {
                            // Last resort: stay in pursuit mode (don't navigate)
                            None
                        }
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

            AiGoal::InterceptDefense | AiGoal::PressureDefense => {
                // Navigate to intercept position on the shot line
                // If opponent is elevated, find a platform at their height to defend from
                if let Some(opp_pos) = opponent_pos {
                    let height_diff = opp_pos.y - ai_pos.y;
                    if height_diff > PLAYER_SIZE.y {
                        // Opponent is elevated - find a defensive platform at their height
                        if let Some(def_node) = nav_graph.find_defensive_platform(
                            opp_pos,
                            own_basket_pos.unwrap_or(Vec2::ZERO),
                            opp_pos.y - PLAYER_SIZE.y, // min height threshold
                        ) {
                            Some(nav_graph.nodes[def_node].center)
                        } else {
                            // No elevated platform found - fall back to intercept position
                            Some(intercept_pos)
                        }
                    } else {
                        Some(intercept_pos)
                    }
                } else {
                    Some(intercept_pos)
                }
            }
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
    nav_graph: Res<NavGraph>,
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

        // Decrement button press cooldown (simulates human mashing speed limit)
        // Use a minimum dt of 1/60 to handle headless mode where delta can be tiny
        let dt = time.delta_secs().max(1.0 / 60.0);
        ai_state.button_press_cooldown = (ai_state.button_press_cooldown - dt).max(0.0);

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

        // Get the AI's own basket (the one they're defending)
        let own_basket_pos = basket_query
            .iter()
            .find(|(_, b)| **b != target_basket_type)
            .map(|(t, _)| t.translation.truncate())
            .unwrap_or(Vec2::new(600.0, 0.0)); // Fallback

        // Calculate intercept position on the shot line
        let intercept_pos = if let Some(opp_pos) = opponent_pos {
            calculate_interception_position(
                opp_pos,
                own_basket_pos,
                profile.pressure_distance,
                profile.defensive_iq,
            )
        } else {
            // Fallback defensive position
            let defensive_y = ARENA_FLOOR_Y + 50.0;
            match *team {
                Team::Left => Vec2::new(-profile.defense_offset, defensive_y),
                Team::Right => Vec2::new(profile.defense_offset, defensive_y),
            }
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

            // Safety check: don't START charging if opponent is very close (steal risk)
            // But if already charging, COMMIT to the shot - aborting leads to steals anyway
            let already_charging = ai_state.current_goal == AiGoal::ChargeShot;
            let opponent_too_close = !already_charging && opponent_pos
                .map(|opp| ai_pos.distance(opp) < profile.steal_range * 1.2)
                .unwrap_or(false);

            // Calculate utility of seeking a better position vs shooting now
            // Only consider seeking if current position meets basic shooting criteria
            let should_seek = if quality_acceptable && in_shoot_range && !already_charging {
                if let Some(best_node_idx) = nav_graph.find_best_shot_position(target_basket_pos) {
                    let best_quality = nav_graph.get_shot_quality(best_node_idx, target_basket_pos);
                    let quality_gain = best_quality - shot_quality;

                    // Only seek if there's meaningful quality to gain
                    if quality_gain > 0.01 {
                        // Opportunity cost factors:
                        // 1. Path cost (time/risk to reach better position)
                        let path_cost = nav_graph.estimate_path_cost(ai_pos, best_node_idx);
                        let path_cost_normalized = (path_cost / 500.0).min(1.0);

                        // 2. Opponent pressure (closer opponent = higher cost to seek)
                        let opponent_pressure = opponent_pos
                            .map(|opp| 1.0 - (ai_pos.distance(opp) / 300.0).min(1.0))
                            .unwrap_or(0.0);

                        // Utility = quality_gain - opportunity_costs, scaled by patience
                        let raw_utility = quality_gain
                            - (path_cost_normalized * 0.3)
                            - (opponent_pressure * 0.2);
                        let seek_utility = raw_utility * profile.position_patience;

                        // Seek if utility exceeds threshold
                        seek_utility > profile.seek_threshold
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            // Only shoot if shot quality is acceptable AND position conditions are met
            // AND opponent isn't too close (or we're already committed to the shot)
            // AND we're not deciding to seek a better position
            if quality_acceptable && (in_shoot_range || reached_target) && !opponent_too_close && !should_seek {
                AiGoal::ChargeShot
            } else if already_charging {
                // Commit to the shot once started
                AiGoal::ChargeShot
            } else {
                AiGoal::AttackWithBall
            }
        } else if opponent_has_ball {
            if let Some(opp_pos) = opponent_pos {
                let distance_to_opponent = ai_pos.distance(opp_pos);
                // Calculate effective pressure threshold based on profile
                // Higher aggression = tighter pressure, lower = zone defense
                let pressure_threshold = profile.pressure_distance * (1.0 + (1.0 - profile.aggression));

                // Determine ideal defensive goal
                let ideal_defense = if distance_to_opponent < profile.steal_range {
                    AiGoal::AttemptSteal
                } else if distance_to_opponent < pressure_threshold {
                    AiGoal::PressureDefense
                } else {
                    AiGoal::InterceptDefense
                };

                // Apply hysteresis: only switch defensive modes if enough time has passed
                // This prevents rapid oscillation when near threshold boundaries
                let elapsed = time.elapsed_secs();
                let time_since_switch = elapsed - ai_state.last_defense_switch;
                let is_defensive_goal = matches!(
                    ai_state.current_goal,
                    AiGoal::InterceptDefense | AiGoal::PressureDefense | AiGoal::AttemptSteal
                );

                if !is_defensive_goal || time_since_switch > 0.4 || ideal_defense == AiGoal::AttemptSteal {
                    // Allow switch if: not in defense mode, enough time passed, or steal opportunity
                    ideal_defense
                } else {
                    // Keep current goal to prevent oscillation
                    ai_state.current_goal
                }
            } else {
                AiGoal::InterceptDefense
            }
        } else {
            // Ball is free
            AiGoal::ChaseBall
        };

        // Update goal (and randomize charge target when starting to charge)
        if new_goal != ai_state.current_goal {
            // Track defensive mode switches for hysteresis
            let is_defensive_switch = matches!(
                new_goal,
                AiGoal::InterceptDefense | AiGoal::PressureDefense | AiGoal::AttemptSteal
            );
            if is_defensive_switch {
                ai_state.last_defense_switch = time.elapsed_secs();
            }

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
                    if dy > PLAYER_SIZE.y && dx.abs() < BALL_PICKUP_RADIUS * 2.0 && grounded.0 {
                        input.jump_buffer_timer = JUMP_BUFFER_TIME;
                        input.jump_held = true;
                    }

                    // Keep holding jump while airborne to reach elevated balls
                    if !grounded.0 && dy > PLAYER_SIZE.y {
                        input.jump_held = true;
                    }

                    // Try to pick up ball when close (respecting button cooldown)
                    let distance_to_ball = ai_pos.distance(ball_pos);
                    if distance_to_ball < BALL_PICKUP_RADIUS
                        && matches!(ball_state, BallState::Free)
                        && ai_state.button_press_cooldown <= 0.0
                    {
                        input.pickup_pressed = true;
                        ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
                    }

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
                        ai_state.jump_shot_timer += dt;

                        // Hold jump for height (same as player capability)
                        if ai_state.jump_shot_timer < 0.25 {
                            input.jump_held = true;
                        } else {
                            input.jump_held = false;
                        }

                        // Start charging after initial jump (around peak)
                        if ai_state.jump_shot_timer > 0.1 {
                            if !input.throw_held && !input.throw_released {
                                input.throw_held = true;
                                ai_state.shot_charge_target = rand::thread_rng()
                                    .gen_range(profile.charge_min..profile.charge_max);
                                // No cap - AI uses full charge range like player
                            } else if input.throw_held {
                                ai_state.shot_charge_target -= dt;
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

                        // Reset if landed without shooting (give more time like player has)
                        if grounded.0 && ai_state.jump_shot_timer > 1.0 {
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
                            ai_state.shot_charge_target -= dt;
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

                        // Check if in steal range and update reaction timer
                        // Use profile's steal_range, not global constant
                        let distance = ai_pos.distance(opp_pos);
                        let in_steal_range = distance < profile.steal_range;

                        if in_steal_range {
                            // Reset timer if just entered range
                            if !ai_state.was_in_steal_range {
                                ai_state.steal_reaction_timer = 0.0;
                            }
                            // Increment timer
                            ai_state.steal_reaction_timer += dt;

                            // Only attempt steal after reaction delay AND button cooldown
                            if ai_state.steal_reaction_timer >= profile.steal_reaction_time
                                && ai_state.button_press_cooldown <= 0.0
                            {
                                input.pickup_pressed = true;
                                // Convert presses/sec to interval
                                ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
                            }
                        } else {
                            ai_state.steal_reaction_timer = 0.0;
                        }
                        ai_state.was_in_steal_range = in_steal_range;
                    }

                    input.throw_held = false;
                }

                AiGoal::InterceptDefense => {
                    // Navigate to intercept position on shot line
                    // If opponent is elevated, prioritize matching their height
                    let target_pos = if let Some(opp_pos) = opponent_pos {
                        let height_diff = opp_pos.y - ai_pos.y;
                        if height_diff > PLAYER_SIZE.y {
                            // Opponent elevated - aim for their Y level
                            Vec2::new(intercept_pos.x, opp_pos.y)
                        } else {
                            intercept_pos
                        }
                    } else {
                        intercept_pos
                    };

                    let dx = target_pos.x - ai_pos.x;
                    if dx.abs() > profile.position_tolerance {
                        input.move_x = dx.signum();
                    }

                    // Jump up to platform if needed - be more aggressive when opponent elevated
                    let dy = target_pos.y - ai_pos.y;
                    if dy > PLAYER_SIZE.y * 0.5 && grounded.0 {
                        input.jump_buffer_timer = JUMP_BUFFER_TIME;
                        input.jump_held = true;
                    }

                    // If we're airborne and need to go higher, keep holding jump
                    if !grounded.0 && dy > PLAYER_SIZE.y {
                        input.jump_held = true;
                    }

                    input.pickup_pressed = false;
                    input.throw_held = false;
                }

                AiGoal::PressureDefense => {
                    // Close-range: chase opponent aggressively and attempt steals
                    if let Some(opp_pos) = opponent_pos {
                        let dx = opp_pos.x - ai_pos.x;
                        // Move directly toward opponent
                        if dx.abs() > profile.position_tolerance * 0.5 {
                            input.move_x = dx.signum();
                        }

                        // Jump if opponent is above us
                        let dy = opp_pos.y - ai_pos.y;
                        if dy > PLAYER_SIZE.y * 0.5 && grounded.0 {
                            input.jump_buffer_timer = JUMP_BUFFER_TIME;
                            input.jump_held = true;
                        }

                        // Keep holding jump while airborne to reach elevated opponents
                        if !grounded.0 && dy > PLAYER_SIZE.y {
                            input.jump_held = true;
                        }

                        // Check if in steal range and update reaction timer
                        // Use profile's steal_range (with 1.2x for pressure defense)
                        let distance = ai_pos.distance(opp_pos);
                        let in_steal_range = distance < profile.steal_range * 1.2;

                        if in_steal_range {
                            // Reset timer if just entered range
                            if !ai_state.was_in_steal_range {
                                ai_state.steal_reaction_timer = 0.0;
                            }
                            // Increment timer
                            ai_state.steal_reaction_timer += dt;

                            // Only attempt steal after reaction delay AND button cooldown
                            if ai_state.steal_reaction_timer >= profile.steal_reaction_time
                                && ai_state.button_press_cooldown <= 0.0
                            {
                                input.pickup_pressed = true;
                                ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
                            }
                        } else {
                            ai_state.steal_reaction_timer = 0.0;
                        }
                        ai_state.was_in_steal_range = in_steal_range;
                    }

                    input.throw_held = false;
                }
            }
        }

        // ChargeShot throw logic runs regardless of navigation
        // (navigation handles movement, this handles the actual throw)
        if ai_state.current_goal == AiGoal::ChargeShot && nav_controlling {
            // We're navigating to a shooting position while in ChargeShot mode
            // Start/continue charging while moving
            if !input.throw_held && !input.throw_released {
                input.throw_held = true;
                ai_state.shot_charge_target =
                    rand::thread_rng().gen_range(profile.charge_min..profile.charge_max);
            } else if input.throw_held {
                ai_state.shot_charge_target -= dt;
                if ai_state.shot_charge_target <= 0.0 {
                    input.throw_held = false;
                    input.throw_released = true;
                }
            }
        }

        // Always allow pickup when near a free ball (respecting button cooldown)
        let distance_to_ball = ai_pos.distance(ball_pos);
        if distance_to_ball < BALL_PICKUP_RADIUS
            && matches!(ball_state, BallState::Free)
            && ai_state.button_press_cooldown <= 0.0
        {
            input.pickup_pressed = true;
            ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
        }

        // Stuck detection: if AI is trying to move but position doesn't change, try to unstick
        let stuck_threshold = 5.0; // Distance that counts as "not stuck"
        let position_changed = ai_state
            .last_position
            .map(|last| ai_pos.distance(last) > stuck_threshold)
            .unwrap_or(true);

        if input.move_x.abs() > 0.1 && !position_changed && grounded.0 {
            // AI is trying to move but stuck
            ai_state.stuck_timer += dt;

            // After being stuck for a bit, try to jump to get unstuck
            if ai_state.stuck_timer > 0.5 {
                input.jump_buffer_timer = JUMP_BUFFER_TIME;
                input.jump_held = true;
                // Also try moving the opposite direction briefly
                if ai_state.stuck_timer > 1.0 {
                    input.move_x = -input.move_x;
                    ai_state.stuck_timer = 0.0; // Reset after trying opposite direction
                }
            }
        } else {
            // Not stuck, reset timer
            ai_state.stuck_timer = 0.0;
        }

        // Update last position for next frame's stuck detection
        ai_state.last_position = Some(ai_pos);

        // Decay jump buffer timer
        input.jump_buffer_timer = (input.jump_buffer_timer - dt).max(0.0);
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
    // Use minimum dt for headless mode compatibility
    let dt = time.delta_secs().max(1.0 / 60.0);

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
                nav_state.jump_timer += dt;
                let target_hold_time = hold_duration * SHOT_CHARGE_TIME; // Scale to actual time

                if nav_state.jump_timer < target_hold_time {
                    // No cap - physics determines max useful hold time
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
                nav_state.jump_timer += dt;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Test that button press cooldown correctly limits press rate.
    /// At 10 presses/sec, interval is 0.1s, so 5 frames at 60fps (0.0833s) shouldn't allow a second press.
    #[test]
    fn test_button_press_cooldown_limits_rate() {
        let presses_per_sec: f32 = 10.0;
        let interval: f32 = 1.0 / presses_per_sec; // 0.1 seconds

        let mut cooldown: f32 = 0.0;
        let mut presses = 0;
        let dt: f32 = 1.0 / 60.0; // 60 fps

        // Simulate 30 frames (0.5 seconds) of wanting to press
        for _ in 0..30 {
            cooldown = (cooldown - dt).max(0.0);
            if cooldown <= 0.0 {
                // Would press
                presses += 1;
                cooldown = interval;
            }
        }

        // At 10 presses/sec over 0.5s, should get ~5 presses (not 30!)
        assert!(
            presses >= 4 && presses <= 6,
            "Expected ~5 presses at 10/sec over 0.5s, got {}",
            presses
        );
    }

    /// Test that reaction timer delays first steal attempt.
    #[test]
    fn test_steal_reaction_timer_delays_first_attempt() {
        let reaction_time: f32 = 0.2; // 200ms
        let dt: f32 = 1.0 / 60.0; // 60 fps

        let mut timer: f32 = 0.0;
        let mut frames_until_first_attempt: usize = 0;

        // Simulate entering steal range and waiting
        for frame in 0..60 {
            timer += dt;
            if timer >= reaction_time {
                frames_until_first_attempt = frame;
                break;
            }
        }

        // At 60fps, 0.2s = 12 frames
        assert!(
            frames_until_first_attempt >= 11 && frames_until_first_attempt <= 13,
            "Expected ~12 frames for 0.2s reaction at 60fps, got {}",
            frames_until_first_attempt
        );
    }

    /// IMPORTANT: This test catches when new button actions are added without cooldown limits.
    ///
    /// Every `input.pickup_pressed = true` in AI decision code MUST:
    /// 1. Check `button_press_cooldown <= 0.0` before pressing
    /// 2. Reset cooldown after pressing: `button_press_cooldown = 1.0 / profile.button_presses_per_sec`
    ///
    /// This prevents the AI from having frame-perfect button timing.
    #[test]
    fn test_all_pickup_pressed_assignments_have_cooldown() {
        let source = fs::read_to_string("src/ai/decision.rs")
            .expect("Should be able to read decision.rs");

        // Only analyze code BEFORE the test module (exclude #[cfg(test)] section)
        let code_to_analyze = source
            .split("#[cfg(test)]")
            .next()
            .unwrap_or(&source);

        // Find all lines with `input.pickup_pressed = true`
        let violations: Vec<(usize, &str)> = code_to_analyze
            .lines()
            .enumerate()
            .filter(|(_, line)| {
                let trimmed = line.trim();
                // Match actual assignments to input.pickup_pressed, not comments
                trimmed.contains("input.pickup_pressed = true")
                    && !trimmed.starts_with("//")
                    && !trimmed.starts_with("*")
            })
            .filter(|(line_num, _)| {
                // Check if this assignment is properly guarded
                // Look at the surrounding context (10 lines before, 3 after)
                let start = line_num.saturating_sub(10);
                let context: String = code_to_analyze
                    .lines()
                    .skip(start)
                    .take(line_num - start + 4)
                    .collect::<Vec<_>>()
                    .join("\n");

                // Must have BOTH:
                // 1. A cooldown check before the assignment
                // 2. A cooldown reset after the assignment
                let has_cooldown_check = context.contains("button_press_cooldown <= 0.0")
                    || context.contains("button_press_cooldown == 0.0");
                let has_cooldown_reset = context.contains("button_press_cooldown =")
                    && context.contains("button_presses_per_sec");

                // Violation if missing either check or reset
                !has_cooldown_check || !has_cooldown_reset
            })
            .collect();

        if !violations.is_empty() {
            let msg = violations
                .iter()
                .map(|(line, content)| format!("  Line {}: {}", line + 1, content.trim()))
                .collect::<Vec<_>>()
                .join("\n");

            panic!(
                "\n\nFOUND UNGUARDED pickup_pressed ASSIGNMENTS!\n\
                 \n\
                 The following lines set `input.pickup_pressed = true` without proper cooldown:\n\
                 {}\n\
                 \n\
                 REQUIRED PATTERN:\n\
                 ```\n\
                 if <condition> && ai_state.button_press_cooldown <= 0.0 {{\n\
                     input.pickup_pressed = true;\n\
                     ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;\n\
                 }}\n\
                 ```\n\
                 \n\
                 This prevents AI from having frame-perfect button timing.\n",
                msg
            );
        }
    }

    /// Test profile values are in reasonable human ranges.
    #[test]
    fn test_profile_button_timing_in_human_range() {
        // Human button mashing is typically 6-15 presses per second
        // Reaction time is typically 150-400ms
        let profiles = crate::ai::AiProfileDatabase::default();

        for profile in profiles.profiles() {
            assert!(
                profile.button_presses_per_sec >= 5.0 && profile.button_presses_per_sec <= 20.0,
                "Profile '{}' has unrealistic button_presses_per_sec: {} (expected 5-20)",
                profile.name,
                profile.button_presses_per_sec
            );

            assert!(
                profile.steal_reaction_time >= 0.05 && profile.steal_reaction_time <= 0.5,
                "Profile '{}' has unrealistic steal_reaction_time: {} (expected 0.05-0.5s)",
                profile.name,
                profile.steal_reaction_time
            );
        }
    }
}
