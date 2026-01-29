//! Bracket seeding methods
//!
//! Provides different methods to seed a bracket tournament.

use rand::seq::SliceRandom;
use rayon::prelude::*;

use crate::ai::AiProfileDatabase;
use crate::levels::LevelDatabase;
use crate::simulation::config::SimConfig;
use crate::simulation::parallel::MatchConfig;
use crate::simulation::runner::run_match;

use super::types::{BracketEntry, SeedingMethod};

/// Warmup seeding result for a single profile
#[derive(Debug, Clone)]
pub struct WarmupResult {
    pub profile_id: String,
    pub profile_name: String,
    pub wins: u32,
    pub losses: u32,
    pub total_score: u32,
    pub total_opp_score: u32,
}

impl WarmupResult {
    pub fn win_rate(&self) -> f32 {
        let total = self.wins + self.losses;
        if total == 0 {
            0.0
        } else {
            self.wins as f32 / total as f32
        }
    }

    pub fn score_diff(&self) -> i32 {
        self.total_score as i32 - self.total_opp_score as i32
    }
}

/// Run warmup seeding games to rank profiles
///
/// Each profile plays `games_per_entrant` games against the baseline profile.
/// Profiles are then ranked by win rate, with score differential as tiebreaker.
pub fn warmup_seeding(
    profiles: &[String],
    baseline_profile: &str,
    games_per_entrant: u32,
    base_config: &SimConfig,
    base_seed: u64,
    level_db: &LevelDatabase,
    profile_db: &AiProfileDatabase,
    quiet: bool,
) -> Vec<WarmupResult> {
    if !quiet {
        println!(
            "Running warmup seeding: {} profiles x {} games vs {}",
            profiles.len(),
            games_per_entrant,
            baseline_profile
        );
    }

    // Build valid levels list
    let valid_levels: Vec<u32> = if base_config.levels.is_empty() {
        (1..=level_db.len() as u32)
            .filter(|&level| {
                if let Some(lvl) = level_db.get((level - 1) as usize) {
                    !lvl.debug && lvl.name != "Pit"
                } else {
                    false
                }
            })
            .collect()
    } else {
        base_config.levels.clone()
    };

    // Build all match configs
    let mut configs = Vec::new();
    let mut seed_counter = 0u64;

    for profile in profiles {
        if profile == baseline_profile {
            continue; // Skip baseline playing against itself
        }

        for _ in 0..games_per_entrant {
            seed_counter += 1;
            let seed = base_seed.wrapping_add(seed_counter);
            let level = base_config
                .level
                .unwrap_or_else(|| valid_levels[(seed as usize) % valid_levels.len()]);

            configs.push((
                profile.clone(),
                MatchConfig {
                    base_config: SimConfig {
                        duration_limit: base_config.duration_limit,
                        score_limit: base_config.score_limit,
                        quiet: true,
                        ..base_config.clone()
                    },
                    level,
                    left_profile: profile.clone(),
                    right_profile: baseline_profile.to_string(),
                    seed,
                },
            ));
        }
    }

    // Run all warmup games
    let total_games = configs.len();
    if !quiet {
        println!("  Running {} warmup games...", total_games);
    }

    let results: Vec<(String, bool, u32, u32)> = if base_config.parallel > 0 {
        configs
            .par_iter()
            .map(|(profile, config)| {
                let result = run_match(&config.base_config, config.seed, level_db, profile_db);
                let won = result.score_left > result.score_right;
                (profile.clone(), won, result.score_left, result.score_right)
            })
            .collect()
    } else {
        configs
            .iter()
            .enumerate()
            .map(|(i, (profile, config))| {
                if !quiet && (i + 1) % 10 == 0 {
                    eprint!("\r  Progress: {}/{}", i + 1, total_games);
                }
                let result = run_match(&config.base_config, config.seed, level_db, profile_db);
                let won = result.score_left > result.score_right;
                (profile.clone(), won, result.score_left, result.score_right)
            })
            .collect()
    };

    if !quiet && base_config.parallel == 0 {
        eprintln!(); // Clear progress line
    }

    // Aggregate results by profile
    let mut profile_results: std::collections::HashMap<String, WarmupResult> =
        std::collections::HashMap::new();

    for profile in profiles {
        if let Some(p) = profile_db.get_by_name(profile) {
            profile_results.insert(
                profile.clone(),
                WarmupResult {
                    profile_id: p.id.clone(),
                    profile_name: profile.clone(),
                    wins: 0,
                    losses: 0,
                    total_score: 0,
                    total_opp_score: 0,
                },
            );
        }
    }

    // Baseline gets a special entry (doesn't play against itself)
    if let Some(baseline) = profile_db.get_by_name(baseline_profile) {
        profile_results.insert(
            baseline_profile.to_string(),
            WarmupResult {
                profile_id: baseline.id.clone(),
                profile_name: baseline_profile.to_string(),
                // Give baseline average stats as a middle seed
                wins: games_per_entrant / 2,
                losses: games_per_entrant / 2,
                total_score: 0,
                total_opp_score: 0,
            },
        );
    }

    for (profile, won, score, opp_score) in results {
        if let Some(entry) = profile_results.get_mut(&profile) {
            if won {
                entry.wins += 1;
            } else {
                entry.losses += 1;
            }
            entry.total_score += score;
            entry.total_opp_score += opp_score;
        }
    }

    // Convert to vec and sort by win rate, then score diff
    let mut ranked: Vec<WarmupResult> = profile_results.into_values().collect();
    ranked.sort_by(|a, b| {
        // Primary: win rate (descending)
        let win_cmp = b
            .win_rate()
            .partial_cmp(&a.win_rate())
            .unwrap_or(std::cmp::Ordering::Equal);
        if win_cmp != std::cmp::Ordering::Equal {
            return win_cmp;
        }
        // Secondary: score differential (descending)
        b.score_diff().cmp(&a.score_diff())
    });

    if !quiet {
        println!("  Warmup seeding complete. Top seeds:");
        for (i, result) in ranked.iter().take(8).enumerate() {
            println!(
                "    #{}: {} ({:.0}% win rate, {}-{}, +{})",
                i + 1,
                result.profile_name,
                result.win_rate() * 100.0,
                result.wins,
                result.losses,
                result.score_diff()
            );
        }
        if ranked.len() > 8 {
            println!("    ... and {} more", ranked.len() - 8);
        }
    }

    ranked
}

