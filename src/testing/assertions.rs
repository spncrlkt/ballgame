//! Assertion checking for test expectations

use super::parser::{ExpectedEvent, StateAssertion};
use crate::events::GameEvent;

/// Error when an assertion fails
#[derive(Debug)]
pub struct AssertionError {
    pub message: String,
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Display for AssertionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n    Expected: {}\n    Actual: {}", self.message, self.expected, self.actual)
    }
}

/// Captured event with timing info
#[derive(Debug, Clone)]
pub struct CapturedEvent {
    pub frame: u64,
    pub event_type: String,
    pub player: Option<String>,
}

impl CapturedEvent {
    pub fn from_game_event(frame: u64, event: &GameEvent, entity_map: &std::collections::HashMap<bevy::prelude::Entity, String>) -> Option<Self> {
        let (event_type, player_entity) = match event {
            GameEvent::Pickup { player } => ("Pickup".to_string(), Some(player)),
            GameEvent::Drop { player } => ("Drop".to_string(), Some(player)),
            GameEvent::ShotStart { player, .. } => ("ShotStart".to_string(), Some(player)),
            GameEvent::ShotRelease { player, .. } => ("ShotRelease".to_string(), Some(player)),
            GameEvent::StealAttempt { attacker } => ("StealAttempt".to_string(), Some(attacker)),
            GameEvent::StealSuccess { attacker } => ("StealSuccess".to_string(), Some(attacker)),
            GameEvent::StealFail { attacker } => ("StealFail".to_string(), Some(attacker)),
            GameEvent::Goal { player, .. } => ("Goal".to_string(), Some(player)),
            _ => return None,
        };

        let player = player_entity.and_then(|p| {
            // Map PlayerId to entity ID string
            match p {
                crate::events::PlayerId::L => entity_map.iter().find(|(_, id)| id.contains("left") || **id == "attacker" || **id == "p1").map(|(_, id)| id.clone()),
                crate::events::PlayerId::R => entity_map.iter().find(|(_, id)| id.contains("right") || **id == "victim" || **id == "p2").map(|(_, id)| id.clone()),
            }
        });

        Some(CapturedEvent {
            frame,
            event_type,
            player,
        })
    }
}

/// Check if captured events match expected sequence
pub fn check_sequence(expected: &[ExpectedEvent], captured: &[CapturedEvent]) -> Result<(), AssertionError> {
    let mut captured_idx = 0;

    for (i, exp) in expected.iter().enumerate() {
        // Find matching event starting from current position
        let found = captured[captured_idx..].iter().enumerate().find(|(_, cap)| {
            if cap.event_type != exp.event {
                return false;
            }
            if let Some(ref exp_player) = exp.player {
                if cap.player.as_ref() != Some(exp_player) {
                    return false;
                }
            }
            true
        });

        match found {
            Some((offset, cap)) => {
                // Check frame bounds if specified
                if let Some(min) = exp.frame_min {
                    if cap.frame < min {
                        return Err(AssertionError {
                            message: format!("Event #{} '{}' occurred too early", i + 1, exp.event),
                            expected: format!("frame >= {}", min),
                            actual: format!("frame {}", cap.frame),
                        });
                    }
                }
                if let Some(max) = exp.frame_max {
                    if cap.frame > max {
                        return Err(AssertionError {
                            message: format!("Event #{} '{}' occurred too late", i + 1, exp.event),
                            expected: format!("frame <= {}", max),
                            actual: format!("frame {}", cap.frame),
                        });
                    }
                }
                captured_idx += offset + 1;
            }
            None => {
                let player_str = exp.player.as_ref().map(|p| format!(" (player: {})", p)).unwrap_or_default();
                return Err(AssertionError {
                    message: format!("Event #{} '{}'{} not found", i + 1, exp.event, player_str),
                    expected: format!("'{}' event in sequence", exp.event),
                    actual: format!("events after position {}: {:?}",
                        captured_idx,
                        captured[captured_idx..].iter().map(|e| &e.event_type).collect::<Vec<_>>()
                    ),
                });
            }
        }
    }

    Ok(())
}

/// World state for assertions
pub struct WorldState {
    pub entities: std::collections::HashMap<String, EntityState>,
    pub ball: Option<BallState>,
    pub score_left: u32,
    pub score_right: u32,
}

pub struct EntityState {
    pub x: f32,
    pub y: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub holding_ball: bool,
    pub grounded: bool,
}

pub struct BallState {
    pub x: f32,
    pub y: f32,
    pub state: String, // "Free", "Held", "InFlight"
}

/// Parse a check string into (path, operator, value)
fn parse_check(check: &str) -> Option<(&str, &str, &str)> {
    // Try operators in order of specificity (>= before >, etc.)
    for op in &[">=", "<=", "!=", "=", ">", "<"] {
        if let Some(idx) = check.find(op) {
            let path = check[..idx].trim();
            let value = check[idx + op.len()..].trim();
            return Some((path, op, value));
        }
    }
    None
}

