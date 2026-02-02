//! Match orchestration for external AI clients
//!
//! Manages the lifecycle of AI client subprocesses and their connection to
//! an embedded WebSocket server. Used when simulations involve external AI
//! clients rather than embedded profiles.
//!
//! ## Workflow
//!
//! 1. Start an embedded WebSocket server on a random available port
//! 2. Spawn AI client executables as subprocesses
//! 3. Wait for clients to connect and complete handshake
//! 4. Auto-assign clients to their designated slots
//! 5. Run the match
//! 6. Clean up processes when the match ends

use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::time::timeout;

use crate::server::{Broadcaster, GameServer, SlotId, SlotManager};

use super::client_db::AiClientDatabase;
use super::participant::{AiParticipant, MatchParticipants};

/// Errors that can occur during orchestration
#[derive(Debug, Clone)]
pub enum OrchError {
    /// Failed to bind to a port for the embedded server
    PortBindFailed(String),
    /// Failed to spawn an AI client process
    ProcessSpawnFailed {
        client_id: String,
        error: String,
    },
    /// Client failed to connect within the timeout period
    ConnectionTimeout {
        client_id: String,
        timeout_secs: u64,
    },
    /// Client not found in the database
    ClientNotFound(String),
    /// Slot assignment failed
    SlotAssignmentFailed {
        client_id: String,
        slot: SlotId,
    },
    /// Server error
    ServerError(String),
}

impl std::fmt::Display for OrchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PortBindFailed(e) => write!(f, "Failed to bind server port: {}", e),
            Self::ProcessSpawnFailed { client_id, error } => {
                write!(f, "Failed to spawn client '{}': {}", client_id, error)
            }
            Self::ConnectionTimeout {
                client_id,
                timeout_secs,
            } => {
                write!(
                    f,
                    "Client '{}' failed to connect within {}s",
                    client_id, timeout_secs
                )
            }
            Self::ClientNotFound(id) => write!(f, "Client '{}' not found in database", id),
            Self::SlotAssignmentFailed { client_id, slot } => {
                write!(
                    f,
                    "Failed to assign client '{}' to slot {}",
                    client_id, slot
                )
            }
            Self::ServerError(e) => write!(f, "Server error: {}", e),
        }
    }
}

impl std::error::Error for OrchError {}

/// Configuration for the orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Timeout for client connections (seconds)
    pub connection_timeout_secs: u64,
    /// Port to use for the embedded server (0 = auto-select)
    pub server_port: u16,
    /// Whether to inherit stdout/stderr from spawned processes
    pub inherit_stdio: bool,
    /// Path to AI clients database file
    pub clients_file: Option<String>,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            connection_timeout_secs: 30,
            server_port: 0, // Auto-select available port
            inherit_stdio: false,
            clients_file: None,
        }
    }
}

/// Information about a successfully setup match
pub struct MatchSetup {
    /// The port the server is running on
    pub server_port: u16,
    /// Slot manager for the match
    pub slots: Arc<SlotManager>,
    /// Broadcaster for sending state to clients
    pub broadcaster: Arc<Broadcaster>,
    /// Map of slot -> client process (for cleanup)
    pub processes: HashMap<SlotId, ClientProcess>,
}

impl std::fmt::Debug for MatchSetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MatchSetup")
            .field("server_port", &self.server_port)
            .field("slots", &"<SlotManager>")
            .field("broadcaster", &"<Broadcaster>")
            .field("processes", &self.processes.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Information about a spawned client process
#[derive(Debug)]
pub struct ClientProcess {
    /// The subprocess handle
    pub child: Child,
    /// Client ID from AiParticipant
    pub client_id: String,
    /// Process ID
    pub pid: u32,
}

impl ClientProcess {
    /// Kill the client process
    pub fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }
}

/// Orchestrates match setup with external AI clients
///
/// Manages the embedded server, spawns client processes, and coordinates
/// slot assignments. Use this when any participant in the match is an
/// external client rather than an embedded profile.
pub struct MatchOrchestrator {
    config: OrchestratorConfig,
    client_db: AiClientDatabase,
}

impl MatchOrchestrator {
    /// Create a new orchestrator with the given configuration
    pub fn new(config: OrchestratorConfig) -> Self {
        let client_db = if let Some(ref path) = config.clients_file {
            AiClientDatabase::load_from_file(path)
        } else {
            AiClientDatabase::load()
        };

        Self { config, client_db }
    }