/// Create bracket entries with seeding
///
/// Returns entries sorted by seed (best seed first).
pub fn seed_entries(
    profiles: &[String],
    method: &SeedingMethod,
    base_config: &SimConfig,
    base_seed: u64,
    level_db: &LevelDatabase,
    profile_db: &AiProfileDatabase,
    quiet: bool,
) -> Vec<BracketEntry> {
    match method {
        SeedingMethod::Random => {
            let mut rng = rand::thread_rng();
            let mut shuffled: Vec<_> = profiles.to_vec();
            shuffled.shuffle(&mut rng);

            shuffled
                .into_iter()
                .enumerate()
                .filter_map(|(i, name)| {
                    let profile = profile_db.get_by_name(&name)?;
                    Some(BracketEntry::new(profile.id.clone(), name, (i + 1) as u32))
                })
                .collect()
        }

        SeedingMethod::Warmup {
            baseline_profile,
            games_per_entrant,
        } => {
            let ranked = warmup_seeding(
                profiles,
                baseline_profile,
                *games_per_entrant,
                base_config,
                base_seed,
                level_db,
                profile_db,
                quiet,
            );

            ranked
                .into_iter()
                .enumerate()
                .map(|(i, result)| {
                    BracketEntry::new(result.profile_id, result.profile_name, (i + 1) as u32)
                })
                .collect()
        }

        SeedingMethod::Manual => {
            // Assume profiles are already in seed order
            profiles
                .iter()
                .enumerate()
                .filter_map(|(i, name)| {
                    let profile = profile_db.get_by_name(name)?;
                    Some(BracketEntry::new(
                        profile.id.clone(),
                        name.clone(),
                        (i + 1) as u32,
                    ))
                })
                .collect()
        }

        SeedingMethod::DatabaseOrder => {
            // Use order from database
            profile_db
                .profiles()
                .iter()
                .enumerate()
                .filter(|(_, p)| profiles.contains(&p.name))
                .map(|(i, p)| BracketEntry::new(p.id.clone(), p.name.clone(), (i + 1) as u32))
                .collect()
        }
    }
}

/// Pad entries to power of 2 by adding byes (empty entries that auto-lose)
///
/// This shouldn't be needed if we filter profiles correctly, but just in case.
pub fn pad_to_power_of_2(entries: &mut Vec<BracketEntry>) {
    let n = entries.len();
    if n.is_power_of_two() {
        return;
    }

    // Find next power of 2
    let target = n.next_power_of_two();

    // Add bye entries
    for i in n..target {
        entries.push(BracketEntry::new(
            format!("bye_{}", i),
            format!("BYE #{}", i - n + 1),
            (i + 1) as u32,
        ));
    }
}

/// Select profiles for the bracket
///
/// If more profiles than entrant slots, returns first N in database order
/// (assuming they're sorted by quality/recency).
pub fn select_profiles(
    profile_db: &AiProfileDatabase,
    config_profiles: &[String],
    max_entrants: usize,
) -> Vec<String> {
    let all_profiles: Vec<String> = if config_profiles.is_empty() {
        profile_db
            .profiles()
            .iter()
            .map(|p| p.name.clone())
            .collect()
    } else {
        config_profiles
            .iter()
            .filter(|name| profile_db.get_by_name(name).is_some())
            .cloned()
            .collect()
    };

    // Take first max_entrants profiles
    all_profiles.into_iter().take(max_entrants).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warmup_result_win_rate() {
        let result = WarmupResult {
            profile_id: "test".to_string(),
            profile_name: "Test".to_string(),
            wins: 3,
            losses: 2,
            total_score: 15,
            total_opp_score: 10,
        };

        assert!((result.win_rate() - 0.6).abs() < 0.001);
        assert_eq!(result.score_diff(), 5);
    }

    #[test]
    fn test_pad_to_power_of_2() {
        let mut entries = vec![
            BracketEntry::new("a".to_string(), "A".to_string(), 1),
            BracketEntry::new("b".to_string(), "B".to_string(), 2),
            BracketEntry::new("c".to_string(), "C".to_string(), 3),
        ];

        pad_to_power_of_2(&mut entries);
        assert_eq!(entries.len(), 4);
        assert!(entries[3].profile_name.contains("BYE"));
    }

    #[test]
    fn test_pad_to_power_of_2_already_power() {
        let mut entries = vec![
            BracketEntry::new("a".to_string(), "A".to_string(), 1),
            BracketEntry::new("b".to_string(), "B".to_string(), 2),
            BracketEntry::new("c".to_string(), "C".to_string(), 3),
            BracketEntry::new("d".to_string(), "D".to_string(), 4),
        ];

        pad_to_power_of_2(&mut entries);
        assert_eq!(entries.len(), 4);
    }
}
