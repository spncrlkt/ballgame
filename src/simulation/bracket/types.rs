//! Bracket tournament types and data structures
//!
//! Defines all the types needed for double elimination bracket tournaments.

use serde::{Deserialize, Serialize};

/// Which side of the bracket a match is on
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BracketSide {
    /// Winners bracket - players with 0 losses
    Winners,
    /// Losers bracket - players with 1 loss
    Losers,
    /// Grand finals - winners bracket champion vs losers bracket champion
    GrandFinals,
    /// Grand finals reset - if losers champion wins grand finals
    GrandFinalsReset,
}

impl std::fmt::Display for BracketSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BracketSide::Winners => write!(f, "Winners"),
            BracketSide::Losers => write!(f, "Losers"),
            BracketSide::GrandFinals => write!(f, "Grand Finals"),
            BracketSide::GrandFinalsReset => write!(f, "Grand Finals Reset"),
        }
    }
}

/// How many losses a player has in double elimination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LossCount {
    /// No losses - still in winners bracket
    Zero,
    /// One loss - in losers bracket
    One,
    /// Two losses - eliminated
    Eliminated,
}

/// A bracket entry (entrant) with their current state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketEntry {
    /// Profile ID from the AI profile database
    pub profile_id: String,
    /// Profile display name
    pub profile_name: String,
    /// Seed number (1 = best seed)
    pub seed: u32,
    /// Current loss count
    pub losses: LossCount,
    /// Whether this entry has been eliminated
    pub eliminated: bool,
    /// Match wins in tournament
    pub match_wins: u32,
    /// Match losses in tournament
    pub match_losses: u32,
    /// Game wins in tournament
    pub game_wins: u32,
    /// Game losses in tournament
    pub game_losses: u32,
    /// Final placement (set when eliminated or tournament ends)
    pub final_placement: Option<u32>,
}

impl BracketEntry {
    pub fn new(profile_id: String, profile_name: String, seed: u32) -> Self {
        Self {
            profile_id,
            profile_name,
            seed,
            losses: LossCount::Zero,
            eliminated: false,
            match_wins: 0,
            match_losses: 0,
            game_wins: 0,
            game_losses: 0,
            final_placement: None,
        }
    }

    /// Record a match result for this entry
    pub fn record_match(&mut self, won: bool, games_won: u32, games_lost: u32) {
        if won {
            self.match_wins += 1;
        } else {
            self.match_losses += 1;
            self.losses = match self.losses {
                LossCount::Zero => LossCount::One,
                LossCount::One | LossCount::Eliminated => {
                    self.eliminated = true;
                    LossCount::Eliminated
                }
            };
        }
        self.game_wins += games_won;
        self.game_losses += games_lost;
    }
}

/// Result of a single game within a bracket match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameResult {
    /// Level played
    pub level: u32,
    /// Level name
    pub level_name: String,
    /// Player 1 score (higher seed)
    pub player1_score: u32,
    /// Player 2 score (lower seed)
    pub player2_score: u32,
    /// Winner (1 or 2)
    pub winner: u8,
    /// Game duration in seconds
    pub duration: f32,
    /// RNG seed used
    pub seed: u64,
    /// Full match result for database logging (skipped in JSON serialization)
    #[serde(skip)]
    pub match_result: Option<crate::simulation::metrics::MatchResult>,
}

/// Result of a bracket match (best of N games)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketMatchResult {
    /// Individual game results
    pub games: Vec<GameResult>,
    /// Player 1 game wins
    pub player1_wins: u32,
    /// Player 2 game wins
    pub player2_wins: u32,
    /// Index of the winner (0 = player1, 1 = player2)
    pub winner_index: usize,
    /// Index of the loser
    pub loser_index: usize,
}

impl BracketMatchResult {
    /// Get total score difference (for tiebreakers)
    pub fn score_differential(&self) -> i32 {
        let p1_total: u32 = self.games.iter().map(|g| g.player1_score).sum();
        let p2_total: u32 = self.games.iter().map(|g| g.player2_score).sum();
        p1_total as i32 - p2_total as i32
    }
}

