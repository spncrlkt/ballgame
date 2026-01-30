//! Pass mechanic for 2v2 mode
//!
//! Allows players to pass the ball to their teammate with auto-aim assist.

use bevy::prelude::*;

use crate::ai::InputState;
use crate::ball::{Ball, BallState, Velocity};
use crate::events::{CharacterId, EventBus, GameEvent};
use crate::player::{Character, HoldingBall, Player, Team};

/// Pass power (lower than shot for shorter, quicker passes)
pub const PASS_POWER: f32 = 400.0;

/// Pass arc angle (degrees) - slightly upward for lob
pub const PASS_ARC_ANGLE: f32 = 25.0;

/// Handle pass input - pass ball to teammate
/// Runs in FixedUpdate, checks for pass_pressed input
pub fn handle_pass(
    mut commands: Commands,
    mut event_bus: ResMut<EventBus>,
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

        // Calculate pass direction with auto-aim
        let pass_direction = (teammate_pos - holder_pos).normalize_or_zero();

        // Add slight upward arc for lob pass
        let arc_radians = PASS_ARC_ANGLE.to_radians();
        let pass_angle = pass_direction.y.atan2(pass_direction.x) + arc_radians;

        // Calculate velocity
        let pass_velocity = Vec2::new(pass_angle.cos(), pass_angle.sin()) * PASS_POWER;

        // Set ball state to InFlight
        *ball_state = BallState::InFlight {
            shooter: holder_entity,
            power: PASS_POWER,
        };
        ball_velocity.0 = pass_velocity;

        // Remove ball from holder
        commands.entity(holder_entity).remove::<HoldingBall>();

        // Emit Pass event
        let from_char = holder_char.map(|c| c.0);
        if let (Some(from), Some(to)) = (from_char, teammate_char_id) {
            event_bus.emit(GameEvent::Pass { from, to });
        }

        info!(
            "PASS from {:?} to {:?} at {:.0} power",
            from_char, teammate_char_id, PASS_POWER
        );

        // Only process one pass per frame
        return;
    }
}
