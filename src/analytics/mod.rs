//! Analytics module for simulation data analysis
//!
//! Provides tools for parsing event logs, computing metrics,
//! generating leaderboards, and suggesting parameter changes.

pub mod db_analytics;
mod defaults;
mod event_audit;
mod focused_analysis;
mod leaderboard;
mod metrics;
pub mod parser;
mod requests;
pub mod suggestions;
mod targets;

pub use db_analytics::{
    DetailedProfileStats, ProfileAnalysis, ProfileComparison, analyze_profile, compare_profiles,
    format_leaderboard, summarize_all_profiles,
};
pub use defaults::{format_update_report, get_current_defaults, update_default_profiles};
pub use event_audit::run_event_audit;
pub use focused_analysis::run_focused_analysis;
pub use leaderboard::{Leaderboard, ProfileRanking};
pub use metrics::{AggregateMetrics, ProfileMetrics};
pub use parser::{ParsedMatch, parse_all_matches_from_db, parse_match_from_db};
pub use requests::{AnalysisRequest, AnalysisRequestFile, AnalysisRunReport, AnalysisQuery, run_request};
pub use suggestions::{ParameterSuggestion, format_suggestions, generate_suggestions};
pub use targets::{TargetDelta, TargetStatus, TuningTargets, default_targets, load_targets};