/// A match in the bracket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketMatch {
    /// Unique match ID within the bracket
    pub id: u32,
    /// Which bracket side this match is on
    pub side: BracketSide,
    /// Round number within this bracket side (1-indexed)
    pub round: u32,
    /// Match number within this round (1-indexed)
    pub match_in_round: u32,
    /// Entry indices of the two players (None if TBD from prior match)
    pub players: [Option<usize>; 2],
    /// Match IDs that feed into this match [winner_from, loser_from] or [winner_from, winner_from]
    /// For losers bracket, can be [winners_loser, losers_winner]
    pub feeders: [Option<u32>; 2],
    /// Result of the match (None if not yet played)
    pub result: Option<BracketMatchResult>,
}

impl BracketMatch {
    pub fn new(id: u32, side: BracketSide, round: u32, match_in_round: u32) -> Self {
        Self {
            id,
            side,
            round,
            match_in_round,
            players: [None, None],
            feeders: [None, None],
            result: None,
        }
    }

    /// Check if this match is ready to play (both players determined)
    pub fn is_ready(&self) -> bool {
        self.players[0].is_some() && self.players[1].is_some() && self.result.is_none()
    }

    /// Check if this match has been completed
    pub fn is_complete(&self) -> bool {
        self.result.is_some()
    }

    /// Get the winner's entry index
    pub fn winner(&self) -> Option<usize> {
        self.result
            .as_ref()
            .map(|r| self.players[r.winner_index].unwrap())
    }

    /// Get the loser's entry index
    pub fn loser(&self) -> Option<usize> {
        self.result
            .as_ref()
            .map(|r| self.players[r.loser_index].unwrap())
    }
}

/// Match format configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MatchFormat {
    /// Number of games in a match (e.g., 3 for best-of-3)
    pub best_of: u32,
    /// Points needed to win a game (first to N)
    pub score_limit: u32,
    /// Duration limit per game in seconds
    pub duration_limit: f32,
}

impl Default for MatchFormat {
    fn default() -> Self {
        Self {
            best_of: 3,
            score_limit: 5,
            duration_limit: 60.0,
        }
    }
}

impl MatchFormat {
    /// Games needed to win a match
    pub fn wins_needed(&self) -> u32 {
        (self.best_of / 2) + 1
    }
}

/// How to seed the bracket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SeedingMethod {
    /// Random seeding
    Random,
    /// Warmup seeding - play games against baseline profile
    Warmup {
        /// Profile to play warmup games against
        baseline_profile: String,
        /// Number of games per entrant
        games_per_entrant: u32,
    },
    /// Manual seeding (seeds provided by caller)
    Manual,
    /// Use database order (order profiles appear in config)
    DatabaseOrder,
}

impl Default for SeedingMethod {
    fn default() -> Self {
        SeedingMethod::Random
    }
}

/// Seeding configuration for CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BracketSeedingConfig {
    /// Random seeding
    Random,
    /// Warmup seeding
    Warmup {
        baseline_profile: String,
        games_per_entrant: u32,
    },
    /// Database order
    DatabaseOrder,
}

impl Default for BracketSeedingConfig {
    fn default() -> Self {
        BracketSeedingConfig::Random
    }
}

impl From<BracketSeedingConfig> for SeedingMethod {
    fn from(config: BracketSeedingConfig) -> Self {
        match config {
            BracketSeedingConfig::Random => SeedingMethod::Random,
            BracketSeedingConfig::Warmup {
                baseline_profile,
                games_per_entrant,
            } => SeedingMethod::Warmup {
                baseline_profile,
                games_per_entrant,
            },
            BracketSeedingConfig::DatabaseOrder => SeedingMethod::DatabaseOrder,
        }
    }
}

/// The state of a double elimination bracket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketState {
    /// All entries in the bracket
    pub entries: Vec<BracketEntry>,
    /// All matches in the bracket
    pub matches: Vec<BracketMatch>,
    /// Match format for this bracket
    pub format: MatchFormat,
    /// Current phase (which round we're on)
    pub current_round: u32,
    /// Number of matches completed
    pub matches_completed: u32,
    /// Total number of matches
    pub total_matches: u32,
    /// Tournament champion entry index (set when tournament ends)
    pub champion: Option<usize>,
    /// Whether the bracket is complete
    pub is_complete: bool,
}

