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
            _ => None,
        }
    }

    /// Get the display name
    pub fn display_name(&self) -> &'static str {
        match self {
            TrainingProtocol::AdvancedPlatform => "Advanced Platform",
            TrainingProtocol::Pursuit => "Pursuit Test",
            TrainingProtocol::Pursuit2 => "Pursuit Test Level 2",
        }
    }

    /// Get the CLI name (used in --protocol argument)
    pub fn cli_name(&self) -> &'static str {
        match self {
            TrainingProtocol::AdvancedPlatform => "advanced-platform",
            TrainingProtocol::Pursuit => "pursuit",
            TrainingProtocol::Pursuit2 => "pursuit2",
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
        }
    }

    /// Get the fixed level name for this protocol (None = random selection)
    pub fn fixed_level(&self) -> Option<&'static str> {
        match self {
            TrainingProtocol::AdvancedPlatform => None,
            TrainingProtocol::Pursuit => Some("Pursuit Arena"),
            TrainingProtocol::Pursuit2 => Some("Pursuit Arena 2"),
        }
    }

    /// Get the default time limit in seconds (None = no limit)
    pub fn default_time_limit(&self) -> Option<f32> {
        match self {
            TrainingProtocol::AdvancedPlatform => None,
            TrainingProtocol::Pursuit => Some(30.0), // 30 second default for pursuit
            TrainingProtocol::Pursuit2 => Some(30.0), // 30 second default for pursuit2
        }
    }

    /// Whether this protocol uses score-based win condition
    pub fn uses_score_win(&self) -> bool {
        match self {
            TrainingProtocol::AdvancedPlatform => true,
            TrainingProtocol::Pursuit | TrainingProtocol::Pursuit2 => true, // Ends on score OR time
        }
    }

    /// Whether player should start with the ball
    pub fn player_starts_with_ball(&self) -> bool {
        match self {
            TrainingProtocol::AdvancedPlatform => true, // Already implemented
            TrainingProtocol::Pursuit | TrainingProtocol::Pursuit2 => true, // AI must chase
        }
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
    }
}
