//! Database-backed analytics for cross-session analysis
//!
//! Provides profile analysis, comparison, and aggregation from SQLite database.

use crate::simulation::{MatchFilter, ProfileStats, SimDatabase};

/// Extended profile analysis from database
#[derive(Debug, Clone)]
pub struct ProfileAnalysis {
    /// Basic stats from database
    pub stats: ProfileStats,
    /// Shot accuracy (goals / shots)
    pub shot_accuracy: f64,
    /// Steal success rate
    pub steal_success_rate: f64,
    /// Goals per match
    pub goals_per_match: f64,
    /// Goal differential per match
    pub goal_differential: f64,
    /// Average match duration
    pub avg_duration: f64,
}

impl ProfileAnalysis {
    /// Format as a report string
    pub fn format_report(&self) -> String {
        let losses = self.stats.losses();
        format!(
            "Profile: {}\n\
             ----------------------------------------\n\
             Matches:     {} ({} W / {} L / {} T)\n\
             Win Rate:    {:.1}%\n\
             Avg Score:   {:.2} vs {:.2} opponent\n\
             Goals/Match: {:.2}\n\
             Goal Diff:   {:+.2}\n\
             Shot Acc:    {:.1}%\n\
             Steal Rate:  {:.1}%\n\
             Avg Duration:{:.1}s\n",
            self.stats.profile,
            self.stats.matches,
            self.stats.wins,
            losses,
            self.stats.ties,
            self.stats.win_rate() * 100.0,
            self.stats.avg_score,
            self.stats.avg_opponent_score,
            self.goals_per_match,
            self.goal_differential,
            self.shot_accuracy * 100.0,
            self.steal_success_rate * 100.0,
            self.avg_duration,
        )
    }
}

/// Compare two or more profiles side by side
#[derive(Debug, Clone)]
pub struct ProfileComparison {
    /// Profiles being compared
    pub profiles: Vec<ProfileAnalysis>,
}

impl ProfileComparison {
    /// Format as a comparison table
    pub fn format_table(&self) -> String {
        if self.profiles.is_empty() {
            return "No profiles to compare".to_string();
        }

        let mut output = String::new();
        output.push_str("PROFILE COMPARISON\n");
        output.push_str("==================\n\n");

        // Header
        output.push_str(&format!("{:<15}", "Metric"));
        for p in &self.profiles {
            output.push_str(&format!("{:>12}", &p.stats.profile));
        }
        output.push('\n');
        output.push_str(&"-".repeat(15 + self.profiles.len() * 12));
        output.push('\n');

        // Matches
        output.push_str(&format!("{:<15}", "Matches"));
        for p in &self.profiles {
            output.push_str(&format!("{:>12}", p.stats.matches));
        }
        output.push('\n');

        // Win Rate
        output.push_str(&format!("{:<15}", "Win Rate"));
        for p in &self.profiles {
            output.push_str(&format!("{:>11.1}%", p.stats.win_rate() * 100.0));
        }
        output.push('\n');

        // Avg Score
        output.push_str(&format!("{:<15}", "Avg Score"));
        for p in &self.profiles {
            output.push_str(&format!("{:>12.2}", p.stats.avg_score));
        }
        output.push('\n');

        // Opp Score
        output.push_str(&format!("{:<15}", "Opp Score"));
        for p in &self.profiles {
            output.push_str(&format!("{:>12.2}", p.stats.avg_opponent_score));
        }
        output.push('\n');

        // Goal Diff
        output.push_str(&format!("{:<15}", "Goal Diff"));
        for p in &self.profiles {
            output.push_str(&format!("{:>+12.2}", p.goal_differential));
        }
        output.push('\n');

        // Shot Accuracy
        output.push_str(&format!("{:<15}", "Shot Acc"));
        for p in &self.profiles {
            output.push_str(&format!("{:>11.1}%", p.shot_accuracy * 100.0));
        }
        output.push('\n');

        // Steal Rate
        output.push_str(&format!("{:<15}", "Steal Rate"));
        for p in &self.profiles {
            output.push_str(&format!("{:>11.1}%", p.steal_success_rate * 100.0));
        }
        output.push('\n');

        output
    }

