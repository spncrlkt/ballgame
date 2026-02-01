//! Ballgame AI Client v1
//!
//! A standalone AI client that connects to a ballgame server via WebSocket
//! and plays the game using decision logic ported from the original AI.

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Ballgame AI Client v1");
    println!("Connecting to: {}", args.server);

    // Connect to server
    let mut client = GameClient::connect(&args.server, &args.name).await?;
    println!("Connected! Waiting for slot assignment...");

    // Wait for welcome message with slot assignment
    let welcome = client.receive_welcome().await?;
    println!(
        "Assigned to slot {} (tick rate: {} Hz)",
        welcome.assigned_slot, welcome.tick_rate_hz
    );

    // Create AI brain for our assigned slot
    let character = CharacterId::from_slot_index(welcome.assigned_slot)
        .ok_or("Invalid slot assignment")?;
    let mut brain = BrainV1::new(character, welcome.game_config);

    println!("Playing as character: {}", character);
    println!("Starting AI loop...");

    // Main game loop
    loop {
        match client.receive().await {
            Ok(msg) => {
                match msg.payload {
                    ServerPayload::State(state) => {
                        // Decide what to do based on game state
                        let decision = brain.decide(&state);

                        // Send input to server
                        client.send_input(msg.tick, decision).await?;

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
                        break;
                    }
                    ServerPayload::Shutdown { reason } => {
                        println!("Server shutting down: {}", reason);
                        break;
                    }
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
                break;
            }
        }
    }

    // Graceful disconnect
    client.disconnect().await?;
    println!("Disconnected.");

    Ok(())
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
