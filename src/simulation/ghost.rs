//! Ghost trial simulation
//!
//! Plays back recorded human inputs against AI to test defensive capability.
//! A ghost trial ends when:
//! - The ghost scores (AI failed to defend)
//! - The AI steals/intercepts (AI successfully defended)
//! - Time limit reached

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::ai::InputState;
use crate::player::{Player, Team};

/// A single input sample at a specific tick
#[derive(Debug, Clone)]
pub struct InputSample {
    /// Tick offset from start of trial (in milliseconds)
    pub tick: u32,
    /// Horizontal movement (-1.0 to 1.0)
    pub move_x: f32,
    /// Jump button pressed
    pub jump: bool,
    /// Throw button pressed
    pub throw: bool,
    /// Pickup button pressed
    pub pickup: bool,
}

/// A ghost trial - recorded human inputs to play back
#[derive(Debug, Clone, Resource)]
pub struct GhostTrial {
    /// Source file path
    pub source_file: String,
    /// Level number
    pub level: u32,
    /// Level name
    pub level_name: String,
    /// Whether the original recording resulted in a score
    pub originally_scored: bool,
    /// Input samples (sorted by tick)
    pub inputs: Vec<InputSample>,
}

/// Result of running a ghost trial
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GhostTrialResult {
    /// Source trial file
    pub source_file: String,
    /// Level played on
    pub level: u32,
    /// Level name
    pub level_name: String,
    /// AI profile used for defense
    pub ai_profile: String,
    /// How the trial ended
    pub outcome: GhostOutcome,
    /// Duration in seconds
    pub duration: f32,
    /// Tick at which trial ended
    pub end_tick: u32,
    /// Total ticks in original trial
    pub total_ticks: u32,
    /// Did the original recording score?
    pub originally_scored: bool,
}

/// How a ghost trial ended
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GhostOutcome {
    /// Ghost scored - AI failed to defend
    GhostScored,
    /// AI stole the ball - successful defense
    AiStole,
    /// AI intercepted a shot - successful defense
    AiIntercepted,
    /// Ghost inputs exhausted without scoring
    InputsExhausted,
    /// Time limit reached
    TimeLimit,
}

impl std::fmt::Display for GhostOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GhostOutcome::GhostScored => write!(f, "ghost_scored"),
            GhostOutcome::AiStole => write!(f, "ai_stole"),
            GhostOutcome::AiIntercepted => write!(f, "ai_intercepted"),
            GhostOutcome::InputsExhausted => write!(f, "inputs_exhausted"),
            GhostOutcome::TimeLimit => write!(f, "time_limit"),
        }
    }
}

impl GhostTrialResult {
    /// Did the AI successfully defend?
    pub fn ai_defended(&self) -> bool {
        matches!(
            self.outcome,
            GhostOutcome::AiStole | GhostOutcome::AiIntercepted | GhostOutcome::InputsExhausted
        )
    }

    /// Survival ratio - how much of the trial the AI survived (0.0 to 1.0)
    pub fn survival_ratio(&self) -> f32 {
        if self.total_ticks == 0 {
            return 1.0;
        }
        self.end_tick as f32 / self.total_ticks as f32
    }
}

