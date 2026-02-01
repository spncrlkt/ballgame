//! Ballgame AI Client v2 Template
//!
//! Minimal client that connects to a ballgame server, reads game state, and sends inputs.
//! This is intended as a base for custom AI development - no decision logic is included.
//!
//! To implement your own AI:
//! 1. Modify the `decide()` function to analyze the game state
//! 2. Return an appropriate `AgentInput` based on your strategy

use clap::Parser;

use ballgame_protocol::{AgentInput, CharacterId, GameStateSnapshot, ServerPayload};

mod client;

use client::GameClient;

/// AI client v2 for ballgame - template for custom AI development
#[derive(Parser, Debug)]
#[command(name = "ballgame-ai-v2")]
#[command(about = "AI client v2 template for ballgame server")]
struct Args {
    /// Server WebSocket URL
    #[arg(short, long, default_value = "ws://localhost:9000")]
    server: String,

    /// Client name for identification
    #[arg(short, long, default_value = "AI-v2")]
    name: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Ballgame AI Client v2 Template");
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

    // Get our character ID
    let character = CharacterId::from_slot_index(welcome.assigned_slot)
        .ok_or("Invalid slot assignment")?;

    println!("Playing as character: {}", character);
    println!("Starting AI loop (template - no decision logic)...");

    // Main game loop
    loop {
        match client.receive().await {
            Ok(msg) => {
                match msg.payload {
                    ServerPayload::State(state) => {
                        // Decide what to do based on game state
                        let input = decide(&state, character);

                        // Send input to server
                        client.send_input(msg.tick, input).await?;

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

/// Placeholder decision function - implement your AI logic here!
///
/// This is where you analyze the game state and decide what input to send.
/// The default implementation returns idle input (no movement or actions).
///
/// # Arguments
/// * `state` - Current game state including all player positions, ball state, score
/// * `character` - Which character we control (L0, L1, R0, or R1)
///
/// # Returns
/// An `AgentInput` describing the actions to take this tick
fn decide(_state: &GameStateSnapshot, _character: CharacterId) -> AgentInput {
    // TODO: Implement your AI logic here!
    //
    // Example: Find our agent in the state
    // if let Some(us) = state.agents.iter().find(|a| a.character == character) {
    //     // Check if we have the ball
    //     if us.holding_ball {
    //         // Shoot toward the basket
    //         return AgentInput::new().with_shoot_held();
    //     } else {
    //         // Move toward the ball
    //         let ball_x = state.ball.position.x;
    //         let move_x = (ball_x - us.position.x).signum();
    //         return AgentInput::with_movement(move_x);
    //     }
    // }

    AgentInput::default()
}

fn print_state_summary(state: &GameStateSnapshot, our_char: CharacterId) {
    if let Some(agent) = state.agents.iter().find(|a| a.character == our_char) {
        println!(
            "Tick {}: pos=({:.0},{:.0}) ball={} score={}-{}",
            state.tick,
            agent.position.x,
            agent.position.y,
            if agent.holding_ball { "HELD" } else { "free" },
            state.score.left,
            state.score.right,
        );
    }
}
