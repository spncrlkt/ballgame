//! Ballgame AI Client v1
//!
//! A standalone AI client that connects to a ballgame server via WebSocket
//! and plays the game using decision logic ported from the original AI.

use std::time::Duration;

use clap::Parser;

use ballgame_protocol::{CharacterId, GameStateSnapshot, ServerPayload};

mod brain;
mod client;

use brain::BrainV1;
use client::GameClient;

/// AI client for ballgame
#[derive(Parser, Debug)]
#[command(name = "ballgame-ai-v1")]
#[command(about = "AI client v1 for ballgame server")]
struct Args {
    /// Server WebSocket URL
    #[arg(short, long, default_value = "ws://localhost:9000")]
    server: String,

    /// Client name for identification
    #[arg(short, long, default_value = "AI-v1")]
    name: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

/// Get a random jitter (0-1000ms) based on current time
fn jitter() -> Duration {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    // Mix bits for better distribution
    let mixed = nanos ^ (nanos >> 17) ^ (nanos >> 31);
    Duration::from_millis(mixed % 1000)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Ballgame AI Client v1");

    // Outer reconnection loop
    loop {
        println!("Connecting to: {}", args.server);

        // Try to connect
        let connect_result = GameClient::connect(&args.server, &args.name).await;

        let mut client = match connect_result {
            Ok(c) => {
                println!("Connected! Waiting for slot assignment...");
                c
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
                let wait = Duration::from_secs(3) + jitter();
                println!("Retrying in {:.1}s...", wait.as_secs_f32());
                tokio::time::sleep(wait).await;
                continue;
            }
        };

        // Wait for welcome message with slot assignment
        let welcome = match client.receive_welcome().await {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Failed to receive welcome: {}", e);
                let wait = Duration::from_secs(3) + jitter();
                println!("Retrying in {:.1}s...", wait.as_secs_f32());
                tokio::time::sleep(wait).await;
                continue;
            }
        };

        println!(
            "Assigned to slot {} (tick rate: {} Hz)",
            welcome.assigned_slot, welcome.tick_rate_hz
        );

        // Create AI brain for our assigned slot
        let character = match CharacterId::from_slot_index(welcome.assigned_slot) {
            Some(c) => c,
            None => {
                eprintln!("Invalid slot assignment: {}", welcome.assigned_slot);
                let wait = Duration::from_secs(3) + jitter();
                tokio::time::sleep(wait).await;
                continue;
            }
        };
        let mut brain = BrainV1::new(character, welcome.game_config);

        println!("Playing as character: {}", character);
        println!("Starting AI loop...");

        // Main game loop
        let disconnect_reason = loop {
            match client.receive().await {
                Ok(msg) => {
                    match msg.payload {
                        ServerPayload::State(state) => {
                            // Decide what to do based on game state
                            let decision = brain.decide(&state);

                            // Send input to server
                            if let Err(e) = client.send_input(msg.tick, decision).await {
                                break format!("Send error: {}", e);
                            }

                            if args.verbose {
                                print_state_summary(&state, character);
                            }
                        }
                        ServerPayload::Event(event) => {
                            if args.verbose {
                                println!("Event: {:?}", event);
                            }
                        }
                        ServerPayload::MatchEnd { winner } => {
                            println!("Match ended! Winner: {:?}", winner);
                            // Don't disconnect - wait for next match or lobby
                        }
                        ServerPayload::Shutdown { reason } => {
                            break format!("Server shutdown: {}", reason);
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    break format!("Connection error: {}", e);
                }
            }
        };

        // Connection lost - try to disconnect gracefully then reconnect
        eprintln!("{}", disconnect_reason);
        let _ = client.disconnect().await;

        let wait = Duration::from_secs(3) + jitter();
        println!("Reconnecting in {:.1}s...", wait.as_secs_f32());
        tokio::time::sleep(wait).await;
    }
}

fn print_state_summary(state: &GameStateSnapshot, our_char: CharacterId) {
    if let Some(agent) = state.agents.iter().find(|a| a.character == our_char) {
        println!(
            "Tick {}: pos=({:.0},{:.0}) ball={} goal={:?}",
            state.tick,
            agent.position.x,
            agent.position.y,
            if agent.holding_ball { "HELD" } else { "free" },
            agent.ai_state.as_ref().map(|s| &s.current_goal)
        );
    }
}
