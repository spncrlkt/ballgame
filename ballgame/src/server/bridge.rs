//! Bridge between Tokio async runtime and Bevy sync game loop
//!
//! Provides synchronous access to the async WebSocket server from Bevy systems.

use std::sync::Arc;
use tokio::runtime::Runtime;
use bevy::prelude::*;

use super::{GameServer, SlotManager, Broadcaster};

/// Resource holding Tokio runtime and server handles for Bevy integration
#[derive(Resource)]
pub struct ServerBridge {
    /// Tokio runtime for async operations
    pub runtime: Arc<Runtime>,
    /// Slot manager for player assignment
    pub slots: Arc<SlotManager>,
    /// Broadcaster for sending state to clients
    pub broadcaster: Arc<Broadcaster>,
    /// Current game tick counter
    pub tick_count: u64,
}

impl ServerBridge {
    /// Create a new server bridge and start the WebSocket server
    pub fn new(port: u16, local_slot: Option<u8>) -> Self {
        let runtime = Arc::new(
            Runtime::new().expect("Failed to create Tokio runtime")
        );

        let server = GameServer::new(port, local_slot);
        let slots = server.slot_manager();
        let broadcaster = server.broadcaster();

        // Spawn server listener in background
        let server_arc = Arc::new(server);
        runtime.spawn(async move {
            if let Err(e) = server_arc.run().await {
                error!("Server error: {}", e);
            }
        });

        info!("Server bridge initialized on port {}", port);

        Self {
            runtime,
            slots,
            broadcaster,
            tick_count: 0,
        }
    }

    /// Increment tick counter and return new value
    pub fn next_tick(&mut self) -> u64 {
        self.tick_count += 1;
        self.tick_count
    }

    /// Get current tick
    pub fn current_tick(&self) -> u64 {
        self.tick_count
    }
}
