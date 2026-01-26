//! In-memory event buffer for simulations/training.

use uuid::Uuid;

use super::format::serialize_event;
use super::types::{GameConfig, GameEvent};

/// Simple in-memory event buffer (no file I/O).
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

    pub fn drain_events(&mut self) -> Vec<(u32, GameEvent)> {
        std::mem::take(&mut self.events)
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
