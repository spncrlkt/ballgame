//! Aggregate metrics computation for analytics

use std::collections::HashMap;

use super::parser::ParsedMatch;
use crate::events::PlayerId;

/// Per-profile aggregated metrics
#[derive(Debug, Clone, Default)]
pub struct ProfileMetrics {
    /// Profile name
    pub name: String,
    /// Number of matches played
    pub matches_played: u32,
    /// Number of matches won
    pub matches_won: u32,
    /// Number of matches lost
    pub matches_lost: u32,
    /// Number of ties
    pub matches_tied: u32,
    /// Total goals scored
    pub total_goals: u32,
    /// Total goals conceded
    pub total_goals_against: u32,
    /// Total shots taken
    pub total_shots: u32,
    /// Total successful shots (goals)
    pub shots_made: u32,
    /// Total steal attempts
    pub steal_attempts: u32,
    /// Total successful steals
    pub steal_successes: u32,
    /// Total pickups
    pub pickups: u32,
    /// Total match time (seconds)
    pub total_match_time: f32,
}

impl ProfileMetrics {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Win rate (0.0 - 1.0)
    pub fn win_rate(&self) -> f32 {
        if self.matches_played == 0 {
            0.0
        } else {
            self.matches_won as f32 / self.matches_played as f32
        }
    }

    /// Win rate including half-wins for ties
    pub fn win_rate_with_ties(&self) -> f32 {
        if self.matches_played == 0 {
            0.0
        } else {
            (self.matches_won as f32 + self.matches_tied as f32 * 0.5) / self.matches_played as f32
        }
    }

    /// Goals per match
    pub fn goals_per_match(&self) -> f32 {
        if self.matches_played == 0 {
            0.0
        } else {
            self.total_goals as f32 / self.matches_played as f32
        }
    }

    /// Shot accuracy (0.0 - 1.0)
    pub fn shot_accuracy(&self) -> f32 {
        if self.total_shots == 0 {
            0.0
        } else {
            self.shots_made as f32 / self.total_shots as f32
        }
    }

    /// Steals per match
    pub fn steals_per_match(&self) -> f32 {
        if self.matches_played == 0 {
            0.0
        } else {
            self.steal_successes as f32 / self.matches_played as f32
        }
    }

    /// Steal success rate
    pub fn steal_success_rate(&self) -> f32 {
        if self.steal_attempts == 0 {
            0.0
        } else {
            self.steal_successes as f32 / self.steal_attempts as f32
        }
    }

    /// Goal differential per match
    pub fn goal_differential(&self) -> f32 {
        if self.matches_played == 0 {
            0.0
        } else {
            (self.total_goals as i32 - self.total_goals_against as i32) as f32
                / self.matches_played as f32
        }
    }

    /// Add stats from a match where this profile was the left player
    pub fn add_match_as_left(&mut self, m: &ParsedMatch) {
        self.matches_played += 1;
        self.total_match_time += m.duration;

        // Win/loss/tie
        match m.winner() {
            "left" => self.matches_won += 1,
            "right" => self.matches_lost += 1,
            _ => self.matches_tied += 1,
        }

        // Goals
        self.total_goals += m.score_left;
        self.total_goals_against += m.score_right;
        self.shots_made += m.goals_for(PlayerId::L) as u32;

        // Shots
        self.total_shots += m.shots_for(PlayerId::L) as u32;

        // Steals
        self.steal_attempts += m.steal_attempts_for(PlayerId::L) as u32;
        self.steal_successes += m.steal_successes_for(PlayerId::L) as u32;

        // Pickups
        self.pickups += m.pickups_for(PlayerId::L) as u32;
    }

    /// Add stats from a match where this profile was the right player
    pub fn add_match_as_right(&mut self, m: &ParsedMatch) {
        self.matches_played += 1;
        self.total_match_time += m.duration;

        // Win/loss/tie
        match m.winner() {
            "right" => self.matches_won += 1,
            "left" => self.matches_lost += 1,
            _ => self.matches_tied += 1,
        }

        // Goals
        self.total_goals += m.score_right;
        self.total_goals_against += m.score_left;
        self.shots_made += m.goals_for(PlayerId::R) as u32;

        // Shots
        self.total_shots += m.shots_for(PlayerId::R) as u32;

        // Steals
        self.steal_attempts += m.steal_attempts_for(PlayerId::R) as u32;
        self.steal_successes += m.steal_successes_for(PlayerId::R) as u32;

        // Pickups
        self.pickups += m.pickups_for(PlayerId::R) as u32;
    }
}