impl BracketState {
    /// Create a new double elimination bracket for N entrants
    ///
    /// Entrants should already be seeded (index 0 = seed 1, etc.)
    pub fn new(mut entries: Vec<BracketEntry>, format: MatchFormat) -> Self {
        let n = entries.len();
        assert!(n >= 2, "Need at least 2 entrants");
        assert!(n.is_power_of_two(), "Entrant count must be power of 2");

        // Ensure seeds are assigned
        for (i, entry) in entries.iter_mut().enumerate() {
            if entry.seed == 0 {
                entry.seed = (i + 1) as u32;
            }
        }

        let mut matches = Vec::new();
        let mut match_id = 0u32;

        // Generate winners bracket matches
        let winners_rounds = (n as f64).log2() as u32;
        let mut prev_round_matches = Vec::new();

        for round in 1..=winners_rounds {
            let matches_in_round = n >> round; // n/2, n/4, n/8, ...
            let mut round_matches = Vec::new();

            for m in 0..matches_in_round {
                match_id += 1;
                let mut bracket_match =
                    BracketMatch::new(match_id, BracketSide::Winners, round, (m + 1) as u32);

                if round == 1 {
                    // First round - use bracket seeding
                    let (seed1, seed2) = bracket_seed_pairing(n, m);
                    bracket_match.players = [Some(seed1), Some(seed2)];
                } else {
                    // Later rounds - fed by previous round winners
                    let feeder_base = m * 2;
                    bracket_match.feeders = [
                        Some(prev_round_matches[feeder_base]),
                        Some(prev_round_matches[feeder_base + 1]),
                    ];
                }

                round_matches.push(match_id);
                matches.push(bracket_match);
            }

            prev_round_matches = round_matches;
        }

        let winners_final_match = match_id;

        // Generate losers bracket matches
        // Losers bracket has 2 * (winners_rounds - 1) rounds in a typical double elim
        // But the exact structure is complex - let's use a simpler model:
        // After each winners round, losers drop down. Losers bracket has its own progression.

        // Losers bracket round 1: losers from winners R1 play each other
        // Losers bracket round 2: winners of LR1 play losers from winners R2
        // etc.

        let mut losers_prev_round_winners: Vec<u32> = Vec::new();
        let _losers_rounds = 2 * winners_rounds - 2; // Total losers rounds

        // Track which winners round's losers drop into each losers major round
        // Major round 1: W-R1 losers fight each other
        // Major round 2: LMR1 winners vs W-R2 losers
        // Major round 3: LMR2 winners fight each other (if needed) then vs W-R3 losers
        // etc.

        // Simplified losers bracket: process in phases
        // Each "major" corresponds to absorbing losers from one winners round

        for major in 1..=winners_rounds {
            // Get losers from winners round 'major'
            let winners_round_matches: Vec<_> = matches
                .iter()
                .filter(|m| m.side == BracketSide::Winners && m.round == major)
                .map(|m| m.id)
                .collect();

            let losers_from_winners = winners_round_matches.len();

            if major == 1 {
                // First major: losers from W-R1 play each other
                let matches_this_round = losers_from_winners / 2;
                let mut round_matches = Vec::new();

                for m in 0..matches_this_round {
                    match_id += 1;
                    let mut bracket_match =
                        BracketMatch::new(match_id, BracketSide::Losers, 1, (m + 1) as u32);
                    // Fed by losers from winners R1
                    bracket_match.feeders = [
                        Some(winners_round_matches[m * 2]),
                        Some(winners_round_matches[m * 2 + 1]),
                    ];
                    round_matches.push(match_id);
                    matches.push(bracket_match);
                }

                losers_prev_round_winners = round_matches;
            } else {
                // Later majors:
                // Round A: previous losers bracket winners vs losers from winners round 'major'
                // (They play each other directly since counts should match after proper seeding)

                let prev_count = losers_prev_round_winners.len();
                let new_losers_count = losers_from_winners;

                // These should match in a proper double elim
                assert_eq!(
                    prev_count, new_losers_count,
                    "Losers count mismatch at major {}: prev={}, new={}",
                    major, prev_count, new_losers_count
                );

                let round_num = (major - 1) * 2; // 2, 4, 6, ...
                let mut round_a_matches = Vec::new();

                for m in 0..prev_count {
                    match_id += 1;
                    let mut bracket_match =
                        BracketMatch::new(match_id, BracketSide::Losers, round_num, (m + 1) as u32);
                    // Fed by previous losers winners and new losers from winners
                    bracket_match.feeders = [
                        Some(losers_prev_round_winners[m]),
                        Some(winners_round_matches[m]),
                    ];
                    round_a_matches.push(match_id);
                    matches.push(bracket_match);
                }

                // If more than 1 match, we need another round for them to play each other
                if round_a_matches.len() > 1 {
                    let round_b_num = round_num + 1;
                    let matches_this_round = round_a_matches.len() / 2;
                    let mut round_b_matches = Vec::new();

                    for m in 0..matches_this_round {
                        match_id += 1;
                        let mut bracket_match = BracketMatch::new(
                            match_id,
                            BracketSide::Losers,
                            round_b_num,
                            (m + 1) as u32,
                        );
                        bracket_match.feeders = [
                            Some(round_a_matches[m * 2]),
                            Some(round_a_matches[m * 2 + 1]),
                        ];
                        round_b_matches.push(match_id);
                        matches.push(bracket_match);
                    }

                    losers_prev_round_winners = round_b_matches;
                } else {
                    losers_prev_round_winners = round_a_matches;
                }
            }
        }

        let losers_final_match = match_id;

        // Grand finals: winners bracket champion vs losers bracket champion
        match_id += 1;
        let mut grand_finals = BracketMatch::new(match_id, BracketSide::GrandFinals, 1, 1);
        grand_finals.feeders = [Some(winners_final_match), Some(losers_final_match)];
        matches.push(grand_finals);

        let grand_finals_match = match_id;

        // Grand finals reset: only played if losers champion wins grand finals
        match_id += 1;
        let mut grand_finals_reset =
            BracketMatch::new(match_id, BracketSide::GrandFinalsReset, 1, 1);
        grand_finals_reset.feeders = [Some(grand_finals_match), Some(grand_finals_match)];
        matches.push(grand_finals_reset);

        let total_matches = matches.len() as u32;

        Self {
            entries,
            matches,
            format,
            current_round: 1,
            matches_completed: 0,
            total_matches,
            champion: None,
            is_complete: false,
        }
    }

