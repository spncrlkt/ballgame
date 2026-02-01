//! WebSocket client wrapper for connecting to ballgame server

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream,
};

use ballgame_protocol::{
    AgentInput, ClientMessage, ClientPayload, ServerMessage, ServerPayload,
    handshake::GameConfig, PROTOCOL_VERSION,
};

/// WebSocket client for ballgame server communication
pub struct GameClient {
    ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    seq: u64,
    last_ack_tick: u64,
}

/// Welcome information from server
pub struct WelcomeInfo {
    pub protocol_version: u32,
    pub assigned_slot: u8,
    pub tick_rate_hz: u8,
    pub game_config: GameConfig,
}

impl GameClient {
    /// Connect to a ballgame server and perform handshake
    pub async fn connect(url: &str, name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (ws, _response) = connect_async(url).await?;

        let mut client = Self {
            ws,
            seq: 0,
            last_ack_tick: 0,
        };

        // Send hello message (v2 AI)
        let hello = ClientPayload::hello_ai(name, "v2");
        client.send_payload(0, 0, hello).await?;

        Ok(client)
    }

    /// Wait for and process welcome message from server
    pub async fn receive_welcome(&mut self) -> Result<WelcomeInfo, Box<dyn std::error::Error>> {
        loop {
            let msg = self.receive().await?;
            match msg.payload {
                ServerPayload::Welcome {
                    protocol_version,
                    assigned_slot,
                    tick_rate_hz,
                    game_config,
                } => {
                    // Verify protocol version
                    if !ballgame_protocol::is_compatible(PROTOCOL_VERSION, protocol_version) {
                        return Err(format!(
                            "Protocol version mismatch: client={}, server={}",
                            PROTOCOL_VERSION, protocol_version
                        ).into());
                    }

                    return Ok(WelcomeInfo {
                        protocol_version,
                        assigned_slot,
                        tick_rate_hz,
                        game_config,
                    });
                }
                ServerPayload::Rejected { reason } => {
                    return Err(format!("Connection rejected: {}", reason).into());
                }
                _ => {
                    // Ignore other messages while waiting for welcome
                    continue;
                }
            }
        }
    }

    /// Receive a message from the server
    pub async fn receive(&mut self) -> Result<ServerMessage, Box<dyn std::error::Error>> {
        loop {
            match self.ws.next().await {
                Some(Ok(Message::Text(text))) => {
                    let msg: ServerMessage = serde_json::from_str(&text)?;
                    self.last_ack_tick = msg.tick;
                    return Ok(msg);
                }
                Some(Ok(Message::Binary(data))) => {
                    let msg: ServerMessage = serde_json::from_slice(&data)?;
                    self.last_ack_tick = msg.tick;
                    return Ok(msg);
                }
                Some(Ok(Message::Ping(data))) => {
                    self.ws.send(Message::Pong(data)).await?;
                }
                Some(Ok(Message::Close(_))) => {
                    return Err("Server closed connection".into());
                }
                Some(Err(e)) => {
                    return Err(e.into());
                }
                None => {
                    return Err("Connection closed".into());
                }
                _ => {}
            }
        }
    }

    /// Send input to the server
    pub async fn send_input(
        &mut self,
        target_tick: u64,
        input: AgentInput,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.send_payload(target_tick, self.last_ack_tick, ClientPayload::Input(input)).await
    }

    /// Send a payload to the server
    async fn send_payload(
        &mut self,
        target_tick: u64,
        ack_tick: u64,
        payload: ClientPayload,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.seq += 1;
        let msg = ClientMessage::new(self.seq, target_tick, ack_tick, payload);
        let json = serde_json::to_string(&msg)?;
        self.ws.send(Message::Text(json)).await?;
        Ok(())
    }

    /// Send goodbye and disconnect gracefully
    pub async fn disconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_payload(0, self.last_ack_tick, ClientPayload::Goodbye).await?;
        self.ws.close(None).await?;
        Ok(())
    }
}
