//! Loader for parsing .evlog files into ReplayData

use bevy::prelude::*;
use std::fs;
use std::path::Path;

use crate::events::{parse_event, GameEvent};
use super::MatchInfo;

/// A single tick frame with positions and velocities for interpolation
#[derive(Debug, Clone)]
pub struct TickFrame {
    /// Time in milliseconds from start
    pub time_ms: u32,
    /// Frame number
    pub frame: u64,
    /// Left player position
    pub left_pos: Vec2,
    /// Left player velocity
    pub left_vel: Vec2,
    /// Right player position
    pub right_pos: Vec2,
    /// Right player velocity
    pub right_vel: Vec2,
    /// Ball position
    pub ball_pos: Vec2,
    /// Ball velocity
    pub ball_vel: Vec2,
    /// Ball state: 'F' = Free, 'H' = Held, 'I' = InFlight
    pub ball_state: char,
}

/// A timed game event (non-tick events like goals, pickups, AI goals)
#[derive(Debug, Clone)]
pub struct TimedEvent {
    /// Time in milliseconds from start
    pub time_ms: u32,
    /// The actual event
    pub event: GameEvent,
}

/// Complete replay data loaded from an .evlog file
#[derive(Resource, Default)]
pub struct ReplayData {
    /// Session ID from the log
    pub session_id: String,
    /// Match information (level, profiles, seed)
    pub match_info: MatchInfo,
    /// Tick frames for position interpolation (sampled at 50ms / 20 Hz)
    pub ticks: Vec<TickFrame>,
    /// Game events (goals, pickups, AI goals, steals, etc.)
    pub events: Vec<TimedEvent>,
    /// Total duration in milliseconds
    pub duration_ms: u32,
}

impl ReplayData {
    /// Get tick frames within a time range (for efficient lookup)
    pub fn ticks_in_range(&self, start_ms: u32, end_ms: u32) -> impl Iterator<Item = &TickFrame> {
        self.ticks.iter().filter(move |t| t.time_ms >= start_ms && t.time_ms <= end_ms)
    }

    /// Find the two tick frames that bracket a given time for interpolation
    pub fn find_bracket(&self, time_ms: u32) -> Option<(&TickFrame, &TickFrame, f32)> {
        if self.ticks.is_empty() {
            return None;
        }

        // Binary search for the insertion point
        let idx = self.ticks.partition_point(|t| t.time_ms <= time_ms);

        if idx == 0 {
            // Before first tick
            let first = &self.ticks[0];
            return Some((first, first, 0.0));
        }
        if idx >= self.ticks.len() {
            // After last tick
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

    /// Get events at or before a given time
    pub fn events_before(&self, time_ms: u32) -> impl Iterator<Item = &TimedEvent> {
        self.events.iter().filter(move |e| e.time_ms <= time_ms)
    }

    /// Get the most recent AI goal for a player at a given time
    pub fn current_ai_goal(&self, time_ms: u32, player: crate::events::PlayerId) -> Option<&str> {
        self.events
            .iter()
            .filter(|e| e.time_ms <= time_ms)
            .rev()
            .find_map(|e| {
                if let GameEvent::AiGoal { player: p, goal } = &e.event {
                    if *p == player {
                        return Some(goal.as_str());
                    }
                }
                None
            })
    }
}

/// Load a replay from an .evlog file
pub fn load_replay<P: AsRef<Path>>(path: P) -> Result<ReplayData, String> {
    let content = fs::read_to_string(path.as_ref())
        .map_err(|e| format!("Failed to read replay file: {}", e))?;

    let mut data = ReplayData::default();
    let mut max_time_ms: u32 = 0;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((time_ms, event)) = parse_event(line) {
            max_time_ms = max_time_ms.max(time_ms);

            match &event {
                GameEvent::SessionStart { session_id, .. } => {
                    data.session_id = session_id.clone();
                }
                GameEvent::MatchStart {
                    level,
                    level_name,
                    left_profile,
                    right_profile,
                    seed,
                } => {
                    data.match_info = MatchInfo {
                        level: *level,
                        level_name: level_name.clone(),
                        left_profile: left_profile.clone(),
                        right_profile: right_profile.clone(),
                        seed: *seed,
                    };
                }
                GameEvent::Tick {
                    frame,
                    left_pos,
                    left_vel,
                    right_pos,
                    right_vel,
                    ball_pos,
                    ball_vel,
                    ball_state,
                } => {
                    data.ticks.push(TickFrame {
                        time_ms,
                        frame: *frame,
                        left_pos: Vec2::new(left_pos.0, left_pos.1),
                        left_vel: Vec2::new(left_vel.0, left_vel.1),
                        right_pos: Vec2::new(right_pos.0, right_pos.1),
                        right_vel: Vec2::new(right_vel.0, right_vel.1),
                        ball_pos: Vec2::new(ball_pos.0, ball_pos.1),
                        ball_vel: Vec2::new(ball_vel.0, ball_vel.1),
                        ball_state: *ball_state,
                    });
                }
                GameEvent::MatchEnd { duration, .. } => {
                    // Use duration from MatchEnd if available
                    data.duration_ms = (*duration * 1000.0) as u32;
                }
                // Store other events for timeline display
                GameEvent::Goal { .. }
                | GameEvent::Pickup { .. }
                | GameEvent::Drop { .. }
                | GameEvent::ShotStart { .. }
                | GameEvent::ShotRelease { .. }
                | GameEvent::StealAttempt { .. }
                | GameEvent::StealSuccess { .. }
                | GameEvent::StealFail { .. }
                | GameEvent::AiGoal { .. } => {
                    data.events.push(TimedEvent {
                        time_ms,
                        event: event.clone(),
                    });
                }
                _ => {
                    // Ignore other events (Config, etc.)
                }
            }
        }
    }

    // Use max observed time if MatchEnd wasn't present
    if data.duration_ms == 0 {
        data.duration_ms = max_time_ms;
    }

    // Sort ticks by time (should already be sorted, but ensure)
    data.ticks.sort_by_key(|t| t.time_ms);

    // Sort events by time
    data.events.sort_by_key(|e| e.time_ms);

    info!(
        "Loaded replay: {} ticks, {} events, duration {}ms",
        data.ticks.len(),
        data.events.len(),
        data.duration_ms
    );

    Ok(data)
}
