//! Analytics module for simulation data analysis
//!
//! Provides tools for parsing event logs, computing metrics,
//! generating leaderboards, and suggesting parameter changes.

pub mod parser;
mod metrics;
mod leaderboard;
mod targets;
pub mod suggestions;
mod defaults;

pub use parser::{parse_event_log, parse_all_logs, ParsedMatch};
pub use metrics::{AggregateMetrics, ProfileMetrics};
pub use leaderboard::{ProfileRanking, Leaderboard};
pub use targets::{TuningTargets, TargetDelta, TargetStatus, load_targets, default_targets};
pub use suggestions::{ParameterSuggestion, generate_suggestions, format_suggestions};
pub use defaults::{update_default_profiles, get_current_defaults, format_update_report};
