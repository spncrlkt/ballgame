//! Steal contest system

use bevy::prelude::*;

use crate::ball::{Ball, BallState};
use crate::player::HoldingBall;

/// Steal contest resource tracking active steal attempts
#[derive(Resource, Default)]
pub struct StealContest {
    pub active: bool,
    pub attacker: Option<Entity>,
    pub defender: Option<Entity>,
    pub attacker_presses: u32,
    pub defender_presses: u32,
    pub timer: f32,
}

/// Update steal contest state
pub fn steal_contest_update(
    mut commands: Commands,
    mut steal_contest: ResMut<StealContest>,
    time: Res<Time>,
    mut ball_query: Query<&mut BallState, With<Ball>>,
    holding_query: Query<&HoldingBall>,
) {
    if !steal_contest.active {
        return;
    }

    steal_contest.timer -= time.delta_secs();

    // TODO: In multiplayer, check defender's button presses here
    // For now, defender gets occasional "presses" to simulate resistance
    if steal_contest.timer > 0.0 && steal_contest.timer % 0.1 < time.delta_secs() {
        steal_contest.defender_presses += 1;
    }

    if steal_contest.timer <= 0.0 {
        // Contest ended - resolve it
        let attacker = steal_contest.attacker.unwrap();
        let defender = steal_contest.defender.unwrap();

        if steal_contest.attacker_presses > steal_contest.defender_presses {
            // Attacker wins - steal the ball
            if let Ok(holding) = holding_query.get(defender) {
                let ball_entity = holding.0;
                if let Ok(mut ball_state) = ball_query.get_mut(ball_entity) {
                    *ball_state = BallState::Held(attacker);
                    commands.entity(defender).remove::<HoldingBall>();
                    commands.entity(attacker).insert(HoldingBall(ball_entity));
                }
            }
        }
        // If defender wins or tie, they keep the ball (no action needed)

        // Reset contest
        *steal_contest = StealContest::default();
    }
}
