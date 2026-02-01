//! Tournament mode configuration and systems
//!
//! Provides match end conditions based on score or time limits.

use bevy::prelude::*;

use ballgame_protocol::game_state::Team as ProtocolTeam;

use super::bridge::ServerBridge;
use crate::{Score, Team};

/// Tournament mode configuration
#[derive(Resource, Default)]
pub struct TournamentConfig {
    /// Whether tournament mode is enabled
    pub enabled: bool,
    /// Score limit to win (first to reach this score wins)
    pub score_limit: Option<u32>,
    /// Time limit in seconds (match ends when elapsed)
    pub time_limit_secs: Option<f32>,
}

impl TournamentConfig {
    /// Create a new tournament config
    pub fn new(score_limit: Option<u32>, time_limit_secs: Option<f32>) -> Self {
        Self {
            enabled: score_limit.is_some() || time_limit_secs.is_some(),
            score_limit,
            time_limit_secs,
        }
    }
}

/// Check if tournament end conditions are met and exit if so
pub fn check_tournament_end(
    config: Res<TournamentConfig>,
    score: Res<Score>,
    time: Res<Time>,
    bridge: Option<Res<ServerBridge>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if !config.enabled {
        return;
    }

    let mut winner: Option<Team> = None;

    // Check score limit
    if let Some(limit) = config.score_limit {
        if score.left >= limit {
            winner = Some(Team::Left);
        } else if score.right >= limit {
            winner = Some(Team::Right);
        }
    }

    // Check time limit (only if no winner from score yet)
    if winner.is_none() {
        if let Some(limit) = config.time_limit_secs {
            if time.elapsed_secs() >= limit {
                // Determine winner by score
                if score.left > score.right {
                    winner = Some(Team::Left);
                } else if score.right > score.left {
                    winner = Some(Team::Right);
                }
                // If tied, winner remains None
            }
        }
    }

    // If we have a winner (or time expired with tie), end the match
    if winner.is_some() || (config.time_limit_secs.is_some() && time.elapsed_secs() >= config.time_limit_secs.unwrap()) {
        // Broadcast match end to clients
        if let Some(bridge) = bridge {
            let protocol_winner = winner.map(|t| match t {
                Team::Left => ProtocolTeam::Left,
                Team::Right => ProtocolTeam::Right,
            });

            bridge.runtime.block_on(async {
                // Create match end message
                let tick = bridge.current_tick();
                for _ in 0..bridge.broadcaster.client_count().await {
                    bridge.broadcaster.send_to(
                        0, // Will be broadcast
                        tick,
                        ballgame_protocol::ServerPayload::MatchEnd { winner: protocol_winner },
                    ).await;
                }
            });

            info!(
                "Tournament ended: {:?} wins (score: {}-{})",
                winner, score.left, score.right
            );
        }

        app_exit.write(AppExit::Success);
    }
}