/// Load a ghost trial from a .ghost or .evlog file
///
/// Supports both formats:
/// - .ghost: Simple format with tick|move_x|flags
/// - .evlog: Full event log, extracts left player (human) inputs
pub fn load_ghost_trial(path: &Path) -> Result<GhostTrial, String> {
    let file = File::open(path).map_err(|e| format!("Failed to open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);

    let source_file = path.file_name().unwrap().to_string_lossy().to_string();
    let is_evlog = path.extension().map_or(false, |ext| ext == "evlog");

    if is_evlog {
        load_from_evlog(reader, source_file)
    } else {
        load_from_ghost(reader, source_file)
    }
}

/// Load from .evlog format (training sessions)
fn load_from_evlog<R: BufRead>(reader: R, source_file: String) -> Result<GhostTrial, String> {
    let mut level = 0u32;
    let mut level_name = String::new();
    let mut inputs = Vec::new();
    let mut left_scored = false;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 2 {
            continue;
        }

        // Parse tick from T:XXXXX format
        let tick = if let Some(tick_str) = parts[0].strip_prefix("T:") {
            tick_str.parse::<u32>().unwrap_or(0)
        } else {
            continue;
        };

        let event_type = parts[1];

        match event_type {
            // Match Setup: T:00000|MS|level|name|...
            "MS" if parts.len() >= 4 => {
                level = parts[2].parse().unwrap_or(0);
                level_name = parts[3].to_string();
            }
            // Input: T:XXXXX|I|player|move_x|flags
            "I" if parts.len() >= 5 => {
                let player = parts[2];
                // Only capture left player (human) inputs
                if player == "L" {
                    let move_x: f32 = parts[3].parse().unwrap_or(0.0);
                    let flags = parts[4];

                    inputs.push(InputSample {
                        tick,
                        move_x,
                        jump: flags.contains('J'),
                        throw: flags.contains('T'),
                        pickup: flags.contains('P'),
                    });
                }
            }
            // Goal: T:XXXXX|G|scorer|...
            "G" if parts.len() >= 3 => {
                if parts[2] == "L" {
                    left_scored = true;
                }
            }
            _ => {}
        }
    }

    Ok(GhostTrial {
        source_file,
        level,
        level_name,
        originally_scored: left_scored,
        inputs,
    })
}

/// Load from .ghost format (extracted drives)
fn load_from_ghost<R: BufRead>(reader: R, source_file: String) -> Result<GhostTrial, String> {
    let mut level = 0u32;
    let mut level_name = String::new();
    let mut inputs = Vec::new();

    // Check if filename contains "_scored" to determine if originally scored
    let originally_scored = source_file.contains("_scored");

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse level info
        if let Some(val) = line.strip_prefix("level:") {
            level = val.trim().parse().unwrap_or(0);
            continue;
        }
        if let Some(val) = line.strip_prefix("level_name:") {
            level_name = val.trim().to_string();
            continue;
        }

        // Parse input samples: tick|move_x|flags
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            let tick: u32 = parts[0].parse().unwrap_or(0);
            let move_x: f32 = parts[1].parse().unwrap_or(0.0);
            let flags = parts[2];

            inputs.push(InputSample {
                tick,
                move_x,
                jump: flags.contains('J'),
                throw: flags.contains('T'),
                pickup: flags.contains('P'),
            });
        }
    }

    Ok(GhostTrial {
        source_file,
        level,
        level_name,
        originally_scored,
        inputs,
    })
}

/// State for ghost trial playback
#[derive(Debug, Clone, Resource)]
pub struct GhostPlaybackState {
    /// Current index into the input samples
    pub current_index: usize,
    /// Elapsed time in milliseconds (to match tick format)
    pub elapsed_ms: u32,
    /// Whether playback has finished (inputs exhausted)
    pub finished: bool,
    /// Outcome (set when trial ends)
    pub outcome: Option<GhostOutcome>,
    /// Tick at which trial ended
    pub end_tick: u32,
}

impl Default for GhostPlaybackState {
    fn default() -> Self {
        Self {
            current_index: 0,
            elapsed_ms: 0,
            finished: false,
            outcome: None,
            end_tick: 0,
        }
    }
}