/// Check state assertions against world state
pub fn check_state(assertion: &StateAssertion, state: &WorldState) -> Result<(), AssertionError> {
    for check in &assertion.checks {
        let (path, operator, expected_value) = parse_check(check).ok_or_else(|| AssertionError {
            message: format!("Invalid check syntax: {}", check),
            expected: "format: 'entity.property = value' or 'entity.property > value'".to_string(),
            actual: check.clone(),
        })?;

        let path_parts: Vec<&str> = path.split('.').collect();

        if path_parts.is_empty() {
            continue;
        }

        // Handle special cases
        if path_parts[0] == "score" {
            match path_parts.get(1) {
                Some(&"left") => {
                    let expected: u32 = expected_value.parse().map_err(|_| AssertionError {
                        message: format!("Invalid value for {}", path),
                        expected: "integer".to_string(),
                        actual: expected_value.to_string(),
                    })?;
                    if state.score_left != expected {
                        return Err(AssertionError {
                            message: format!("Score check failed: {}", check),
                            expected: expected_value.to_string(),
                            actual: state.score_left.to_string(),
                        });
                    }
                }
                Some(&"right") => {
                    let expected: u32 = expected_value.parse().map_err(|_| AssertionError {
                        message: format!("Invalid value for {}", path),
                        expected: "integer".to_string(),
                        actual: expected_value.to_string(),
                    })?;
                    if state.score_right != expected {
                        return Err(AssertionError {
                            message: format!("Score check failed: {}", check),
                            expected: expected_value.to_string(),
                            actual: state.score_right.to_string(),
                        });
                    }
                }
                _ => {}
            }
            continue;
        }

        if path_parts[0] == "ball" {
            let ball = state.ball.as_ref().ok_or_else(|| AssertionError {
                message: "Ball state check failed".to_string(),
                expected: "ball exists".to_string(),
                actual: "no ball".to_string(),
            })?;

            match path_parts.get(1) {
                Some(&"x") => check_float_comparison(path, ball.x, operator, expected_value)?,
                Some(&"y") => check_float_comparison(path, ball.y, operator, expected_value)?,
                Some(&"state") => {
                    let expected = expected_value.trim_matches('"');
                    if ball.state != expected {
                        return Err(AssertionError {
                            message: format!("Ball state check failed: {}", check),
                            expected: expected.to_string(),
                            actual: ball.state.clone(),
                        });
                    }
                }
                _ => {}
            }
            continue;
        }

        // Entity checks
        let entity_id = path_parts[0];
        let entity = state.entities.get(entity_id).ok_or_else(|| AssertionError {
            message: format!("Entity '{}' not found", entity_id),
            expected: format!("entity '{}'", entity_id),
            actual: format!("available: {:?}", state.entities.keys().collect::<Vec<_>>()),
        })?;

        match path_parts.get(1) {
            Some(&"x") => check_float_comparison(path, entity.x, operator, expected_value)?,
            Some(&"y") => check_float_comparison(path, entity.y, operator, expected_value)?,
            Some(&"velocity_x") => check_float_comparison(path, entity.velocity_x, operator, expected_value)?,
            Some(&"velocity_y") => check_float_comparison(path, entity.velocity_y, operator, expected_value)?,
            Some(&"holding_ball") => {
                let expected = expected_value == "true";
                if entity.holding_ball != expected {
                    return Err(AssertionError {
                        message: format!("Check failed: {}", check),
                        expected: expected_value.to_string(),
                        actual: entity.holding_ball.to_string(),
                    });
                }
            }
            Some(&"grounded") => {
                let expected = expected_value == "true";
                if entity.grounded != expected {
                    return Err(AssertionError {
                        message: format!("Check failed: {}", check),
                        expected: expected_value.to_string(),
                        actual: entity.grounded.to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    Ok(())
}

/// Check float comparison with operator
fn check_float_comparison(path: &str, actual: f32, operator: &str, expected_str: &str) -> Result<(), AssertionError> {
    let value: f32 = expected_str.trim().parse().map_err(|_| AssertionError {
        message: format!("Invalid value for {}", path),
        expected: "number".to_string(),
        actual: expected_str.to_string(),
    })?;

    let pass = match operator {
        ">=" => actual >= value,
        "<=" => actual <= value,
        ">" => actual > value,
        "<" => actual < value,
        "=" | "==" => (actual - value).abs() < 0.1,
        "!=" => (actual - value).abs() >= 0.1,
        _ => true, // Unknown operator, pass by default
    };

    if !pass {
        return Err(AssertionError {
            message: format!("Check failed: {} {} {} (actual: {:.1})", path, operator, expected_str, actual),
            expected: format!("{} {} {}", path, operator, value),
            actual: format!("{:.1}", actual),
        });
    }

    Ok(())
}
