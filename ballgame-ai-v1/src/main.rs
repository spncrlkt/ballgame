//! Ballgame AI Client v1
//!
//! A standalone AI client that connects to a ballgame server via WebSocket
//! and plays the game using decision logic ported from the original AI.

use std::time::Duration;
use std::fs;
use std::path::Path;

use chrono::Local;
use clap::Parser;
use tracing::{debug, error, info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

use ballgame_protocol::{CharacterId, GameStateSnapshot, ServerPayload};

mod brain;
mod client;

use brain::BrainV1;
use client::GameClient;

/// Log directory
const LOG_DIR: &str = "logs";

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

    /// Enable verbose logging (debug level)
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

/// Initialize logging to both console and file
fn init_logging(verbose: bool) -> String {
    // Ensure log directory exists
    fs::create_dir_all(LOG_DIR).expect("Failed to create log directory");

    // Create timestamped log file name
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let log_filename = format!("ai_client_{}.log", timestamp);
    let log_path = Path::new(LOG_DIR).join(&log_filename);

    // Create file appender (non-rolling, single file per session)
    let file_appender = RollingFileAppender::new(Rotation::NEVER, LOG_DIR, &log_filename);

    // Set up filter based on verbosity
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    // Create console layer
    let console_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false);

    // Create file layer
    let file_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false)
        .with_writer(file_appender);

    // Initialize subscriber with both layers
    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    log_path.to_string_lossy().to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    let log_file = init_logging(args.verbose);

    info!("===========================================");
    info!("Ballgame AI Client v1");
    info!("Started at: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    info!("Log file: {}", log_file);
    info!("Server: {}", args.server);
    info!("Client name: {}", args.name);
    info!("Verbose: {}", args.verbose);
    info!("===========================================");

    // Outer reconnection loop
    'connection: loop {
        info!("Connecting to: {}", args.server);

        // Try to connect
        let connect_result = GameClient::connect(&args.server, &args.name).await;

        let mut client = match connect_result {
            Ok(c) => {
                info!("WebSocket connected! Waiting for slot assignment...");
                c
            }
            Err(e) => {
                error!("Connection failed: {}", e);
                let wait = Duration::from_secs(3) + jitter();
                info!("Retrying in {:.1}s...", wait.as_secs_f32());
                tokio::time::sleep(wait).await;
                continue;
            }
        };

        // Wait for welcome message with slot assignment
        let welcome = match client.receive_welcome().await {
            Ok(w) => {
                info!("Received Welcome message");
                w
            }
            Err(e) => {
                error!("Failed to receive welcome: {}", e);
                let wait = Duration::from_secs(3) + jitter();
                info!("Retrying in {:.1}s...", wait.as_secs_f32());
                tokio::time::sleep(wait).await;
                continue;
            }
        };

        info!("=== SLOT ASSIGNMENT ===");
        info!("  Slot: {}", if welcome.assigned_slot == 255 { "WAITING".to_string() } else { welcome.assigned_slot.to_string() });
        info!("  Tick rate: {} Hz", welcome.tick_rate_hz);
        info!("  Arena: {}x{}", welcome.game_config.arena_width, welcome.game_config.arena_height);
        info!("=======================");

        // If slot is 255, we need to wait for SlotAssigned message
        let character = if welcome.assigned_slot == 255 {
            info!("Waiting for server to assign slot...");

            // Keep receiving messages until we get SlotAssigned
            loop {
                match client.receive().await {
                    Ok(msg) => {
                        if let ServerPayload::SlotAssigned { character } = msg.payload {
                            info!("Server assigned us to character: {}", character);
                            break character;
                        }
                        // Ignore other messages while waiting
                    }
                    Err(e) => {
                        error!("Disconnected while waiting for slot: {}", e);
                        let wait = Duration::from_secs(3) + jitter();
                        tokio::time::sleep(wait).await;
                        continue 'connection;
                    }
                }
            }
        } else {
            // Legacy: got slot directly in Welcome
            match CharacterId::from_slot_index(welcome.assigned_slot) {
                Some(c) => c,
                None => {
                    error!("Invalid slot assignment: {}", welcome.assigned_slot);
                    let wait = Duration::from_secs(3) + jitter();
                    tokio::time::sleep(wait).await;
                    continue;
                }
            }
        };
        let mut brain = BrainV1::new(character, welcome.game_config);

        info!("Playing as character: {} (slot {})", character, character.to_slot_index());
        info!("Starting AI loop - waiting for game state...");

        let mut tick_count: u64 = 0;
        let mut last_goal = brain.state.current_goal;

        // Main game loop
        let disconnect_reason = loop {
            match client.receive().await {
                Ok(msg) => {
                    debug!("Received message: tick={} payload_type={}", msg.tick, payload_type_name(&msg.payload));

                    match msg.payload {
                        ServerPayload::State(state) => {
                            tick_count += 1;

                            // Decide what to do based on game state
                            let decision = brain.decide(&state);

                            // Log goal changes
                            if brain.state.current_goal != last_goal {
                                info!("Goal changed: {:?} -> {:?}", last_goal, brain.state.current_goal);
                                last_goal = brain.state.current_goal;
                            }

                            // Periodic status (every ~1 second)
                            if tick_count % 60 == 0 {
                                info!(
                                    "Tick {}: goal={:?} input=(move={:.1}, jump={}, action={}) pos=({:.0},{:.0})",
                                    msg.tick,
                                    brain.state.current_goal,
                                    decision.move_x,
                                    decision.jump_pressed,
                                    decision.action_pressed,
                                    state.agents.iter()
                                        .find(|a| a.character == character)
                                        .map(|a| a.position.x)
                                        .unwrap_or(0.0),
                                    state.agents.iter()
                                        .find(|a| a.character == character)
                                        .map(|a| a.position.y)
                                        .unwrap_or(0.0),
                                );
                            }

                            // Send input to server
                            if let Err(e) = client.send_input(msg.tick, decision).await {
                                break format!("Send error: {}", e);
                            }

                            if args.verbose {
                                print_state_summary(&state, character);
                            }
                        }
                        ServerPayload::LobbyUpdate(lobby) => {
                            debug!("Lobby update: {} slots, level={}",
                                lobby.slots.iter().filter(|s| s.state != ballgame_protocol::SlotState::Empty).count(),
                                lobby.level_id);
                        }
                        ServerPayload::MatchStarting { level_id, countdown_secs } => {
                            info!("Match starting! Level: {}, countdown: {:.1}s", level_id, countdown_secs);
                        }
                        ServerPayload::Event(event) => {
                            debug!("Event: {:?}", event);
                        }
                        ServerPayload::MatchEnd { winner } => {
                            info!("Match ended! Winner: {:?}", winner);
                            // Don't disconnect - wait for next match or lobby
                        }
                        ServerPayload::Shutdown { reason } => {
                            warn!("Server shutdown: {}", reason);
                            break format!("Server shutdown: {}", reason);
                        }
                        _ => {
                            debug!("Unhandled payload type");
                        }
                    }
                }
                Err(e) => {
                    break format!("Connection error: {}", e);
                }
            }
        };

        // Connection lost - try to disconnect gracefully then reconnect
        error!("Disconnected: {}", disconnect_reason);
        let _ = client.disconnect().await;

        let wait = Duration::from_secs(3) + jitter();
        info!("Reconnecting in {:.1}s...", wait.as_secs_f32());
        tokio::time::sleep(wait).await;
    }
}

