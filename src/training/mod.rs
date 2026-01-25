//! Training mode for playing against AI and collecting analysis data

mod session;
mod settings;
mod state;

pub use session::{
    GameSummary, SessionSummary, ensure_session_dir, evlog_path_for_game, print_session_summary,
    write_session_summary,
};
pub use settings::{TrainingMode, TrainingSettings};
pub use state::{GameResult, TrainingPhase, TrainingState, Winner};
