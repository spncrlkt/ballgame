//! Game state snapshot creation for network transmission

use bevy::prelude::*;

use ballgame_protocol::{
    AgentSnapshot, AiStateView, BallSnapshot, BallStateKind, GameStateSnapshot,
    Score as ProtocolScore, Vec2 as ProtocolVec2,
    game_state::{Basket as ProtocolBasket, CharacterId as ProtocolCharacterId, Team as ProtocolTeam},
};

use crate::{
    AiState, Ball, BallState, BallStyle, Character, ChargingShot, Grounded, HoldingBall,
    Player, Score, TargetBasket, Team, TurboGauge, Velocity,
    player::{Facing, BlockState},
    countdown::MatchCountdown,
    scoring::CurrentLevel,
};

/// Convert Bevy Vec2 to Protocol Vec2
fn to_protocol_vec2(v: bevy::math::Vec2) -> ProtocolVec2 {
    ProtocolVec2::new(v.x, v.y)
}

/// Convert Bevy Team to Protocol Team
fn to_protocol_team(team: &Team) -> ProtocolTeam {
    match team {
        Team::Left => ProtocolTeam::Left,
        Team::Right => ProtocolTeam::Right,
    }
}

/// Convert CharacterId to protocol version
fn to_protocol_character(char: &crate::events::CharacterId) -> ProtocolCharacterId {
    match char {
        crate::events::CharacterId::L0 => ProtocolCharacterId::L0,
        crate::events::CharacterId::L1 => ProtocolCharacterId::L1,
        crate::events::CharacterId::R0 => ProtocolCharacterId::R0,
        crate::events::CharacterId::R1 => ProtocolCharacterId::R1,
    }
}

/// Convert TargetBasket to protocol Basket
fn to_protocol_basket(basket: &crate::world::Basket) -> ProtocolBasket {
    match basket {
        crate::world::Basket::Left => ProtocolBasket::Left,
        crate::world::Basket::Right => ProtocolBasket::Right,
    }
}

/// Convert BallState to protocol BallStateKind
fn to_protocol_ball_state(
    state: &BallState,
    players: &Query<(Entity, &Character), With<Player>>,
) -> BallStateKind {
    match state {
        BallState::Free => BallStateKind::Free,
        BallState::Held(entity) => {
            let holder = players
                .iter()
                .find(|(e, _)| *e == *entity)
                .map(|(_, c)| to_protocol_character(&c.0))
                .unwrap_or(ProtocolCharacterId::L0);
            BallStateKind::Held { holder }
        }
        BallState::InFlight { shooter, power } => {
            let shooter_char = players
                .iter()
                .find(|(e, _)| *e == *shooter)
                .map(|(_, c)| to_protocol_character(&c.0))
                .unwrap_or(ProtocolCharacterId::L0);
            BallStateKind::InFlight {
                shooter: shooter_char,
                power: *power,
            }
        }
        BallState::PassInFlight { passer, target } => {
            let passer_char = players
                .iter()
                .find(|(e, _)| *e == *passer)
                .map(|(_, c)| to_protocol_character(&c.0))
                .unwrap_or(ProtocolCharacterId::L0);
            let target_char = players
                .iter()
                .find(|(e, _)| *e == *target)
                .map(|(_, c)| to_protocol_character(&c.0))
                .unwrap_or(ProtocolCharacterId::L1);
            BallStateKind::PassInFlight {
                passer: passer_char,
                target: target_char,
            }
        }
    }
}

/// System to create game state snapshot for network transmission
pub fn create_game_snapshot(
    tick: u64,
    time: f32,
    players: Query<
        (
            Entity,
            &Character,
            &Transform,
            &Velocity,
            &Team,
            &Grounded,
            &Facing,
            &TargetBasket,
            &TurboGauge,
            Option<&HoldingBall>,
            Option<&ChargingShot>,
            Option<&AiState>,
            Option<&BlockState>,
        ),
        With<Player>,
    >,
    ball_query: Query<(&Transform, &Velocity, &BallState, &BallStyle), With<Ball>>,
    score: &Score,
    current_level: &CurrentLevel,
    countdown: &MatchCountdown,
    player_entities: &Query<(Entity, &Character), With<Player>>,
) -> GameStateSnapshot {
    // Build agent snapshots
    let agents: Vec<AgentSnapshot> = players
        .iter()
        .map(
            |(
                _entity,
                character,
                transform,
                velocity,
                team,
                grounded,
                facing,
                target_basket,
                turbo,
                holding,
                charging,
                ai_state,
                block_state,
            )| {
                let charge_progress = charging.map(|c| c.charge_time / 1.0).unwrap_or(0.0); // Normalize to 0-1

                AgentSnapshot {
                    character: to_protocol_character(&character.0),
                    team: to_protocol_team(team),
                    position: to_protocol_vec2(transform.translation.truncate()),
                    velocity: to_protocol_vec2(velocity.0),
                    grounded: grounded.0,
                    holding_ball: holding.is_some(),
                    charging_shot: charging.is_some(),
                    charge_progress,
                    facing: facing.0,
                    target_basket: to_protocol_basket(&target_basket.0),
                    turbo_gauge: turbo.current / turbo.max,
                    block_active: block_state.map(|b| b.active).unwrap_or(false),
                    ai_state: ai_state.map(|ai| AiStateView {
                        current_goal: format!("{:?}", ai.current_goal),
                        ball_hold_time: ai.ball_hold_time,
                        steal_reaction_timer: ai.steal_reaction_timer,
                        profile_id: ai.profile_id.clone(),
                    }),
                }
            },
        )
        .collect();

    // Build ball snapshot
    let ball = ball_query
        .iter()
        .next()
        .map(|(transform, velocity, state, style)| BallSnapshot {
            position: to_protocol_vec2(transform.translation.truncate()),
            velocity: to_protocol_vec2(velocity.0),
            state: to_protocol_ball_state(state, player_entities),
            style: style.0.clone(),
        })
        .unwrap_or_default();

    GameStateSnapshot {
        tick,
        time,
        agents,
        ball,
        score: ProtocolScore::new(score.left, score.right),
        level_id: current_level.0.clone(),
        countdown: if countdown.active { countdown.timer } else { 0.0 },
    }
}
