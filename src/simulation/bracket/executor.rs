//! Bracket tournament executor
//!
//! Handles running bracket matches with parallel execution support.

use rayon::prelude::*;

use crate::ai::AiProfileDatabase;
use crate::levels::LevelDatabase;

use super::types::{BracketMatchResult, BracketState, GameResult, MatchFormat};
use crate::simulation::config::SimConfig;
use crate::simulation::parallel::MatchConfig;
use crate::simulation::runner::run_match;

/// Executor for running bracket tournaments
pub struct BracketExecutor<'a> {
    /// Level database
    level_db: &'a LevelDatabase,
    /// Profile database
    profile_db: &'a AiProfileDatabase,
    /// Base simulation config
    base_config: &'a SimConfig,
    /// Valid levels to use
    valid_levels: Vec<u32>,
    /// Base RNG seed
    base_seed: u64,
    /// Counter for generating unique seeds
    seed_counter: u64,
    /// Whether to print progress
    quiet: bool,
}

impl<'a> BracketExecutor<'a> {
    /// Create a new bracket executor
    pub fn new(
        level_db: &'a LevelDatabase,
        profile_db: &'a AiProfileDatabase,
        base_config: &'a SimConfig,
        base_seed: u64,
    ) -> Self {
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

        Self {
            level_db,
            profile_db,
            base_config,
            valid_levels,
            base_seed,
            seed_counter: 0,
            quiet: base_config.quiet,
        }
    }

    /// Generate a unique seed for a game
    fn next_seed(&mut self) -> u64 {
        self.seed_counter += 1;
        self.base_seed.wrapping_add(self.seed_counter)
    }

    /// Get a level for a game based on seed
    fn get_level(&self, seed: u64) -> u32 {
        if let Some(level) = self.base_config.level {
            level
        } else {
            let idx = (seed as usize) % self.valid_levels.len();
            self.valid_levels[idx]
        }
    }

    /// Run the entire bracket tournament
    pub fn run_tournament(&mut self, bracket: &mut BracketState) {
        while !bracket.is_complete {
            let ready_ids = bracket.ready_match_ids();

            if ready_ids.is_empty() {
                // Check if we need grand finals reset
                if bracket.needs_grand_finals_reset() {
                    // Set up grand finals reset if needed
                    // First, get the players from grand finals match
                    let gf_players = bracket
                        .matches
                        .iter()
                        .find(|m| m.side == super::types::BracketSide::GrandFinals)
                        .map(|m| m.players);

                    // Then update the reset match
                    if let Some(players) = gf_players {
                        if let Some(reset_match) = bracket
                            .matches
                            .iter_mut()
                            .find(|m| m.side == super::types::BracketSide::GrandFinalsReset)
                        {
                            if reset_match.result.is_none() {
                                reset_match.players = players;
                            }
                        }
                    }
                    continue;
                }

                // No more matches to play
                break;
            }

            self.execute_round(bracket, &ready_ids);
        }
    }

