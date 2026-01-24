//! Profile leaderboard generation

use super::metrics::ProfileMetrics;

/// Profile ranking entry
#[derive(Debug, Clone)]
pub struct ProfileRanking {
    pub rank: usize,
    pub profile: String,
    pub win_rate: f32,
    pub goals_per_match: f32,
    pub shot_accuracy: f32,
    pub steals_per_match: f32,
    pub goal_differential: f32,
    pub matches_played: u32,
}

/// Leaderboard of profiles sorted by performance
#[derive(Debug, Clone, Default)]
pub struct Leaderboard {
    pub rankings: Vec<ProfileRanking>,
}

impl Leaderboard {
    /// Create leaderboard from profile metrics
    pub fn from_metrics(profiles: &[ProfileMetrics]) -> Self {
        let mut rankings: Vec<ProfileRanking> = profiles
            .iter()
            .filter(|p| p.matches_played > 0)
            .map(|p| ProfileRanking {
                rank: 0,
                profile: p.name.clone(),
                win_rate: p.win_rate(),
                goals_per_match: p.goals_per_match(),
                shot_accuracy: p.shot_accuracy(),
                steals_per_match: p.steals_per_match(),
                goal_differential: p.goal_differential(),
                matches_played: p.matches_played,
            })
            .collect();

        // Sort by win rate (descending), then by goals per match as tiebreaker
        rankings.sort_by(|a, b| {
            b.win_rate
                .partial_cmp(&a.win_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    b.goals_per_match
                        .partial_cmp(&a.goals_per_match)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        });

        // Assign ranks
        for (i, r) in rankings.iter_mut().enumerate() {
            r.rank = i + 1;
        }

        Self { rankings }
    }

    /// Get top N profiles
    pub fn top(&self, n: usize) -> Vec<&ProfileRanking> {
        self.rankings.iter().take(n).collect()
    }

    /// Get the best profile name
    pub fn best_profile(&self) -> Option<&str> {
        self.rankings.first().map(|r| r.profile.as_str())
    }

    /// Get the second best profile name (avoiding duplicates with first)
    pub fn second_best_profile(&self) -> Option<&str> {
        self.rankings.get(1).map(|r| r.profile.as_str())
    }

    /// Format as ASCII table
    pub fn format_table(&self) -> String {
        let mut output = String::new();
        output.push_str("\nPROFILE LEADERBOARD:\n");
        output.push_str("  Rank  Profile         Win%   Goals/Match  Accuracy  Steals  +/-\n");
        output.push_str("  ────────────────────────────────────────────────────────────────\n");

        for r in &self.rankings {
            output.push_str(&format!(
                "  {:>2}.   {:<14}  {:>5.1}%      {:>5.1}     {:>5.1}%    {:>4.1}  {:>+5.1}\n",
                r.rank,
                &r.profile[..r.profile.len().min(14)],
                r.win_rate * 100.0,
                r.goals_per_match,
                r.shot_accuracy * 100.0,
                r.steals_per_match,
                r.goal_differential,
            ));
        }

        output
    }

    /// Format compact summary
    pub fn format_compact(&self) -> String {
        if self.rankings.is_empty() {
            return "No profiles with matches\n".to_string();
        }

        let best = &self.rankings[0];
        format!(
            "Best: {} ({:.0}% win rate, {:.1} goals/match)\n",
            best.profile,
            best.win_rate * 100.0,
            best.goals_per_match
        )
    }
}
