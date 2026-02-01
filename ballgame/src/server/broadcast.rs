//! State broadcasting to connected clients
//!
//! Manages sending game state snapshots to all connected clients.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use bevy::prelude::*;

use ballgame_protocol::{GameStateSnapshot, ServerMessage, ServerPayload, GameEvent};

use super::slots::SlotId;

/// Manages broadcast channels to all connected clients
pub struct Broadcaster {
    /// Map from slot ID to message sender
    channels: Arc<RwLock<HashMap<SlotId, mpsc::UnboundedSender<ServerMessage>>>>,
    /// Server sequence counter
    seq: Arc<RwLock<u64>>,
}

impl Broadcaster {
    /// Create a new broadcaster
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            seq: Arc::new(RwLock::new(0)),
        }
    }

    /// Register a client channel
    pub async fn register(&self, slot_id: SlotId, tx: mpsc::UnboundedSender<ServerMessage>) {
        let mut channels = self.channels.write().await;
        channels.insert(slot_id, tx);
    }

    /// Unregister a client channel
    pub async fn unregister(&self, slot_id: SlotId) {
        let mut channels = self.channels.write().await;
        channels.remove(&slot_id);
    }

    /// Broadcast game state to all connected clients
    pub async fn broadcast_state(&self, tick: u64, state: GameStateSnapshot) {
        let channels = self.channels.read().await;
        let mut seq = self.seq.write().await;
        *seq += 1;

        let msg = ServerMessage::new(*seq, tick, ServerPayload::State(state));

        for (slot_id, tx) in channels.iter() {
            if tx.send(msg.clone()).is_err() {
                debug!("Failed to send to slot {}", slot_id);
            }
        }
    }

    /// Broadcast a game event to all connected clients
    pub async fn broadcast_event(&self, tick: u64, event: GameEvent) {
        let channels = self.channels.read().await;
        let mut seq = self.seq.write().await;
        *seq += 1;

        let msg = ServerMessage::new(*seq, tick, ServerPayload::Event(event));

        for (slot_id, tx) in channels.iter() {
            if tx.send(msg.clone()).is_err() {
                debug!("Failed to send event to slot {}", slot_id);
            }
        }
    }

    /// Send a message to a specific client
    pub async fn send_to(&self, slot_id: SlotId, tick: u64, payload: ServerPayload) {
        let channels = self.channels.read().await;
        let mut seq = self.seq.write().await;
        *seq += 1;

        let msg = ServerMessage::new(*seq, tick, payload);

        if let Some(tx) = channels.get(&slot_id) {
            let _ = tx.send(msg);
        }
    }

    /// Get the number of connected clients
    pub async fn client_count(&self) -> usize {
        self.channels.read().await.len()
    }

    /// Broadcast a payload to all connected clients
    pub async fn broadcast(&self, tick: u64, payload: ServerPayload) {
        let channels = self.channels.read().await;
        let mut seq = self.seq.write().await;
        *seq += 1;

        let msg = ServerMessage::new(*seq, tick, payload);

        for (slot_id, tx) in channels.iter() {
            if tx.send(msg.clone()).is_err() {
                debug!("Failed to send payload to slot {}", slot_id);
            }
        }
    }
}

impl Default for Broadcaster {
    fn default() -> Self {
        Self::new()
    }
}
