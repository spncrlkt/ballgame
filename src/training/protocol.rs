//! Training protocol definitions
//!
//! Protocols define different training configurations for testing specific AI behaviors.

use serde::{Deserialize, Serialize};

/// Training protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrainingProtocol {
    /// Full 1v1 games on random levels (original training mode)
    /// - Random level selection (excluding debug levels)
    /// - Win condition: first to N points
    /// - Metrics: possession, shots, steals, goals
    #[default]
    AdvancedPlatform,

    /// AI pursuit verification test
    /// - Fixed flat level (Pursuit Arena)
    /// - Player starts with ball, AI must chase
    /// - End condition: score or time limit
    /// - Metrics: distance over time, closing rate, stuck detection
    Pursuit,

    /// AI pursuit verification test level 2
    /// - Fixed level with center platform (Pursuit Arena 2)
    /// - Player starts with ball, AI must chase
    /// - End condition: score or time limit
    /// - Metrics: distance over time, closing rate, stuck detection
    Pursuit2,

    /// Solo level exploration for reachability analysis
    /// - No AI opponent (AI spawned but idle)
    /// - Iterates through all non-debug levels in order
    /// - Player presses LB to advance to next level
    /// - No win condition - player decides when done with each level
    /// - Captures position data for coverage analysis
    Reachability,

    /// Automated reachability exploration via random walk and hop
    /// - No human input needed - character moves automatically
    /// - Walks along platform surfaces, jumps at edges with random timing
    /// - Iterates through all non-debug levels
    /// - Auto-advances when coverage plateaus or time limit reached
    /// - Generates comprehensive reachability heatmaps
    AutoReachability,

    /// Team interaction training - practice passing with CatchPartner AI
    /// - Fixed flat level with center platform
    /// - Teammate uses CatchPartner AI profile (catches and returns passes)
    /// - No enemy AI, no scoring - purely cooperative pass practice
    /// - D-pad: Down=restart, Left=advance level
    TeamInteraction,

    /// Keep-away training - ball possession practice
    /// - Fixed flat level
    /// - Uses KeepAway AI profile
    /// - Cooperative mode for practicing keep-away gameplay
    KeepAway,
}

// TODO: add a shooting training protocol for basket position calculations.

