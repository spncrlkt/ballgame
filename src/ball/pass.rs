//! Pass mechanic for 2v2 mode
//!
//! Allows players to pass the ball to their teammate with auto-aim assist.
//! Pass speed and arc are calculated based on relative distance/height to teammate.

use bevy::prelude::*;

use crate::ai::InputState;
use crate::ball::{Ball, BallState, Velocity};
use crate::events::{CharacterId, EventBus, GameEvent};
use crate::player::{Character, HoldingBall, Player, Team};
use crate::tuning::PhysicsTweaks;

/// Handle pass input - pass ball to teammate
/// Runs in FixedUpdate, checks for pass_pressed input
///
/// Pass physics:
/// - Arc angle is calculated based on horizontal distance and height difference
/// - Speed is physics-based to reach the target at the calculated arc
pub fn handle_pass(
    mut commands: Commands,
    mut event_bus: ResMut<EventBus>,
    tweaks: Res<PhysicsTweaks>,
    mut holding_query: Query<
        (
            Entity,
            &Transform,
            &Team,
            Option<&Character>,
            &HoldingBall,
            &mut InputState,
        ),
        With<Player>,
    >,
    teammate_query: Query<(Entity, &Transform, &Team, Option<&Character>), (With<Player>, Without<HoldingBall>)>,
    mut ball_query: Query<(&mut BallState, &mut Velocity), With<Ball>>,
) {
    // Find the player holding the ball who wants to pass
    for (holder_entity, holder_transform, holder_team, holder_char, holding, mut input) in
        &mut holding_query
    {
        if !input.pass_pressed {
            continue;
        }

        // Consume the input
        input.pass_pressed = false;

        // Find teammate (same team, not holding ball)
        let mut best_teammate: Option<(Entity, Vec2, Option<CharacterId>)> = None;
        let holder_pos = holder_transform.translation.truncate();

        for (teammate_entity, teammate_transform, teammate_team, teammate_char) in &teammate_query {
            // Must be same team
            if teammate_team != holder_team {
                continue;
            }

            // Can't pass to self (shouldn't happen but be safe)
            if teammate_entity == holder_entity {
                continue;
            }

            let teammate_pos = teammate_transform.translation.truncate();
            let distance = holder_pos.distance(teammate_pos);

            // Prefer closer teammate (in 2v2, there's only one teammate anyway)
            if best_teammate
                .map(|(_, pos, _)| holder_pos.distance(pos))
                .unwrap_or(f32::MAX)
                > distance
            {
                let char_id = teammate_char.map(|c| c.0);
                best_teammate = Some((teammate_entity, teammate_pos, char_id));
            }
        }

        // No teammate found - can't pass
        let Some((_teammate_entity, teammate_pos, teammate_char_id)) = best_teammate else {
            // Could add feedback here (no teammate available)
            continue;
        };

        // Get the ball
        let Ok((mut ball_state, mut ball_velocity)) = ball_query.get_mut(holding.0) else {
            continue;
        };

        // Calculate relative position to teammate
        let delta = teammate_pos - holder_pos;
        let dx = delta.x.abs();
        let dy = delta.y; // positive = teammate is above

        // Calculate arc angle based on distance and height
        // Distance component: longer passes get higher arc
        let distance_arc =
            (dx / tweaks.pass_distance_arc_scale).clamp(0.0, 1.0) * tweaks.pass_max_distance_arc;
        // Height component: passing up adds arc, passing down reduces arc
        let height_arc =
            (dy / tweaks.pass_height_arc_scale).clamp(-1.0, 1.0) * tweaks.pass_max_height_arc;
        let arc_angle_degrees = (tweaks.pass_base_arc + distance_arc + height_arc)
            .clamp(tweaks.pass_min_arc, tweaks.pass_max_arc);
        let arc_angle = arc_angle_degrees.to_radians();

        // Calculate physics-based speed to reach target at this arc
        // Using projectile motion: v = sqrt(g * dx^2 / (2 * cos^2(theta) * (dx * tan(theta) - dy)))
        let cos_theta = arc_angle.cos();
        let tan_theta = arc_angle.tan();
        let denominator = 2.0 * cos_theta * cos_theta * (dx * tan_theta - dy);
        let physics_speed = if denominator > 0.001 {
            (tweaks.ball_gravity * dx * dx / denominator).sqrt()
        } else {
            tweaks.pass_min_speed // Fallback for vertical/edge cases
        };
        let pass_speed = physics_speed.clamp(tweaks.pass_min_speed, tweaks.pass_max_speed);

        // Calculate final velocity direction (preserving horizontal sign)
        let horizontal_sign = delta.x.signum();
        let pass_velocity = Vec2::new(
            horizontal_sign * pass_speed * cos_theta,
            pass_speed * arc_angle.sin(),
        );

        // Set ball state to PassInFlight with the intended target
        *ball_state = BallState::PassInFlight {
            passer: holder_entity,
            target: best_teammate.map(|(e, _, _)| e).unwrap(),
        };
        ball_velocity.0 = pass_velocity;

        // Remove ball from holder
        commands.entity(holder_entity).remove::<HoldingBall>();

        // Emit Pass event
        let from_char = holder_char.map(|c| c.0);
        if let (Some(from), Some(to)) = (from_char, teammate_char_id) {
            event_bus.emit(GameEvent::Pass {
                from,
                to,
                velocity: (pass_velocity.x, pass_velocity.y),
            });
        }

        info!(
            "PASS from {:?} to {:?} - speed={:.0}, arc={:.1}°, dx={:.0}, dy={:.0}",
            from_char, teammate_char_id, pass_speed, arc_angle_degrees, dx, dy
        );

        // Only process one pass per frame
        return;
    }
}
