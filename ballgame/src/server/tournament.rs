//! Tournament mode configuration and systems
//!
//! Provides match end conditions based on score or time limits.

use bevy::prelude::*;

use ballgame_protocol::game_state::Team as ProtocolTeam;

use super::bridge::ServerBridge;
use super::lobby::LobbyState;
use crate::scoring::GamePaused;
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
    /// Accumulated match time in seconds (pauses when game is paused)
    pub match_elapsed_secs: f32,
    /// Whether the match timer is running
    pub match_active: bool,
}

impl TournamentConfig {
    /// Create a new tournament config
    pub fn new(score_limit: Option<u32>, time_limit_secs: Option<f32>) -> Self {
        Self {
            enabled: score_limit.is_some() || time_limit_secs.is_some(),
            score_limit,
            time_limit_secs,
            match_elapsed_secs: 0.0,
            match_active: false,
        }
    }

    /// Prepare for match (timer will start when countdown ends)
    pub fn prepare_match(&mut self) {
        self.match_elapsed_secs = 0.0;
        self.match_active = false; // Will be set true when countdown ends
    }

    /// Start the match timer (called when countdown ends)
    pub fn start_match(&mut self) {
        self.match_elapsed_secs = 0.0;
        self.match_active = true;
    }

    /// Get remaining time (if time limit is set)
    pub fn remaining_secs(&self) -> Option<f32> {
        self.time_limit_secs.map(|limit| (limit - self.match_elapsed_secs).max(0.0))
    }

    /// Format elapsed time as MM:SS
    pub fn format_elapsed(&self) -> String {
        let secs = self.match_elapsed_secs as u32;
        let mins = secs / 60;
        let secs = secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }

    /// Format remaining time as MM:SS (or elapsed if no limit)
    pub fn format_time_display(&self) -> String {
        if let Some(remaining) = self.remaining_secs() {
            let secs = remaining as u32;
            let mins = secs / 60;
            let secs = secs % 60;
            format!("{:02}:{:02}", mins, secs)
        } else {
            self.format_elapsed()
        }
    }
}

/// Update match timer (respects pause state)
pub fn update_match_timer(
    mut config: ResMut<TournamentConfig>,
    game_paused: Res<GamePaused>,
    time: Res<Time>,
) {
    if config.match_active && !game_paused.0 {
        config.match_elapsed_secs += time.delta_secs();
    }
}

/// Check if tournament end conditions are met and return to lobby if so
pub fn check_tournament_end(
    mut config: ResMut<TournamentConfig>,
    mut score: ResMut<Score>,
    bridge: Option<Res<ServerBridge>>,
    lobby_state: Option<ResMut<LobbyState>>,
) {
    if !config.enabled || !config.match_active {
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
            if config.match_elapsed_secs >= limit {
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
    let time_expired = config.time_limit_secs.map(|limit| config.match_elapsed_secs >= limit).unwrap_or(false);
    if winner.is_some() || time_expired {
        // Broadcast match end to clients
        if let Some(ref bridge) = bridge {
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
                "Match ended: {:?} wins (score: {}-{})",
                winner, score.left, score.right
            );
        }

        // Reset for next match
        score.left = 0;
        score.right = 0;
        config.match_elapsed_secs = 0.0;
        config.match_active = false;

        // Return to lobby if it exists
        if let Some(mut lobby) = lobby_state {
            lobby.active = true;
            // Clear ServerAi slots so remote clients can connect
            if let Some(ref bridge) = bridge {
                bridge.runtime.block_on(bridge.slots.clear_server_ai_slots());
            }
            info!("Returning to lobby");
        }
    }
}
