//! Snapshot system - captures game state and screenshots on events
//!
//! Provides automated capture of game state (JSON) and optional screenshots
//! triggered by game events like scoring, steals, and level changes.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use chrono::Local;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

use crate::ai::AiState;
use crate::ball::{Ball, BallState, CurrentPalette};
use crate::player::{HoldingBall, HumanControlled, Player, Team, Velocity};
use crate::scoring::{CurrentLevel, Score};
use crate::steal::StealContest;
use crate::shooting::LastShotInfo;
use crate::world::Basket;

/// Directory where snapshots are saved
const SNAPSHOT_DIR: &str = "snapshots";

/// Configuration for what triggers snapshots
#[derive(Resource)]
pub struct SnapshotConfig {
    /// Capture on score changes
    pub on_score: bool,
    /// Capture on steal attempts (success or failure)
    pub on_steal: bool,
    /// Capture on level changes
    pub on_level_change: bool,
    /// Also save a screenshot with each snapshot
    pub save_screenshots: bool,
    /// Enable/disable the entire system
    pub enabled: bool,
    /// Exit app after startup screenshot (for --screenshot-and-quit mode)
    pub exit_after_startup: bool,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            on_score: true,
            on_steal: true,
            on_level_change: true,
            save_screenshots: true,
            enabled: true,
            exit_after_startup: false,
        }
    }
}

/// Tracks previous frame's state to detect changes
#[derive(Resource)]
pub struct SnapshotTriggerState {
    pub prev_score_left: u32,
    pub prev_score_right: u32,
    pub prev_level: u32,
    pub prev_steal_failed: bool,
    pub frame_count: u64,
    /// Take a startup screenshot after this many frames (0 = disabled)
    pub startup_screenshot_frame: u64,
    /// Frames to wait after startup screenshot before exiting (for screenshot to save)
    pub exit_delay_frames: Option<u64>,
}

impl Default for SnapshotTriggerState {
    fn default() -> Self {
        Self {
            prev_score_left: 0,
            prev_score_right: 0,
            prev_level: 1, // Game starts at level 1, don't trigger on startup
            prev_steal_failed: false,
            frame_count: 0,
            startup_screenshot_frame: 60, // Take screenshot after ~1 second
            exit_delay_frames: None,
        }
    }
}

/// Serializable snapshot of the entire game state
#[derive(Serialize)]
pub struct GameSnapshot {
    /// Timestamp when snapshot was taken
    pub timestamp: String,
    /// Frame number
    pub frame: u64,
    /// What triggered this snapshot
    pub trigger: String,
    /// Current score
    pub score: ScoreSnapshot,
    /// Current level
    pub level: u32,
    /// Current palette index
    pub palette: usize,
    /// Ball state
    pub ball: Option<BallSnapshot>,
    /// Player states
    pub players: Vec<PlayerSnapshot>,
    /// Last shot info (if any)
    pub last_shot: Option<ShotSnapshot>,
    /// Path to screenshot (if saved)
    pub screenshot_path: Option<String>,
}

#[derive(Serialize)]
pub struct ScoreSnapshot {
    pub left: u32,
    pub right: u32,
}

#[derive(Serialize)]
pub struct BallSnapshot {
    pub position: (f32, f32),
    pub velocity: (f32, f32),
    pub state: String,
    pub holder_team: Option<String>,
}

#[derive(Serialize)]
pub struct PlayerSnapshot {
    pub team: String,
    pub position: (f32, f32),
    pub velocity: (f32, f32),
    pub is_human: bool,
    pub holding_ball: bool,
    pub ai_goal: Option<String>,
}

#[derive(Serialize)]
pub struct ShotSnapshot {
    pub angle_degrees: f32,
    pub speed: f32,
    pub total_variance: f32,
    pub target: Option<String>,
}