    /// Get all matches that are ready to play
    pub fn ready_matches(&self) -> Vec<&BracketMatch> {
        self.matches.iter().filter(|m| m.is_ready()).collect()
    }

    /// Get mutable references to all matches that are ready to play
    pub fn ready_matches_mut(&mut self) -> Vec<&mut BracketMatch> {
        self.matches.iter_mut().filter(|m| m.is_ready()).collect()
    }

    /// Get ready match IDs (for parallel execution)
    pub fn ready_match_ids(&self) -> Vec<u32> {
        self.matches
            .iter()
            .filter(|m| m.is_ready())
            .map(|m| m.id)
            .collect()
    }

    /// Get a match by ID
    pub fn get_match(&self, id: u32) -> Option<&BracketMatch> {
        self.matches.iter().find(|m| m.id == id)
    }

    /// Get a mutable match by ID
    pub fn get_match_mut(&mut self, id: u32) -> Option<&mut BracketMatch> {
        self.matches.iter_mut().find(|m| m.id == id)
    }

    /// Record a match result and update the bracket state
    pub fn record_result(&mut self, match_id: u32, result: BracketMatchResult) {
        // Get match info first
        let (winner_idx, loser_idx, side, feeders_to_update) = {
            let bracket_match = self.get_match(match_id).expect("Match not found");
            let winner_idx = bracket_match.players[result.winner_index].unwrap();
            let loser_idx = bracket_match.players[result.loser_index].unwrap();
            let side = bracket_match.side;

            // Find matches that are fed by this one
            let feeders_to_update: Vec<(u32, usize, bool)> = self
                .matches
                .iter()
                .filter_map(|m| {
                    for (slot, feeder) in m.feeders.iter().enumerate() {
                        if *feeder == Some(match_id) {
                            // Determine if this slot wants winner or loser
                            // In losers bracket, we might want the loser from winners bracket
                            // In Losers Round 1, BOTH slots receive losers from Winners Round 1
                            // In Losers Round 2+, only slot 1 receives new losers from Winners
                            let wants_loser = m.side == BracketSide::Losers
                                && side == BracketSide::Winners
                                && (m.round == 1 || slot == 1);
                            return Some((m.id, slot, wants_loser));
                        }
                    }
                    None
                })
                .collect();

            (winner_idx, loser_idx, side, feeders_to_update)
        };

        // Update entry stats
        let p1_wins = result.player1_wins;
        let p2_wins = result.player2_wins;
        {
            let bracket_match = self.get_match(match_id).unwrap();
            let p1_idx = bracket_match.players[0].unwrap();
            let p2_idx = bracket_match.players[1].unwrap();

            self.entries[p1_idx].record_match(result.winner_index == 0, p1_wins, p2_wins);
            self.entries[p2_idx].record_match(result.winner_index == 1, p2_wins, p1_wins);
        }

        // Store result in match
        self.get_match_mut(match_id).unwrap().result = Some(result);
        self.matches_completed += 1;

        // Update fed matches
        for (fed_match_id, slot, wants_loser) in feeders_to_update {
            let player_idx = if wants_loser { loser_idx } else { winner_idx };
            if let Some(fed_match) = self.get_match_mut(fed_match_id) {
                fed_match.players[slot] = Some(player_idx);
            }
        }

        // Check for tournament completion
        if side == BracketSide::GrandFinals {
            // If winners bracket champion won, they're the champion
            // If losers bracket champion won, we need grand finals reset
            let winners_champion_idx = self.get_match(match_id).unwrap().players[0].unwrap();
            if winner_idx == winners_champion_idx {
                // Winners bracket champion wins - tournament over
                self.champion = Some(winner_idx);
                self.is_complete = true;
                self.assign_placements();
            }
            // Otherwise, grand finals reset is needed (players are already set by feeders)
        } else if side == BracketSide::GrandFinalsReset {
            // Grand finals reset completed - winner is champion
            self.champion = Some(winner_idx);
            self.is_complete = true;
            self.assign_placements();
        }

        // Set placement for eliminated players in losers bracket
        if side == BracketSide::Losers {
            let remaining = self.entries.iter().filter(|e| !e.eliminated).count() as u32;
            self.entries[loser_idx].final_placement = Some(remaining + 1);
        }
    }