    /// Create with a pre-loaded client database
    pub fn with_client_db(config: OrchestratorConfig, client_db: AiClientDatabase) -> Self {
        Self { config, client_db }
    }

    /// Find an available port for the server
    async fn find_available_port(&self) -> Result<u16, OrchError> {
        if self.config.server_port != 0 {
            return Ok(self.config.server_port);
        }

        // Bind to port 0 to get an available port from the OS
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| OrchError::PortBindFailed(e.to_string()))?;

        let port = listener
            .local_addr()
            .map_err(|e| OrchError::PortBindFailed(e.to_string()))?
            .port();

        // Drop the listener so we can reuse the port
        drop(listener);

        Ok(port)
    }

    /// Spawn a client process for the given participant
    fn spawn_client(
        &self,
        participant: &AiParticipant,
        server_port: u16,
        slot: SlotId,
    ) -> Result<ClientProcess, OrchError> {
        let (client_id, extra_args) = match participant {
            AiParticipant::Client { id, args, .. } => (id.clone(), args.clone()),
            AiParticipant::Profile { .. } => {
                // Profiles don't need processes
                return Err(OrchError::ProcessSpawnFailed {
                    client_id: "profile".to_string(),
                    error: "Cannot spawn process for profile participant".to_string(),
                });
            }
        };

        // Look up client in database
        let client_info = self
            .client_db
            .get(&client_id)
            .ok_or_else(|| OrchError::ClientNotFound(client_id.clone()))?;

        // Build command
        let (executable, mut args) = client_info.spawn_command(server_port, &extra_args);

        // Add slot hint argument so client knows which slot to request
        args.push("--slot".to_string());
        args.push(slot.to_string());

        // Spawn process
        let mut cmd = Command::new(&executable);
        cmd.args(&args);

        if self.config.inherit_stdio {
            cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        } else {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }

        let child = cmd.spawn().map_err(|e| OrchError::ProcessSpawnFailed {
            client_id: client_id.clone(),
            error: format!("{}: {}", executable, e),
        })?;

        let pid = child.id();

        Ok(ClientProcess {
            child,
            client_id,
            pid,
        })
    }

    /// Setup a match with the given participants
    ///
    /// This will:
    /// 1. Start an embedded server
    /// 2. Spawn processes for all client participants
    /// 3. Wait for clients to connect
    /// 4. Assign clients to their designated slots
    ///
    /// Returns a `MatchSetup` with all the resources needed to run the match.
    /// The caller is responsible for calling `teardown_match` when done.
    pub async fn setup_match(
        &self,
        participants: &MatchParticipants,
    ) -> Result<MatchSetup, OrchError> {
        // Find available port
        let server_port = self.find_available_port().await?;

        // Create server components (no local player)
        let slots = Arc::new(SlotManager::new());
        let broadcaster = Arc::new(Broadcaster::new());

        // Track which slots need client connections
        let mut client_slots: Vec<(SlotId, &AiParticipant)> = Vec::new();
        let mut processes: HashMap<SlotId, ClientProcess> = HashMap::new();

        // Identify client slots and spawn processes
        for (slot, participant) in participants.iter_with_slots() {
            let slot_id = slot as SlotId;
            match participant {
                AiParticipant::Client { .. } => {
                    // Spawn process for this client
                    let process = self.spawn_client(participant, server_port, slot_id)?;
                    processes.insert(slot_id, process);
                    client_slots.push((slot_id, participant));
                }
                AiParticipant::Profile { name } => {
                    // Set up server-side AI for profile slots
                    slots.set_ai_profile(slot_id, name.clone()).await;
                }
            }
        }

        // If we have client slots, we need to start the server and wait for connections
        if !client_slots.is_empty() {
            // Start the actual server
            let server = GameServer {
                port: server_port,
                slots: slots.clone(),
                broadcaster: broadcaster.clone(),
                game_config: ballgame_protocol::handshake::GameConfig::default_config(),
            };

            // Spawn server in background
            let server_slots = server.slots.clone();
            tokio::spawn(async move {
                if let Err(e) = server.run().await {
                    eprintln!("Server error: {}", e);
                }
            });

            // Wait for all clients to connect with timeout
            let timeout_duration = Duration::from_secs(self.config.connection_timeout_secs);

            for (slot_id, participant) in &client_slots {
                let client_id = participant.display_id().to_string();

                // Wait for a client to connect and be assigned to this slot
                let connected = timeout(timeout_duration, async {
                    loop {
                        // Check waiting clients
                        let waiting = server_slots.get_waiting_clients().await;

                        // Look for a client that matches this slot
                        // Clients are expected to request their designated slot
                        for waiting_client in &waiting {
                            // Auto-assign first waiting client to this slot
                            if waiting_client.assigned_slot.is_none() {
                                if server_slots
                                    .assign_waiting_to_slot(waiting_client.client_id, *slot_id)
                                    .await
                                {
                                    return true;
                                }
                            }
                        }

                        // Check if slot is already occupied (client connected directly)
                        if server_slots.is_slot_occupied(*slot_id).await {
                            return true;
                        }

                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                })
                .await;

                match connected {
                    Ok(true) => {
                        // Client connected successfully
                    }
                    Ok(false) | Err(_) => {
                        // Timeout or connection failed - clean up processes
                        for (_, mut process) in processes {
                            let _ = process.kill();
                        }
                        return Err(OrchError::ConnectionTimeout {
                            client_id,
                            timeout_secs: self.config.connection_timeout_secs,
                        });
                    }
                }
            }
        }

        Ok(MatchSetup {
            server_port,
            slots,
            broadcaster,
            processes,
        })
    }

    /// Get the client database
    pub fn client_db(&self) -> &AiClientDatabase {
        &self.client_db
    }

    /// Check if a client ID is known
    pub fn has_client(&self, id: &str) -> bool {
        self.client_db.contains(id)
    }
}

