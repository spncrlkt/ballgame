//! Training session management and summary generation

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use super::state::{TrainingState, Winner};

/// Session summary for JSON output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub games_played: u32,
    pub player_wins: u32,
    pub ai_wins: u32,
    pub ai_profile: String,
    pub total_player_score: u32,
    pub total_ai_score: u32,
    pub total_duration_secs: f32,
    pub games: Vec<GameSummary>,
}

/// Summary of a single game
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSummary {
    pub game_number: u32,
    pub level: u32,
    pub level_name: String,
    pub player_score: u32,
    pub ai_score: u32,
    pub winner: String,
    pub duration_secs: f32,
    pub evlog: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl SessionSummary {
    /// Create summary from training state
    pub fn from_state(state: &TrainingState) -> Self {
        let (player_wins, ai_wins) = state.wins();

        let games: Vec<GameSummary> = state
            .game_results
            .iter()
            .map(|r| GameSummary {
                game_number: r.game_number,
                level: r.level,
                level_name: r.level_name.clone(),
                player_score: r.human_score,
                ai_score: r.ai_score,
                winner: r.winner.to_string(),
                duration_secs: r.duration_secs,
                evlog: r
                    .evlog_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                notes: r.notes.clone(),
            })
            .collect();

        let total_player_score: u32 = state.game_results.iter().map(|r| r.human_score).sum();
        let total_ai_score: u32 = state.game_results.iter().map(|r| r.ai_score).sum();
        let total_duration_secs: f32 = state.game_results.iter().map(|r| r.duration_secs).sum();

        Self {
            session_id: state.session_id.clone(),
            games_played: state.game_results.len() as u32,
            player_wins,
            ai_wins,
            ai_profile: state.ai_profile.clone(),
            total_player_score,
            total_ai_score,
            total_duration_secs,
            games,
        }
    }

    /// Write summary to JSON file
    pub fn write_to_file(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
}

/// Ensure session directory exists
pub fn ensure_session_dir(state: &TrainingState) -> std::io::Result<()> {
    fs::create_dir_all(&state.session_dir)
}

/// Get evlog path for current game
pub fn evlog_path_for_game(state: &TrainingState) -> std::path::PathBuf {
    state.session_dir.join(format!(
        "game_{}_level{}.evlog",
        state.game_number, state.current_level
    ))
}

/// Write session summary to file
pub fn write_session_summary(state: &TrainingState) -> std::io::Result<()> {
    let summary = SessionSummary::from_state(state);
    let path = state.session_dir.join("summary.json");
    summary.write_to_file(&path)?;
    println!("\nSession summary written to: {}", path.display());
    Ok(())
}

/// Print session summary to console
pub fn print_session_summary(state: &TrainingState) {
    let (player_wins, ai_wins) = state.wins();
    let total_player: u32 = state.game_results.iter().map(|r| r.human_score).sum();
    let total_ai: u32 = state.game_results.iter().map(|r| r.ai_score).sum();

    println!("\n========================================");
    println!("       TRAINING SESSION COMPLETE");
    println!("========================================");
    println!();
    println!("  Opponent: {} (AI)", state.ai_profile);
    println!("  Games: {} played", state.game_results.len());
    println!();
    println!("  RESULTS: You {} - {} {}", player_wins, ai_wins, state.ai_profile);
    println!("  TOTAL SCORE: {} - {}", total_player, total_ai);
    println!();

    for result in &state.game_results {
        let winner_marker = match result.winner {
            Winner::Human => "[WIN]",
            Winner::AI => "[LOSS]",
        };
        println!(
            "  Game {}: {} {}-{} on {} ({:.1}s) {}",
            result.game_number,
            winner_marker,
            result.human_score,
            result.ai_score,
            result.level_name,
            result.duration_secs,
            result.evlog_path.file_name().unwrap_or_default().to_string_lossy()
        );
    }

    println!();
    println!("  Logs saved to: {}", state.session_dir.display());
    println!("========================================");
    println!();
}
