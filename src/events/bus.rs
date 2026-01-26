//! Event Bus - central hub for cross-module communication
//!
//! The EventBus enables decoupled communication between game systems:
//! - Input systems emit ControllerInput events
//! - Scoring systems emit Goal events
//! - Other systems consume events and react
//!
//! All events are automatically logged to SQLite for full auditability.

use bevy::prelude::*;

use super::types::GameEvent;

/// Timestamped event for the event bus
#[derive(Debug, Clone)]
pub struct BusEvent {
    /// Time in milliseconds since match start
    pub time_ms: u32,
    /// The event data
    pub event: GameEvent,
}

/// Central event bus for cross-module communication
///
/// Systems emit events to the bus, and other systems consume them.
/// All events are logged to SQLite for full auditability.
#[derive(Resource, Default)]
pub struct EventBus {
    /// Events emitted this frame, waiting to be consumed
    pending: Vec<BusEvent>,

    /// Events that have been consumed (for logging)
    processed: Vec<BusEvent>,

    /// Current elapsed time in milliseconds (for timestamping)
    elapsed_ms: u32,

    /// Whether the bus is enabled (for testing/simulation)
    enabled: bool,
}

impl EventBus {
    /// Create a new enabled event bus
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Create a disabled event bus (events are dropped)
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Update the elapsed time (called each frame)
    pub fn update_time(&mut self, elapsed_secs: f32) {
        self.elapsed_ms = (elapsed_secs * 1000.0) as u32;
    }

    /// Emit an event to the bus
    pub fn emit(&mut self, event: GameEvent) {
        if !self.enabled {
            return;
        }
        self.pending.push(BusEvent {
            time_ms: self.elapsed_ms,
            event,
        });
    }

    /// Emit multiple events at once
    pub fn emit_all(&mut self, events: impl IntoIterator<Item = GameEvent>) {
        if !self.enabled {
            return;
        }
        for event in events {
            self.pending.push(BusEvent {
                time_ms: self.elapsed_ms,
                event,
            });
        }
    }

    /// Get pending events for consumption (does not drain)
    pub fn peek(&self) -> &[BusEvent] {
        &self.pending
    }

    /// Drain pending events, moving them to processed
    pub fn drain(&mut self) -> Vec<BusEvent> {
        let events = std::mem::take(&mut self.pending);
        self.processed.extend(events.clone());
        events
    }

    /// Get all processed events (for logging)
    pub fn processed(&self) -> &[BusEvent] {
        &self.processed
    }

    /// Clear processed events (after logging to DB)
    pub fn clear_processed(&mut self) {
        self.processed.clear();
    }

    /// Get the number of pending events
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Check if the bus has any pending events
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Check if the bus is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable the bus
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get current elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u32 {
        self.elapsed_ms
    }

    /// Export pending events as (time_ms, GameEvent) tuples for EventBuffer
    pub fn export_events(&mut self) -> Vec<(u32, super::types::GameEvent)> {
        let events = std::mem::take(&mut self.pending);
        self.processed.extend(events.clone());
        events.into_iter().map(|e| (e.time_ms, e.event)).collect()
    }
}

/// System to update the event bus time each frame
pub fn update_event_bus_time(mut bus: ResMut<EventBus>, time: Res<Time>) {
    bus.update_time(time.elapsed_secs());
}

/// Resource to track previous level for change detection
#[derive(Resource, Default)]
pub struct LevelChangeTracker {
    pub prev_level_id: String,
}

/// System to emit LevelChange events when CurrentLevel changes.
/// Runs after systems that might change the level.
pub fn emit_level_change_events(
    current_level: Res<crate::scoring::CurrentLevel>,
    mut tracker: ResMut<LevelChangeTracker>,
    mut bus: ResMut<EventBus>,
) {
    let level_id = &current_level.0;
    if level_id != &tracker.prev_level_id && !tracker.prev_level_id.is_empty() {
        bus.emit(super::types::GameEvent::LevelChange { level_id: level_id.clone() });
    }
    tracker.prev_level_id = level_id.clone();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::types::{ControllerSource, PlayerId};

    #[test]
    fn test_emit_and_drain() {
        let mut bus = EventBus::new();
        bus.update_time(1.5);

        bus.emit(GameEvent::ControllerInput {
            player: PlayerId::L,
            source: ControllerSource::Human,
            move_x: 0.5,
            jump: true,
            jump_pressed: true,
            throw: false,
            throw_released: false,
            pickup: false,
        });

        assert_eq!(bus.pending_count(), 1);
        assert!(bus.has_pending());

        let events = bus.drain();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].time_ms, 1500);
        assert_eq!(bus.pending_count(), 0);
        assert_eq!(bus.processed().len(), 1);
    }

    #[test]
    fn test_disabled_bus() {
        let mut bus = EventBus::disabled();
        bus.emit(GameEvent::ResetScores);
        assert_eq!(bus.pending_count(), 0);
    }

    #[test]
    fn test_control_swap_event() {
        let mut bus = EventBus::new();
        bus.emit(GameEvent::ControlSwap {
            from_player: Some(PlayerId::L),
            to_player: Some(PlayerId::R),
        });

        let events = bus.drain();
        assert_eq!(events.len(), 1);
        if let GameEvent::ControlSwap { from_player, to_player } = &events[0].event {
            assert_eq!(*from_player, Some(PlayerId::L));
            assert_eq!(*to_player, Some(PlayerId::R));
        } else {
            panic!("Wrong event type");
        }
    }
}
