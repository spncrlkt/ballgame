//! Replay data structures shared by loaders and replay systems.

use bevy::prelude::*;

use super::MatchInfo;
use crate::events::{CharacterId, GameEvent};
use crate::input::InputSourceId;

/// Character frame data for 2v2 replay format
#[derive(Debug, Clone)]
pub struct CharacterFrame {
    /// Character ID (L0, L1, R0, R1)
    pub id: CharacterId,
    /// Position
    pub pos: Vec2,
    /// Velocity
    pub vel: Vec2,
    /// Controller source ID
    pub controller: InputSourceId,
}

/// A single tick frame with positions and velocities for interpolation.
#[derive(Debug, Clone)]
pub struct TickFrame {
    /// Time in milliseconds from start.
    pub time_ms: u32,
    /// Frame number.
    pub frame: u64,
    /// Character data (variable length for 1v1 vs 2v2)
    /// If empty, falls back to legacy left_pos/right_pos fields
    pub characters: Vec<CharacterFrame>,
    /// Left player position (legacy 1v1 format).
    pub left_pos: Vec2,
    /// Left player velocity (legacy 1v1 format).
    pub left_vel: Vec2,
    /// Right player position (legacy 1v1 format).
    pub right_pos: Vec2,
    /// Right player velocity (legacy 1v1 format).
    pub right_vel: Vec2,
    /// Ball position.
    pub ball_pos: Vec2,
    /// Ball velocity.
    pub ball_vel: Vec2,
    /// Ball state: 'F' = Free, 'H' = Held, 'I' = InFlight.
    pub ball_state: char,
}

impl TickFrame {
    /// Get position for a character (uses characters vec if available, else legacy fields)
    pub fn position_for(&self, char_id: CharacterId) -> Vec2 {
        // First try new format
        if let Some(cf) = self.characters.iter().find(|c| c.id == char_id) {
            return cf.pos;
        }
        // Fall back to legacy format
        match char_id {
            CharacterId::L0 | CharacterId::L1 => self.left_pos,
            CharacterId::R0 | CharacterId::R1 => self.right_pos,
        }
    }

    /// Get velocity for a character (uses characters vec if available, else legacy fields)
    pub fn velocity_for(&self, char_id: CharacterId) -> Vec2 {
        // First try new format
        if let Some(cf) = self.characters.iter().find(|c| c.id == char_id) {
            return cf.vel;
        }
        // Fall back to legacy format
        match char_id {
            CharacterId::L0 | CharacterId::L1 => self.left_vel,
            CharacterId::R0 | CharacterId::R1 => self.right_vel,
        }
    }
}

/// A timed game event (non-tick events like goals, pickups, AI goals).
#[derive(Debug, Clone)]
pub struct TimedEvent {
    /// Time in milliseconds from start.
    pub time_ms: u32,
    /// The actual event.
    pub event: GameEvent,
}

/// Complete replay data loaded from a database.
#[derive(Resource, Default)]
pub struct ReplayData {
    /// Session ID from the log.
    pub session_id: String,
    /// Match information (level, profiles, seed).
    pub match_info: MatchInfo,
    /// Tick frames for position interpolation (sampled at 50ms / 20 Hz).
    pub ticks: Vec<TickFrame>,
    /// Game events (goals, pickups, AI goals, steals, etc.).
    pub events: Vec<TimedEvent>,
    /// Total duration in milliseconds.
    pub duration_ms: u32,
}

impl ReplayData {
    /// Get tick frames within a time range (for efficient lookup).
    pub fn ticks_in_range(&self, start_ms: u32, end_ms: u32) -> impl Iterator<Item = &TickFrame> {
        self.ticks
            .iter()
            .filter(move |t| t.time_ms >= start_ms && t.time_ms <= end_ms)
    }

    /// Find the two tick frames that bracket a given time for interpolation.
    pub fn find_bracket(&self, time_ms: u32) -> Option<(&TickFrame, &TickFrame, f32)> {
        if self.ticks.is_empty() {
            return None;
        }

        // Binary search for the insertion point.
        let idx = self.ticks.partition_point(|t| t.time_ms <= time_ms);

        if idx == 0 {
            // Before first tick.
            let first = &self.ticks[0];
            return Some((first, first, 0.0));
        }
        if idx >= self.ticks.len() {
            // After last tick.
            let last = self.ticks.last().unwrap();
            return Some((last, last, 1.0));
        }

        let prev = &self.ticks[idx - 1];
        let next = &self.ticks[idx];

        let t = if next.time_ms > prev.time_ms {
            (time_ms - prev.time_ms) as f32 / (next.time_ms - prev.time_ms) as f32
        } else {
            0.0
        };

        Some((prev, next, t))
    }

    /// Get events at or before a given time.
    pub fn events_before(&self, time_ms: u32) -> impl Iterator<Item = &TimedEvent> {
        self.events.iter().filter(move |e| e.time_ms <= time_ms)
    }

    /// Get the most recent AI goal for a character at a given time.
    /// Checks both legacy AiGoal (PlayerId) and new AiGoal2 (CharacterId) events.
    pub fn current_ai_goal(&self, time_ms: u32, character: CharacterId) -> Option<&str> {
        self.events
            .iter()
            .filter(|e| e.time_ms <= time_ms)
            .rev()
            .find_map(|e| {
                match &e.event {
                    // Prefer CharacterId-based events
                    GameEvent::AiGoal2 { character: c, goal } if *c == character => {
                        Some(goal.as_str())
                    }
                    // Fall back to legacy PlayerId-based events (L→L0, R→R0)
                    GameEvent::AiGoal { player, goal } => {
                        let player_char = player.to_character_id();
                        if player_char == character {
                            Some(goal.as_str())
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            })
    }
}
