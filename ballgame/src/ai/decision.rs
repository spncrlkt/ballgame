//! AI decision system - updates InputState based on game state

use bevy::prelude::*;
use rand::Rng;

use crate::ai::navigation::{find_escape_x, has_ceiling_above};
use crate::ai::{
    AiCapabilities, AiGoal, AiNavState, AiProfileDatabase, AiState, HeatmapBundle, InputState,
    NavAction, NavGraph, find_path, find_path_to_shoot,
    shot_quality::{evaluate_shot_quality, scale_min_quality_for_level},
};
use crate::ball::{Ball, BallState, Velocity};
use crate::constants::*;
use crate::events::{CharacterId, EventBus, GameEvent};
use crate::levels::LevelDatabase;
use crate::player::{
    Character, Grounded, HoldingBall, HumanControlled, Player, RemoteControlled, TargetBasket, Team,
};
use crate::scoring::CurrentLevel;
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

    // Position CLOSER to opponent rather than ahead of them
    // Use a fraction of pressure_distance to stay tight on the ball carrier
    // Higher defensive_iq = tighter positioning (closer to opponent)
    let effective_distance = pressure_distance * (0.3 + (1.0 - defensive_iq) * 0.4);

    // Base intercept: between opponent and basket, but biased toward opponent
    let base_intercept = opponent_pos + shot_direction * effective_distance;

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

    Vec2::new(unclamped.x.clamp(min_x, max_x), unclamped.y.max(min_y))
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
        (With<Player>, Without<HumanControlled>, Without<RemoteControlled>),
    >,
    all_players: Query<
        (
            Entity,
            &Transform,
            &Team,
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

    for (
        ai_entity,
        ai_transform,
        ai_team,
        ai_state,
        mut nav_state,
        target_basket,
        _holding,
        _grounded,
    ) in &mut ai_query
    {
        // Skip Idle AI
        if ai_state.current_goal == AiGoal::Idle {
            nav_state.clear();
            continue;
        }

        let profile = profile_db
            .get_by_id(&ai_state.profile_id)
            .unwrap_or_else(|| profile_db.default_profile());

        // Skip Dummy profile - no navigation needed for characters that do nothing
        if profile.name == "Dummy" {
            nav_state.clear();
            continue;
        }

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

        // Find nearest opponent (different team) - prefer ball carrier, then human, then closest
        let opponents: Vec<(Entity, Vec2, bool, bool)> = all_players
            .iter()
            .filter(|(e, _, t, _, _)| *e != ai_entity && *t != ai_team)
            .map(|(e, tr, _, holding, human)| {
                (e, tr.translation.truncate(), holding.is_some(), human.is_some())
            })
            .collect();

        let opponent_pos = opponents
            .iter()
            .filter(|(_, _, has_ball, _)| *has_ball)
            .map(|(_, pos, _, _)| *pos)
            .next()
            .or_else(|| {
                opponents
                    .iter()
                    .filter(|(_, _, _, is_human)| *is_human)
                    .map(|(_, pos, _, _)| *pos)
                    .next()
            })
            .or_else(|| {
                opponents
                    .iter()
                    .min_by(|(_, pos_a, _, _), (_, pos_b, _, _)| {
                        let dist_a = ai_pos.distance(*pos_a);
                        let dist_b = ai_pos.distance(*pos_b);
                        dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(_, pos, _, _)| *pos)
            });

        // Get the AI's own basket (the one they're defending)
        // AI defends the opposite basket from what they're targeting
        let own_basket_pos = basket_query
            .iter()
            .find(|(_, b)| **b != target_basket.0)
            .map(|(t, _)| t.translation.truncate());

        // Calculate defensive/interception position based on opponent and own basket
        let intercept_pos =
            if let (Some(opp_pos), Some(own_basket)) = (opponent_pos, own_basket_pos) {
                // Check if opponent is significantly elevated above floor
                let floor_y = ARENA_FLOOR_Y + PLAYER_SIZE.y / 2.0;
                let opp_elevated = opp_pos.y > floor_y + PLAYER_SIZE.y * 2.0;

                if opp_elevated {
                    // For elevated opponents, position horizontally between opponent and basket
                    // at the opponent's height level (not on the upward-pointing shot line)
                    let intercept_x = if own_basket.x < opp_pos.x {
                        opp_pos.x - profile.pressure_distance
                    } else {
                        opp_pos.x + profile.pressure_distance
                    };
                    Vec2::new(
                        intercept_x.clamp(-ARENA_WIDTH / 2.0 + 100.0, ARENA_WIDTH / 2.0 - 100.0),
                        opp_pos.y,
                    )
                } else {
                    // Ground-level: use standard shot line intercept
                    calculate_interception_position(
                        opp_pos,
                        own_basket,
                        profile.pressure_distance,
                        profile.defensive_iq,
                    )
                }
            } else if let Some(opp_pos) = opponent_pos {
                // Fallback: just get close to opponent
                opp_pos
            } else {
                // Fallback defensive position (old logic)
                let defensive_y = ARENA_FLOOR_Y + 50.0;
                match *ai_team {
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
                        let target_node = &nav_graph.nodes[path_result.goal_node];
                        info!(
                            "AI AttackWithBall: Found shooting path to node {} @ ({:.0},{:.0}), {} actions, cost={:.1}",
                            path_result.goal_node,
                            target_node.center.x,
                            target_node.center.y,
                            path_result.actions.len(),
                            path_result.total_cost
                        );
                        Some(target_node.center)
                    } else {
                        // Fallback: find best elevated platform instead of going under basket
                        // This prevents AI from getting stuck under the basket
                        warn!(
                            "AI AttackWithBall: No shooting path found from ({:.0},{:.0}) to basket ({:.0},{:.0}), trying elevated platform",
                            ai_pos.x, ai_pos.y, basket_pos.x, basket_pos.y
                        );
                        if let Some(elevated_idx) =
                            nav_graph.find_elevated_platform(basket_pos, profile.min_shot_quality)
                        {
                            let target_node = &nav_graph.nodes[elevated_idx];
                            info!(
                                "AI AttackWithBall: Found elevated platform fallback node {} @ ({:.0},{:.0})",
                                elevated_idx, target_node.center.x, target_node.center.y
                            );
                            Some(target_node.center)
                        } else {
                            // Last resort: stay in pursuit mode (don't navigate)
                            warn!("AI AttackWithBall: No elevated platform found - using simple movement fallback");
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

            AiGoal::PassToTeammate => {
                // Don't navigate while passing - stay put
                None
            }

            AiGoal::AttemptSteal => {
                // Use navigation when opponent is on elevated platform we can't jump to directly
                if let Some(opp_pos) = opponent_pos {
                    let height_diff = opp_pos.y - ai_pos.y;
                    if height_diff > PLAYER_SIZE.y * 1.5 {
                        // Opponent is elevated - find path to their platform
                        if let Some(def_node) = nav_graph.find_defensive_platform(
                            opp_pos,
                            own_basket_pos.unwrap_or(Vec2::ZERO),
                            opp_pos.y - PLAYER_SIZE.y,
                        ) {
                            Some(nav_graph.nodes[def_node].center)
                        } else {
                            None // No path found, use simple movement
                        }
                    } else {
                        None // Opponent reachable, use simple movement
                    }
                } else {
                    None
                }
            }

            AiGoal::InterceptDefense | AiGoal::PressureDefense => {
                // When far from opponent, target them directly to close distance
                // When close, use intercept positioning
                if let Some(opp_pos) = opponent_pos {
                    let distance_to_opponent = ai_pos.distance(opp_pos);

                    // If far from opponent, chase them directly
                    // Use max of profile-based threshold and fixed minimum (300px)
                    // This ensures aggressive pursuit even with low steal_range profiles
                    let chase_threshold = (profile.steal_range * 3.5).max(300.0);
                    if distance_to_opponent > chase_threshold
                        || ai_state.current_goal == AiGoal::InterceptDefense
                            && distance_to_opponent > profile.pressure_distance * 1.2
                    {
                        Some(opp_pos)
                    } else {
                        // Close enough - use intercept positioning
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
                                Some(intercept_pos)
                            }
                        } else {
                            Some(intercept_pos)
                        }
                    }
                } else {
                    Some(intercept_pos)
                }
            }

            // === CatchPartner goals - debug teammate behavior ===
            AiGoal::MoveToOpenSpot => {
                // Move to a position where we can receive a pass from teammate
                // Use distance_drill_target for spacing (starts at 1200, decreases 100px after each pass)
                if let Some(tm_pos) = all_players
                    .iter()
                    .filter(|(e, _, t, _, _)| *e != ai_entity && *t == ai_team)
                    .map(|(_, tr, _, _, _)| tr.translation.truncate())
                    .next()
                {
                    // Position on opposite side from teammate at drill distance
                    let direction = if ai_pos.x > tm_pos.x { 1.0 } else { -1.0 };
                    let target_x = tm_pos.x + (direction * ai_state.distance_drill_target);
                    Some(Vec2::new(target_x, ARENA_FLOOR_Y + PLAYER_SIZE.y / 2.0))
                } else {
                    // No teammate found, just stand in center
                    Some(Vec2::new(0.0, ARENA_FLOOR_Y + PLAYER_SIZE.y / 2.0))
                }
            }

            AiGoal::ReceivePass => {
                // Track incoming ball, move toward where it will land
                ball_pos
            }

            AiGoal::ChaseMissedBall => {
                // Chase a missed/loose ball to retrieve it
                ball_pos
            }

            AiGoal::HoldAndPass => {
                // Stay in place while holding the ball before passing
                None
            }

            AiGoal::RepositionToDistance => {
                // Reposition to target distance from teammate
                if let Some(tm_pos) = all_players
                    .iter()
                    .filter(|(e, _, t, _, _)| *e != ai_entity && *t == ai_team)
                    .map(|(_, tr, _, _, _)| tr.translation.truncate())
                    .next()
                {
                    // Calculate target x: maintain distance on the side we're already on
                    let direction = if ai_pos.x > tm_pos.x { 1.0 } else { -1.0 };
                    let target_x = tm_pos.x + (direction * ai_state.distance_drill_target);
                    Some(Vec2::new(target_x, ARENA_FLOOR_Y + PLAYER_SIZE.y / 2.0))
                } else {
                    None
                }
            }

            // === KeepAway goals ===
            AiGoal::EvadeAndPass => {
                // Teammate: move to position away from adversary pressure
                // Find the adversary (opponent team) and evade perpendicular to their approach
                let adversary_pos = all_players
                    .iter()
                    .filter(|(e, _, t, _, _)| *e != ai_entity && *t != ai_team)
                    .map(|(_, tr, _, _, _)| tr.translation.truncate())
                    .next();

                if let Some(adv_pos) = adversary_pos {
                    // Calculate escape vector perpendicular to adversary approach
                    let to_ai = ai_pos - adv_pos;
                    let perpendicular = Vec2::new(-to_ai.y, to_ai.x).normalize_or_zero();

                    // Escape to the side that keeps us more in bounds
                    let escape_dist = 150.0;
                    let escape_a = ai_pos + perpendicular * escape_dist;
                    let escape_b = ai_pos - perpendicular * escape_dist;

                    // Pick the escape that's further from walls
                    let arena_center = 0.0;
                    let target = if (escape_a.x - arena_center).abs()
                        < (escape_b.x - arena_center).abs()
                    {
                        escape_a
                    } else {
                        escape_b
                    };

                    Some(Vec2::new(
                        target.x.clamp(
                            -ARENA_WIDTH / 2.0 + 100.0,
                            ARENA_WIDTH / 2.0 - 100.0,
                        ),
                        ARENA_FLOOR_Y + PLAYER_SIZE.y / 2.0,
                    ))
                } else {
                    // No adversary found, just stay in center
                    Some(Vec2::new(0.0, ARENA_FLOOR_Y + PLAYER_SIZE.y / 2.0))
                }
            }

            AiGoal::PressureBallCarrier => {
                // Adversary: chase ball carrier directly
                let ball_carrier_pos = all_players
                    .iter()
                    .filter(|(e, _, _, holding, _)| *e != ai_entity && holding.is_some())
                    .map(|(_, tr, _, _, _)| tr.translation.truncate())
                    .next();

                ball_carrier_pos.or(ball_pos)
            }

            AiGoal::PredictIntercept => {
                // Adversary: position between ball carrier and their teammate
                let ball_carrier = all_players
                    .iter()
                    .filter(|(e, _, _, holding, _)| *e != ai_entity && holding.is_some())
                    .map(|(_, tr, t, _, _)| (tr.translation.truncate(), *t))
                    .next();

                if let Some((carrier_pos, carrier_team)) = ball_carrier {
                    // Find the carrier's teammate (pass target)
                    let pass_target_pos = all_players
                        .iter()
                        .filter(|(_, _, t, holding, _)| {
                            **t == carrier_team && holding.is_none()
                        })
                        .map(|(_, tr, _, _, _)| tr.translation.truncate())
                        .next();

                    if let Some(target_pos) = pass_target_pos {
                        // Position 30% along the line from carrier to target
                        let intercept = carrier_pos + (target_pos - carrier_pos) * 0.3;
                        Some(Vec2::new(
                            intercept.x.clamp(
                                -ARENA_WIDTH / 2.0 + 50.0,
                                ARENA_WIDTH / 2.0 - 50.0,
                            ),
                            intercept.y.max(ARENA_FLOOR_Y + PLAYER_SIZE.y / 2.0),
                        ))
                    } else {
                        // No pass target, just chase carrier
                        Some(carrier_pos)
                    }
                } else {
                    ball_pos
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
        let path_invalid =
            nav_state.path_complete() || (!nav_state.current_path.is_empty() && !nav_state.active);

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
                    debug!(
                        "AI nav: Needs navigation to ({:.0},{:.0}), height_diff={:.0}, h_dist={:.0}",
                        target.x, target.y, height_diff, horizontal_dist
                    );
                    if let Some(path_result) = find_path(&nav_graph, ai_pos, target) {
                        info!(
                            "AI nav: Path found with {} actions to ({:.0},{:.0})",
                            path_result.actions.len(), target.x, target.y
                        );
                        nav_state.set_path(path_result.actions, target);
                    } else {
                        // No path found - clear and let simple movement take over
                        debug!(
                            "AI nav: No path found from ({:.0},{:.0}) to ({:.0},{:.0})",
                            ai_pos.x, ai_pos.y, target.x, target.y
                        );
                        nav_state.clear();
                    }
                } else {
                    // Target reachable by simple walking
                    debug!(
                        "AI nav: Target ({:.0},{:.0}) reachable by walking, no nav needed",
                        target.x, target.y
                    );
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
    capabilities: Res<AiCapabilities>,
    profile_db: Res<AiProfileDatabase>,
    nav_graph: Res<NavGraph>,
    heatmaps: Res<HeatmapBundle>,
    level_db: Res<LevelDatabase>,
    current_level: Res<CurrentLevel>,
    mut event_bus: ResMut<EventBus>,
    mut ai_query: Query<
        (
            Entity,
            &Transform,
            &Team,
            Option<&Character>,
            &mut InputState,
            &mut AiState,
            &mut AiNavState,
            &TargetBasket,
            Option<&HoldingBall>,
            &Grounded,
        ),
        (With<Player>, Without<HumanControlled>, Without<RemoteControlled>),
    >,
    all_players: Query<
        (
            Entity,
            &Transform,
            &Team,
            Option<&Character>,
            Option<&HoldingBall>,
            Option<&HumanControlled>,
        ),
        With<Player>,
    >,
    ball_query: Query<(&Transform, &BallState, &Velocity), With<Ball>>,
    basket_query: Query<(&Transform, &Basket)>,
) {
    let level_settings = level_db
        .get_by_id(&current_level.0)
        .or_else(|| level_db.get_by_name(&current_level.0));
    let level_score_weight = level_settings
        .map(|level| level.heatmap_score_weight)
        .unwrap_or(1.0);
    let los_threshold = level_settings
        .map(|level| level.heatmap_los_threshold)
        .unwrap_or(HEATMAP_LOS_THRESHOLD_DEFAULT);
    let los_margin = level_settings
        .map(|level| level.heatmap_los_margin)
        .unwrap_or(HEATMAP_LOS_MARGIN_DEFAULT);

    for (
        ai_entity,
        ai_transform,
        ai_team,
        ai_character,
        mut input,
        mut ai_state,
        mut nav_state,
        target_basket,
        holding,
        grounded,
    ) in &mut ai_query
    {
        // Get AI profile for this player
        let profile = profile_db
            .get_by_id(&ai_state.profile_id)
            .unwrap_or_else(|| profile_db.default_profile());

        // Dummy profile: do nothing - skip all AI processing
        // This is used for unassigned characters that should stand still
        if profile.name == "Dummy" {
            input.move_x = 0.0;
            input.jump_held = false;
            input.pickup_pressed = false;
            input.throw_held = false;
            input.throw_released = false;
            continue;
        }

        // Idle goal: do nothing, skip all AI logic
        if ai_state.current_goal == AiGoal::Idle {
            input.move_x = 0.0;
            input.jump_held = false;
            input.pickup_pressed = false;
            input.throw_held = false;
            input.throw_released = false;
            continue;
        }

        // Update CatchPartner mode from profile
        ai_state.catch_partner_mode = profile.catch_partner;

        // Update KeepAwayAdversary mode from profile
        ai_state.keep_away_adversary_mode = profile.keep_away_adversary;

        // Initialize distance drill target if not set (default is 0.0 from derive(Default))
        if ai_state.catch_partner_mode && ai_state.distance_drill_target == 0.0 {
            ai_state.distance_drill_target = 1200.0;
            ai_state.distance_drill_shrinking = true; // Start by shrinking toward 100
        }

        // Decrement button press cooldown (simulates human mashing speed limit)
        // Use a minimum dt of 1/60 to handle headless mode where delta can be tiny
        let dt = time.delta_secs().max(1.0 / 60.0);
        ai_state.button_press_cooldown = (ai_state.button_press_cooldown - dt).max(0.0);

        // Decrement steal commitment timer
        ai_state.steal_commit_timer = (ai_state.steal_commit_timer - dt).max(0.0);

        // Decrement post-pass wait timer (for distance drill)
        ai_state.post_pass_wait_timer = (ai_state.post_pass_wait_timer - dt).max(0.0);

        let ai_pos = ai_transform.translation.truncate();

        // Get ball info
        let (ball_transform, ball_state, ball_velocity) = match ball_query.iter().next() {
            Some(b) => b,
            None => continue,
        };
        let ball_pos = ball_transform.translation.truncate();

        // Check if AI is holding the ball
        let ai_has_ball = holding.is_some();

        // Track ball hold time for desperation shots
        if ai_has_ball {
            // Detect when AI just caught the ball (first frame of holding)
            if ai_state.ball_hold_time == 0.0 && ai_state.catch_partner_mode {
                // AI just caught the ball - needs to reposition before passing
                ai_state.distance_drill_reposition = true;
                // Adjust distance for next iteration (oscillates 1200 → 100 → 1200)
                if ai_state.distance_drill_shrinking {
                    ai_state.distance_drill_target -= 100.0;
                    if ai_state.distance_drill_target <= 100.0 {
                        ai_state.distance_drill_target = 100.0;
                        ai_state.distance_drill_shrinking = false; // Start growing
                    }
                } else {
                    ai_state.distance_drill_target += 100.0;
                    if ai_state.distance_drill_target >= 1200.0 {
                        ai_state.distance_drill_target = 1200.0;
                        ai_state.distance_drill_shrinking = true; // Start shrinking
                    }
                }
            }
            ai_state.ball_hold_time += dt;
        } else {
            ai_state.ball_hold_time = 0.0;
        }

        // Collect opponents (different team) and teammates (same team, not self)
        let mut opponents: Vec<(Entity, Vec2, bool, bool, Option<CharacterId>)> = Vec::new();
        let mut teammate_pos: Option<Vec2> = None;

        for (e, tr, t, char_opt, holding_opt, human_opt) in &all_players {
            if e == ai_entity {
                continue;
            }
            let pos = tr.translation.truncate();
            let char_id = char_opt.map(|c| c.0);

            if t != ai_team {
                // Opponent
                opponents.push((e, pos, holding_opt.is_some(), human_opt.is_some(), char_id));
            } else {
                // Teammate (same team)
                teammate_pos = Some(pos);
            }
        }

        // Check if any opponent has ball
        let opponent_has_ball = opponents.iter().any(|(_, _, has_ball, _, _)| *has_ball);

        // Find nearest/most relevant opponent: prefer ball carrier, then human, then closest
        let opponent_pos = opponents
            .iter()
            .filter(|(_, _, has_ball, _, _)| *has_ball)
            .map(|(_, pos, _, _, _)| *pos)
            .next()
            .or_else(|| {
                opponents
                    .iter()
                    .filter(|(_, _, _, is_human, _)| *is_human)
                    .map(|(_, pos, _, _, _)| *pos)
                    .next()
            })
            .or_else(|| {
                opponents
                    .iter()
                    .min_by(|(_, pos_a, _, _, _), (_, pos_b, _, _, _)| {
                        let dist_a = ai_pos.distance(*pos_a);
                        let dist_b = ai_pos.distance(*pos_b);
                        dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(_, pos, _, _, _)| *pos)
            });

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
            match *ai_team {
                Team::Left => Vec2::new(-profile.defense_offset, defensive_y),
                Team::Right => Vec2::new(profile.defense_offset, defensive_y),
            }
        };

        // Decide current goal (using profile values)
        // KeepAwayAdversary mode: aggressive chase, intercept passes
        // CatchPartner mode: cooperative catch/pass behavior
        let new_goal = if ai_state.keep_away_adversary_mode {
            // KeepAwayAdversary AI: Chase ball carrier aggressively, predict passes
            // Find who has the ball (on the opponent team = Team::Left in keep-away)
            let ball_carrier = opponents
                .iter()
                .filter(|(_, _, has_ball, _, _)| *has_ball)
                .map(|(_, pos, _, _, _)| *pos)
                .next();

            if let Some(carrier_pos) = ball_carrier {
                let distance_to_carrier = ai_pos.distance(carrier_pos);

                // Within steal range: attempt steal
                if distance_to_carrier < profile.steal_range {
                    AiGoal::AttemptSteal
                } else if distance_to_carrier < profile.pressure_distance * 3.0 {
                    // Close enough for direct pressure
                    AiGoal::PressureBallCarrier
                } else {
                    // Far away: predict pass line and intercept
                    // Check if teammate exists (to predict pass target)
                    if teammate_pos.is_some() {
                        AiGoal::PredictIntercept
                    } else {
                        AiGoal::PressureBallCarrier
                    }
                }
            } else if matches!(ball_state, BallState::Free) {
                // Ball is loose - chase it
                AiGoal::ChaseBall
            } else if matches!(ball_state, BallState::PassInFlight { .. }) {
                // Ball in flight - try to intercept
                AiGoal::PredictIntercept
            } else {
                // Default: chase the ball
                AiGoal::ChaseBall
            }
        } else if ai_state.catch_partner_mode {
            // CatchPartner AI: cooperative catch/pass behavior for distance drill
            // Sequence: catch ball → reposition while holding → wait 1s → pass → wait 1s → (chase if close)
            if ai_has_ball {
                if ai_state.distance_drill_reposition {
                    // Step 2: Reposition to target distance while holding ball
                    AiGoal::RepositionToDistance
                } else {
                    // Step 3-4: Wait 1s then pass
                    AiGoal::HoldAndPass
                }
            } else if matches!(ball_state, BallState::PassInFlight { target, .. } if *target == ai_entity) {
                // Ball is being passed to us - track and receive it
                AiGoal::ReceivePass
            } else if matches!(ball_state, BallState::Free) {
                // Ball is loose - only chase if post_pass_wait expired AND ball is closer to AI than to human
                let ball_dist = ai_pos.distance(ball_pos);
                let human_dist = teammate_pos.map(|tm| tm.distance(ball_pos)).unwrap_or(f32::MAX);

                // Only chase if: wait timer expired AND ball is within 90% of distance to human
                if ai_state.post_pass_wait_timer <= 0.0 && ball_dist < human_dist * 0.9 {
                    AiGoal::ChaseMissedBall
                } else {
                    // Wait in open spot - don't chase balls closer to human
                    AiGoal::MoveToOpenSpot
                }
            } else if ai_state.distance_drill_reposition {
                // Need to reposition (human just picked up ball)
                AiGoal::RepositionToDistance
            } else {
                // Teammate has ball or ball in flight to teammate - move to open position
                AiGoal::MoveToOpenSpot
            }
        } else if ai_has_ball {
            // Check if AI is in "front court" (front 1/3 of arena, close to target basket)
            // Front court = near target basket, where shots are too close/easy to block
            // If targeting right basket (x > 0): front court = right 1/3 (x > ARENA_WIDTH/6)
            // If targeting left basket (x < 0): front court = left 1/3 (x < -ARENA_WIDTH/6)
            let front_court_threshold = ARENA_WIDTH / 6.0;
            let in_front_court = if target_basket_pos.x > 0.0 {
                ai_pos.x > front_court_threshold // Right 1/3 when targeting right
            } else {
                ai_pos.x < -front_court_threshold // Left 1/3 when targeting left
            };

            // Apply front-court penalty: reduce effective shot quality when too close
            // This discourages but doesn't prevent close-range shots
            let front_court_quality_penalty = if in_front_court { 0.15 } else { 0.0 };

            // Check if AI is behind the basket (impossible to score from there)
            // For left basket: AI must be to the right of it (ai_pos.x > basket.x)
            // For right basket: AI must be to the left of it (ai_pos.x < basket.x)
            let is_behind_basket = if target_basket_pos.x < 0.0 {
                ai_pos.x < target_basket_pos.x // Behind left basket
            } else {
                ai_pos.x > target_basket_pos.x // Behind right basket
            };

            // Check if already charging - once committed, don't abort
            let already_charging = ai_state.current_goal == AiGoal::ChargeShot;

            // Force shot after holding ball for 2+ seconds (prevents stalling)
            // BUT NOT if behind the basket - that would guarantee a miss
            // Reduced from 3s to increase shooting activity
            if ai_state.ball_hold_time > 2.0 && !is_behind_basket {
                AiGoal::ChargeShot
            } else if is_behind_basket && !already_charging {
                // Behind basket and not yet charging - must reposition first
                AiGoal::AttackWithBall
            } else if already_charging {
                // Already charging - commit to the shot (don't abort mid-charge)
                AiGoal::ChargeShot
            } else {
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
                // Apply front-court penalty to discourage close-range shots
                let base_quality = (evaluate_shot_quality(ai_pos, target_basket_pos)
                    - front_court_quality_penalty)
                    .clamp(0.0, 1.0);
                let score_heatmap = heatmaps.score_for_basket(target_basket_type, ai_pos);
                let heatmap_multiplier =
                    1.0 + (HEATMAP_SCORE_WEIGHT_DEFAULT * level_score_weight * score_heatmap);
                let shot_quality = (base_quality * heatmap_multiplier).clamp(0.0, 1.0);

                let los_value = heatmaps.line_of_sight_for_basket(target_basket_type, ai_pos);
                let los_ok = los_value + los_margin >= los_threshold;

                // Scale min_shot_quality based on what's achievable on this level
                // Flat levels (max ~0.50) get lower thresholds so AI still shoots
                let level_scaled_min = scale_min_quality_for_level(
                    profile.min_shot_quality,
                    nav_graph.level_max_shot_quality,
                );

                // Desperation factor: after 1 second holding ball, gradually lower threshold
                // Ramps from 1.0 at 1s to 0.5 at 2s (when force shot kicks in)
                // This encourages shooting sooner rather than waiting for perfect position
                let desperation_factor = if ai_state.ball_hold_time > 1.0 {
                    1.0 - ((ai_state.ball_hold_time - 1.0) * 0.5).min(0.5)
                } else {
                    1.0
                };
                let effective_min_quality = level_scaled_min * desperation_factor;
                let quality_acceptable = shot_quality >= effective_min_quality && los_ok;

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
                // Use actual STEAL_RANGE constant * 1.5 (90px) rather than profile.steal_range
                // which can be much larger and block shooting opportunities unnecessarily
                let already_charging = ai_state.current_goal == AiGoal::ChargeShot;
                let opponent_too_close = !already_charging
                    && opponent_pos
                        .map(|opp| ai_pos.distance(opp) < STEAL_RANGE * 1.5)
                        .unwrap_or(false);

                // Calculate utility of seeking a better position vs shooting now
                // Only consider seeking if current position meets basic shooting criteria
                let should_seek = if quality_acceptable && in_shoot_range && !already_charging {
                    if let Some(best_node_idx) =
                        nav_graph.find_best_shot_position(target_basket_pos)
                    {
                        let best_node = &nav_graph.nodes[best_node_idx];
                        let best_quality =
                            nav_graph.get_shot_quality(best_node_idx, target_basket_pos);
                        let quality_gain = best_quality - shot_quality;

                        // Only seek if there's SIGNIFICANT quality to gain (10%+ improvement)
                        // Was 0.01 which made AI too hesitant to shoot from "good enough" spots
                        if quality_gain > 0.10 {
                            // Opportunity cost factors:
                            // 1. Path cost (time/risk to reach better position)
                            let path_cost = nav_graph.estimate_path_cost(ai_pos, best_node_idx);
                            let path_cost_normalized = (path_cost / 400.0).min(1.0);

                            // 2. Opponent pressure (closer opponent = higher cost to seek)
                            let opponent_pressure = opponent_pos
                                .map(|opp| 1.0 - (ai_pos.distance(opp) / 300.0).min(1.0))
                                .unwrap_or(0.0);

                            // 3. Height bonus - strongly reward seeking elevated platforms
                            // Tournament data shows elevated shots (Y > -300) have 20-35% success
                            // vs floor shots (Y < -350) at only 3-10% success
                            let height_above_floor =
                                (best_node.top_y - ARENA_FLOOR_Y - 50.0).max(0.0);
                            let height_bonus = (height_above_floor / 200.0).min(0.5) * 0.6; // Up to +0.30

                            // 4. Floor penalty - urgently seek height if currently at floor level
                            // This helps even low-patience profiles climb before shooting
                            // Increased from 0.15 to 0.25 based on tournament data showing
                            // floor shots have 9-13% success vs elevated shots at 20-35%
                            let current_height = ai_pos.y - ARENA_FLOOR_Y;
                            let at_floor_level = current_height < 100.0;
                            let floor_urgency = if at_floor_level { 0.25 } else { 0.0 };

                            // Utility = quality_gain + height_bonus + floor_urgency - opportunity_costs
                            // Floor urgency bypasses patience scaling to help impatient profiles seek height
                            let raw_utility = quality_gain + height_bonus
                                - (path_cost_normalized * 0.15)
                                - (opponent_pressure * 0.2);
                            let seek_utility =
                                raw_utility * profile.position_patience + floor_urgency;

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

                // Check if passing to teammate is a better option
                let should_pass = if profile.pass_willingness > 0.0 && !already_charging {
                    if let Some(tm_pos) = teammate_pos {
                        // Get teammate's shot quality from their position
                        let tm_quality = heatmaps.score_for_basket(target_basket_type, tm_pos);
                        let tm_los = heatmaps.line_of_sight_for_basket(target_basket_type, tm_pos);

                        // Teammate must have reasonable line of sight
                        let tm_has_los = tm_los + los_margin >= los_threshold;

                        // Pass conditions:
                        // 1. Teammate has better shot quality than us
                        // 2. We're under pressure (opponent close)
                        // 3. Random check against pass_willingness
                        let quality_advantage = tm_quality - shot_quality;
                        let under_pressure = opponent_pos
                            .map(|opp| ai_pos.distance(opp) < STEAL_RANGE * 2.0)
                            .unwrap_or(false);

                        // More likely to pass when:
                        // - Teammate has significantly better position (quality_advantage > 0.15)
                        // - OR we're under pressure AND teammate has any advantage
                        let pass_utility = if quality_advantage > 0.15 {
                            0.8 // Strong advantage - very likely to pass
                        } else if under_pressure && quality_advantage > 0.0 && tm_has_los {
                            0.6 // Under pressure with teammate advantage
                        } else if under_pressure && ai_state.ball_hold_time > 1.5 {
                            0.4 // Desperate - held too long under pressure
                        } else {
                            0.0 // No good reason to pass
                        };

                        // Random check: pass if pass_utility * pass_willingness exceeds threshold
                        let pass_threshold = 0.3;
                        let pass_roll = rand::thread_rng().gen_range(0.0..1.0_f32);
                        pass_utility * profile.pass_willingness > pass_threshold
                            && pass_roll < pass_utility
                    } else {
                        false // No teammate
                    }
                } else {
                    false // pass_willingness is 0 or already charging
                };

                // Only shoot if shot quality is acceptable AND position conditions are met
                // AND opponent isn't too close (or we're already committed to the shot)
                // AND we're not deciding to seek a better position
                if should_pass {
                    AiGoal::PassToTeammate
                } else if quality_acceptable
                    && (in_shoot_range || reached_target)
                    && !opponent_too_close
                    && !should_seek
                {
                    AiGoal::ChargeShot
                } else if already_charging {
                    // Commit to the shot once started
                    AiGoal::ChargeShot
                } else {
                    // Debug: log why AI isn't shooting (once per second to avoid spam)
                    if ai_state.ball_hold_time > 0.5
                        && (ai_state.ball_hold_time * 10.0) as u32 % 10 == 0
                    {
                        info!(
                            "AI NOT SHOOTING: quality={:.2} (need>={:.2}) ok={} | los={:.2} (need>={:.2}) ok={} | range={:.0} (need<{:.0}) ok={} | opp_dist={:.0} close={} | seek={}",
                            shot_quality,
                            effective_min_quality,
                            quality_acceptable,
                            los_value,
                            los_threshold,
                            los_ok,
                            effective_distance,
                            profile.shoot_range,
                            in_shoot_range,
                            opponent_pos.map(|o| ai_pos.distance(o)).unwrap_or(999.0),
                            opponent_too_close,
                            should_seek
                        );
                    }
                    AiGoal::AttackWithBall
                }
            } // End of else block for forced shot after 12s
        } else if matches!(ball_state, BallState::PassInFlight { target, .. } if *target == ai_entity) {
            // Teammate is passing to us - receive the pass
            AiGoal::ReceivePass
        } else if opponent_has_ball {
            // Update steal proximity tracking BEFORE goal decision
            // This ensures timer persists across goal switches
            // IMPORTANT: Use the game's STEAL_RANGE constant (60px), not profile.steal_range
            // The profile.steal_range is for goal selection, but actual steals require STEAL_RANGE
            if let Some(opp_pos) = opponent_pos {
                let distance = ai_pos.distance(opp_pos);
                let in_steal_range = distance < STEAL_RANGE;

                if in_steal_range {
                    if !ai_state.was_in_steal_range {
                        ai_state.steal_reaction_timer = 0.0;
                    }
                    ai_state.steal_reaction_timer += dt;
                } else {
                    ai_state.steal_reaction_timer = 0.0;
                }
                ai_state.was_in_steal_range = in_steal_range;
            }

            if let Some(opp_pos) = opponent_pos {
                let distance_to_opponent = ai_pos.distance(opp_pos);
                // Calculate effective pressure threshold based on profile
                // Higher aggression = tighter pressure, lower = zone defense
                // Use larger multiplier (1.5 base) for wider PressureDefense window
                let pressure_threshold =
                    profile.pressure_distance * (1.5 + (1.0 - profile.aggression));

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

                // Check if currently committed to a steal attempt
                let steal_committed = ai_state.current_goal == AiGoal::AttemptSteal
                    && ai_state.steal_commit_timer > 0.0;

                if steal_committed {
                    // Stay in AttemptSteal until commitment expires
                    AiGoal::AttemptSteal
                } else if !is_defensive_goal || time_since_switch > 0.4 {
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
            // Start steal commitment timer when entering AttemptSteal
            if new_goal == AiGoal::AttemptSteal {
                ai_state.steal_commit_timer = 0.5; // Commit for 0.5s
                // Only clear nav if opponent is at reachable height
                // Navigation will be set up in ai_navigation_update if opponent is elevated
                if let Some(opp_pos) = opponent_pos {
                    let height_diff = opp_pos.y - ai_pos.y;
                    if height_diff <= PLAYER_SIZE.y * 1.5 {
                        nav_state.clear(); // Opponent reachable, use simple horizontal chase
                    }
                    // If elevated, keep nav active to reach their platform
                } else {
                    nav_state.clear();
                }
            }
            if new_goal == AiGoal::ChargeShot {
                let mut rng = rand::thread_rng();
                ai_state.shot_charge_target = rng.gen_range(profile.charge_min..profile.charge_max);
            }
        }

        // Decrement stuck reversal timer
        ai_state.stuck_reverse_timer = (ai_state.stuck_reverse_timer - dt).max(0.0);

        // Reset inputs each frame (will be set below)
        input.move_x = 0.0;
        input.jump_held = false;
        input.turbo_held = false;
        input.block_pressed = false;
        input.pass_pressed = false;
        // Don't reset jump_buffer_timer - it counts down
        // Don't reset throw_released - it's consumed by throw system

        // Check if navigation is controlling movement
        let nav_controlling = nav_state.active && !nav_state.path_complete();

        // Log nav state periodically (once per second to avoid spam)
        if ai_state.ball_hold_time > 0.1 && (ai_state.ball_hold_time * 10.0) as u32 % 10 == 0 {
            if nav_controlling {
                debug!(
                    "AI nav state: ACTIVE, {} actions remaining, target {:?}, current action: {:?}",
                    nav_state.current_path.len() - nav_state.path_index,
                    nav_state.nav_target,
                    nav_state.current_action()
                );
            } else {
                debug!(
                    "AI nav state: INACTIVE (active={}, path_complete={})",
                    nav_state.active, nav_state.path_complete()
                );
            }
        }

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
                    // Move toward target basket
                    let dx = target_basket_pos.x - ai_pos.x;
                    if dx.abs() > profile.position_tolerance {
                        input.move_x = dx.signum();
                    }

                    // Jump fallback: when navigation fails and basket is elevated,
                    // actively seek elevated positions by jumping toward ramps/platforms
                    let floor_y = ARENA_FLOOR_Y + PLAYER_SIZE.y / 2.0;
                    let ai_at_floor = ai_pos.y < floor_y + PLAYER_SIZE.y;
                    let basket_elevated = target_basket_pos.y > floor_y + PLAYER_SIZE.y * 2.0;

                    if ai_at_floor && basket_elevated && grounded.0 {
                        // AI is on floor but basket is elevated - need to climb
                        // Move toward the corner ramp on the basket's side to gain elevation
                        let arena_edge = ARENA_WIDTH / 2.0 - WALL_THICKNESS - 100.0;
                        let ramp_target_x = if target_basket_pos.x > 0.0 {
                            arena_edge // Target right basket, use right ramp
                        } else {
                            -arena_edge // Target left basket, use left ramp
                        };

                        let ramp_dx = ramp_target_x - ai_pos.x;
                        if ramp_dx.abs() > profile.position_tolerance {
                            input.move_x = ramp_dx.signum();
                        }

                        // Jump continuously when on/near ramp area to climb steps
                        let on_ramp_area =
                            ai_pos.x.abs() > ARENA_WIDTH / 2.0 - WALL_THICKNESS - 300.0;
                        if on_ramp_area {
                            input.jump_buffer_timer = JUMP_BUFFER_TIME;
                            input.jump_held = true;
                        }
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

                        // Drift toward optimal shooting position (not basket wall)
                        // Optimal is ~200px in front of basket
                        let basket_is_left = target_basket_pos.x < 0.0;
                        let optimal_x = if basket_is_left {
                            target_basket_pos.x + 200.0 // Right of left basket
                        } else {
                            target_basket_pos.x - 200.0 // Left of right basket
                        };
                        // Only drift if significantly mispositioned
                        let dx = optimal_x - ai_pos.x;
                        if dx.abs() > 50.0 {
                            input.move_x = dx.signum() * 0.2;
                        } else {
                            input.move_x = 0.0; // Stay put
                        }

                        // Reset if landed without shooting (give more time like player has)
                        if grounded.0 && ai_state.jump_shot_timer > 1.0 {
                            ai_state.jump_shot_active = false;
                        }
                    } else {
                        // Normal ground shot
                        input.move_x = 0.0;

                        if !input.throw_held && !input.throw_released {
                            input.throw_held = true;
                            ai_state.shot_charge_target = rand::thread_rng()
                                .gen_range(profile.charge_min..profile.charge_max);
                        } else if input.throw_held {
                            ai_state.shot_charge_target -= dt;
                            if ai_state.shot_charge_target <= 0.0 {
                                input.throw_held = false;
                                input.throw_released = true;
                            }
                        }
                    }
                }

                AiGoal::PassToTeammate => {
                    // Execute pass to teammate
                    input.move_x = 0.0; // Stay still while passing
                    if ai_state.button_press_cooldown <= 0.0 {
                        input.pass_pressed = true;
                        ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
                    }
                    input.throw_held = false;
                }

                AiGoal::AttemptSteal => {
                    // Chase opponent aggressively during steal attempts
                    if let Some(opp_pos) = opponent_pos {
                        let height_diff = opp_pos.y - ai_pos.y;

                        // If opponent is elevated and we have a nav path, follow it
                        if height_diff > PLAYER_SIZE.y * 1.5 && nav_state.active {
                            execute_nav_action(
                                &mut input,
                                &mut nav_state,
                                ai_pos,
                                grounded.0,
                                &time,
                            );
                            nav_state.update_completion();
                        } else {
                            // Direct pursuit logic for reachable opponents
                            let dx = opp_pos.x - ai_pos.x;
                            let distance = ai_pos.distance(opp_pos);

                            // Check for ceiling before deciding to jump
                            let has_ceiling = has_ceiling_above(ai_pos, &capabilities, &nav_graph);

                            if height_diff > PLAYER_SIZE.y * 0.5 && has_ceiling && grounded.0 {
                                // Ceiling is blocking our jump - move to escape position first
                                if let Some(escape_x) =
                                    find_escape_x(ai_pos, opp_pos.y, &capabilities, &nav_graph)
                                {
                                    let escape_dx = escape_x - ai_pos.x;
                                    input.move_x = escape_dx.signum();
                                    // Don't jump while escaping from under ceiling
                                } else {
                                    // No escape found, try moving toward opponent anyway
                                    if dx.abs() > 10.0 {
                                        input.move_x = dx.signum();
                                    }
                                }
                            } else {
                                // No ceiling blocking - use normal pursuit logic
                                // Always move toward opponent unless extremely close (< 10px)
                                if dx.abs() > 10.0 {
                                    input.move_x = dx.signum();
                                } else if distance > profile.steal_range * 0.5 {
                                    // If within 10px horizontally but still far vertically,
                                    // keep moving to maintain contact
                                    input.move_x = dx.signum();
                                }

                                // Jump if opponent is above us (and no ceiling blocking)
                                if height_diff > PLAYER_SIZE.y * 0.5 {
                                    if grounded.0 {
                                        input.jump_buffer_timer = JUMP_BUFFER_TIME;
                                    }
                                    input.jump_held = true; // Maintain jump while stealing
                                }
                            }

                            // Keep holding jump while airborne if still need to gain height
                            if !grounded.0 && height_diff > PLAYER_SIZE.y {
                                input.jump_held = true;
                            }
                        }

                        // Attempt steal if timer met and cooldown ready
                        // (steal proximity tracking is centralized before goal decision)
                        if ai_state.steal_reaction_timer >= profile.steal_reaction_time
                            && ai_state.button_press_cooldown <= 0.0
                            && ai_state.was_in_steal_range
                        {
                            input.pickup_pressed = true;
                            ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
                        }
                    }
                    input.throw_held = false;
                }

                AiGoal::InterceptDefense => {
                    // When at same height level as opponent, chase them directly
                    // rather than targeting a static intercept point
                    if let Some(opp_pos) = opponent_pos {
                        let distance_to_opponent = ai_pos.distance(opp_pos);
                        let height_diff = opp_pos.y - ai_pos.y;
                        let same_height_level = height_diff.abs() < PLAYER_SIZE.y * 1.5;

                        // Check if opponent is significantly elevated (needs climbing)
                        let floor_y = ARENA_FLOOR_Y + PLAYER_SIZE.y;
                        let ai_near_floor = ai_pos.y < floor_y + PLAYER_SIZE.y;
                        let opponent_highly_elevated = height_diff > NAV_MAX_JUMP_HEIGHT * 0.5;

                        // Check for ceiling before deciding to jump
                        let has_ceiling = has_ceiling_above(ai_pos, &capabilities, &nav_graph);

                        // When far from opponent (> 300px), chase directly UNLESS opponent is
                        // highly elevated and we're on the floor (need to climb, not chase)
                        let very_far = distance_to_opponent > 300.0;

                        // PRIORITY 1: If opponent is highly elevated and AI is on floor, climb first
                        // This takes precedence over "very far" because horizontal chase won't help
                        if ai_near_floor && opponent_highly_elevated {
                            // Opponent is high above and we're at ground level
                            // Move toward the ramp on the OPPONENT'S side to climb and intercept
                            let arena_edge = ARENA_WIDTH / 2.0 - WALL_THICKNESS - 100.0;
                            let ramp_target_x = if opp_pos.x > 0.0 {
                                arena_edge // Opponent on right side, use right ramp
                            } else {
                                -arena_edge // Opponent on left side, use left ramp
                            };

                            let dx = ramp_target_x - ai_pos.x;
                            if dx.abs() > profile.position_tolerance {
                                input.move_x = dx.signum();
                            }

                            // Jump continuously when on the ramp area to climb steps
                            // (ramps are designed for climbing, ceiling check not needed)
                            let on_ramp_area =
                                ai_pos.x.abs() > ARENA_WIDTH / 2.0 - WALL_THICKNESS - 300.0;
                            if on_ramp_area && grounded.0 {
                                input.jump_buffer_timer = JUMP_BUFFER_TIME;
                                input.jump_held = true;
                            }
                        } else if very_far || same_height_level {
                            // PRIORITY 2: Chase opponent directly when very far or same height
                            let chase_tolerance = if very_far {
                                profile.position_tolerance
                            } else {
                                profile.steal_range * 0.3 // ~38px - closer than steal range
                            };
                            let dx = opp_pos.x - ai_pos.x;

                            // Check if we need to escape from under a ceiling first
                            if height_diff > PLAYER_SIZE.y * 0.5 && has_ceiling && grounded.0 {
                                // Ceiling blocking - move to escape position
                                if let Some(escape_x) =
                                    find_escape_x(ai_pos, opp_pos.y, &capabilities, &nav_graph)
                                {
                                    let escape_dx = escape_x - ai_pos.x;
                                    input.move_x = escape_dx.signum();
                                    // Don't jump while escaping
                                } else if dx.abs() > chase_tolerance {
                                    input.move_x = dx.signum();
                                }
                            } else {
                                if dx.abs() > chase_tolerance {
                                    input.move_x = dx.signum();
                                }

                                // Jump if opponent is above us (no ceiling blocking)
                                if height_diff > PLAYER_SIZE.y * 0.5 && grounded.0 {
                                    input.jump_buffer_timer = JUMP_BUFFER_TIME;
                                    input.jump_held = true;
                                }
                            }
                        } else {
                            // PRIORITY 3: Moderate height difference - chase + jump
                            let dx = opp_pos.x - ai_pos.x;

                            // Check if we need to escape from under a ceiling first
                            if height_diff > PLAYER_SIZE.y * 0.5 && has_ceiling && grounded.0 {
                                // Ceiling blocking - move to escape position
                                if let Some(escape_x) =
                                    find_escape_x(ai_pos, opp_pos.y, &capabilities, &nav_graph)
                                {
                                    let escape_dx = escape_x - ai_pos.x;
                                    input.move_x = escape_dx.signum();
                                    // Don't jump while escaping
                                } else if dx.abs() > profile.position_tolerance {
                                    input.move_x = dx.signum();
                                }
                            } else {
                                if dx.abs() > profile.position_tolerance {
                                    input.move_x = dx.signum();
                                }

                                // Jump when opponent is above and we're close horizontally
                                if height_diff > PLAYER_SIZE.y * 0.5 && grounded.0 {
                                    input.jump_buffer_timer = JUMP_BUFFER_TIME;
                                    input.jump_held = true;
                                }
                            }
                        }

                        // Hold jump while airborne if still need to gain height
                        if !grounded.0 && height_diff > PLAYER_SIZE.y {
                            input.jump_held = true;
                        }
                    } else {
                        // No opponent visible - use intercept position
                        let dx = intercept_pos.x - ai_pos.x;
                        if dx.abs() > profile.position_tolerance {
                            input.move_x = dx.signum();
                        }
                    }

                    input.pickup_pressed = false;
                    input.throw_held = false;
                }

                AiGoal::PressureDefense => {
                    // Close-range: chase opponent aggressively and attempt steals
                    if let Some(opp_pos) = opponent_pos {
                        let dx = opp_pos.x - ai_pos.x;
                        let dy = opp_pos.y - ai_pos.y;

                        // Check for ceiling before deciding to jump
                        let has_ceiling = has_ceiling_above(ai_pos, &capabilities, &nav_graph);

                        if dy > PLAYER_SIZE.y * 0.5 && has_ceiling && grounded.0 {
                            // Ceiling blocking - move to escape position first
                            if let Some(escape_x) =
                                find_escape_x(ai_pos, opp_pos.y, &capabilities, &nav_graph)
                            {
                                let escape_dx = escape_x - ai_pos.x;
                                input.move_x = escape_dx.signum();
                                // Don't jump while escaping
                            } else if dx.abs() > profile.position_tolerance * 0.5 {
                                input.move_x = dx.signum();
                            }
                        } else {
                            // No ceiling blocking - move directly toward opponent
                            if dx.abs() > profile.position_tolerance * 0.5 {
                                input.move_x = dx.signum();
                            }

                            // Jump if opponent is above us
                            if dy > PLAYER_SIZE.y * 0.5 && grounded.0 {
                                input.jump_buffer_timer = JUMP_BUFFER_TIME;
                                input.jump_held = true;
                            }
                        }

                        // Keep holding jump while airborne to reach elevated opponents
                        if !grounded.0 && dy > PLAYER_SIZE.y {
                            input.jump_held = true;
                        }

                        // Attempt steal if timer met and cooldown ready
                        // (steal proximity tracking is centralized before goal decision)
                        if ai_state.steal_reaction_timer >= profile.steal_reaction_time
                            && ai_state.button_press_cooldown <= 0.0
                            && ai_state.was_in_steal_range
                        {
                            input.pickup_pressed = true;
                            ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
                        }
                    }
                    input.throw_held = false;
                }

                // === CatchPartner goals - debug teammate behavior ===
                AiGoal::MoveToOpenSpot => {
                    // Move to open position to receive a pass
                    // Use distance_drill_target for spacing
                    let tm_pos = all_players
                        .iter()
                        .filter(|(e, _, t, _, _, _)| *e != ai_entity && *t == ai_team)
                        .map(|(_, tr, _, _, _, _)| tr.translation.truncate())
                        .next();

                    if let Some(tm) = tm_pos {
                        // Position at distance_drill_target from teammate
                        let direction = if ai_pos.x > tm.x { 1.0 } else { -1.0 };
                        let target_x = tm.x + (direction * ai_state.distance_drill_target);
                        let dx = target_x - ai_pos.x;
                        if dx.abs() > profile.position_tolerance {
                            input.move_x = dx.signum();
                        }
                    }
                    input.throw_held = false;
                }

                AiGoal::ReceivePass => {
                    // Predict where ball will land and only move if needed
                    // This prevents AI from running toward passer when pass is incoming

                    let catch_tolerance = BALL_PICKUP_RADIUS * 1.5; // How close is "close enough"

                    // Calculate time for ball to reach AI's y-position using projectile motion
                    // y = y0 + vy*t - 0.5*g*t^2
                    // Solving for when ball_y = ai_y
                    let dy_to_ai = ai_pos.y - ball_pos.y;
                    let ball_vel = ball_velocity.0;
                    let gravity = BALL_GRAVITY;

                    // Quadratic formula: 0.5*g*t^2 - vy*t + dy = 0
                    let a = 0.5 * gravity;
                    let b = -ball_vel.y;
                    let c = dy_to_ai;
                    let discriminant = b * b - 4.0 * a * c;

                    let predicted_x = if discriminant >= 0.0 && a.abs() > 0.001 {
                        // Take positive root (future time)
                        let t1 = (-b + discriminant.sqrt()) / (2.0 * a);
                        let t2 = (-b - discriminant.sqrt()) / (2.0 * a);
                        let t = if t1 > 0.0 && t2 > 0.0 {
                            t1.min(t2)
                        } else {
                            t1.max(t2).max(0.0)
                        };
                        ball_pos.x + ball_vel.x * t
                    } else {
                        ball_pos.x // Fallback to current ball position
                    };

                    // Only move if predicted landing is outside catch tolerance
                    let dx = predicted_x - ai_pos.x;
                    if dx.abs() > catch_tolerance {
                        input.move_x = dx.signum();
                    }
                    // Otherwise stay in place

                    // Only jump if ball is DESCENDING and will remain too high to catch
                    // Don't jump for high-arc passes that will come down on their own
                    let ball_descending = ball_vel.y < 0.0;
                    let ball_close_horizontally =
                        (ball_pos.x - ai_pos.x).abs() < BALL_PICKUP_RADIUS * 3.0;

                    if ball_descending && ball_close_horizontally && grounded.0 {
                        // Predict ball height when it reaches AI's x position
                        let time_to_ai_x = if ball_vel.x.abs() > 0.1 {
                            ((ai_pos.x - ball_pos.x) / ball_vel.x).abs()
                        } else {
                            0.0
                        };
                        // Ball height at that time: y = y0 + vy*t - 0.5*g*t^2
                        let predicted_ball_y = ball_pos.y + ball_vel.y * time_to_ai_x
                            - 0.5 * BALL_GRAVITY * time_to_ai_x * time_to_ai_x;
                        let dy_predicted = predicted_ball_y - ai_pos.y;

                        // Only jump if ball will still be above catch height
                        if dy_predicted > PLAYER_SIZE.y * 0.8 {
                            input.jump_buffer_timer = JUMP_BUFFER_TIME;
                            input.jump_held = true;
                        }
                    }

                    // Try to catch the ball when close
                    let distance_to_ball = ai_pos.distance(ball_pos);
                    if distance_to_ball < BALL_PICKUP_RADIUS
                        && ai_state.button_press_cooldown <= 0.0
                    {
                        input.pickup_pressed = true;
                        ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
                    }
                    input.throw_held = false;
                }

                AiGoal::ChaseMissedBall => {
                    // Same as ChaseBall - move toward ball and pick it up
                    let dx = ball_pos.x - ai_pos.x;
                    if dx.abs() > profile.position_tolerance {
                        input.move_x = dx.signum();
                    }

                    let dy = ball_pos.y - ai_pos.y;
                    if dy > PLAYER_SIZE.y && dx.abs() < BALL_PICKUP_RADIUS * 2.0 && grounded.0 {
                        input.jump_buffer_timer = JUMP_BUFFER_TIME;
                        input.jump_held = true;
                    }

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

                AiGoal::HoldAndPass => {
                    // Hold the ball, count up timer, then pass back to teammate
                    ai_state.hold_and_pass_timer += dt;

                    // After 1 second, pass to teammate
                    if ai_state.hold_and_pass_timer >= 1.0 {
                        input.pass_pressed = true;
                        ai_state.hold_and_pass_timer = 0.0;
                        // Set post-pass wait timer (1s before chasing loose balls)
                        ai_state.post_pass_wait_timer = 1.0;
                    }
                    input.throw_held = false;
                }

                AiGoal::RepositionToDistance => {
                    // Move to target distance from teammate (works with or without ball)
                    let tm_pos = all_players
                        .iter()
                        .filter(|(e, _, t, _, _, _)| *e != ai_entity && *t == ai_team)
                        .map(|(_, tr, _, _, _, _)| tr.translation.truncate())
                        .next();

                    if let Some(tm) = tm_pos {
                        // Calculate target x: maintain distance on the side we're already on
                        let direction = if ai_pos.x > tm.x { 1.0 } else { -1.0 };
                        let target_x = tm.x + (direction * ai_state.distance_drill_target);
                        let dx = target_x - ai_pos.x;

                        if dx.abs() > 20.0 {
                            // Still moving to position
                            input.move_x = dx.signum();
                        } else {
                            // Arrived at target distance
                            // Clear reposition flag - will transition to HoldAndPass if holding ball
                            // (distance is decreased when AI catches ball, not here)
                            ai_state.distance_drill_reposition = false;
                        }
                    } else {
                        // No teammate, just clear reposition flag
                        ai_state.distance_drill_reposition = false;
                    }
                    input.throw_held = false;
                }

                // === KeepAway goals ===
                AiGoal::EvadeAndPass => {
                    // Teammate: evade adversary pressure, pass when safe
                    // Find adversary position
                    let adversary_pos = opponents
                        .iter()
                        .map(|(_, pos, _, _, _)| *pos)
                        .next();

                    if let Some(adv_pos) = adversary_pos {
                        let distance_to_adversary = ai_pos.distance(adv_pos);

                        // If adversary is close, move away
                        if distance_to_adversary < 150.0 {
                            let dx = ai_pos.x - adv_pos.x; // Move away from adversary
                            if dx.abs() > 10.0 {
                                input.move_x = dx.signum();
                            }
                            // Use turbo to escape
                            input.turbo_held = true;
                        } else if ai_has_ball && distance_to_adversary > 200.0 {
                            // Safe distance with ball - pass to teammate
                            if ai_state.button_press_cooldown <= 0.0 {
                                input.pass_pressed = true;
                                ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
                            }
                        }
                    } else if ai_has_ball {
                        // No adversary visible with ball - pass to teammate
                        if ai_state.button_press_cooldown <= 0.0 {
                            input.pass_pressed = true;
                            ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
                        }
                    }
                    input.throw_held = false;
                }

                AiGoal::PressureBallCarrier => {
                    // Adversary: aggressive direct chase of ball carrier
                    let ball_carrier_pos = opponents
                        .iter()
                        .filter(|(_, _, has_ball, _, _)| *has_ball)
                        .map(|(_, pos, _, _, _)| *pos)
                        .next()
                        .unwrap_or(ball_pos);

                    let dx = ball_carrier_pos.x - ai_pos.x;
                    let dy = ball_carrier_pos.y - ai_pos.y;
                    let distance = ai_pos.distance(ball_carrier_pos);

                    // Always move toward carrier
                    if dx.abs() > 10.0 {
                        input.move_x = dx.signum();
                    }

                    // Jump if carrier is elevated
                    if dy > PLAYER_SIZE.y * 0.5 && grounded.0 {
                        input.jump_buffer_timer = JUMP_BUFFER_TIME;
                        input.jump_held = true;
                    }

                    // Keep holding jump while airborne
                    if !grounded.0 && dy > PLAYER_SIZE.y {
                        input.jump_held = true;
                    }

                    // Use turbo for aggressive chase
                    if distance > 100.0 {
                        input.turbo_held = true;
                    }

                    // Attempt steal when in range
                    if distance < STEAL_RANGE
                        && ai_state.steal_reaction_timer >= profile.steal_reaction_time
                        && ai_state.button_press_cooldown <= 0.0
                    {
                        input.pickup_pressed = true;
                        ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
                    }

                    input.throw_held = false;
                }

                AiGoal::PredictIntercept => {
                    // Adversary: position between ball carrier and pass target
                    let ball_carrier_pos = opponents
                        .iter()
                        .filter(|(_, _, has_ball, _, _)| *has_ball)
                        .map(|(_, pos, _, _, _)| *pos)
                        .next();

                    // Find the other opponent (pass target)
                    let pass_target_pos = opponents
                        .iter()
                        .filter(|(_, _, has_ball, _, _)| !*has_ball)
                        .map(|(_, pos, _, _, _)| *pos)
                        .next();

                    let target = if let (Some(carrier), Some(pass_target)) =
                        (ball_carrier_pos, pass_target_pos)
                    {
                        // Position 30% along the line from carrier to target
                        carrier + (pass_target - carrier) * 0.3
                    } else {
                        // No clear pass line, just chase ball
                        ball_pos
                    };

                    let dx = target.x - ai_pos.x;
                    let dy = target.y - ai_pos.y;

                    // Move toward intercept position
                    if dx.abs() > profile.position_tolerance {
                        input.move_x = dx.signum();
                    }

                    // Jump if target is elevated
                    if dy > PLAYER_SIZE.y * 0.5 && grounded.0 {
                        input.jump_buffer_timer = JUMP_BUFFER_TIME;
                        input.jump_held = true;
                    }

                    // Try to pick up ball if it's in flight nearby (intercept)
                    let distance_to_ball = ai_pos.distance(ball_pos);
                    if distance_to_ball < BALL_PICKUP_RADIUS * 1.5
                        && ai_state.button_press_cooldown <= 0.0
                    {
                        input.pickup_pressed = true;
                        ai_state.button_press_cooldown = 1.0 / profile.button_presses_per_sec;
                    }

                    input.throw_held = false;
                }
            }
        }

        // Teammate crowding avoidance: if too close to teammate, adjust movement
        // This prevents AI teammates from bunching up on the same spot
        if let Some(tm_pos) = teammate_pos {
            let distance_to_teammate = ai_pos.distance(tm_pos);
            if distance_to_teammate < MIN_TEAMMATE_SPACING {
                // Too close to teammate - adjust movement to spread out
                let dx_to_teammate = tm_pos.x - ai_pos.x;

                // If moving toward teammate, reduce or reverse movement
                if input.move_x * dx_to_teammate > 0.0 {
                    // Moving toward teammate - try to stop or back off
                    // Only back off if not chasing the ball or doing something urgent
                    let is_urgent = matches!(
                        ai_state.current_goal,
                        AiGoal::ChaseBall | AiGoal::AttemptSteal | AiGoal::ChargeShot
                    );

                    if !is_urgent {
                        // Reverse direction to maintain spacing
                        input.move_x = -dx_to_teammate.signum() * 0.5;
                    }
                    // If urgent, continue toward goal but teammate avoidance is overridden
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

        // === TURBO DECISION ===
        // Use turbo when chasing and there's distance to close, based on turbo_usage profile
        let should_turbo = match ai_state.current_goal {
            AiGoal::ChaseBall => {
                // Use turbo when chasing ball and it's far away
                let distance_to_ball = ai_pos.distance(ball_pos);
                distance_to_ball > 150.0 && profile.turbo_usage > 0.3
            }
            AiGoal::InterceptDefense | AiGoal::PressureDefense | AiGoal::AttemptSteal => {
                // Use turbo when chasing opponent with ball
                if let Some(opp_pos) = opponent_pos {
                    let distance_to_opponent = ai_pos.distance(opp_pos);
                    // More likely to turbo the higher turbo_usage is
                    // Also more likely when opponent is farther away
                    distance_to_opponent > 100.0 && rand::thread_rng().gen_range(0.0..1.0) < profile.turbo_usage * 0.5
                } else {
                    false
                }
            }
            AiGoal::AttackWithBall => {
                // Use turbo when escaping with ball and defender is close
                if let Some(opp_pos) = opponent_pos {
                    let distance_to_opponent = ai_pos.distance(opp_pos);
                    // Turbo to create separation when defender is close
                    distance_to_opponent < 200.0 && profile.turbo_usage > 0.4
                } else {
                    false
                }
            }
            _ => false,
        };
        input.turbo_held = should_turbo;

        // === BLOCK DECISION ===
        // Use block when defending and opponent is charging a shot
        let should_block = match ai_state.current_goal {
            AiGoal::InterceptDefense | AiGoal::PressureDefense => {
                // Check if any opponent is charging a shot
                let opponent_charging = all_players
                    .iter()
                    .filter(|(e, _, t, _, _, _)| *e != ai_entity && *t != ai_team)
                    .any(|(_e, _, _, _, holding, _)| {
                        // Check if this opponent has ball and might be shooting
                        holding.is_some()
                    });

                if opponent_charging {
                    // Block based on reaction threshold
                    // Higher block_reaction = more likely to block
                    rand::thread_rng().gen_range(0.0..1.0) < profile.block_reaction * 0.3
                } else {
                    false
                }
            }
            _ => false,
        };
        input.block_pressed = should_block;

        // Stuck detection: track cumulative movement over a time window
        // This catches cases where micro-jitter prevents frame-to-frame detection
        let stuck_window_duration = 0.3; // Check movement over 300ms window
        let stuck_distance_threshold = 15.0; // Must move at least 15px in window to not be stuck

        // Update stuck window tracking
        if input.move_x.abs() > 0.1 && grounded.0 {
            // AI is trying to move - track window
            if ai_state.stuck_window_start.is_none() {
                // Start new window
                ai_state.stuck_window_start = Some(ai_pos);
                ai_state.stuck_window_timer = 0.0;
            }

            ai_state.stuck_window_timer += dt;

            // Check if window has elapsed
            if ai_state.stuck_window_timer >= stuck_window_duration {
                let start_pos = ai_state.stuck_window_start.unwrap_or(ai_pos);
                let distance_moved = ai_pos.distance(start_pos);

                if distance_moved < stuck_distance_threshold {
                    // Stuck! Increment stuck timer
                    ai_state.stuck_timer += stuck_window_duration;

                    // After being stuck for a bit, try to jump to get unstuck
                    if ai_state.stuck_timer > 0.3 {
                        input.jump_buffer_timer = JUMP_BUFFER_TIME;
                        input.jump_held = true;
                    }
                    // After more time, try moving the opposite direction
                    // Set a reversal timer so the direction persists for 0.5s
                    if ai_state.stuck_timer > 0.8 && ai_state.stuck_reverse_timer <= 0.0 {
                        ai_state.stuck_reverse_direction = -input.move_x.signum();
                        ai_state.stuck_reverse_timer = 0.5; // Persist reversal for 0.5s
                        ai_state.stuck_timer = 0.0; // Reset stuck timer
                    }
                } else {
                    // Not stuck, reset
                    ai_state.stuck_timer = 0.0;
                }

                // Reset window for next check
                ai_state.stuck_window_start = Some(ai_pos);
                ai_state.stuck_window_timer = 0.0;
            }
        } else {
            // Not trying to move or airborne - reset stuck tracking
            ai_state.stuck_timer = 0.0;
            ai_state.stuck_window_start = None;
            ai_state.stuck_window_timer = 0.0;
        }

        // Apply stuck reversal override if active
        // This overrides goal-based movement to escape walls
        if ai_state.stuck_reverse_timer > 0.0 {
            input.move_x = ai_state.stuck_reverse_direction;
        }

        // Update last position (kept for compatibility)
        ai_state.last_position = Some(ai_pos);

        // Decay jump buffer timer
        input.jump_buffer_timer = (input.jump_buffer_timer - dt).max(0.0);

        // Emit ControllerInput event for auditability
        // Use CharacterId from Character component, or derive from Team for backward compat
        let character = ai_character
            .map(|c| c.0)
            .unwrap_or_else(|| match *ai_team {
                Team::Left => CharacterId::L0,
                Team::Right => CharacterId::R0,
            });
        event_bus.emit(GameEvent::ControllerInput {
            character,
            source_id: crate::input::AI_SOURCE_ID_START, // AI uses a reserved source ID
            move_x: input.move_x,
            jump: input.jump_held,
            jump_pressed: input.jump_buffer_timer > 0.0,
            throw: input.throw_held,
            throw_released: input.throw_released,
            pickup: input.pickup_pressed,
            pass: input.pass_pressed,
        });
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

                // Always move toward landing point during the jump arc
                // This is critical - without horizontal movement, the AI falls straight down
                if let Some(NavAction::WalkTo { x: land_x }) =
                    nav_state.current_path.get(nav_state.path_index + 1)
                {
                    let dx = land_x - ai_pos.x;
                    input.move_x = dx.signum();
                }

                if nav_state.jump_timer < target_hold_time {
                    // No cap - physics determines max useful hold time
                    input.jump_held = true;
                } else {
                    // Release jump button
                    input.jump_held = false;

                    // Check if we've landed
                    if grounded && nav_state.jump_timer > 0.1 {
                        nav_state.advance();
                    }
                    // Note: horizontal movement already set above, continues until landing
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
        let source =
            fs::read_to_string("src/ai/decision.rs").expect("Should be able to read decision.rs");

        // Only analyze code BEFORE the test module (exclude #[cfg(test)] section)
        let code_to_analyze = source.split("#[cfg(test)]").next().unwrap_or(&source);

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
