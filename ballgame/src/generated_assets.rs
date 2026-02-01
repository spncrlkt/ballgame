//! Track generated asset timestamps for quick reference
//!
//! This module provides a central registry for tracking when generated assets
//! (databases, heatmaps, textures, etc.) were last updated. This helps developers
//! quickly see what's stale and needs regeneration.

use chrono::Local;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Path to the generated assets tracking file (gitignored)
const ASSETS_FILE: &str = "config/generated_assets.json";

/// Path to the template file (tracked in git)
const TEMPLATE_FILE: &str = "config/generated_assets.template.json";

/// Root structure for tracking all generated assets
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneratedAssets {
    /// Database file tracking
    #[serde(default)]
    pub databases: DatabaseAssets,
    /// Heatmap file tracking
    #[serde(default)]
    pub heatmaps: HeatmapAssets,
    /// Ball texture tracking
    #[serde(default)]
    pub ball_textures: AssetInfo,
    /// Snapshot tracking
    #[serde(default)]
    pub snapshots: AssetInfo,
    /// Ghost trial tracking
    #[serde(default)]
    pub ghost_trials: AssetInfo,
    /// Training log tracking
    #[serde(default)]
    pub training_logs: TrainingLogAssets,
    /// Ranking file tracking
    #[serde(default)]
    pub rankings: RankingAssets,
}

/// Generic asset info for simple tracked resources
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AssetInfo {
    /// ISO timestamp of last update
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    /// Path to the most recent file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<String>,
    /// Number of files in this category
    #[serde(default)]
    pub count: u32,
}

/// Database asset tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseAssets {
    /// Most recent training database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_training: Option<String>,
    /// Most recent tournament database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_tournament: Option<String>,
    /// Most recent bracket database
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_bracket: Option<String>,
}

/// Heatmap asset tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HeatmapAssets {
    /// Speed heatmaps
    #[serde(default)]
    pub speed: AssetInfo,
    /// Score heatmaps
    #[serde(default)]
    pub score: AssetInfo,
    /// Reachability heatmaps
    #[serde(default)]
    pub reachability: AssetInfo,
}

/// Training log tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrainingLogAssets {
    /// Most recent session directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_session: Option<String>,
}

/// Ranking file tracking
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RankingAssets {
    /// Tournament rankings last updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tournament: Option<String>,
    /// Bracket rankings last updated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bracket: Option<String>,
}

impl GeneratedAssets {
    /// Load the generated assets tracking file
    ///
    /// Returns default (empty) assets if the file doesn't exist or can't be parsed.
    pub fn load() -> Self {
        let path = Path::new(ASSETS_FILE);
        if !path.exists() {
            // Try to copy from template
            if Path::new(TEMPLATE_FILE).exists() {
                let _ = std::fs::copy(TEMPLATE_FILE, ASSETS_FILE);
            }
        }

        std::fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Save the generated assets tracking file
    pub fn save(&self) -> std::io::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(ASSETS_FILE, content)
    }

    /// Get current timestamp in ISO format
    fn now() -> String {
        Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
    }

    /// Record a database was created
    pub fn record_database(&mut self, db_type: &str, path: &str) {
        let path_str = path.to_string();
        match db_type {
            "training" => self.databases.latest_training = Some(path_str),
            "tournament" => self.databases.latest_tournament = Some(path_str),
            "bracket" => self.databases.latest_bracket = Some(path_str),
            _ => {}
        }
    }

    /// Record heatmaps were generated
    pub fn record_heatmaps(&mut self, heatmap_type: &str, count: u32) {
        let info = match heatmap_type {
            "speed" => &mut self.heatmaps.speed,
            "score" => &mut self.heatmaps.score,
            "reachability" => &mut self.heatmaps.reachability,
            _ => return,
        };
        info.updated_at = Some(Self::now());
        info.count = count;
    }

    /// Record ball textures were generated
    pub fn record_ball_textures(&mut self, count: u32) {
        self.ball_textures.updated_at = Some(Self::now());
        self.ball_textures.count = count;
    }

    /// Record a snapshot was created
    pub fn record_snapshot(&mut self, path: &str) {
        self.snapshots.updated_at = Some(Self::now());
        self.snapshots.latest = Some(path.to_string());
        self.snapshots.count += 1;
    }

    /// Record a ghost trial was created
    pub fn record_ghost_trial(&mut self, path: &str) {
        self.ghost_trials.updated_at = Some(Self::now());
        self.ghost_trials.latest = Some(path.to_string());
        self.ghost_trials.count += 1;
    }

    /// Record a training session was created
    pub fn record_training_session(&mut self, session_dir: &str) {
        self.training_logs.latest_session = Some(session_dir.to_string());
    }

    /// Record rankings were updated
    pub fn record_rankings(&mut self, ranking_type: &str) {
        let timestamp = Self::now();
        match ranking_type {
            "tournament" => self.rankings.tournament = Some(timestamp),
            "bracket" => self.rankings.bracket = Some(timestamp),
            _ => {}
        }
    }
}

/// Convenience function to record an asset and save
///
/// This is the main entry point for recording assets from other binaries.
/// It loads the current state, updates it, and saves in one call.
pub fn record_database(db_type: &str, path: &str) {
    let mut assets = GeneratedAssets::load();
    assets.record_database(db_type, path);
    let _ = assets.save();
}

/// Record heatmap generation
pub fn record_heatmaps(heatmap_type: &str, count: u32) {
    let mut assets = GeneratedAssets::load();
    assets.record_heatmaps(heatmap_type, count);
    let _ = assets.save();
}

/// Record ball texture generation
pub fn record_ball_textures(count: u32) {
    let mut assets = GeneratedAssets::load();
    assets.record_ball_textures(count);
    let _ = assets.save();
}

/// Record snapshot creation
pub fn record_snapshot(path: &str) {
    let mut assets = GeneratedAssets::load();
    assets.record_snapshot(path);
    let _ = assets.save();
}

/// Record ghost trial creation
pub fn record_ghost_trial(path: &str) {
    let mut assets = GeneratedAssets::load();
    assets.record_ghost_trial(path);
    let _ = assets.save();
}

/// Record training session creation
pub fn record_training_session(session_dir: &str) {
    let mut assets = GeneratedAssets::load();
    assets.record_training_session(session_dir);
    let _ = assets.save();
}

/// Record rankings update
pub fn record_rankings(ranking_type: &str) {
    let mut assets = GeneratedAssets::load();
    assets.record_rankings(ranking_type);
    let _ = assets.save();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_assets() {
        let assets = GeneratedAssets::default();
        assert!(assets.databases.latest_training.is_none());
        assert!(assets.heatmaps.speed.updated_at.is_none());
    }

    #[test]
    fn test_record_database() {
        let mut assets = GeneratedAssets::default();
        assets.record_database("training", "db/training_20260130_120000.db");
        assert_eq!(
            assets.databases.latest_training,
            Some("db/training_20260130_120000.db".to_string())
        );
    }

    #[test]
    fn test_record_heatmaps() {
        let mut assets = GeneratedAssets::default();
        assets.record_heatmaps("speed", 10);
        assert!(assets.heatmaps.speed.updated_at.is_some());
        assert_eq!(assets.heatmaps.speed.count, 10);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let mut assets = GeneratedAssets::default();
        assets.record_database("training", "db/test.db");
        assets.record_ball_textures(5);

        let json = serde_json::to_string(&assets).unwrap();
        let parsed: GeneratedAssets = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.databases.latest_training, assets.databases.latest_training);
        assert_eq!(parsed.ball_textures.count, assets.ball_textures.count);
    }
}
