//! Replay playback state

use bevy::prelude::*;

/// Available playback speeds
pub const PLAYBACK_SPEEDS: [f32; 5] = [0.25, 0.5, 1.0, 2.0, 4.0];

/// Replay playback state
#[derive(Resource)]
pub struct ReplayState {
    /// Current playback time in milliseconds
    pub current_time_ms: u32,
    /// Playback speed multiplier (0.25, 0.5, 1.0, 2.0, 4.0)
    pub playback_speed: f32,
    /// Whether playback is paused
    pub is_paused: bool,
    /// Whether we're in frame-stepping mode
    pub is_stepping: bool,
    /// Index of current speed in PLAYBACK_SPEEDS
    pub speed_index: usize,
    /// Whether replay has finished
    pub finished: bool,
}

impl Default for ReplayState {
    fn default() -> Self {
        Self {
            current_time_ms: 0,
            playback_speed: 1.0,
            is_paused: false,
            is_stepping: false,
            speed_index: 2, // 1.0x
            finished: false,
        }
    }
}

impl ReplayState {
    /// Increase playback speed
    pub fn speed_up(&mut self) {
        if self.speed_index < PLAYBACK_SPEEDS.len() - 1 {
            self.speed_index += 1;
            self.playback_speed = PLAYBACK_SPEEDS[self.speed_index];
        }
    }

    /// Decrease playback speed
    pub fn speed_down(&mut self) {
        if self.speed_index > 0 {
            self.speed_index -= 1;
            self.playback_speed = PLAYBACK_SPEEDS[self.speed_index];
        }
    }

    /// Toggle pause state
    pub fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
    }

    /// Step forward one tick (50ms)
    pub fn step_forward(&mut self) {
        if self.is_paused {
            self.is_stepping = true;
            self.current_time_ms = self.current_time_ms.saturating_add(50);
        }
    }

    /// Step backward one tick (50ms)
    pub fn step_backward(&mut self) {
        if self.is_paused {
            self.is_stepping = true;
            self.current_time_ms = self.current_time_ms.saturating_sub(50);
        }
    }

    /// Jump to start
    pub fn jump_to_start(&mut self) {
        self.current_time_ms = 0;
        self.finished = false;
    }

    /// Jump to end
    pub fn jump_to_end(&mut self, duration_ms: u32) {
        self.current_time_ms = duration_ms;
        self.finished = true;
    }

    /// Seek to a specific time (e.g., from timeline click)
    pub fn seek_to(&mut self, time_ms: u32, duration_ms: u32) {
        self.current_time_ms = time_ms.min(duration_ms);
        self.finished = self.current_time_ms >= duration_ms;
    }

    /// Get formatted speed string for display
    pub fn speed_string(&self) -> String {
        if self.is_paused {
            "PAUSED".to_string()
        } else {
            format!("{:.2}x", self.playback_speed)
        }
    }

    /// Get formatted time string for display (current / total)
    pub fn time_string(&self, duration_ms: u32) -> String {
        let current_secs = self.current_time_ms as f32 / 1000.0;
        let total_secs = duration_ms as f32 / 1000.0;
        format!("{:.1}s / {:.1}s", current_secs, total_secs)
    }
}
