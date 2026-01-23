//! Steal contest system

use bevy::prelude::*;

use crate::ai::InputState;
use crate::ball::{Ball, BallState};
use crate::player::{HoldingBall, Player};

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

/// Update steal contest state.
/// All players read from their InputState component.
pub fn steal_contest_update(
    mut commands: Commands,
    mut steal_contest: ResMut<StealContest>,
    time: Res<Time>,
    mut ball_query: Query<&mut BallState, With<Ball>>,
    holding_query: Query<&HoldingBall>,
    mut players: Query<&mut InputState, With<Player>>,
) {
    if !steal_contest.active {
        return;
    }

    steal_contest.timer -= time.delta_secs();

    // Count defender presses
    if let Some(defender_entity) = steal_contest.defender {
        if let Ok(mut input) = players.get_mut(defender_entity) {
            if input.pickup_pressed {
                steal_contest.defender_presses += 1;
                input.pickup_pressed = false;
            }
        }
    }

    // Count attacker presses
    if let Some(attacker_entity) = steal_contest.attacker {
        if let Ok(mut input) = players.get_mut(attacker_entity) {
            if input.pickup_pressed {
                steal_contest.attacker_presses += 1;
                input.pickup_pressed = false;
            }
        }
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