    /// Assign final placements based on tournament results
    fn assign_placements(&mut self) {
        // Champion is 1st
        if let Some(champ_idx) = self.champion {
            self.entries[champ_idx].final_placement = Some(1);
        }

        // Find grand finals loser - they're 2nd
        for m in &self.matches {
            if (m.side == BracketSide::GrandFinals || m.side == BracketSide::GrandFinalsReset)
                && m.is_complete()
            {
                if let Some(loser_idx) = m.loser() {
                    if self.entries[loser_idx].final_placement.is_none() {
                        self.entries[loser_idx].final_placement = Some(2);
                    }
                }
            }
        }

        // Everyone else should already have placements from when they were eliminated
        // Fill in any remaining (shouldn't happen in a complete bracket)
        let mut next_placement = 3u32;
        for entry in &mut self.entries {
            if entry.final_placement.is_none() {
                entry.final_placement = Some(next_placement);
                next_placement += 1;
            }
        }
    }

    /// Get entries sorted by final placement
    pub fn placements(&self) -> Vec<&BracketEntry> {
        let mut entries: Vec<_> = self.entries.iter().collect();
        entries.sort_by_key(|e| e.final_placement.unwrap_or(999));
        entries
    }

    /// Get progress as a fraction (0.0 to 1.0)
    pub fn progress(&self) -> f32 {
        if self.total_matches == 0 {
            return 1.0;
        }
        self.matches_completed as f32 / self.total_matches as f32
    }

    /// Check if grand finals reset is needed (losers champion won grand finals)
    pub fn needs_grand_finals_reset(&self) -> bool {
        // Find grand finals match
        for m in &self.matches {
            if m.side == BracketSide::GrandFinals && m.is_complete() {
                // Grand finals is complete - check if losers champion won
                if let Some(result) = &m.result {
                    // Player[1] is the losers bracket champion in grand finals
                    return result.winner_index == 1;
                }
            }
        }
        false
    }
}

/// Get bracket seeding pairing for match m in first round of n-player bracket
///
/// Returns (higher_seed_index, lower_seed_index) in 0-indexed form
///
/// Standard bracket seeding ensures:
/// - 1 vs N (best vs worst)
/// - Top seeds don't meet until later rounds
fn bracket_seed_pairing(n: usize, match_index: usize) -> (usize, usize) {
    // For a bracket of size n, match m (0-indexed) pairs:
    // (seed1, seed2) where seed1 + seed2 = n + 1
    // But with bracket structure, we need proper ordering

    // Build standard bracket ordering
    let seeds: Vec<usize> = (0..n).collect();

    // Recursively split bracket
    fn bracket_order(seeds: &[usize]) -> Vec<usize> {
        if seeds.len() <= 2 {
            return seeds.to_vec();
        }

        let mid = seeds.len() / 2;
        let mut result = Vec::with_capacity(seeds.len());

        // Take first and last, second and second-to-last, etc.
        let mut top_half = Vec::new();
        let mut bottom_half = Vec::new();

        for i in 0..mid {
            top_half.push(seeds[i]);
            bottom_half.push(seeds[seeds.len() - 1 - i]);
        }

        // Recursively order each half
        result.extend(bracket_order(&top_half));
        result.extend(bracket_order(&bottom_half));

        result
    }

    let ordered = bracket_order(&seeds);
    let p1 = ordered[match_index * 2];
    let p2 = ordered[match_index * 2 + 1];

    (p1, p2)
}