impl TrainingProtocol {
    /// Parse protocol from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        let normalized = s.trim().to_lowercase().replace('_', "-").replace(' ', "-");
        match normalized.as_str() {
            "advanced-platform" | "advancedplatform" | "advanced" | "platform" => {
                Some(TrainingProtocol::AdvancedPlatform)
            }
            "pursuit" | "chase" => Some(TrainingProtocol::Pursuit),
            "pursuit2" | "pursuit-2" | "pursuit-level-2" => Some(TrainingProtocol::Pursuit2),
            "reachability" | "reach" | "exploration" => Some(TrainingProtocol::Reachability),
            "auto-reachability" | "autoreachability" | "auto-reach" | "autoreach" | "auto" => {
                Some(TrainingProtocol::AutoReachability)
            }
            "team-interaction" | "teaminteraction" | "team" | "catch" | "catch-partner" => {
                Some(TrainingProtocol::TeamInteraction)
            }
            "keep-away" | "keepaway" => Some(TrainingProtocol::KeepAway),
            _ => None,
        }
    }

    /// Get the display name
    pub fn display_name(&self) -> &'static str {
        match self {
            TrainingProtocol::AdvancedPlatform => "Advanced Platform",
            TrainingProtocol::Pursuit => "Pursuit Test",
            TrainingProtocol::Pursuit2 => "Pursuit Test Level 2",
            TrainingProtocol::Reachability => "Reachability Exploration",
            TrainingProtocol::AutoReachability => "Auto Reachability",
            TrainingProtocol::TeamInteraction => "Team Interaction",
            TrainingProtocol::KeepAway => "Keep Away",
        }
    }

    /// Get the CLI name (used in --protocol argument)
    pub fn cli_name(&self) -> &'static str {
        match self {
            TrainingProtocol::AdvancedPlatform => "advanced-platform",
            TrainingProtocol::Pursuit => "pursuit",
            TrainingProtocol::Pursuit2 => "pursuit2",
            TrainingProtocol::Reachability => "reachability",
            TrainingProtocol::AutoReachability => "auto-reachability",
            TrainingProtocol::TeamInteraction => "team-interaction",
            TrainingProtocol::KeepAway => "keep-away",
        }
    }

    /// Get the description
    pub fn description(&self) -> &'static str {
        match self {
            TrainingProtocol::AdvancedPlatform => {
                "Full 1v1 games on random levels with comprehensive analysis"
            }
            TrainingProtocol::Pursuit => "Flat level chase test - verifies AI pursues the player",
            TrainingProtocol::Pursuit2 => "Platform chase test - pursuit with center obstacle",
            TrainingProtocol::Reachability => {
                "Solo level exploration - iterate through all levels for coverage mapping"
            }
            TrainingProtocol::AutoReachability => {
                "Automated random walk/hop exploration for comprehensive reachability mapping"
            }
            TrainingProtocol::TeamInteraction => {
                "Cooperative pass practice with CatchPartner AI teammate"
            }
            TrainingProtocol::KeepAway => "Keep-away training - ball possession practice",
        }
    }

    /// Get the fixed level name for this protocol (None = random selection)
    pub fn fixed_level(&self) -> Option<&'static str> {
        match self {
            TrainingProtocol::AdvancedPlatform => None,
            TrainingProtocol::Pursuit => Some("Pursuit Arena"),
            TrainingProtocol::Pursuit2 => Some("Pursuit Arena 2"),
            TrainingProtocol::Reachability => None, // Iterates all levels
            TrainingProtocol::AutoReachability => None, // Iterates all levels
            TrainingProtocol::TeamInteraction => Some("Team Interaction"),
            TrainingProtocol::KeepAway => Some("Keep Away"),
        }
    }

    /// Get the default time limit in seconds (None = no limit)
    pub fn default_time_limit(&self) -> Option<f32> {
        match self {
            TrainingProtocol::AdvancedPlatform => None,
            TrainingProtocol::Pursuit => Some(30.0), // 30 second default for pursuit
            TrainingProtocol::Pursuit2 => Some(30.0), // 30 second default for pursuit2
            TrainingProtocol::Reachability => None,  // Player decides when done
            TrainingProtocol::AutoReachability => Some(60.0), // 60 seconds per level
            TrainingProtocol::TeamInteraction => None, // No time limit - practice mode
            TrainingProtocol::KeepAway => None,      // No time limit - practice mode
        }
    }

    /// Whether this protocol uses score-based win condition
    pub fn uses_score_win(&self) -> bool {
        match self {
            TrainingProtocol::AdvancedPlatform => true,
            TrainingProtocol::Pursuit | TrainingProtocol::Pursuit2 => true, // Ends on score OR time
            TrainingProtocol::Reachability
            | TrainingProtocol::AutoReachability
            | TrainingProtocol::TeamInteraction
            | TrainingProtocol::KeepAway => false, // No win condition
        }
    }

    /// Whether player should start with the ball
    pub fn player_starts_with_ball(&self) -> bool {
        match self {
            TrainingProtocol::AdvancedPlatform => true, // Already implemented
            TrainingProtocol::Pursuit | TrainingProtocol::Pursuit2 => true, // AI must chase
            TrainingProtocol::Reachability => true,     // Exploration mode
            TrainingProtocol::AutoReachability => false, // No ball needed for exploration
            TrainingProtocol::TeamInteraction => true,  // Player starts with ball to pass
            TrainingProtocol::KeepAway => true,         // Player starts with ball
        }
    }

    /// Whether this is a solo exploration mode (no active AI opponent)
    pub fn is_solo_mode(&self) -> bool {
        matches!(
            self,
            TrainingProtocol::Reachability | TrainingProtocol::AutoReachability
        )
    }

    /// Whether this protocol uses CatchPartner AI for teammate
    /// Note: KeepAway uses KeepAwayTeammate instead (similar but with evasion)
    pub fn uses_catch_partner(&self) -> bool {
        matches!(self, TrainingProtocol::TeamInteraction)
    }

    /// Whether this protocol iterates through all levels sequentially
    pub fn iterates_all_levels(&self) -> bool {
        matches!(
            self,
            TrainingProtocol::Reachability | TrainingProtocol::AutoReachability
        )
    }

    /// Whether this protocol uses automated input (no human control)
    pub fn is_automated(&self) -> bool {
        matches!(self, TrainingProtocol::AutoReachability)
    }

    /// Whether this is a cooperative 2-player mode (no enemy AI)
    /// Note: KeepAway is NOT coop - it has an adversary
    pub fn is_coop_mode(&self) -> bool {
        matches!(self, TrainingProtocol::TeamInteraction)
    }

    /// Whether this is keep-away mode (3 players: human + teammate vs adversary)
    pub fn is_keep_away_mode(&self) -> bool {
        matches!(self, TrainingProtocol::KeepAway)
    }
}

impl std::fmt::Display for TrainingProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.cli_name())
    }
}

/// Protocol-specific configuration
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    /// The protocol type
    pub protocol: TrainingProtocol,
    /// Level name (resolved from fixed_level or user setting)
    pub level_name: Option<String>,
    /// Time limit in seconds
    pub time_limit_secs: Option<f32>,
    /// Win score (1 for single-goal tests)
    pub win_score: u32,
}