/// Get a human-readable name for payload type
fn payload_type_name(payload: &ServerPayload) -> &'static str {
    match payload {
        ServerPayload::Welcome { .. } => "Welcome",
        ServerPayload::Rejected { .. } => "Rejected",
        ServerPayload::State(_) => "State",
        ServerPayload::LobbyUpdate(_) => "LobbyUpdate",
        ServerPayload::MatchStarting { .. } => "MatchStarting",
        ServerPayload::MatchStart { .. } => "MatchStart",
        ServerPayload::SlotAssigned { .. } => "SlotAssigned",
        ServerPayload::Event(_) => "Event",
        ServerPayload::MatchEnd { .. } => "MatchEnd",
        ServerPayload::Ping { .. } => "Ping",
        ServerPayload::Shutdown { .. } => "Shutdown",
    }
}

fn print_state_summary(state: &GameStateSnapshot, our_char: CharacterId) {
    if let Some(agent) = state.agents.iter().find(|a| a.character == our_char) {
        debug!(
            "State: pos=({:.0},{:.0}) vel=({:.0},{:.0}) ball={} grounded={}",
            agent.position.x,
            agent.position.y,
            agent.velocity.x,
            agent.velocity.y,
            if agent.holding_ball { "HELD" } else { "free" },
            agent.grounded,
        );
    } else {
        warn!("Our agent {} not found in state!", our_char);
    }
}
