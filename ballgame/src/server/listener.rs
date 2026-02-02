//! WebSocket server listener
//!
//! Accepts incoming client connections and spawns session handlers.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::accept_async;
use futures_util::StreamExt;
use bevy::prelude::*;

use ballgame_protocol::{ServerMessage, handshake::GameConfig};

use super::broadcast::Broadcaster;
use super::session::{Session, run_session};
use super::slots::SlotManager;

/// Game server that accepts WebSocket connections
pub struct GameServer {
    /// Port to listen on
    pub port: u16,
    /// Slot manager for player assignment
    pub slots: Arc<SlotManager>,
    /// Broadcaster for sending state to clients
    pub broadcaster: Arc<Broadcaster>,
    /// Game configuration
    pub game_config: GameConfig,
}

impl GameServer {
    /// Create a new game server
    pub fn new(port: u16, local_slot: Option<u8>) -> Self {
        let slots = if let Some(slot) = local_slot {
            Arc::new(SlotManager::with_local_slot(slot))
        } else {
            Arc::new(SlotManager::new())
        };

        Self {
            port,
            slots,
            broadcaster: Arc::new(Broadcaster::new()),
            game_config: GameConfig::default_config(),
        }
    }

    /// Start the server and accept connections
    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = TcpListener::bind(addr).await?;

        info!("Game server listening on port {}", self.port);

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New connection from {}", addr);
                    self.spawn_session(stream).await;
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    /// Spawn a new session handler for a client connection
    async fn spawn_session(&self, stream: TcpStream) {
        let slots = self.slots.clone();
        let broadcaster = self.broadcaster.clone();
        let game_config = self.game_config.clone();

        tokio::spawn(async move {
            match accept_async(stream).await {
                Ok(ws_stream) => {
                    let (ws_tx, ws_rx) = ws_stream.split();

                    // Create channel for server -> client messages
                    let (tx, rx) = mpsc::unbounded_channel::<ServerMessage>();

                    // Create session with broadcaster for registration
                    let session = Session::new(tx.clone(), slots.clone(), broadcaster);

                    // Run the session
                    run_session(session, ws_rx, ws_tx, rx, game_config).await;
                }
                Err(e) => {
                    eprintln!("WebSocket handshake failed: {}", e);
                }
            }
        });
    }

    /// Get the slot manager for external access
    pub fn slot_manager(&self) -> Arc<SlotManager> {
        self.slots.clone()
    }

    /// Get the broadcaster for external access
    pub fn broadcaster(&self) -> Arc<Broadcaster> {
        self.broadcaster.clone()
    }
}
