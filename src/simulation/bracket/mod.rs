//! Double elimination bracket tournament system
//!
//! This module provides a complete double elimination bracket implementation
//! for running AI profile tournaments.
//!
//! # Usage
//!
//! ```bash
//! # Basic 64-player bracket with random seeding
//! cargo run --bin simulate -- --bracket 64 --parallel 16
//!
//! # With warmup seeding (5 games vs v11_Blend_A baseline)
//! cargo run --bin simulate -- --bracket 64 --parallel 16 \
//!     --warmup-seeding v11_Blend_A 5
//!
//! # BO5 matches with first-to-3 games
//! cargo run --bin simulate -- --bracket 64 --parallel 16 \
//!     --best-of 5 --score-limit 3
//! ```

mod executor;
mod seeding;
mod types;

pub use executor::{BracketExecutor, format_standings};
pub use seeding::{WarmupResult, pad_to_power_of_2, seed_entries, select_profiles, warmup_seeding};
pub use types::{
    BracketEntry, BracketMatch, BracketMatchResult, BracketSeedingConfig, BracketSide,
    BracketState, GameResult, LossCount, MatchFormat, Placement, SeedingMethod,
};