/// Clean up a match setup, killing all client processes
pub fn teardown_match(setup: &mut MatchSetup) {
    for (_, process) in &mut setup.processes {
        let _ = process.kill();
    }
    setup.processes.clear();
}

/// Resolve participant configuration to concrete MatchParticipants
///
/// Handles backward compatibility:
/// - If `participants` is set, use it directly
/// - If `left_team`/`right_team` are set, combine them
/// - Otherwise, fall back to `left_profile`/`right_profile` (duplicated for 2v2)
pub fn resolve_participants(
    participants: Option<&MatchParticipants>,
    left_team: Option<&[AiParticipant; 2]>,
    right_team: Option<&[AiParticipant; 2]>,
    left_profile: &str,
    right_profile: &str,
) -> MatchParticipants {
    // Priority 1: Explicit participants
    if let Some(p) = participants {
        return p.clone();
    }

    // Priority 2: Team specifications
    if let (Some(left), Some(right)) = (left_team, right_team) {
        return MatchParticipants::new(
            left[0].clone(),
            left[1].clone(),
            right[0].clone(),
            right[1].clone(),
        );
    }

    // Priority 3: Profile names (backward compat, duplicated for 2v2)
    MatchParticipants::from_profile_names(left_profile, right_profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_participants_from_profiles() {
        let participants =
            resolve_participants(None, None, None, "Balanced", "Aggressive");

        assert!(participants.all_profiles());
        assert_eq!(participants.slots[0].display_id(), "Balanced");
        assert_eq!(participants.slots[1].display_id(), "Balanced");
        assert_eq!(participants.slots[2].display_id(), "Aggressive");
        assert_eq!(participants.slots[3].display_id(), "Aggressive");
    }

    #[test]
    fn test_resolve_participants_explicit() {
        let explicit = MatchParticipants::new(
            AiParticipant::client("ai-v1"),
            AiParticipant::profile("Balanced"),
            AiParticipant::client("ai-v2"),
            AiParticipant::profile("Aggressive"),
        );

        let participants = resolve_participants(
            Some(&explicit),
            None,
            None,
            "ignored",
            "ignored",
        );

        assert!(!participants.all_profiles());
        assert!(participants.has_clients());
        assert_eq!(participants.client_count(), 2);
    }

    #[test]
    fn test_orchestrator_config_defaults() {
        let config = OrchestratorConfig::default();
        assert_eq!(config.connection_timeout_secs, 30);
        assert_eq!(config.server_port, 0);
        assert!(!config.inherit_stdio);
    }

    #[test]
    fn test_orch_error_display() {
        let err = OrchError::ConnectionTimeout {
            client_id: "ai-v1".to_string(),
            timeout_secs: 30,
        };
        assert!(err.to_string().contains("ai-v1"));
        assert!(err.to_string().contains("30"));
    }
}