/// System that detects events and triggers snapshots
#[allow(clippy::too_many_arguments)]
pub fn snapshot_trigger_system(
    mut commands: Commands,
    config: Res<SnapshotConfig>,
    mut trigger_state: ResMut<SnapshotTriggerState>,
    score: Res<Score>,
    current_level: Res<CurrentLevel>,
    current_palette: Res<CurrentPalette>,
    steal_contest: Res<StealContest>,
    last_shot: Res<LastShotInfo>,
    ball_query: Query<(&Transform, &Velocity, &BallState), With<Ball>>,
    player_query: Query<
        (
            &Transform,
            &Velocity,
            &Team,
            Option<&HumanControlled>,
            Option<&HoldingBall>,
            &AiState,
        ),
        With<Player>,
    >,
) {
    if !config.enabled {
        trigger_state.frame_count += 1;
        return;
    }

    let mut trigger: Option<String> = None;
    let mut start_exit_countdown = false;

    // Check for startup screenshot (one-time, after a short delay for rendering to settle)
    if trigger_state.startup_screenshot_frame > 0
        && trigger_state.frame_count == trigger_state.startup_screenshot_frame
    {
        trigger = Some("startup".to_string());
        if config.exit_after_startup {
            start_exit_countdown = true;
        }
    }

    // Check for score change
    if config.on_score
        && (score.left != trigger_state.prev_score_left
            || score.right != trigger_state.prev_score_right)
    {
        let points_left = score.left.saturating_sub(trigger_state.prev_score_left);
        let points_right = score.right.saturating_sub(trigger_state.prev_score_right);
        if points_left > 0 {
            trigger = Some(format!("score_left_{}", points_left));
        } else if points_right > 0 {
            trigger = Some(format!("score_right_{}", points_right));
        }
    }

    // Check for steal attempt
    if config.on_steal && steal_contest.last_attempt_failed && !trigger_state.prev_steal_failed {
        trigger = Some("steal_failed".to_string());
    }

    // Check for level change
    if config.on_level_change && current_level.0 != trigger_state.prev_level {
        trigger = Some(format!("level_change_{}", current_level.0));
    }

    // Update tracking state
    trigger_state.prev_score_left = score.left;
    trigger_state.prev_score_right = score.right;
    trigger_state.prev_level = current_level.0;
    trigger_state.prev_steal_failed = steal_contest.last_attempt_failed;
    trigger_state.frame_count += 1;

    // If triggered, capture snapshot
    if let Some(trigger_name) = trigger {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S_%3f").to_string();
        let frame = trigger_state.frame_count;

        // Ensure snapshot directory exists
        if let Err(e) = fs::create_dir_all(SNAPSHOT_DIR) {
            error!("Failed to create snapshot directory: {}", e);
            return;
        }

        // Build snapshot data
        let ball_snapshot = ball_query.iter().next().map(|(transform, velocity, state)| {
            // Find which team is holding the ball (if any)
            let holder_team = player_query
                .iter()
                .find(|(_, _, _, _, holding, _)| holding.is_some())
                .map(|(_, _, team, _, _, _)| format!("{:?}", team));

            BallSnapshot {
                position: (transform.translation.x, transform.translation.y),
                velocity: (velocity.0.x, velocity.0.y),
                state: format!("{:?}", state),
                holder_team,
            }
        });

        let players: Vec<PlayerSnapshot> = player_query
            .iter()
            .map(|(transform, velocity, team, human, holding, ai_state)| PlayerSnapshot {
                team: format!("{:?}", team),
                position: (transform.translation.x, transform.translation.y),
                velocity: (velocity.0.x, velocity.0.y),
                is_human: human.is_some(),
                holding_ball: holding.is_some(),
                ai_goal: if human.is_none() {
                    Some(format!("{:?}", ai_state.current_goal))
                } else {
                    None
                },
            })
            .collect();

        let shot_snapshot = if last_shot.target.is_some() {
            Some(ShotSnapshot {
                angle_degrees: last_shot.angle_degrees,
                speed: last_shot.speed,
                total_variance: last_shot.total_variance,
                target: last_shot.target.map(|b| match b {
                    Basket::Left => "Left".to_string(),
                    Basket::Right => "Right".to_string(),
                }),
            })
        } else {
            None
        };

        // Screenshot path (if enabled)
        let screenshot_filename = format!("{}_{}.png", timestamp, trigger_name);
        let screenshot_path = if config.save_screenshots {
            Some(format!("{}/{}", SNAPSHOT_DIR, screenshot_filename))
        } else {
            None
        };

        let snapshot = GameSnapshot {
            timestamp: timestamp.clone(),
            frame,
            trigger: trigger_name.clone(),
            score: ScoreSnapshot {
                left: score.left,
                right: score.right,
            },
            level: current_level.0,
            palette: current_palette.0,
            ball: ball_snapshot,
            players,
            last_shot: shot_snapshot,
            screenshot_path: screenshot_path.clone(),
        };

        // Save JSON
        let json_path = format!("{}/{}_{}.json", SNAPSHOT_DIR, timestamp, trigger_name);
        match serde_json::to_string_pretty(&snapshot) {
            Ok(json) => {
                if let Err(e) = fs::write(&json_path, json) {
                    error!("Failed to write snapshot JSON: {}", e);
                } else {
                    info!("Snapshot saved: {}", json_path);
                }
            }
            Err(e) => error!("Failed to serialize snapshot: {}", e),
        }

        // Trigger screenshot capture (if enabled)
        if config.save_screenshots {
            let path = PathBuf::from(format!("{}/{}", SNAPSHOT_DIR, screenshot_filename));
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
            info!("Screenshot queued: {}", screenshot_filename);
        }

        // Start exit countdown if this was the startup screenshot in exit mode
        if start_exit_countdown {
            // Wait 30 frames (~0.5s) for screenshot to be saved to disk
            trigger_state.exit_delay_frames = Some(trigger_state.frame_count + 30);
            info!("Screenshot-and-quit mode: will exit in 30 frames");
        }
    }

    // Check if we should exit (exit_delay_frames countdown elapsed)
    if let Some(exit_frame) = trigger_state.exit_delay_frames
        && trigger_state.frame_count >= exit_frame
    {
        info!("Exiting after startup screenshot");
        std::process::exit(0);
    }
}

