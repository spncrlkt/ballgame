//! Training session state management

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

use super::protocol::TrainingProtocol;

/// Training session phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrainingPhase {
    /// Waiting for first ball pickup to start game timer
    #[default]
    WaitingToStart,
    /// Game is actively being played
    Playing,
    /// Game is paused (Start button to resume)
    Paused,
    /// Game ended, recording result
    GameEnded,
    /// Transitioning to next game
    StartingNext,
    /// All games complete, showing summary
    SessionComplete,
}

/// Winner of a game
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Winner {
    Human,
    AI,
}

impl std::fmt::Display for Winner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Winner::Human => write!(f, "player"),
            Winner::AI => write!(f, "ai"),
        }
    }
}

/// Result of a single game within a training session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameResult {
    pub game_number: u32,
    pub level: u32,
    pub level_name: String,
    pub human_score: u32,
    pub ai_score: u32,
    pub winner: Winner,
    pub duration_secs: f32,
    pub match_id: Option<i64>,
    /// Optional notes entered by player after the game
    pub notes: Option<String>,
}

/// Main training session state resource
#[derive(Resource)]
pub struct TrainingState {
    /// Training protocol being used
    pub protocol: TrainingProtocol,
    /// Unique session identifier (timestamp-based)
    pub session_id: String,
    /// Current game number (1-based)
    pub game_number: u32,
    /// Total games in session
    pub games_total: u32,
    /// Results from completed games
    pub game_results: Vec<GameResult>,
    /// Current level index (1-based)
    pub current_level: u32,
    /// Current level name
    pub current_level_name: String,
    /// Session output directory
    pub session_dir: PathBuf,
    /// SQLite session identifier
    pub sqlite_session_id: Option<String>,
    /// Current SQLite match ID
    pub current_match_id: Option<i64>,
    /// Current phase
    pub phase: TrainingPhase,
    /// Time game started (for duration tracking)
    pub game_start_time: Option<Instant>,
    /// Elapsed game time in seconds
    pub game_elapsed: f32,
    /// AI profile name being trained against
    pub ai_profile: String,
    /// Score needed to win (first-to-N)
    pub win_score: u32,
    /// Time spent in between-game transition
    pub transition_timer: f32,
    /// Time limit per game in seconds (None = no limit)
    pub time_limit_secs: Option<f32>,
    /// Timeout if no score within this many seconds (None = no timeout)
    pub first_point_timeout_secs: Option<f32>,
    /// Ordered list of level indices for sequential iteration (Reachability protocol)
    pub level_sequence: Vec<usize>,
    /// Current position in level_sequence
    pub level_sequence_index: usize,
}

impl Default for TrainingState {
    fn default() -> Self {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let session_dir = PathBuf::from("training_logs").join(format!("session_{}", timestamp));

        Self {
            protocol: TrainingProtocol::default(),
            session_id: timestamp,
            game_number: 1,
            games_total: 5,
            game_results: Vec::new(),
            current_level: 2, // Start with level 2 (skip debug level 1)
            current_level_name: String::new(),
            session_dir,
            sqlite_session_id: None,
            current_match_id: None,
            phase: TrainingPhase::WaitingToStart,
            game_start_time: None,
            game_elapsed: 0.0,
            ai_profile: "Balanced".to_string(),
            win_score: 5,
            transition_timer: 0.0,
            time_limit_secs: None,
            first_point_timeout_secs: None,
            level_sequence: Vec::new(),
            level_sequence_index: 0,
        }
    }
}

use crate::levels::LevelDatabase;

impl TrainingState {
    /// Create a new training state with specified games and AI profile
    pub fn new(games_total: u32, ai_profile: &str) -> Self {
        let mut state = Self::default();
        state.games_total = games_total;
        state.ai_profile = ai_profile.to_string();
        state
    }

    /// Get win counts
    pub fn wins(&self) -> (u32, u32) {
        let human_wins = self
            .game_results
            .iter()
            .filter(|r| r.winner == Winner::Human)
            .count() as u32;
        let ai_wins = self
            .game_results
            .iter()
            .filter(|r| r.winner == Winner::AI)
            .count() as u32;
        (human_wins, ai_wins)
    }

    /// Start the game timer
    pub fn start_game_timer(&mut self) {
        self.game_start_time = Some(Instant::now());
        self.game_elapsed = 0.0;
        self.phase = TrainingPhase::Playing;
    }

    /// Update elapsed time
    pub fn update_elapsed(&mut self) {
        if let Some(start) = self.game_start_time {
            self.game_elapsed = start.elapsed().as_secs_f32();
        }
    }

    /// Record a game result
    pub fn record_result(&mut self, human_score: u32, ai_score: u32, match_id: Option<i64>) {
        let winner = if human_score >= self.win_score {
            Winner::Human
        } else {
            Winner::AI
        };

        let result = GameResult {
            game_number: self.game_number,
            level: self.current_level,
            level_name: self.current_level_name.clone(),
            human_score,
            ai_score,
            winner,
            duration_secs: self.game_elapsed,
            match_id,
            notes: None,
        };

        self.game_results.push(result);
        self.phase = TrainingPhase::GameEnded;
    }

    /// Advance to next game
    pub fn next_game(&mut self) {
        self.game_number += 1;
        self.phase = TrainingPhase::WaitingToStart;
        self.game_start_time = None;
        self.game_elapsed = 0.0;
        self.transition_timer = 0.0;
    }

    /// Check if session is complete
    pub fn is_complete(&self) -> bool {
        self.game_number > self.games_total
    }

    /// Initialize level sequence for sequential iteration (Reachability protocol)
    /// Builds ordered list of non-debug level indices
    pub fn init_level_sequence(&mut self, level_db: &LevelDatabase) {
        self.level_sequence = level_db
            .all()
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.debug && !l.regression)
            .map(|(i, _)| i)
            .collect();
        self.level_sequence_index = 0;

        // Set games_total to match level count
        self.games_total = self.level_sequence.len() as u32;
    }

    /// Advance to next level in sequence (for Reachability protocol)
    /// Returns true if there are more levels, false if sequence is complete
    pub fn advance_to_next_level(&mut self) -> bool {
        self.level_sequence_index += 1;
        self.level_sequence_index < self.level_sequence.len()
    }

    /// Get current level index from sequence (0-based)
    pub fn current_sequence_level(&self) -> Option<usize> {
        self.level_sequence.get(self.level_sequence_index).copied()
    }
}
