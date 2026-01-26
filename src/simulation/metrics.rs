//! Metrics collection for AI simulation

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ai::AiGoal;
use crate::events::GameEvent;

/// Statistics for a single player during a match
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerStats {
    /// Goals scored
    pub goals: u32,
    /// Shots attempted
    pub shots_attempted: u32,
    /// Shots that scored
    pub shots_made: u32,
    /// Shot accuracy (0.0 - 1.0)
    pub accuracy: f32,
    /// Time holding the ball (seconds)
    pub possession_time: f32,
    /// Steal attempts
    pub steals_attempted: u32,
    /// Successful steals
    pub steals_successful: u32,
    /// Total jumps
    pub jumps: u32,
    /// Total distance traveled (pixels)
    pub distance_traveled: f32,
    /// Time spent in each AI goal state (seconds)
    pub goal_time: HashMap<String, f32>,
    /// Navigation paths completed
    pub nav_paths_completed: u32,
    /// Navigation paths failed/aborted
    pub nav_paths_failed: u32,
    /// Average shot X position (finalized after match)
    pub avg_shot_x: f32,
    /// Average shot Y position (finalized after match)
    pub avg_shot_y: f32,
    /// Average shot quality at release (finalized after match)
    pub avg_shot_quality: f32,
    /// Internal: Sum of shot X positions (for computing average)
    #[serde(skip)]
    pub shot_positions_sum_x: f32,
    /// Internal: Sum of shot Y positions (for computing average)
    #[serde(skip)]
    pub shot_positions_sum_y: f32,
    /// Internal: Sum of shot qualities (for computing average)
    #[serde(skip)]
    pub shot_quality_sum: f32,
}

impl PlayerStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate derived statistics
    pub fn finalize(&mut self) {
        if self.shots_attempted > 0 {
            self.accuracy = self.shots_made as f32 / self.shots_attempted as f32;
            self.avg_shot_x = self.shot_positions_sum_x / self.shots_attempted as f32;
            self.avg_shot_y = self.shot_positions_sum_y / self.shots_attempted as f32;
            self.avg_shot_quality = self.shot_quality_sum / self.shots_attempted as f32;
        }
    }
}

/// Result of a single match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchResult {
    /// Level played
    pub level: u32,
    /// Level name
    pub level_name: String,
    /// Left player profile
    pub left_profile: String,
    /// Right player profile
    pub right_profile: String,
    /// Match duration (seconds)
    pub duration: f32,
    /// Final score
    pub score_left: u32,
    pub score_right: u32,
    /// Winner ("left", "right", or "tie")
    pub winner: String,
    /// Left player stats
    pub left_stats: PlayerStats,
    /// Right player stats
    pub right_stats: PlayerStats,
    /// RNG seed used
    pub seed: u64,
    /// Logged events for this match (used for DB persistence)
    #[serde(skip)]
    pub events: Vec<(u32, GameEvent)>,
}

impl MatchResult {
    pub fn determine_winner(&mut self) {
        self.winner = if self.score_left > self.score_right {
            "left".to_string()
        } else if self.score_right > self.score_left {
            "right".to_string()
        } else {
            "tie".to_string()
        };
    }
}

/// Results from a tournament
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TournamentResult {
    /// All match results
    pub matches: Vec<MatchResult>,
    /// Win rate matrix: win_rates[left_profile][right_profile] = win rate for left
    pub win_rates: HashMap<String, HashMap<String, f32>>,
    /// Overall win rate per profile
    pub overall_win_rates: HashMap<String, f32>,
    /// Best performing profile
    pub best_profile: String,
}

impl TournamentResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate win rates from match results
    pub fn calculate_win_rates(&mut self) {
        // Count wins per matchup
        let mut wins: HashMap<String, HashMap<String, u32>> = HashMap::new();
        let mut total: HashMap<String, HashMap<String, u32>> = HashMap::new();
        let mut profile_wins: HashMap<String, u32> = HashMap::new();
        let mut profile_total: HashMap<String, u32> = HashMap::new();

        for result in &self.matches {
            // Initialize if needed
            wins.entry(result.left_profile.clone())
                .or_default()
                .entry(result.right_profile.clone())
                .or_insert(0);
            *total
                .entry(result.left_profile.clone())
                .or_default()
                .entry(result.right_profile.clone())
                .or_insert(0) += 1;

            // Count overall games
            *profile_total.entry(result.left_profile.clone()).or_insert(0) += 1;
            *profile_total
                .entry(result.right_profile.clone())
                .or_insert(0) += 1;

            // Count wins
            match result.winner.as_str() {
                "left" => {
                    *wins
                        .get_mut(&result.left_profile)
                        .unwrap()
                        .get_mut(&result.right_profile)
                        .unwrap() += 1;
                    *profile_wins.entry(result.left_profile.clone()).or_insert(0) += 1;
                }
                "right" => {
                    *profile_wins
                        .entry(result.right_profile.clone())
                        .or_insert(0) += 1;
                }
                _ => {
                    // Tie - count as 0.5 win for each
                }
            }
        }

        // Calculate win rates
        for (left, right_map) in &wins {
            for (right, win_count) in right_map {
                let total_count = total[left][right];
                let rate = if total_count > 0 {
                    *win_count as f32 / total_count as f32
                } else {
                    0.0
                };
                self.win_rates
                    .entry(left.clone())
                    .or_default()
                    .insert(right.clone(), rate);
            }
        }

        // Calculate overall win rates
        let mut best_rate = 0.0;
        for (profile, total) in &profile_total {
            let wins = profile_wins.get(profile).unwrap_or(&0);
            let rate = *wins as f32 / *total as f32;
            self.overall_win_rates.insert(profile.clone(), rate);
            if rate > best_rate {
                best_rate = rate;
                self.best_profile = profile.clone();
            }
        }
    }

    /// Format as ASCII table
    pub fn format_table(&self, profiles: &[String]) -> String {
        let mut output = String::new();
        output.push_str("\nProfile Matchup Win Rates:\n\n");

        // Header
        output.push_str(&format!("{:>12} |", ""));
        for p in profiles {
            output.push_str(&format!(" {:>10} |", &p[..p.len().min(10)]));
        }
        output.push('\n');

        // Separator
        output.push_str(&format!("{:-<13}+", ""));
        for _ in profiles {
            output.push_str(&format!("{:-<12}+", ""));
        }
        output.push('\n');

        // Rows
        for left in profiles {
            output.push_str(&format!("{:>12} |", &left[..left.len().min(12)]));
            for right in profiles {
                if left == right {
                    output.push_str("      -     |");
                } else if let Some(rate) = self.win_rates.get(left).and_then(|m| m.get(right)) {
                    output.push_str(&format!("    {:>5.1}% |", rate * 100.0));
                } else {
                    output.push_str("      ?     |");
                }
            }
            output.push('\n');
        }

        output.push_str(&format!(
            "\nBest overall: {} ({:.1}% win rate)\n",
            self.best_profile,
            self.overall_win_rates
                .get(&self.best_profile)
                .unwrap_or(&0.0)
                * 100.0
        ));

        output
    }
}