    /// Execute all matches in a round (in parallel if configured)
    fn execute_round(&mut self, bracket: &mut BracketState, match_ids: &[u32]) {
        if !self.quiet {
            let first_match = bracket.get_match(match_ids[0]);
            if let Some(m) = first_match {
                println!(
                    "  {} Round {} ({} matches)",
                    m.side,
                    m.round,
                    match_ids.len()
                );
            }
        }

        // Collect match info for parallel execution
        let match_infos: Vec<_> = match_ids
            .iter()
            .filter_map(|&id| {
                let m = bracket.get_match(id)?;
                let p1_idx = m.players[0]?;
                let p2_idx = m.players[1]?;
                let p1 = &bracket.entries[p1_idx];
                let p2 = &bracket.entries[p2_idx];
                Some((id, p1.profile_name.clone(), p2.profile_name.clone()))
            })
            .collect();

        // Generate seeds for all games upfront
        let games_per_match = bracket.format.best_of;
        let total_games = match_infos.len() * games_per_match as usize;
        let seeds: Vec<u64> = (0..total_games).map(|_| self.next_seed()).collect();

        // Execute all matches in parallel
        let results: Vec<(u32, BracketMatchResult)> = if self.base_config.parallel > 0 {
            match_infos
                .par_iter()
                .enumerate()
                .map(|(match_idx, (match_id, p1_name, p2_name))| {
                    let seed_start = match_idx * games_per_match as usize;
                    let match_seeds = &seeds[seed_start..seed_start + games_per_match as usize];
                    let result = self.execute_bracket_match_parallel(
                        p1_name,
                        p2_name,
                        &bracket.format,
                        match_seeds,
                    );
                    (*match_id, result)
                })
                .collect()
        } else {
            match_infos
                .iter()
                .enumerate()
                .map(|(match_idx, (match_id, p1_name, p2_name))| {
                    let seed_start = match_idx * games_per_match as usize;
                    let match_seeds = &seeds[seed_start..seed_start + games_per_match as usize];
                    let result =
                        self.execute_bracket_match(p1_name, p2_name, &bracket.format, match_seeds);
                    (*match_id, result)
                })
                .collect()
        };

        // Record results (sequential to avoid borrow issues)
        for (match_id, result) in results {
            // Print result if not quiet
            if !self.quiet {
                let m = bracket.get_match(match_id).unwrap();
                let p1_name = &bracket.entries[m.players[0].unwrap()].profile_name;
                let p2_name = &bracket.entries[m.players[1].unwrap()].profile_name;
                let winner_name = if result.winner_index == 0 {
                    p1_name
                } else {
                    p2_name
                };
                println!(
                    "    {} vs {} -> {} ({}-{})",
                    truncate_name(p1_name, 12),
                    truncate_name(p2_name, 12),
                    truncate_name(winner_name, 12),
                    result.player1_wins,
                    result.player2_wins
                );
            }

            bracket.record_result(match_id, result);
        }
    }

    /// Execute a single bracket match (best of N games) - sequential version
    fn execute_bracket_match(
        &self,
        p1_name: &str,
        p2_name: &str,
        format: &MatchFormat,
        seeds: &[u64],
    ) -> BracketMatchResult {
        let wins_needed = format.wins_needed();
        let mut p1_wins = 0u32;
        let mut p2_wins = 0u32;
        let mut games = Vec::new();

        for &seed in seeds {
            if p1_wins >= wins_needed || p2_wins >= wins_needed {
                break;
            }

            let game_result = self.execute_game(p1_name, p2_name, format, seed);
            if game_result.winner == 1 {
                p1_wins += 1;
            } else {
                p2_wins += 1;
            }
            games.push(game_result);
        }

        let winner_index = if p1_wins >= wins_needed { 0 } else { 1 };

        BracketMatchResult {
            games,
            player1_wins: p1_wins,
            player2_wins: p2_wins,
            winner_index,
            loser_index: 1 - winner_index,
        }
    }

    /// Execute a single bracket match (best of N games) - parallel version
    /// Uses speculative execution: runs all possible games in parallel, then trims
    fn execute_bracket_match_parallel(
        &self,
        p1_name: &str,
        p2_name: &str,
        format: &MatchFormat,
        seeds: &[u64],
    ) -> BracketMatchResult {
        // Run all games in parallel (speculative)
        let all_games: Vec<GameResult> = seeds
            .par_iter()
            .map(|&seed| self.execute_game(p1_name, p2_name, format, seed))
            .collect();

        // Determine winner by taking games in order until one player has enough wins
        let wins_needed = format.wins_needed();
        let mut p1_wins = 0u32;
        let mut p2_wins = 0u32;
        let mut games = Vec::new();

        for game in all_games {
            if p1_wins >= wins_needed || p2_wins >= wins_needed {
                break;
            }

            if game.winner == 1 {
                p1_wins += 1;
            } else {
                p2_wins += 1;
            }
            games.push(game);
        }

        let winner_index = if p1_wins >= wins_needed { 0 } else { 1 };

        BracketMatchResult {
            games,
            player1_wins: p1_wins,
            player2_wins: p2_wins,
            winner_index,
            loser_index: 1 - winner_index,
        }
    }

