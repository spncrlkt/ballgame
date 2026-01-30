//! Training session state management

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

use super::protocol::TrainingProtocol;

/// Collects position data for reachability heatmap export
pub struct ReachabilityCollector {
    pub level_id: String,
    pub level_name: String,
    pub start_time: Instant,
    /// New positions collected this session
    pub positions: Vec<(f32, f32)>,
    /// Pre-loaded positions from existing heatmap (for visualization)
    pub preloaded_positions: Vec<(f32, f32)>,
}

impl ReachabilityCollector {
    pub fn new(level_id: String, level_name: String) -> Self {
        Self {
            level_id,
            level_name,
            start_time: Instant::now(),
            positions: Vec::with_capacity(1000),
            preloaded_positions: Vec::new(),
        }
    }

    /// Create a collector and load existing heatmap data if available
    pub fn new_with_preload(level_id: String, level_name: String) -> Self {
        let mut collector = Self::new(level_id.clone(), level_name.clone());

        // Try to load existing heatmap
        let safe_name = sanitize_level_name(&level_name);
        let path = format!(
            "showcase/heatmaps/heatmap_reachability_{}_{}.txt",
            safe_name, level_id
        );

        if let Ok(content) = std::fs::read_to_string(&path) {
            collector.preloaded_positions = parse_heatmap_positions(&content);
            if !collector.preloaded_positions.is_empty() {
                info!(
                    "Loaded {} positions from existing heatmap for {}",
                    collector.preloaded_positions.len(),
                    level_name
                );
            }
        }

        collector
    }

    pub fn elapsed_secs(&self) -> f32 {
        self.start_time.elapsed().as_secs_f32()
    }

    /// Get all positions (preloaded + new) for export
    pub fn all_positions(&self) -> impl Iterator<Item = &(f32, f32)> {
        self.preloaded_positions.iter().chain(self.positions.iter())
    }
}

/// Sanitize level name for use in filenames
fn sanitize_level_name(name: &str) -> String {
    let mut out = String::new();
    let mut last_was_underscore = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_was_underscore = false;
        } else if !last_was_underscore {
            out.push('_');
            last_was_underscore = true;
        }
    }
    out.trim_matches('_').to_string()
}

/// Parse heatmap CSV file and extract positions with non-zero values
fn parse_heatmap_positions(content: &str) -> Vec<(f32, f32)> {
    let mut positions = Vec::new();

    for line in content.lines().skip(1) {
        // Skip header
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 3 {
            if let (Ok(x), Ok(y), Ok(value)) = (
                parts[0].trim().parse::<f32>(),
                parts[1].trim().parse::<f32>(),
                parts[2].trim().parse::<f32>(),
            ) {
                // Only include positions that were visited (value > 0)
                if value > 0.0 {
                    positions.push((x, y));
                }
            }
        }
    }

    positions
}

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
    /// Reachability position collector (for auto-export heatmaps)
    pub reachability_collector: Option<ReachabilityCollector>,
    /// Whether advance button has been released at least once (prevents spurious input on startup)
    pub advance_button_armed: bool,
    /// List of profile names to iterate through (if profile-list mode)
    pub profile_list: Option<Vec<String>>,
    /// Current index in profile list
    pub profile_list_index: usize,
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
            reachability_collector: None,
            advance_button_armed: false,
            profile_list: None,
            profile_list_index: 0,
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

    /// Advance to next profile in list (for profile-list mode)
    /// Returns true if there are more profiles, false if list is exhausted
    pub fn advance_profile(&mut self) -> bool {
        if let Some(ref list) = self.profile_list {
            self.profile_list_index += 1;
            if self.profile_list_index < list.len() {
                self.ai_profile = list[self.profile_list_index].clone();
                return true;
            }
        }
        false
    }

    /// Get current profile name from list (if in profile-list mode)
    pub fn current_profile(&self) -> Option<&str> {
        self.profile_list
            .as_ref()
            .and_then(|list| list.get(self.profile_list_index))
            .map(|s| s.as_str())
    }
}