impl ProtocolConfig {
    /// Create config for a protocol with defaults
    pub fn new(protocol: TrainingProtocol) -> Self {
        Self {
            protocol,
            level_name: protocol.fixed_level().map(String::from),
            time_limit_secs: protocol.default_time_limit(),
            win_score: match protocol {
                TrainingProtocol::AdvancedPlatform => 5,
                TrainingProtocol::Pursuit | TrainingProtocol::Pursuit2 => 1, // End on first score
                TrainingProtocol::Reachability
                | TrainingProtocol::AutoReachability
                | TrainingProtocol::TeamInteraction
                | TrainingProtocol::KeepAway => 0, // No score-based win
            },
        }
    }

    /// Override time limit
    pub fn with_time_limit(mut self, secs: f32) -> Self {
        self.time_limit_secs = Some(secs);
        self
    }

    /// Override win score
    pub fn with_win_score(mut self, score: u32) -> Self {
        self.win_score = score;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_parsing() {
        assert_eq!(
            TrainingProtocol::from_str("pursuit"),
            Some(TrainingProtocol::Pursuit)
        );
        assert_eq!(
            TrainingProtocol::from_str("PURSUIT"),
            Some(TrainingProtocol::Pursuit)
        );
        assert_eq!(
            TrainingProtocol::from_str("advanced-platform"),
            Some(TrainingProtocol::AdvancedPlatform)
        );
        assert_eq!(
            TrainingProtocol::from_str("advanced_platform"),
            Some(TrainingProtocol::AdvancedPlatform)
        );
        assert_eq!(
            TrainingProtocol::from_str("advanced"),
            Some(TrainingProtocol::AdvancedPlatform)
        );
        assert_eq!(TrainingProtocol::from_str("invalid"), None);
        // Pursuit2 parsing
        assert_eq!(
            TrainingProtocol::from_str("pursuit2"),
            Some(TrainingProtocol::Pursuit2)
        );
        assert_eq!(
            TrainingProtocol::from_str("pursuit-2"),
            Some(TrainingProtocol::Pursuit2)
        );
        assert_eq!(
            TrainingProtocol::from_str("pursuit-level-2"),
            Some(TrainingProtocol::Pursuit2)
        );
        // Reachability parsing
        assert_eq!(
            TrainingProtocol::from_str("reachability"),
            Some(TrainingProtocol::Reachability)
        );
        assert_eq!(
            TrainingProtocol::from_str("reach"),
            Some(TrainingProtocol::Reachability)
        );
        assert_eq!(
            TrainingProtocol::from_str("exploration"),
            Some(TrainingProtocol::Reachability)
        );
        // AutoReachability parsing
        assert_eq!(
            TrainingProtocol::from_str("auto-reachability"),
            Some(TrainingProtocol::AutoReachability)
        );
        assert_eq!(
            TrainingProtocol::from_str("auto-reach"),
            Some(TrainingProtocol::AutoReachability)
        );
        assert_eq!(
            TrainingProtocol::from_str("auto"),
            Some(TrainingProtocol::AutoReachability)
        );
    }

    #[test]
    fn test_protocol_config_defaults() {
        let pursuit = ProtocolConfig::new(TrainingProtocol::Pursuit);
        assert_eq!(pursuit.level_name, Some("Pursuit Arena".to_string()));
        assert_eq!(pursuit.time_limit_secs, Some(30.0));
        assert_eq!(pursuit.win_score, 1);

        let pursuit2 = ProtocolConfig::new(TrainingProtocol::Pursuit2);
        assert_eq!(pursuit2.level_name, Some("Pursuit Arena 2".to_string()));
        assert_eq!(pursuit2.time_limit_secs, Some(30.0));
        assert_eq!(pursuit2.win_score, 1);

        let advanced = ProtocolConfig::new(TrainingProtocol::AdvancedPlatform);
        assert_eq!(advanced.level_name, None);
        assert_eq!(advanced.time_limit_secs, None);
        assert_eq!(advanced.win_score, 5);

        let reachability = ProtocolConfig::new(TrainingProtocol::Reachability);
        assert_eq!(reachability.level_name, None);
        assert_eq!(reachability.time_limit_secs, None);
        assert_eq!(reachability.win_score, 0);
        assert!(TrainingProtocol::Reachability.is_solo_mode());
        assert!(TrainingProtocol::Reachability.iterates_all_levels());
        assert!(!TrainingProtocol::Reachability.is_automated());

        // AutoReachability config
        let auto_reach = ProtocolConfig::new(TrainingProtocol::AutoReachability);
        assert_eq!(auto_reach.level_name, None);
        assert_eq!(auto_reach.time_limit_secs, Some(60.0)); // 60 second default
        assert_eq!(auto_reach.win_score, 0);
        assert!(TrainingProtocol::AutoReachability.is_solo_mode());
        assert!(TrainingProtocol::AutoReachability.iterates_all_levels());
        assert!(TrainingProtocol::AutoReachability.is_automated());
    }
}
