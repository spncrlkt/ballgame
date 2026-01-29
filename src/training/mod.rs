//! Training mode for playing against AI and collecting analysis data

mod analysis;
mod protocol;
mod session;
mod settings;
mod state;

pub use analysis::{
    PursuitAnalysis, PursuitIterationStats, SessionAnalysis, analyze_pursuit_session_from_db,
    analyze_session_from_db, format_pursuit_analysis_markdown, generate_analysis_request,
    write_analysis_files,
};
pub use protocol::{ProtocolConfig, TrainingProtocol};
pub use session::{
    GameSummary, SessionSummary, ensure_session_dir, print_session_summary, write_session_summary,
};
pub use settings::{LevelSelector, TrainingMode, TrainingSettings};
pub use state::{GameResult, ReachabilityCollector, TrainingPhase, TrainingState, Winner};
