//! TOML test file parsing

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Complete test definition from TOML file
#[derive(Debug, Deserialize)]
pub struct TestDefinition {
    pub name: String,
    pub description: Option<String>,
    pub setup: TestSetup,
    #[serde(default)]
    pub input: Vec<FrameInput>,
    pub expect: TestExpectations,
}

/// Test setup configuration
#[derive(Debug, Deserialize)]
pub struct TestSetup {
    pub level: String,
    pub seed: Option<u64>,
    #[serde(default)]
    pub entities: Vec<EntityDef>,
}

/// Entity definition for spawning
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum EntityDef {
    #[serde(rename = "player")]
    Player {
        id: String,
        team: String,
        x: f32,
        y: f32,
        #[serde(default = "default_facing")]
        facing: f32,
        #[serde(default)]
        holding_ball: bool,
    },
    #[serde(rename = "ball")]
    Ball {
        x: f32,
        y: f32,
        #[serde(default)]
        velocity_x: f32,
        #[serde(default)]
        velocity_y: f32,
    },
}

fn default_facing() -> f32 {
    1.0
}

/// Input state at a specific frame
#[derive(Debug, Deserialize)]
pub struct FrameInput {
    pub frame: u64,
    #[serde(flatten)]
    pub inputs: HashMap<String, InputSnapshot>,
}

/// Snapshot of input state for one entity
#[derive(Debug, Clone, Default, Deserialize)]
pub struct InputSnapshot {
    #[serde(default)]
    pub move_x: Option<f32>,
    #[serde(default)]
    pub move_left: Option<bool>,
    #[serde(default)]
    pub move_right: Option<bool>,
    #[serde(default)]
    pub jump: Option<bool>,
    #[serde(default)]
    pub pickup: Option<bool>,
    #[serde(default)]
    pub throw_held: Option<bool>,
}

impl InputSnapshot {
    /// Convert to effective move_x value
    pub fn effective_move_x(&self) -> f32 {
        if let Some(x) = self.move_x {
            return x;
        }
        let mut x = 0.0;
        if self.move_left.unwrap_or(false) {
            x -= 1.0;
        }
        if self.move_right.unwrap_or(false) {
            x += 1.0;
        }
        x
    }
}

/// Expected test outcomes
#[derive(Debug, Default, Deserialize)]
pub struct TestExpectations {
    #[serde(default)]
    pub sequence: Vec<ExpectedEvent>,
    /// Multiple state assertions at different frames (uses [[expect.state]] TOML syntax)
    #[serde(default)]
    pub state: Vec<StateAssertion>,
}

/// Expected event in sequence
#[derive(Debug, Deserialize)]
pub struct ExpectedEvent {
    pub event: String,
    pub player: Option<String>,
    pub frame_min: Option<u64>,
    pub frame_max: Option<u64>,
    #[serde(default = "default_tolerance")]
    pub tolerance: u64,
}

fn default_tolerance() -> u64 {
    5
}

/// State assertion after simulation
#[derive(Debug, Clone, Deserialize)]
pub struct StateAssertion {
    pub after_frame: u64,
    #[serde(default)]
    pub checks: Vec<String>,
}

/// Parse a test file from path
pub fn parse_test_file(path: &Path) -> Result<TestDefinition, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    toml::from_str(&content).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic() {
        let toml = r#"
name = "Test"
[setup]
level = "test_flat_floor"
[[setup.entities]]
type = "player"
id = "p1"
team = "left"
x = 100.0
y = 200.0

[expect]
"#;
        let def: TestDefinition = toml::from_str(toml).unwrap();
        assert_eq!(def.name, "Test");
        assert_eq!(def.setup.level, "test_flat_floor");
    }
}
