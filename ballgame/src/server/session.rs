//! Client session management
//!
//! Handles individual client connections and message processing.

use std::sync::Arc;
use tokio::sync::mpsc;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;
use bevy::prelude::*;

use ballgame_protocol::{
    ClientMessage, ClientPayload, ServerMessage, ServerPayload,
    handshake::GameConfig, PROTOCOL_VERSION, is_compatible,
};

use super::slots::{SlotId, SlotManager};

/// A connected client session
pub struct Session {
    /// Assigned slot (None until handshake complete)
    pub slot: Option<SlotId>,
    /// Client ID (assigned during handshake)
    pub client_id: Option<u64>,
    /// Channel for sending messages to this client
    pub tx: mpsc::UnboundedSender<ServerMessage>,
    /// Reference to slot manager
    slots: Arc<SlotManager>,
    /// Server sequence number
    seq: u64,
}

impl Session {
    /// Create a new session
    pub fn new(tx: mpsc::UnboundedSender<ServerMessage>, slots: Arc<SlotManager>) -> Self {
        Self {
            slot: None,
            client_id: None,
            tx,
            slots,
            seq: 0,
        }
    }

    /// Handle an incoming message from the client
    pub async fn handle_message(
        &mut self,
        msg: ClientMessage,
        game_config: &GameConfig,
        current_tick: u64,
    ) -> Result<(), String> {
        match msg.payload {
            ClientPayload::Hello {
                protocol_version,
                client_name,
                client_type,
            } => {
                // Check protocol compatibility
                if !is_compatible(protocol_version, PROTOCOL_VERSION) {
                    self.send_rejected(&format!(
                        "Protocol mismatch: server={}, client={}",
                        PROTOCOL_VERSION, protocol_version
                    ), current_tick);
                    return Err("Protocol mismatch".to_string());
                }

                // Try to assign a slot
                match self.slots.assign_remote(client_type.clone(), client_name.clone()).await {
                    Some((slot_id, client_id)) => {
                        self.slot = Some(slot_id);
                        self.client_id = Some(client_id);

                        // Send welcome
                        self.send(ServerPayload::Welcome {
                            protocol_version: PROTOCOL_VERSION,
                            assigned_slot: slot_id,
                            tick_rate_hz: 60,
                            game_config: game_config.clone(),
                        }, current_tick);

                        info!(
                            "Client '{}' ({}) assigned to slot {}",
                            client_name,
                            client_type,
                            slot_id
                        );
                    }
                    None => {
                        self.send_rejected("No slots available", current_tick);
                        return Err("No slots available".to_string());
                    }
                }
            }

            ClientPayload::Input(input) => {
                if let Some(slot) = self.slot {
                    self.slots.set_input(slot, input, msg.ack_tick).await;
                }
            }

            ClientPayload::Pong { server_time, client_time } => {
                // Could use this for latency calculation
                let _latency = client_time - server_time;
            }

            ClientPayload::Goodbye => {
                if let Some(slot) = self.slot {
                    self.slots.release(slot).await;
                    info!("Client disconnected from slot {}", slot);
                }
                self.slot = None;
                self.client_id = None;
            }
        }

        Ok(())
    }

    /// Send a message to this client
    pub fn send(&mut self, payload: ServerPayload, tick: u64) {
        self.seq += 1;
        let msg = ServerMessage::new(self.seq, tick, payload);
        let _ = self.tx.send(msg);
    }

    /// Send a rejection message
    fn send_rejected(&mut self, reason: &str, tick: u64) {
        self.send(ServerPayload::rejected(reason), tick);
    }

    /// Clean up when session ends
    pub async fn cleanup(&mut self) {
        if let Some(slot) = self.slot {
            self.slots.release(slot).await;
            info!("Session cleanup: released slot {}", slot);
        }
        self.slot = None;
        self.client_id = None;
    }
}

/// Run the session message loop
pub async fn run_session<S>(
    mut session: Session,
    mut ws_rx: futures_util::stream::SplitStream<S>,
    mut ws_tx: futures_util::stream::SplitSink<S, Message>,
    mut server_rx: mpsc::UnboundedReceiver<ServerMessage>,
    game_config: GameConfig,
) where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + futures_util::Sink<Message>
        + Unpin,
{
    let mut current_tick: u64 = 0;

    loop {
        tokio::select! {
            // Receive from WebSocket
            ws_msg = ws_rx.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(msg) => {
                                if let Err(e) = session.handle_message(msg, &game_config, current_tick).await {
                                    warn!("Session error: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Invalid message: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        match serde_json::from_slice::<ClientMessage>(&data) {
                            Ok(msg) => {
                                if let Err(e) = session.handle_message(msg, &game_config, current_tick).await {
                                    warn!("Session error: {}", e);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Invalid binary message: {}", e);
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws_tx.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Client closed connection");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            }

            // Receive from server (outgoing messages)
            server_msg = server_rx.recv() => {
                match server_msg {
                    Some(msg) => {
                        current_tick = msg.tick;
                        match serde_json::to_string(&msg) {
                            Ok(json) => {
                                if ws_tx.send(Message::Text(json)).await.is_err() {
                                    warn!("Failed to send to client");
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to serialize message: {}", e);
                            }
                        }
                    }
                    None => {
                        info!("Server channel closed");
                        break;
                    }
                }
            }
        }
    }

    session.cleanup().await;
}
