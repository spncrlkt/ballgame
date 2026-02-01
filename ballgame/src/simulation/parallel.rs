//! Parallel simulation execution
//!
//! Uses Rayon to run multiple simulations concurrently.
//! Each simulation runs in its own Bevy app with minimal threading
//! to avoid hitting OS thread limits.

use rayon::prelude::*;

use crate::ai::AiProfileDatabase;
use crate::levels::LevelDatabase;

use super::config::SimConfig;
use super::metrics::MatchResult;
use super::runner::run_match;

/// Configuration for parallel execution
pub struct ParallelConfig {
    /// Number of threads to use (0 = auto-detect)
    pub threads: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self { threads: 0 }
    }
}

/// Initialize parallel execution with the given thread count.
/// Call this once at startup before running parallel simulations.
pub fn init_parallel(threads: usize) {
    if threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .expect("Failed to initialize Rayon thread pool");
    }
    // If threads == 0, use Rayon's default (auto-detect)
}

/// Run multiple matches in parallel
///
/// Each match gets a unique seed derived from the base seed.
/// Returns results in the same order as configs.
pub fn run_matches_parallel(
    configs: &[MatchConfig],
    level_db: &LevelDatabase,
    profile_db: &AiProfileDatabase,
) -> Vec<MatchResult> {
    configs
        .par_iter()
        .map(|cfg| {
            let mut sim_config = cfg.base_config.clone();
            sim_config.level = Some(cfg.level);
            sim_config.left_profile = cfg.left_profile.clone();
            sim_config.right_profile = cfg.right_profile.clone();
            run_match(&sim_config, cfg.seed, level_db, profile_db)
        })
        .collect()
}

/// Configuration for a single match in a parallel batch
#[derive(Clone)]
pub struct MatchConfig {
    /// Base simulation config (mode, limits, etc.)
    pub base_config: SimConfig,
    /// Level to play on
    pub level: u32,
    /// Left player profile name
    pub left_profile: String,
    /// Right player profile name
    pub right_profile: String,
    /// RNG seed for this match
    pub seed: u64,
}

/// Run a tournament in parallel
///
/// Runs all profile matchups concurrently, collecting results.
pub fn run_tournament_parallel(
    base_config: &SimConfig,
    matches_per_pair: u32,
    base_seed: u64,
    level_db: &LevelDatabase,
    profile_db: &AiProfileDatabase,
) -> Vec<MatchResult> {
    // Use config profiles if specified, otherwise use all profiles from database
    let profiles: Vec<String> = if base_config.profiles.is_empty() {
        profile_db
            .profiles()
            .iter()
            .map(|p| p.name.clone())
            .collect()
    } else {
        base_config
            .profiles
            .iter()
            .filter(|p| profile_db.get_by_name(p).is_some())
            .cloned()
            .collect()
    };

    // Use config levels if specified, otherwise build list excluding debug levels and Pit
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

    // Build all match configurations
    let mut configs = Vec::new();
    let mut match_num = 0u64;

    for left in &profiles {
        for right in &profiles {
            if left == right {
                continue;
            }
            for _ in 0..matches_per_pair {
                match_num += 1;
                let seed = base_seed.wrapping_add(match_num);
                // Use specified level or pick random based on seed
                let level = base_config.level.unwrap_or_else(|| {
                    let idx = (seed as usize) % valid_levels.len();
                    valid_levels[idx]
                });
                configs.push(MatchConfig {
                    base_config: base_config.clone(),
                    level,
                    left_profile: left.clone(),
                    right_profile: right.clone(),
                    seed,
                });
            }
        }
    }

    run_matches_parallel(&configs, level_db, profile_db)
}

/// Run multi-match in parallel
///
/// Runs the same matchup multiple times concurrently.
pub fn run_multi_match_parallel(
    base_config: &SimConfig,
    count: u32,
    base_seed: u64,
    level_db: &LevelDatabase,
    profile_db: &AiProfileDatabase,
) -> Vec<MatchResult> {
    // Use config levels if specified, otherwise build list excluding debug levels and Pit
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

    let configs: Vec<_> = (0..count)
        .map(|i| {
            let seed = base_seed.wrapping_add(i as u64);
            let level = base_config.level.unwrap_or_else(|| {
                let idx = (seed as usize) % valid_levels.len();
                valid_levels[idx]
            });
            MatchConfig {
                base_config: base_config.clone(),
                level,
                left_profile: base_config.left_profile.clone(),
                right_profile: base_config.right_profile.clone(),
                seed,
            }
        })
        .collect();

    run_matches_parallel(&configs, level_db, profile_db)
}

/// Run level sweep in parallel
///
/// Runs matches across all levels concurrently.
pub fn run_level_sweep_parallel(
    base_config: &SimConfig,
    matches_per_level: u32,
    base_seed: u64,
    level_db: &LevelDatabase,
    profile_db: &AiProfileDatabase,
) -> Vec<MatchResult> {
    let mut configs = Vec::new();
    let mut match_num = 0u64;

    for level_idx in 0..level_db.len() {
        // Skip debug levels
        if level_db.get(level_idx).is_some_and(|l| l.debug) {
            continue;
        }

        let level = (level_idx + 1) as u32;
        for _ in 0..matches_per_level {
            match_num += 1;
            configs.push(MatchConfig {
                base_config: base_config.clone(),
                level,
                left_profile: base_config.left_profile.clone(),
                right_profile: base_config.right_profile.clone(),
                seed: base_seed.wrapping_add(match_num),
            });
        }
    }

    run_matches_parallel(&configs, level_db, profile_db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_config_clone() {
        let config = MatchConfig {
            base_config: SimConfig::default(),
            level: 3,
            left_profile: "Test".to_string(),
            right_profile: "Other".to_string(),
            seed: 12345,
        };
        let cloned = config.clone();
        assert_eq!(cloned.level, 3);
        assert_eq!(cloned.seed, 12345);
    }
}