/// Toggle snapshot system on/off with F2 key
pub fn toggle_snapshot_system(keyboard: Res<ButtonInput<KeyCode>>, mut config: ResMut<SnapshotConfig>) {
    if keyboard.just_pressed(KeyCode::F2) {
        config.enabled = !config.enabled;
        info!(
            "Snapshot system: {}",
            if config.enabled { "ENABLED" } else { "DISABLED" }
        );
    }
}

/// Toggle screenshot capture (keep JSON, disable images)
pub fn toggle_screenshot_capture(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<SnapshotConfig>,
) {
    if keyboard.just_pressed(KeyCode::F3) {
        config.save_screenshots = !config.save_screenshots;
        info!(
            "Screenshot capture: {}",
            if config.save_screenshots {
                "ENABLED"
            } else {
                "DISABLED (JSON only)"
            }
        );
    }
}

/// Manual snapshot trigger (F4 key)
#[allow(clippy::too_many_arguments)]
pub fn manual_snapshot(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    config: Res<SnapshotConfig>,
    trigger_state: Res<SnapshotTriggerState>,
    score: Res<Score>,
    current_level: Res<CurrentLevel>,
    current_palette: Res<CurrentPalette>,
    last_shot: Res<LastShotInfo>,
    ball_query: Query<(&Transform, &Velocity, &BallState), With<Ball>>,
    player_query: Query<
        (
            &Transform,
            &Velocity,
            &Team,
            Option<&HumanControlled>,
            Option<&HoldingBall>,
            &AiState,
        ),
        With<Player>,
    >,
) {
    if !keyboard.just_pressed(KeyCode::F4) {
        return;
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S_%3f").to_string();
    let frame = trigger_state.frame_count;
    let trigger_name = "manual";

    // Ensure snapshot directory exists
    if let Err(e) = fs::create_dir_all(SNAPSHOT_DIR) {
        error!("Failed to create snapshot directory: {}", e);
        return;
    }

    // Build snapshot data (same as trigger system)
    let ball_snapshot = ball_query.iter().next().map(|(transform, velocity, state)| {
        BallSnapshot {
            position: (transform.translation.x, transform.translation.y),
            velocity: (velocity.0.x, velocity.0.y),
            state: format!("{:?}", state),
            holder_team: None,
        }
    });

    let players: Vec<PlayerSnapshot> = player_query
        .iter()
        .map(|(transform, velocity, team, human, holding, ai_state)| PlayerSnapshot {
            team: format!("{:?}", team),
            position: (transform.translation.x, transform.translation.y),
            velocity: (velocity.0.x, velocity.0.y),
            is_human: human.is_some(),
            holding_ball: holding.is_some(),
            ai_goal: if human.is_none() {
                Some(format!("{:?}", ai_state.current_goal))
            } else {
                None
            },
        })
        .collect();

    let shot_snapshot = if last_shot.target.is_some() {
        Some(ShotSnapshot {
            angle_degrees: last_shot.angle_degrees,
            speed: last_shot.speed,
            total_variance: last_shot.total_variance,
            target: last_shot.target.map(|b| match b {
                Basket::Left => "Left".to_string(),
                Basket::Right => "Right".to_string(),
            }),
        })
    } else {
        None
    };

    let screenshot_filename = format!("{}_{}.png", timestamp, trigger_name);
    let screenshot_path = if config.save_screenshots {
        Some(format!("{}/{}", SNAPSHOT_DIR, screenshot_filename))
    } else {
        None
    };

    let snapshot = GameSnapshot {
        timestamp: timestamp.clone(),
        frame,
        trigger: trigger_name.to_string(),
        score: ScoreSnapshot {
            left: score.left,
            right: score.right,
        },
        level: current_level.0,
        palette: current_palette.0,
        ball: ball_snapshot,
        players,
        last_shot: shot_snapshot,
        screenshot_path: screenshot_path.clone(),
    };

    // Save JSON
    let json_path = format!("{}/{}_{}.json", SNAPSHOT_DIR, timestamp, trigger_name);
    match serde_json::to_string_pretty(&snapshot) {
        Ok(json) => {
            if let Err(e) = fs::write(&json_path, json) {
                error!("Failed to write snapshot JSON: {}", e);
            } else {
                info!("Manual snapshot saved: {}", json_path);
            }
        }
        Err(e) => error!("Failed to serialize snapshot: {}", e),
    }

    // Trigger screenshot capture (if enabled)
    if config.save_screenshots {
        let path = PathBuf::from(format!("{}/{}", SNAPSHOT_DIR, screenshot_filename));
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        info!("Screenshot queued: {}", screenshot_filename);
    }
}
