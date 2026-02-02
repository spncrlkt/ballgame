//! AI Participant types for simulation matches
//!
//! Supports both embedded AI profiles and external AI clients that connect
//! via WebSocket. This enables testing different AI architectures against
//! each other in 2v2 training protocols and tournaments.

use serde::{Deserialize, Serialize};

/// Represents an AI participant in a match or tournament
///
/// A participant can be either an embedded AI profile (fast path, no network)
/// or an external AI client that connects via WebSocket (supports different
/// AI architectures like v1, v2, etc.).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum AiParticipant {
    /// Embedded AI using a profile from AiProfileDatabase
    ///
    /// This is the fast path - no network overhead, AI runs in-process.
    Profile {
        /// Profile name (e.g., "Balanced", "v11_Blend_A")
        name: String,
    },

    /// External AI client that connects via WebSocket
    ///
    /// The orchestrator will spawn this client as a subprocess and wait
    /// for it to connect to the embedded server.
    Client {
        /// Client identifier (e.g., "ai-v1", "ai-v2")
        /// Used to look up executable path in AiClientDatabase
        id: String,

        /// Display name for UI/logging (e.g., "AI v1 (Original)")
        #[serde(default)]
        display_name: Option<String>,

        /// Additional CLI arguments to pass to the client
        #[serde(default)]
        args: Vec<String>,
    },
}

impl AiParticipant {
    /// Create a profile-based participant
    pub fn profile(name: impl Into<String>) -> Self {
        Self::Profile { name: name.into() }
    }

    /// Create a client-based participant
    pub fn client(id: impl Into<String>) -> Self {
        Self::Client {
            id: id.into(),
            display_name: None,
            args: Vec::new(),
        }
    }

    /// Create a client-based participant with display name
    pub fn client_with_name(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self::Client {
            id: id.into(),
            display_name: Some(display_name.into()),
            args: Vec::new(),
        }
    }

    /// Check if this participant is a client (requires network)
    pub fn is_client(&self) -> bool {
        matches!(self, Self::Client { .. })
    }

    /// Check if this participant is a profile (embedded, no network)
    pub fn is_profile(&self) -> bool {
        matches!(self, Self::Profile { .. })
    }

    /// Get the participant's identifier for display/logging
    pub fn display_id(&self) -> &str {
        match self {
            Self::Profile { name } => name,
            Self::Client { id, .. } => id,
        }
    }

    /// Get the full display name
    pub fn display_name(&self) -> String {
        match self {
            Self::Profile { name } => name.clone(),
            Self::Client {
                id, display_name, ..
            } => display_name.clone().unwrap_or_else(|| id.clone()),
        }
    }

    /// Get the participant type as a string for database storage
    pub fn participant_type(&self) -> &'static str {
        match self {
            Self::Profile { .. } => "profile",
            Self::Client { .. } => "client",
        }
    }

    /// Get the participant's ID for database storage
    pub fn participant_id(&self) -> &str {
        match self {
            Self::Profile { name } => name,
            Self::Client { id, .. } => id,
        }
    }
}

impl Default for AiParticipant {
    fn default() -> Self {
        Self::profile("Balanced")
    }
}

impl std::fmt::Display for AiParticipant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Profile { name } => write!(f, "Profile({})", name),
            Self::Client {
                id, display_name, ..
            } => {
                if let Some(name) = display_name {
                    write!(f, "Client({}: {})", id, name)
                } else {
                    write!(f, "Client({})", id)
                }
            }
        }
    }
}

/// A team of two participants for 2v2 matches
///
/// In 2v2 format, each team has two slots:
/// - Left team: L0 (primary), L1 (secondary)
/// - Right team: R0 (primary), R1 (secondary)
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TeamParticipants {
    /// Primary player (L0 or R0)
    pub primary: AiParticipant,
    /// Secondary player (L1 or R1)
    pub secondary: AiParticipant,
}

impl TeamParticipants {
    /// Create a team with two participants
    pub fn new(primary: AiParticipant, secondary: AiParticipant) -> Self {
        Self { primary, secondary }
    }

    /// Create a team where both players use the same profile
    pub fn same_profile(profile: impl Into<String>) -> Self {
        let name = profile.into();
        Self {
            primary: AiParticipant::profile(&name),
            secondary: AiParticipant::profile(name),
        }
    }

    /// Create a team where both players use the same client
    pub fn same_client(client_id: impl Into<String>) -> Self {
        let id = client_id.into();
        Self {
            primary: AiParticipant::client(&id),
            secondary: AiParticipant::client(id),
        }
    }

    /// Check if any participant on this team is a client
    pub fn has_clients(&self) -> bool {
        self.primary.is_client() || self.secondary.is_client()
    }

    /// Check if all participants are profiles (no clients)
    pub fn all_profiles(&self) -> bool {
        self.primary.is_profile() && self.secondary.is_profile()
    }

    /// Get as array [primary, secondary]
    pub fn as_array(&self) -> [&AiParticipant; 2] {
        [&self.primary, &self.secondary]
    }

    /// Get display string for this team composition
    pub fn display(&self) -> String {
        if self.primary == self.secondary {
            format!("[{}, {}]", self.primary.display_id(), self.primary.display_id())
        } else {
            format!(
                "[{}, {}]",
                self.primary.display_id(),
                self.secondary.display_id()
            )
        }
    }
}