    /// Identify the best profile for each metric
    pub fn best_for_each_metric(&self) -> Vec<(&str, &str)> {
        if self.profiles.is_empty() {
            return vec![];
        }

        let mut bests = Vec::new();

        // Win Rate
        if let Some(best) = self
            .profiles
            .iter()
            .max_by(|a, b| a.stats.win_rate().partial_cmp(&b.stats.win_rate()).unwrap())
        {
            bests.push(("Win Rate", best.stats.profile.as_str()));
        }

        // Goal Differential
        if let Some(best) = self.profiles.iter().max_by(|a, b| {
            a.goal_differential
                .partial_cmp(&b.goal_differential)
                .unwrap()
        }) {
            bests.push(("Goal Diff", best.stats.profile.as_str()));
        }

        // Shot Accuracy
        if let Some(best) = self
            .profiles
            .iter()
            .max_by(|a, b| a.shot_accuracy.partial_cmp(&b.shot_accuracy).unwrap())
        {
            bests.push(("Shot Accuracy", best.stats.profile.as_str()));
        }

        // Steal Rate
        if let Some(best) = self.profiles.iter().max_by(|a, b| {
            a.steal_success_rate
                .partial_cmp(&b.steal_success_rate)
                .unwrap()
        }) {
            bests.push(("Steal Rate", best.stats.profile.as_str()));
        }

        bests
    }
}

/// Analyze a profile from database results
pub fn analyze_profile(db: &SimDatabase, profile: &str) -> Result<ProfileAnalysis, String> {
    let stats = db
        .get_profile_stats(profile)
        .map_err(|e| format!("Database error: {}", e))?;

    // Get all matches for this profile to compute additional metrics
    let filter = MatchFilter {
        profile: Some(profile.to_string()),
        ..Default::default()
    };
    let matches = db
        .query_matches(&filter)
        .map_err(|e| format!("Database error: {}", e))?;

    // Compute additional metrics from match data
    let total_duration: f32 = matches.iter().map(|m| m.duration).sum();
    let avg_duration = if matches.is_empty() {
        0.0
    } else {
        total_duration as f64 / matches.len() as f64
    };

    // For shot accuracy and steal rate, we'd need the player_stats table
    // For now, use approximations from available data
    let goal_differential = stats.avg_score - stats.avg_opponent_score;
    let goals_per_match = stats.avg_score;

    // These would ideally come from player_stats table queries
    // For now, return placeholder values
    let shot_accuracy = 0.0; // Would need shots_attempted, shots_made
    let steal_success_rate = 0.0; // Would need steals_attempted, steals_successful

    Ok(ProfileAnalysis {
        stats,
        shot_accuracy,
        steal_success_rate,
        goals_per_match,
        goal_differential,
        avg_duration,
    })
}

/// Compare multiple profiles from database
pub fn compare_profiles(db: &SimDatabase, profiles: &[&str]) -> Result<ProfileComparison, String> {
    let mut analyses = Vec::new();

    for profile in profiles {
        let analysis = analyze_profile(db, profile)?;
        analyses.push(analysis);
    }

    Ok(ProfileComparison { profiles: analyses })
}

/// Get detailed profile stats including player_stats data
pub fn get_detailed_profile_stats(
    db: &SimDatabase,
    profile: &str,
) -> Result<DetailedProfileStats, String> {
    // First get basic stats
    let basic = db
        .get_profile_stats(profile)
        .map_err(|e| format!("Database error: {}", e))?;

    // Query player_stats for this profile
    // We need to join matches and player_stats
    let _conn = &db;

    // For now, return a struct with the basic stats
    // Full implementation would query player_stats table
    Ok(DetailedProfileStats {
        basic,
        total_shots: 0,
        total_goals: 0,
        total_steals_attempted: 0,
        total_steals_successful: 0,
        total_possession_time: 0.0,
        total_distance_traveled: 0.0,
    })
}

/// Detailed profile statistics including player_stats data
#[derive(Debug, Clone)]
pub struct DetailedProfileStats {
    /// Basic win/loss stats
    pub basic: ProfileStats,
    /// Total shots attempted
    pub total_shots: u32,
    /// Total goals scored
    pub total_goals: u32,
    /// Total steal attempts
    pub total_steals_attempted: u32,
    /// Total successful steals
    pub total_steals_successful: u32,
    /// Total possession time (seconds)
    pub total_possession_time: f64,
    /// Total distance traveled
    pub total_distance_traveled: f64,
}