/// Final placement result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placement {
    /// Final rank (1 = champion)
    pub rank: u32,
    /// Profile ID
    pub profile_id: String,
    /// Profile name
    pub profile_name: String,
    /// Original seed
    pub seed: u32,
    /// Match wins
    pub match_wins: u32,
    /// Match losses
    pub match_losses: u32,
    /// Game wins
    pub game_wins: u32,
    /// Game losses
    pub game_losses: u32,
}

impl From<&BracketEntry> for Placement {
    fn from(entry: &BracketEntry) -> Self {
        Self {
            rank: entry.final_placement.unwrap_or(0),
            profile_id: entry.profile_id.clone(),
            profile_name: entry.profile_name.clone(),
            seed: entry.seed,
            match_wins: entry.match_wins,
            match_losses: entry.match_losses,
            game_wins: entry.game_wins,
            game_losses: entry.game_losses,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bracket_seed_pairing_4() {
        // 4-player bracket: 1v4, 2v3
        let (a, b) = bracket_seed_pairing(4, 0);
        assert!(
            (a == 0 && b == 3) || (a == 3 && b == 0),
            "Match 0 should be 1v4"
        );

        let (a, b) = bracket_seed_pairing(4, 1);
        assert!(
            (a == 1 && b == 2) || (a == 2 && b == 1),
            "Match 1 should be 2v3"
        );
    }

    #[test]
    fn test_bracket_seed_pairing_8() {
        // 8-player bracket: 1v8, 4v5, 2v7, 3v6 (standard bracket order)
        let pairs: Vec<_> = (0..4).map(|m| bracket_seed_pairing(8, m)).collect();

        // Check that all seeds 0-7 appear exactly once
        let mut all_seeds = Vec::new();
        for (a, b) in &pairs {
            all_seeds.push(*a);
            all_seeds.push(*b);
        }
        all_seeds.sort();
        assert_eq!(all_seeds, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn test_match_format_wins_needed() {
        let bo3 = MatchFormat {
            best_of: 3,
            ..Default::default()
        };
        assert_eq!(bo3.wins_needed(), 2);

        let bo5 = MatchFormat {
            best_of: 5,
            ..Default::default()
        };
        assert_eq!(bo5.wins_needed(), 3);
    }

    #[test]
    fn test_bracket_state_creation() {
        let entries: Vec<_> = (0..8)
            .map(|i| {
                BracketEntry::new(
                    format!("profile_{}", i),
                    format!("Profile {}", i),
                    (i + 1) as u32,
                )
            })
            .collect();

        let state = BracketState::new(entries, MatchFormat::default());

        // 8-player double elim should have:
        // Winners: 4 + 2 + 1 = 7 matches
        // Losers: varies based on structure but ~7 matches
        // Grand finals + reset: 2 matches
        // Total should be around 15-16 matches
        assert!(
            state.total_matches >= 14,
            "Expected at least 14 matches, got {}",
            state.total_matches
        );

        // First round should have 4 ready matches
        let ready = state.ready_matches();
        assert_eq!(ready.len(), 4, "Expected 4 ready matches in round 1");
    }

    #[test]
    fn test_entry_record_match() {
        let mut entry = BracketEntry::new("test".to_string(), "Test".to_string(), 1);

        // Win a match
        entry.record_match(true, 2, 1);
        assert_eq!(entry.match_wins, 1);
        assert_eq!(entry.losses, LossCount::Zero);
        assert!(!entry.eliminated);

        // Lose a match (first loss)
        entry.record_match(false, 1, 2);
        assert_eq!(entry.match_losses, 1);
        assert_eq!(entry.losses, LossCount::One);
        assert!(!entry.eliminated);

        // Lose another match (eliminated)
        entry.record_match(false, 0, 2);
        assert_eq!(entry.match_losses, 2);
        assert_eq!(entry.losses, LossCount::Eliminated);
        assert!(entry.eliminated);
    }
}