/// All four participants in a 2v2 match
///
/// Slot assignments:
/// - slots[0] = L0 (left team primary)
/// - slots[1] = L1 (left team secondary)
/// - slots[2] = R0 (right team primary)
/// - slots[3] = R1 (right team secondary)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatchParticipants {
    /// All four participants in slot order [L0, L1, R0, R1]
    pub slots: [AiParticipant; 4],
}

impl MatchParticipants {
    /// Create from two teams
    pub fn from_teams(left: TeamParticipants, right: TeamParticipants) -> Self {
        Self {
            slots: [left.primary, left.secondary, right.primary, right.secondary],
        }
    }

    /// Create from individual participants
    pub fn new(l0: AiParticipant, l1: AiParticipant, r0: AiParticipant, r1: AiParticipant) -> Self {
        Self {
            slots: [l0, l1, r0, r1],
        }
    }

    /// Create a match where all four players use the same profile
    pub fn all_same_profile(profile: impl Into<String>) -> Self {
        let name = profile.into();
        Self {
            slots: [
                AiParticipant::profile(&name),
                AiParticipant::profile(&name),
                AiParticipant::profile(&name),
                AiParticipant::profile(&name),
            ],
        }
    }

    /// Create a match from two profile names (duplicated for 2v2)
    ///
    /// This is for backward compatibility with the existing `left_profile`/`right_profile`
    /// configuration. Each profile fills both slots on its team.
    pub fn from_profile_names(left_profile: &str, right_profile: &str) -> Self {
        Self::from_teams(
            TeamParticipants::same_profile(left_profile),
            TeamParticipants::same_profile(right_profile),
        )
    }

    /// Get the left team
    pub fn left_team(&self) -> TeamParticipants {
        TeamParticipants {
            primary: self.slots[0].clone(),
            secondary: self.slots[1].clone(),
        }
    }

    /// Get the right team
    pub fn right_team(&self) -> TeamParticipants {
        TeamParticipants {
            primary: self.slots[2].clone(),
            secondary: self.slots[3].clone(),
        }
    }

    /// Check if any participant is a client (requires network/orchestrator)
    pub fn has_clients(&self) -> bool {
        self.slots.iter().any(|p| p.is_client())
    }

    /// Check if all participants are profiles (fast path, no network)
    pub fn all_profiles(&self) -> bool {
        self.slots.iter().all(|p| p.is_profile())
    }

    /// Count how many client participants are in this match
    pub fn client_count(&self) -> usize {
        self.slots.iter().filter(|p| p.is_client()).count()
    }

    /// Get participant by slot index
    pub fn get(&self, slot: usize) -> Option<&AiParticipant> {
        self.slots.get(slot)
    }

    /// Iterate over all participants with their slot indices
    pub fn iter_with_slots(&self) -> impl Iterator<Item = (usize, &AiParticipant)> {
        self.slots.iter().enumerate()
    }
}

impl Default for MatchParticipants {
    fn default() -> Self {
        Self::all_same_profile("Balanced")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_participant_creation() {
        let profile = AiParticipant::profile("Balanced");
        assert!(profile.is_profile());
        assert!(!profile.is_client());
        assert_eq!(profile.display_id(), "Balanced");
        assert_eq!(profile.participant_type(), "profile");

        let client = AiParticipant::client("ai-v1");
        assert!(client.is_client());
        assert!(!client.is_profile());
        assert_eq!(client.display_id(), "ai-v1");
        assert_eq!(client.participant_type(), "client");
    }

    #[test]
    fn test_team_creation() {
        let team = TeamParticipants::same_profile("Aggressive");
        assert!(team.all_profiles());
        assert!(!team.has_clients());

        let mixed_team = TeamParticipants::new(
            AiParticipant::profile("Balanced"),
            AiParticipant::client("ai-v1"),
        );
        assert!(!mixed_team.all_profiles());
        assert!(mixed_team.has_clients());
    }

    #[test]
    fn test_match_participants_from_profiles() {
        let participants = MatchParticipants::from_profile_names("Balanced", "Aggressive");
        assert!(participants.all_profiles());
        assert!(!participants.has_clients());
        assert_eq!(participants.client_count(), 0);

        // Verify slot assignments
        assert_eq!(participants.slots[0].display_id(), "Balanced"); // L0
        assert_eq!(participants.slots[1].display_id(), "Balanced"); // L1
        assert_eq!(participants.slots[2].display_id(), "Aggressive"); // R0
        assert_eq!(participants.slots[3].display_id(), "Aggressive"); // R1
    }

    #[test]
    fn test_match_participants_with_clients() {
        let participants = MatchParticipants::from_teams(
            TeamParticipants::same_client("ai-v1"),
            TeamParticipants::same_client("ai-v2"),
        );
        assert!(!participants.all_profiles());
        assert!(participants.has_clients());
        assert_eq!(participants.client_count(), 4);
    }

    #[test]
    fn test_participant_serialization() {
        let profile = AiParticipant::profile("Balanced");
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("\"type\":\"Profile\""));
        assert!(json.contains("\"name\":\"Balanced\""));

        let client = AiParticipant::client_with_name("ai-v1", "AI v1 Original");
        let json = serde_json::to_string(&client).unwrap();
        assert!(json.contains("\"type\":\"Client\""));
        assert!(json.contains("\"id\":\"ai-v1\""));

        // Round-trip
        let parsed: AiParticipant = serde_json::from_str(&json).unwrap();
        assert_eq!(client, parsed);
    }
}
