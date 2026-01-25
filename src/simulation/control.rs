//! Simulation control resources
//!
//! Contains the core resources used to control simulation execution
//! and event logging.

use bevy::prelude::*;
use std::path::PathBuf;

use crate::events::{EmitterConfig, EventBuffer, EventEmitterState};

use super::config::SimConfig;

/// Resource to control simulation execution
#[derive(Resource)]
pub struct SimControl {
    /// Configuration for this simulation run
    pub config: SimConfig,
    /// Flag to signal simulation should exit
    pub should_exit: bool,
    /// Current RNG seed for reproducibility
    pub current_seed: u64,
}

/// Resource for event logging in simulation
#[derive(Resource)]
pub struct SimEventBuffer {
    /// The event buffer containing logged events
    pub buffer: EventBuffer,
    /// Whether event logging is enabled
    pub enabled: bool,
    /// Directory to write log files
    pub log_dir: PathBuf,
    /// Shared emitter state for detecting changes
    pub emitter_state: EventEmitterState,
}

impl Default for SimEventBuffer {
    fn default() -> Self {
        Self {
            buffer: EventBuffer::default(),
            enabled: false,
            log_dir: PathBuf::default(),
            emitter_state: EventEmitterState::with_config(EmitterConfig {
                track_both_ai_goals: true,
            }),
        }
    }
}