impl DetailedProfileStats {
    /// Shot accuracy (0.0 - 1.0)
    pub fn shot_accuracy(&self) -> f64 {
        if self.total_shots == 0 {
            0.0
        } else {
            self.total_goals as f64 / self.total_shots as f64
        }
    }

    /// Steal success rate (0.0 - 1.0)
    pub fn steal_success_rate(&self) -> f64 {
        if self.total_steals_attempted == 0 {
            0.0
        } else {
            self.total_steals_successful as f64 / self.total_steals_attempted as f64
        }
    }
}

/// Summary of all profiles in the database
pub fn summarize_all_profiles(db: &SimDatabase) -> Result<Vec<ProfileAnalysis>, String> {
    // Get unique profiles from matches
    let all_matches = db
        .query_matches(&MatchFilter::default())
        .map_err(|e| format!("Database error: {}", e))?;

    let mut profiles: std::collections::HashSet<String> = std::collections::HashSet::new();
    for m in &all_matches {
        profiles.insert(m.left_profile.clone());
        profiles.insert(m.right_profile.clone());
    }

    let mut analyses = Vec::new();
    for profile in profiles {
        if let Ok(analysis) = analyze_profile(db, &profile) {
            analyses.push(analysis);
        }
    }

    // Sort by win rate descending
    analyses.sort_by(|a, b| b.stats.win_rate().partial_cmp(&a.stats.win_rate()).unwrap());

    Ok(analyses)
}

/// Format a leaderboard of all profiles
pub fn format_leaderboard(analyses: &[ProfileAnalysis]) -> String {
    let mut output = String::new();
    output.push_str("PROFILE LEADERBOARD\n");
    output.push_str("===================\n\n");
    output.push_str(&format!(
        "{:<3} {:<12} {:>6} {:>8} {:>8} {:>10}\n",
        "#", "Profile", "Games", "Win%", "GoalDif", "AvgScore"
    ));
    output.push_str(&"-".repeat(52));
    output.push('\n');

    for (i, a) in analyses.iter().enumerate() {
        output.push_str(&format!(
            "{:<3} {:<12} {:>6} {:>7.1}% {:>+8.2} {:>10.2}\n",
            i + 1,
            &a.stats.profile,
            a.stats.matches,
            a.stats.win_rate() * 100.0,
            a.goal_differential,
            a.stats.avg_score,
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::metrics::{MatchResult, PlayerStats};

    fn create_test_db() -> SimDatabase {
        let db = SimDatabase::open_in_memory().unwrap();
        let session_id = db.create_session("test", None).unwrap();

        // Add some test matches
        for i in 0..5 {
            let result = MatchResult {
                level: 3,
                level_name: "Test Level".to_string(),
                left_profile: "Aggressive".to_string(),
                right_profile: "Defensive".to_string(),
                duration: 45.0,
                score_left: 3 + (i % 2),
                score_right: 2,
                winner: "left".to_string(),
                left_stats: PlayerStats::default(),
                right_stats: PlayerStats::default(),
                seed: i as u64,
                events: Vec::new(),
            };
            db.insert_match(&session_id, &result).unwrap();
        }

        db
    }

    #[test]
    fn test_analyze_profile() {
        let db = create_test_db();
        let analysis = analyze_profile(&db, "Aggressive").unwrap();

        assert_eq!(analysis.stats.matches, 5);
        assert_eq!(analysis.stats.wins, 5);
        assert!(analysis.stats.win_rate() > 0.9);
    }

    #[test]
    fn test_compare_profiles() {
        let db = create_test_db();
        let comparison = compare_profiles(&db, &["Aggressive", "Defensive"]).unwrap();

        assert_eq!(comparison.profiles.len(), 2);

        let table = comparison.format_table();
        assert!(table.contains("Aggressive"));
        assert!(table.contains("Defensive"));
    }

    #[test]
    fn test_summarize_all() {
        let db = create_test_db();
        let analyses = summarize_all_profiles(&db).unwrap();

        assert_eq!(analyses.len(), 2);
        // Aggressive should be first (higher win rate)
        assert_eq!(analyses[0].stats.profile, "Aggressive");
    }
}
