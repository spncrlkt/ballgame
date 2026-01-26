//! Event logger for game analytics
//!
//! Provides centralized logging for all game runs (gameplay, simulation, etc.)

use bevy::prelude::*;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use uuid::Uuid;

use super::format::serialize_event;
use super::types::{GameConfig, GameEvent};

/// Configuration for event logging
#[derive(Resource, Clone)]
pub struct EventLogConfig {
    /// Directory for log files
    pub log_dir: PathBuf,
    /// Whether logging is enabled
    pub enabled: bool,
    /// Sample rate for tick events (every N ms, 0 = disabled)
    pub tick_sample_ms: u32,
    /// Sample rate for input events (every N ms, 0 = disabled)
    pub input_sample_ms: u32,
}

impl Default for EventLogConfig {
    fn default() -> Self {
        Self {
            log_dir: PathBuf::from("logs"),
            enabled: true,
            tick_sample_ms: 100, // Sample every 100ms
            input_sample_ms: 0,  // Disabled by default
        }
    }
}

/// Active event logger with file handle
#[derive(Resource)]
pub struct EventLogger {
    writer: Option<BufWriter<File>>,
    session_id: String,
    start_time: f32,
    last_tick_time: f32,
    last_input_time: f32,
    config: EventLogConfig,
}

impl EventLogger {
    /// Create a new event logger (but don't open file yet)
    pub fn new(config: EventLogConfig) -> Self {
        Self {
            writer: None,
            session_id: String::new(),
            start_time: 0.0,
            last_tick_time: 0.0,
            last_input_time: 0.0,
            config,
        }
    }

    /// Start a new log session (called at match start)
    /// Generates a new UUID for this session and logs SessionStart event
    pub fn start_session(&mut self, timestamp: &str) {
        if !self.config.enabled {
            return;
        }

        // Generate new session UUID
        self.session_id = Uuid::new_v4().to_string();

        // Ensure log directory exists
        if let Err(e) = std::fs::create_dir_all(&self.config.log_dir) {
            warn!("Failed to create log directory: {}", e);
            return;
        }

        // Use session_id in filename for uniqueness
        let filename = format!("{}_{}.evlog", timestamp, &self.session_id[..8]);
        let path = self.config.log_dir.join(filename);

        match OpenOptions::new().create(true).write(true).truncate(true).open(&path) {
            Ok(file) => {
                self.writer = Some(BufWriter::new(file));
                self.start_time = 0.0;
                self.last_tick_time = 0.0;
                self.last_input_time = 0.0;
                info!("Event logging started: {} (session: {})", path.display(), &self.session_id[..8]);

                // Log session start event
                self.log(0.0, GameEvent::SessionStart {
                    session_id: self.session_id.clone(),
                    timestamp: timestamp.to_string(),
                });
            }
            Err(e) => {
                warn!("Failed to open event log: {}", e);
            }
        }
    }

    /// Log the game configuration (call after start_session)
    pub fn log_config(&mut self, config: GameConfig) {
        self.log(0.0, GameEvent::Config(config));
    }

    /// Get the current session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// End the current log session
    pub fn end_session(&mut self) {
        if let Some(mut writer) = self.writer.take()
            && let Err(e) = writer.flush()
        {
            warn!("Failed to flush event log: {}", e);
        }
    }

    /// Log an event
    pub fn log(&mut self, time: f32, event: GameEvent) {
        let Some(writer) = &mut self.writer else {
            return;
        };

        // Calculate time in milliseconds since session start
        let time_ms = ((time - self.start_time) * 1000.0) as u32;
        let line = serialize_event(time_ms, &event);

        if let Err(e) = writeln!(writer, "{}", line) {
            warn!("Failed to write event: {}", e);
        }
    }

    /// Check if a tick event should be logged (based on sample rate)
    pub fn should_log_tick(&mut self, time: f32) -> bool {
        if self.config.tick_sample_ms == 0 {
            return false;
        }
        let interval = self.config.tick_sample_ms as f32 / 1000.0;
        if time - self.last_tick_time >= interval {
            self.last_tick_time = time;
            true
        } else {
            false
        }
    }

    /// Check if an input event should be logged (based on sample rate)
    pub fn should_log_input(&mut self, time: f32) -> bool {
        if self.config.input_sample_ms == 0 {
            return false;
        }
        let interval = self.config.input_sample_ms as f32 / 1000.0;
        if time - self.last_input_time >= interval {
            self.last_input_time = time;
            true
        } else {
            false
        }
    }

    /// Set the session start time
    pub fn set_start_time(&mut self, time: f32) {
        self.start_time = time;
    }

    /// Check if logging is active
    pub fn is_active(&self) -> bool {
        self.writer.is_some()
    }
}

impl Default for EventLogger {
    fn default() -> Self {
        Self::new(EventLogConfig::default())
    }
}

/// Simple in-memory event buffer for simulation (no file I/O)
#[derive(Default)]
pub struct EventBuffer {
    events: Vec<(u32, GameEvent)>,
    session_id: String,
    start_time: f32,
}

impl EventBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new session with a fresh UUID
    pub fn start_session(&mut self, timestamp: &str) {
        self.clear();
        self.session_id = Uuid::new_v4().to_string();
        self.log(0.0, GameEvent::SessionStart {
            session_id: self.session_id.clone(),
            timestamp: timestamp.to_string(),
        });
    }

    /// Log the game configuration
    pub fn log_config(&mut self, config: GameConfig) {
        self.log(0.0, GameEvent::Config(config));
    }

    /// Get the current session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.session_id.clear();
        self.start_time = 0.0;
    }

    pub fn set_start_time(&mut self, time: f32) {
        self.start_time = time;
    }

    pub fn log(&mut self, time: f32, event: GameEvent) {
        let time_ms = ((time - self.start_time) * 1000.0) as u32;
        self.events.push((time_ms, event));
    }

    pub fn events(&self) -> &[(u32, GameEvent)] {
        &self.events
    }

    /// Import events from an external source (like EventBus)
    pub fn import_events(&mut self, events: Vec<(u32, GameEvent)>) {
        self.events.extend(events);
    }

    /// Serialize all events to a log string
    pub fn serialize(&self) -> String {
        self.events
            .iter()
            .map(|(ts, e)| serialize_event(*ts, e))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