/// System to apply ghost inputs to the left player
/// Runs in FixedUpdate for consistent timing (60Hz = 16.67ms per tick)
pub fn ghost_input_system(
    trial: Res<GhostTrial>,
    mut playback: ResMut<GhostPlaybackState>,
    mut players: Query<(&Team, &mut InputState), With<Player>>,
) {
    if playback.finished {
        return;
    }

    // Fixed timestep: 60Hz = 16.67ms per tick
    const FIXED_DELTA_MS: u32 = 17;
    playback.elapsed_ms += FIXED_DELTA_MS;

    // Find left player and apply ghost inputs
    for (team, mut input_state) in &mut players {
        if *team != Team::Left {
            continue;
        }

        // Find the most recent input sample at or before current time
        let mut best_sample: Option<&InputSample> = None;

        while playback.current_index < trial.inputs.len() {
            let sample = &trial.inputs[playback.current_index];
            if sample.tick <= playback.elapsed_ms {
                best_sample = Some(sample);
                playback.current_index += 1;
            } else {
                break;
            }
        }

        // Apply the input
        if let Some(sample) = best_sample {
            input_state.move_x = sample.move_x;
            // For jump, set a buffer timer when pressed (will be consumed by apply_input)
            if sample.jump {
                input_state.jump_buffer_timer = 0.1; // 100ms buffer
            }
            input_state.jump_held = sample.jump;
            input_state.throw_held = sample.throw;
            input_state.throw_released = !sample.throw; // Release when not held
            input_state.pickup_pressed = sample.pickup;
        } else if playback.current_index == 0 {
            // No samples yet, use neutral input
            input_state.move_x = 0.0;
            input_state.jump_buffer_timer = 0.0;
            input_state.jump_held = false;
            input_state.throw_held = false;
            input_state.throw_released = false;
            input_state.pickup_pressed = false;
        }

        // Check if we've exhausted inputs
        if playback.current_index >= trial.inputs.len() && best_sample.is_some() {
            playback.finished = true;
            playback.outcome = Some(GhostOutcome::InputsExhausted);
            playback.end_tick = playback.elapsed_ms;
        }
    }
}

/// Check for ghost trial end conditions (goal or turnover)
pub fn ghost_check_end_conditions(
    score: Res<crate::scoring::Score>,
    mut playback: ResMut<GhostPlaybackState>,
    trial: Res<GhostTrial>,
    players: Query<(&Team, Option<&crate::player::HoldingBall>), With<Player>>,
    mut control: ResMut<super::control::SimControl>,
) {
    if playback.outcome.is_some() {
        // Trial already ended
        control.should_exit = true;
        return;
    }

    // Grace period - don't check steal conditions until ghost has had time to act
    // (500ms = 500 ticks allows ghost inputs to start flowing)
    let grace_period_ms = 500;
    if playback.elapsed_ms < grace_period_ms {
        return;
    }

    // Check if inputs exhausted
    if playback.current_index >= trial.inputs.len() && !playback.finished {
        playback.outcome = Some(GhostOutcome::InputsExhausted);
        playback.end_tick = playback.elapsed_ms;
        playback.finished = true;
        control.should_exit = true;
        return;
    }

    // Check if ghost (left) scored
    if score.left > 0 {
        playback.outcome = Some(GhostOutcome::GhostScored);
        playback.end_tick = playback.elapsed_ms;
        playback.finished = true;
        control.should_exit = true;
        return;
    }

    // Check if AI (right) has the ball - means they stole/intercepted
    for (team, holding_ball) in &players {
        if *team == Team::Right && holding_ball.is_some() {
            playback.outcome = Some(GhostOutcome::AiStole);
            playback.end_tick = playback.elapsed_ms;
            playback.finished = true;
            control.should_exit = true;
            return;
        }
    }

    // Check time limit
    let elapsed_secs = playback.elapsed_ms as f32 / 1000.0;
    if elapsed_secs > control.config.duration_limit {
        playback.outcome = Some(GhostOutcome::TimeLimit);
        playback.end_tick = playback.elapsed_ms;
        playback.finished = true;
        control.should_exit = true;
    }
}

/// Get the maximum tick value from a ghost trial
pub fn max_tick(trial: &GhostTrial) -> u32 {
    trial.inputs.last().map(|s| s.tick).unwrap_or(0)
}
