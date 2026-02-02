//! AI Client Database
//!
//! Loads AI client definitions from a registry file. Each client entry
//! specifies the executable path and display name for an external AI
//! that can connect via WebSocket.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Default path to the AI clients registry file
pub const AI_CLIENTS_FILE: &str = "config/ai_clients.txt";

/// Information about a registered AI client
#[derive(Debug, Clone)]
pub struct AiClientInfo {
    /// Client identifier (e.g., "ai-v1")
    pub id: String,

    /// Path to the executable (relative to project root or absolute)
    pub executable: String,

    /// Human-readable display name
    pub display_name: String,

    /// Default arguments to pass to the executable
    pub default_args: Vec<String>,
}

impl AiClientInfo {
    /// Get the full command to spawn this client with the given port
    pub fn spawn_command(&self, server_port: u16, extra_args: &[String]) -> (String, Vec<String>) {
        let mut args = self.default_args.clone();
        args.push("--port".to_string());
        args.push(server_port.to_string());
        args.extend(extra_args.iter().cloned());
        (self.executable.clone(), args)
    }
}

/// Database of registered AI clients
///
/// Loaded from a configuration file, provides lookup by client ID.
#[derive(Debug, Clone, Default)]
pub struct AiClientDatabase {
    clients: HashMap<String, AiClientInfo>,
    order: Vec<String>, // Preserve insertion order for iteration
}

impl AiClientDatabase {
    /// Create an empty database
    pub fn new() -> Self {
        Self::default()
    }

    /// Load clients from the default config file
    pub fn load() -> Self {
        Self::load_from_file(AI_CLIENTS_FILE)
    }

    /// Load clients from a specific file
    ///
    /// File format:
    /// ```text
    /// # Comments start with #
    ///
    /// client:ai-v1
    ///   executable:./target/release/ballgame-ai-v1
    ///   display_name:AI v1 (Original)
    ///   args:--verbose
    ///
    /// client:ai-v2
    ///   executable:./target/release/ballgame-ai-v2
    ///   display_name:AI v2 (Template)
    /// ```
    pub fn load_from_file(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "Warning: Could not read AI clients file '{}': {}",
                    path.display(),
                    e
                );
                return Self::default();
            }
        };

        Self::parse(&content)
    }

    /// Parse client definitions from text content
    pub fn parse(content: &str) -> Self {
        let mut db = Self::new();
        let mut current_client: Option<AiClientInfo> = None;

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Check for new client definition
            if let Some(id) = line.strip_prefix("client:") {
                // Save previous client if any
                if let Some(client) = current_client.take() {
                    db.add(client);
                }

                // Start new client
                current_client = Some(AiClientInfo {
                    id: id.trim().to_string(),
                    executable: String::new(),
                    display_name: String::new(),
                    default_args: Vec::new(),
                });
                continue;
            }

            // Parse client properties
            if let Some(ref mut client) = current_client {
                if let Some(value) = line.strip_prefix("executable:") {
                    client.executable = value.trim().to_string();
                } else if let Some(value) = line.strip_prefix("display_name:") {
                    client.display_name = value.trim().to_string();
                } else if let Some(value) = line.strip_prefix("args:") {
                    // Parse space-separated arguments
                    client.default_args = value
                        .split_whitespace()
                        .map(|s| s.to_string())
                        .collect();
                }
            }
        }

        // Save last client
        if let Some(client) = current_client {
            db.add(client);
        }

        db
    }

    /// Add a client to the database
    pub fn add(&mut self, client: AiClientInfo) {
        if !self.clients.contains_key(&client.id) {
            self.order.push(client.id.clone());
        }
        self.clients.insert(client.id.clone(), client);
    }

    /// Look up a client by ID
    pub fn get(&self, id: &str) -> Option<&AiClientInfo> {
        self.clients.get(id)
    }

    /// Check if a client ID exists
    pub fn contains(&self, id: &str) -> bool {
        self.clients.contains_key(id)
    }

    /// Get all clients in registration order
    pub fn all(&self) -> Vec<&AiClientInfo> {
        self.order
            .iter()
            .filter_map(|id| self.clients.get(id))
            .collect()
    }

    /// Get all client IDs
    pub fn ids(&self) -> &[String] {
        &self.order
    }

    /// Number of registered clients
    pub fn len(&self) -> usize {
        self.clients.len()
    }

    /// Check if database is empty
    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    /// Iterate over all clients
    pub fn iter(&self) -> impl Iterator<Item = &AiClientInfo> {
        self.order.iter().filter_map(|id| self.clients.get(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        let db = AiClientDatabase::parse("");
        assert!(db.is_empty());
    }

    #[test]
    fn test_parse_comments_only() {
        let db = AiClientDatabase::parse(
            r#"
            # This is a comment
            # Another comment
        "#,
        );
        assert!(db.is_empty());
    }

    #[test]
    fn test_parse_single_client() {
        let db = AiClientDatabase::parse(
            r#"
            client:ai-v1
              executable:./target/release/ballgame-ai-v1
              display_name:AI v1 (Original)
        "#,
        );

        assert_eq!(db.len(), 1);
        let client = db.get("ai-v1").unwrap();
        assert_eq!(client.id, "ai-v1");
        assert_eq!(client.executable, "./target/release/ballgame-ai-v1");
        assert_eq!(client.display_name, "AI v1 (Original)");
    }

    #[test]
    fn test_parse_multiple_clients() {
        let db = AiClientDatabase::parse(
            r#"
            # AI Clients Registry

            client:ai-v1
              executable:./target/release/ballgame-ai-v1
              display_name:AI v1 (Original)

            client:ai-v2
              executable:./target/release/ballgame-ai-v2
              display_name:AI v2 (Template)
              args:--verbose --debug
        "#,
        );

        assert_eq!(db.len(), 2);

        let v1 = db.get("ai-v1").unwrap();
        assert_eq!(v1.id, "ai-v1");
        assert!(v1.default_args.is_empty());

        let v2 = db.get("ai-v2").unwrap();
        assert_eq!(v2.id, "ai-v2");
        assert_eq!(v2.default_args, vec!["--verbose", "--debug"]);
    }

    #[test]
    fn test_spawn_command() {
        let client = AiClientInfo {
            id: "ai-v1".to_string(),
            executable: "./target/release/ballgame-ai-v1".to_string(),
            display_name: "AI v1".to_string(),
            default_args: vec!["--verbose".to_string()],
        };

        let (exe, args) = client.spawn_command(8080, &["--extra".to_string()]);
        assert_eq!(exe, "./target/release/ballgame-ai-v1");
        assert_eq!(args, vec!["--verbose", "--port", "8080", "--extra"]);
    }

    #[test]
    fn test_iteration_order() {
        let db = AiClientDatabase::parse(
            r#"
            client:c
              executable:c
            client:a
              executable:a
            client:b
              executable:b
        "#,
        );

        let ids: Vec<_> = db.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, vec!["c", "a", "b"]);
    }
}