/// Resource for collecting metrics during simulation
#[derive(Resource)]
pub struct SimMetrics {
    /// Current match stats for left player
    pub left: PlayerStats,
    /// Current match stats for right player
    pub right: PlayerStats,
    /// Match start time
    pub start_time: f32,
    /// Current elapsed time
    pub elapsed: f32,
    /// Last known positions (for distance tracking)
    pub last_pos_left: Option<Vec2>,
    pub last_pos_right: Option<Vec2>,
    /// Last known AI goals (for time tracking)
    pub last_goal_left: Option<AiGoal>,
    pub last_goal_right: Option<AiGoal>,
    /// Last update time for goal tracking
    pub last_goal_update: f32,
    /// Time since last score (for stalemate detection)
    pub time_since_score: f32,
    /// Match ended flag
    pub match_ended: bool,
    /// End reason
    pub end_reason: String,
    /// Previous jump state for transition detection [left, right]
    pub prev_jumping: [bool; 2],
    /// Previous nav active state [left, right]
    pub prev_nav_active: [bool; 2],
    /// Previous nav path length (to detect completion vs clear) [left, right]
    pub prev_nav_path_len: [usize; 2],
    /// Previous ball holder entity (for detecting shot release)
    pub prev_ball_holder: Option<Entity>,
}

impl Default for SimMetrics {
    fn default() -> Self {
        Self {
            left: PlayerStats::new(),
            right: PlayerStats::new(),
            start_time: 0.0,
            elapsed: 0.0,
            last_pos_left: None,
            last_pos_right: None,
            last_goal_left: None,
            last_goal_right: None,
            last_goal_update: 0.0,
            time_since_score: 0.0,
            match_ended: false,
            end_reason: String::new(),
            prev_jumping: [false, false],
            prev_nav_active: [false, false],
            prev_nav_path_len: [0, 0],
            prev_ball_holder: None,
        }
    }
}

impl SimMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Level sweep result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelSweepResult {
    pub profile: String,
    pub results_by_level: HashMap<u32, Vec<MatchResult>>,
    pub avg_score_by_level: HashMap<u32, f32>,
    pub win_rate_by_level: HashMap<u32, f32>,
}

impl LevelSweepResult {
    pub fn new(profile: &str) -> Self {
        Self {
            profile: profile.to_string(),
            results_by_level: HashMap::new(),
            avg_score_by_level: HashMap::new(),
            win_rate_by_level: HashMap::new(),
        }
    }

    pub fn calculate_stats(&mut self) {
        for (level, results) in &self.results_by_level {
            let total = results.len() as f32;
            if total == 0.0 {
                continue;
            }

            let total_score: u32 = results.iter().map(|r| r.score_left).sum();
            let wins = results.iter().filter(|r| r.winner == "left").count();

            self.avg_score_by_level
                .insert(*level, total_score as f32 / total);
            self.win_rate_by_level
                .insert(*level, wins as f32 / total);
        }
    }

    pub fn format_table(&self, level_names: &HashMap<u32, String>) -> String {
        let mut output = String::new();
        output.push_str(&format!(
            "\nLevel Sweep Results for {}:\n\n",
            self.profile
        ));
        output.push_str(&format!(
            "{:>5} | {:>20} | {:>10} | {:>10}\n",
            "Level", "Name", "Avg Score", "Win Rate"
        ));
        output.push_str(&format!("{:-<5}-+-{:-<20}-+-{:-<10}-+-{:-<10}\n", "", "", "", ""));

        let mut levels: Vec<_> = self.results_by_level.keys().collect();
        levels.sort();

        for level in levels {
            let name = level_names
                .get(level)
                .map(|s| s.as_str())
                .unwrap_or("Unknown");
            let avg = self.avg_score_by_level.get(level).unwrap_or(&0.0);
            let win = self.win_rate_by_level.get(level).unwrap_or(&0.0);
            output.push_str(&format!(
                "{:>5} | {:>20} | {:>10.2} | {:>9.1}%\n",
                level,
                &name[..name.len().min(20)],
                avg,
                win * 100.0
            ));
        }

        output
    }
}
