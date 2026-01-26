//! Training mode for playing against AI and collecting analysis data

mod analysis;
mod protocol;
mod session;
mod settings;
mod state;

pub use analysis::{
    analyze_pursuit_session, analyze_session, format_pursuit_analysis_markdown,
    generate_claude_prompt, write_analysis_files, PursuitAnalysis, PursuitIterationStats,
    SessionAnalysis,
};
pub use protocol::{ProtocolConfig, TrainingProtocol};
pub use session::{
    GameSummary, SessionSummary, ensure_session_dir, evlog_path_for_game, print_session_summary,
    write_session_summary,
};
pub use settings::{LevelSelector, TrainingMode, TrainingSettings};
pub use state::{GameResult, TrainingPhase, TrainingState, Winner};