    /// Execute a single game
    fn execute_game(
        &self,
        p1_name: &str,
        p2_name: &str,
        format: &MatchFormat,
        seed: u64,
    ) -> GameResult {
        let level = self.get_level(seed);
        let level_name = self
            .level_db
            .get((level - 1) as usize)
            .map(|l| l.name.clone())
            .unwrap_or_else(|| format!("Level {}", level));

        let config = MatchConfig {
            base_config: SimConfig {
                duration_limit: format.duration_limit,
                score_limit: format.score_limit,
                quiet: true, // Always quiet for individual games
                ..self.base_config.clone()
            },
            level,
            left_profile: p1_name.to_string(),
            right_profile: p2_name.to_string(),
            seed,
        };

        let result = run_match(&config.base_config, seed, self.level_db, self.profile_db);

        let winner = if result.score_left > result.score_right {
            1
        } else if result.score_right > result.score_left {
            2
        } else {
            // Tie - use higher seed (p1) as tiebreaker
            1
        };

        GameResult {
            level,
            level_name,
            player1_score: result.score_left,
            player2_score: result.score_right,
            winner,
            duration: result.duration,
            seed,
        }
    }
}

/// Format bracket results as a standings table
pub fn format_standings(bracket: &BracketState) -> String {
    let mut output = String::new();

    output.push_str("\n=== Double Elimination Bracket Results ===\n\n");

    // Final standings
    output.push_str("Final Standings:\n");
    output.push_str(&format!(
        "{:>4} | {:>16} | {:>4} | {:>5} | {:>5}\n",
        "Rank", "Profile", "Seed", "Match", "Game"
    ));
    output.push_str(&format!("{:-<4}-+-{:-<16}-+-{:-<4}-+-{:-<5}-+-{:-<5}\n", "", "", "", "", ""));

    let placements = bracket.placements();
    for entry in placements.iter().take(16) {
        // Show top 16
        output.push_str(&format!(
            "{:>4} | {:>16} | {:>4} | {:>2}-{:<2} | {:>2}-{:<2}\n",
            entry.final_placement.unwrap_or(0),
            truncate_name(&entry.profile_name, 16),
            entry.seed,
            entry.match_wins,
            entry.match_losses,
            entry.game_wins,
            entry.game_losses,
        ));
    }

    if bracket.entries.len() > 16 {
        output.push_str(&format!("... and {} more\n", bracket.entries.len() - 16));
    }

    // Champion highlight
    if let Some(champ_idx) = bracket.champion {
        let champ = &bracket.entries[champ_idx];
        output.push_str(&format!(
            "\nChampion: {} (Seed {}, {}-{} matches, {}-{} games)\n",
            champ.profile_name,
            champ.seed,
            champ.match_wins,
            champ.match_losses,
            champ.game_wins,
            champ.game_losses,
        ));
    }

    // Progress
    output.push_str(&format!(
        "\nMatches completed: {}/{} ({:.1}%)\n",
        bracket.matches_completed,
        bracket.total_matches,
        bracket.progress() * 100.0
    ));

    output
}

/// Truncate a name to max length with ellipsis
fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}...", &name[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::bracket::types::BracketEntry;

    #[test]
    fn test_truncate_name() {
        assert_eq!(truncate_name("Short", 10), "Short");
        assert_eq!(truncate_name("VeryLongProfileName", 10), "VeryLon...");
    }

    fn make_test_entries(count: usize) -> Vec<BracketEntry> {
        (0..count)
            .map(|i| {
                BracketEntry::new(
                    format!("profile_{}", i),
                    format!("Profile{}", i),
                    (i + 1) as u32,
                )
            })
            .collect()
    }

    #[test]
    fn test_bracket_state_8_players() {
        let entries = make_test_entries(8);
        let bracket = BracketState::new(entries, MatchFormat::default());

        // 8-player bracket should have first round ready
        let ready = bracket.ready_match_ids();
        assert_eq!(ready.len(), 4, "Expected 4 ready matches");
    }
}
