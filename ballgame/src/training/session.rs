//! Training session management and summary generation

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use super::protocol::TrainingProtocol;
use super::state::{TrainingState, Winner};
use crate::generated_assets;
use crate::run_summary::{FileCategory, FileEntry, NextStep, RunSummary};

/// Session summary for JSON output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub protocol: String,
    pub protocol_description: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_id: Option<i64>,
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
                match_id: r.match_id,
                notes: r.notes.clone(),
            })
            .collect();

        let total_player_score: u32 = state.game_results.iter().map(|r| r.human_score).sum();
        let total_ai_score: u32 = state.game_results.iter().map(|r| r.ai_score).sum();
        let total_duration_secs: f32 = state.game_results.iter().map(|r| r.duration_secs).sum();

        Self {
            session_id: state.session_id.clone(),
            protocol: state.protocol.cli_name().to_string(),
            protocol_description: state.protocol.description().to_string(),
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
    fs::create_dir_all(&state.session_dir)?;
    // Record in generated assets tracker
    generated_assets::record_training_session(&state.session_dir.display().to_string());
    Ok(())
}

/// Write session summary to file
pub fn write_session_summary(state: &TrainingState) -> std::io::Result<()> {
    let summary = SessionSummary::from_state(state);
    let path = state.session_dir.join("summary.json");
    summary.write_to_file(&path)?;
    println!("\nSession summary written to: {}", path.display());
    Ok(())
}

/// Print session summary to console using unified RunSummary format
pub fn print_session_summary(state: &TrainingState, db_path: &str) {
    let (player_wins, ai_wins) = state.wins();
    let total_duration_secs: f32 = state.game_results.iter().map(|r| r.duration_secs).sum();

    // Build game results string
    let mut games_info = String::new();
    for result in &state.game_results {
        let winner_marker = match result.winner {
            Winner::Human => "[WIN]",
            Winner::AI => "[LOSS]",
        };
        if !games_info.is_empty() {
            games_info.push_str(" | ");
        }
        games_info.push_str(&format!(
            "G{}: {} {}-{}",
            result.game_number, winner_marker, result.human_score, result.ai_score
        ));
    }

    let summary_path = state.session_dir.join("summary.json");

    let mut summary = RunSummary::new("Training Session Complete")
        .duration(Duration::from_secs_f32(total_duration_secs))
        .stat(
            "Result",
            format!("You {} - {} {}", player_wins, ai_wins, state.ai_profile),
        )
        .stat("Protocol", state.protocol.display_name().to_string())
        .stat("Games", games_info)
        .file(FileEntry::new(db_path, FileCategory::Database))
        .file(FileEntry::new(
            summary_path.display().to_string(),
            FileCategory::Report,
        ));

    // Add protocol-specific analysis next step
    if state.protocol == TrainingProtocol::TeamInteraction {
        summary = summary.next_step(NextStep::primary(
            format!("cargo run --bin analyze -- --team-interaction {}", db_path),
            "Analyze team interactions (passes, blocks)",
        ));
    } else {
        summary = summary.next_step(NextStep::primary(
            format!("cargo run --bin analyze -- --training-db {}", db_path),
            "Generate detailed analysis report",
        ));
    }

    summary
        .next_step(NextStep::secondary(
            format!(
                "cargo run --bin training -- -n {} -p {}",
                state.game_results.len(),
                state.ai_profile
            ),
            "Continue training against same opponent",
        ))
        .print();
}