/// Aggregate metrics across all matches
#[derive(Debug, Clone, Default)]
pub struct AggregateMetrics {
    /// Total number of matches analyzed
    pub total_matches: u32,
    /// Total simulated time (seconds)
    pub total_time: f32,
    /// Average match duration
    pub avg_duration: f32,
    /// Average total score per match (left + right)
    pub avg_total_score: f32,
    /// Average score differential
    pub avg_score_differential: f32,
    /// Total turnovers (steals + drops)
    pub total_turnovers: u32,
    /// Average turnovers per match
    pub avg_turnovers: f32,
    /// Total missed shots
    pub total_missed_shots: u32,
    /// Average missed shots per match
    pub avg_missed_shots: f32,
    /// Total shots
    pub total_shots: u32,
    /// Total goals
    pub total_goals: u32,
    /// Per-profile metrics
    pub by_profile: HashMap<String, ProfileMetrics>,
}

impl AggregateMetrics {
    /// Compute aggregate metrics from parsed matches
    pub fn from_matches(matches: &[ParsedMatch]) -> Self {
        let mut agg = Self::default();
        agg.total_matches = matches.len() as u32;

        for m in matches {
            agg.total_time += m.duration;
            agg.total_goals += m.score_left + m.score_right;

            // Shots
            let shots = m.shots.len() as u32;
            let goals = m.score_left + m.score_right;
            agg.total_shots += shots;
            agg.total_missed_shots += shots.saturating_sub(goals);

            // Turnovers = steals + drops
            agg.total_turnovers += (m.steal_successes.len() + m.drops.len()) as u32;

            // Update per-profile metrics
            let left_profile = &m.left_profile;
            let right_profile = &m.right_profile;

            agg.by_profile
                .entry(left_profile.clone())
                .or_insert_with(|| ProfileMetrics::new(left_profile))
                .add_match_as_left(m);

            agg.by_profile
                .entry(right_profile.clone())
                .or_insert_with(|| ProfileMetrics::new(right_profile))
                .add_match_as_right(m);
        }

        // Compute averages
        if agg.total_matches > 0 {
            let n = agg.total_matches as f32;
            agg.avg_duration = agg.total_time / n;
            agg.avg_total_score = agg.total_goals as f32 / n;
            agg.avg_turnovers = agg.total_turnovers as f32 / n;
            agg.avg_missed_shots = agg.total_missed_shots as f32 / n;

            // Score differential
            let total_diff: i32 = matches
                .iter()
                .map(|m| (m.score_left as i32 - m.score_right as i32).abs())
                .sum();
            agg.avg_score_differential = total_diff as f32 / n;
        }

        agg
    }

    /// Format a summary report
    pub fn format_summary(&self) -> String {
        let hours = self.total_time / 3600.0;
        let mins = (self.total_time % 3600.0) / 60.0;

        format!(
            "SIMULATION SUMMARY ({} matches, {}h {:.0}m simulated)\n\
             ============================================================\n\n\
             Duration:            avg {:.1}s per match\n\
             Score:               avg {:.1} total per match\n\
             Score Differential:  avg {:.1} per match\n\
             Turnovers:           avg {:.1} per match ({} total)\n\
             Missed Shots:        avg {:.1} per match ({} total)\n\
             Shot Accuracy:       {:.1}% overall\n",
            self.total_matches,
            hours as u32,
            mins,
            self.avg_duration,
            self.avg_total_score,
            self.avg_score_differential,
            self.avg_turnovers,
            self.total_turnovers,
            self.avg_missed_shots,
            self.total_missed_shots,
            if self.total_shots > 0 {
                (self.total_goals as f32 / self.total_shots as f32) * 100.0
            } else {
                0.0
            }
        )
    }
}
