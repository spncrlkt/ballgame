//! Assertion checking for test expectations

use super::parser::{ExpectedEvent, StateAssertion};
use crate::events::GameEvent;

/// Error when an assertion fails
#[derive(Debug, Clone)]
pub struct AssertionError {
    pub message: String,
    pub expected: String,
    pub actual: String,
}

impl std::fmt::Display for AssertionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\n    Expected: {}\n    Actual: {}",
            self.message, self.expected, self.actual
        )
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
    pub fn from_game_event(
        frame: u64,
        event: &GameEvent,
        entity_map: &std::collections::HashMap<bevy::prelude::Entity, String>,
    ) -> Option<Self> {
        // Extract event type and character ID from both legacy PlayerId and new CharacterId events
        let (event_type, character_id) = match event {
            // Prefer *2 variants (CharacterId)
            GameEvent::Pickup2 { character } => ("Pickup".to_string(), Some(*character)),
            GameEvent::Drop2 { character } => ("Drop".to_string(), Some(*character)),
            GameEvent::ShotStart2 { character, .. } => ("ShotStart".to_string(), Some(*character)),
            GameEvent::ShotRelease2 { character, .. } => ("ShotRelease".to_string(), Some(*character)),
            GameEvent::StealAttempt2 { attacker } => ("StealAttempt".to_string(), Some(*attacker)),
            GameEvent::StealSuccess2 { attacker } => ("StealSuccess".to_string(), Some(*attacker)),
            GameEvent::StealFail2 { attacker } => ("StealFail".to_string(), Some(*attacker)),
            GameEvent::StealOutOfRange2 { attacker } => ("StealOutOfRange".to_string(), Some(*attacker)),
            GameEvent::Goal2 { character, .. } => ("Goal".to_string(), Some(*character)),
            // Fall back to legacy PlayerId events (convert to CharacterId)
            GameEvent::Pickup { player } => ("Pickup".to_string(), Some(player.to_character_id())),
            GameEvent::Drop { player } => ("Drop".to_string(), Some(player.to_character_id())),
            GameEvent::ShotStart { player, .. } => ("ShotStart".to_string(), Some(player.to_character_id())),
            GameEvent::ShotRelease { player, .. } => ("ShotRelease".to_string(), Some(player.to_character_id())),
            GameEvent::StealAttempt { attacker } => ("StealAttempt".to_string(), Some(attacker.to_character_id())),
            GameEvent::StealSuccess { attacker } => ("StealSuccess".to_string(), Some(attacker.to_character_id())),
            GameEvent::StealFail { attacker } => ("StealFail".to_string(), Some(attacker.to_character_id())),
            GameEvent::StealOutOfRange { attacker } => ("StealOutOfRange".to_string(), Some(attacker.to_character_id())),
            GameEvent::Goal { player, .. } => ("Goal".to_string(), Some(player.to_character_id())),
            _ => return None,
        };

        let player = character_id.and_then(|c| {
            // Map CharacterId to entity ID string based on team
            match c.team() {
                crate::events::TeamId::Left => entity_map
                    .iter()
                    .find(|(_, id)| id.contains("left") || **id == "attacker" || **id == "p1")
                    .map(|(_, id)| id.clone()),
                crate::events::TeamId::Right => entity_map
                    .iter()
                    .find(|(_, id)| id.contains("right") || **id == "victim" || **id == "p2")
                    .map(|(_, id)| id.clone()),
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
pub fn check_sequence(
    expected: &[ExpectedEvent],
    captured: &[CapturedEvent],
) -> Result<(), AssertionError> {
    let mut captured_idx = 0;

    for (i, exp) in expected.iter().enumerate() {
        // Find matching event starting from current position
        let found = captured[captured_idx..]
            .iter()
            .enumerate()
            .find(|(_, cap)| {
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
                let player_str = exp
                    .player
                    .as_ref()
                    .map(|p| format!(" (player: {})", p))
                    .unwrap_or_default();
                return Err(AssertionError {
                    message: format!("Event #{} '{}'{} not found", i + 1, exp.event, player_str),
                    expected: format!("'{}' event in sequence", exp.event),
                    actual: format!(
                        "events after position {}: {:?}",
                        captured_idx,
                        captured[captured_idx..]
                            .iter()
                            .map(|e| &e.event_type)
                            .collect::<Vec<_>>()
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
    pub velocity_x: f32,
    pub velocity_y: f32,
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
        let (path, operator, expected_value) =
            parse_check(check).ok_or_else(|| AssertionError {
                message: format!("Invalid check syntax: {}", check),
                expected: "format: 'entity.property = value' or 'entity.property > value'"
                    .to_string(),
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
                Some(&"velocity_x") => {
                    check_float_comparison(path, ball.velocity_x, operator, expected_value)?
                }
                Some(&"velocity_y") => {
                    check_float_comparison(path, ball.velocity_y, operator, expected_value)?
                }
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
        let entity = state
            .entities
            .get(entity_id)
            .ok_or_else(|| AssertionError {
                message: format!("Entity '{}' not found", entity_id),
                expected: format!("entity '{}'", entity_id),
                actual: format!("available: {:?}", state.entities.keys().collect::<Vec<_>>()),
            })?;

        match path_parts.get(1) {
            Some(&"x") => check_float_comparison(path, entity.x, operator, expected_value)?,
            Some(&"y") => check_float_comparison(path, entity.y, operator, expected_value)?,
            Some(&"velocity_x") => {
                check_float_comparison(path, entity.velocity_x, operator, expected_value)?
            }
            Some(&"velocity_y") => {
                check_float_comparison(path, entity.velocity_y, operator, expected_value)?
            }
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
fn check_float_comparison(
    path: &str,
    actual: f32,
    operator: &str,
    expected_str: &str,
) -> Result<(), AssertionError> {
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
            message: format!(
                "Check failed: {} {} {} (actual: {:.1})",
                path, operator, expected_str, actual
            ),
            expected: format!("{} {} {}", path, operator, value),
            actual: format!("{:.1}", actual),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{CharacterId, GameEvent};
    #[allow(deprecated)]
    use crate::events::PlayerId;
    use bevy::prelude::Entity;
    use std::collections::HashMap;

    fn create_entity_map() -> HashMap<Entity, String> {
        let mut map = HashMap::new();
        // Create fake entities with incrementing indices
        map.insert(Entity::from_raw_u32(1).expect("valid entity"), "left".to_string());
        map.insert(Entity::from_raw_u32(2).expect("valid entity"), "right".to_string());
        map.insert(Entity::from_raw_u32(3).expect("valid entity"), "attacker".to_string());
        map.insert(Entity::from_raw_u32(4).expect("valid entity"), "victim".to_string());
        map
    }

    // === CapturedEvent::from_game_event tests ===

    #[test]
    fn test_captured_event_from_pickup2() {
        let entity_map = create_entity_map();
        let event = GameEvent::Pickup2 {
            character: CharacterId::L0,
        };
        let captured = CapturedEvent::from_game_event(100, &event, &entity_map);
        assert!(captured.is_some());
        let captured = captured.unwrap();
        assert_eq!(captured.frame, 100);
        assert_eq!(captured.event_type, "Pickup");
        // Should map to left team entity
        assert!(captured.player.is_some());
    }

    #[test]
    fn test_captured_event_from_goal2_all_characters() {
        let entity_map = create_entity_map();
        for character in CharacterId::all() {
            let event = GameEvent::Goal2 {
                character,
                score_left: 1,
                score_right: 0,
            };
            let captured = CapturedEvent::from_game_event(200, &event, &entity_map);
            assert!(captured.is_some());
            let captured = captured.unwrap();
            assert_eq!(captured.event_type, "Goal");
        }
    }

    #[test]
    fn test_captured_event_from_steal_events_2v2() {
        let entity_map = create_entity_map();

        // StealAttempt2
        let event = GameEvent::StealAttempt2 { attacker: CharacterId::R0 };
        let captured = CapturedEvent::from_game_event(300, &event, &entity_map).unwrap();
        assert_eq!(captured.event_type, "StealAttempt");

        // StealSuccess2
        let event = GameEvent::StealSuccess2 { attacker: CharacterId::L1 };
        let captured = CapturedEvent::from_game_event(400, &event, &entity_map).unwrap();
        assert_eq!(captured.event_type, "StealSuccess");

        // StealFail2
        let event = GameEvent::StealFail2 { attacker: CharacterId::R1 };
        let captured = CapturedEvent::from_game_event(500, &event, &entity_map).unwrap();
        assert_eq!(captured.event_type, "StealFail");

        // StealOutOfRange2
        let event = GameEvent::StealOutOfRange2 { attacker: CharacterId::L0 };
        let captured = CapturedEvent::from_game_event(600, &event, &entity_map).unwrap();
        assert_eq!(captured.event_type, "StealOutOfRange");
    }

    #[test]
    fn test_captured_event_from_shot_events_2v2() {
        let entity_map = create_entity_map();

        // ShotStart2
        let event = GameEvent::ShotStart2 {
            character: CharacterId::L0,
            pos: (-100.0, -350.0),
            quality: 0.8,
        };
        let captured = CapturedEvent::from_game_event(700, &event, &entity_map).unwrap();
        assert_eq!(captured.event_type, "ShotStart");

        // ShotRelease2
        let event = GameEvent::ShotRelease2 {
            character: CharacterId::R1,
            charge: 0.75,
            angle: 45.0,
            power: 600.0,
        };
        let captured = CapturedEvent::from_game_event(800, &event, &entity_map).unwrap();
        assert_eq!(captured.event_type, "ShotRelease");
    }

    #[test]
    #[allow(deprecated)]
    fn test_captured_event_from_legacy_pickup() {
        let entity_map = create_entity_map();
        let event = GameEvent::Pickup { player: PlayerId::L };
        let captured = CapturedEvent::from_game_event(900, &event, &entity_map);
        assert!(captured.is_some());
        let captured = captured.unwrap();
        assert_eq!(captured.event_type, "Pickup");
    }

    #[test]
    #[allow(deprecated)]
    fn test_captured_event_from_legacy_goal() {
        let entity_map = create_entity_map();
        let event = GameEvent::Goal {
            player: PlayerId::R,
            score_left: 0,
            score_right: 1,
        };
        let captured = CapturedEvent::from_game_event(1000, &event, &entity_map).unwrap();
        assert_eq!(captured.event_type, "Goal");
    }

    #[test]
    fn test_captured_event_unsupported_events() {
        let entity_map = create_entity_map();

        // Events that don't have player/character should return None
        let event = GameEvent::ResetScores;
        assert!(CapturedEvent::from_game_event(0, &event, &entity_map).is_none());

        let event = GameEvent::ResetBall;
        assert!(CapturedEvent::from_game_event(0, &event, &entity_map).is_none());

        let event = GameEvent::LevelChange {
            level_id: "test".to_string(),
        };
        assert!(CapturedEvent::from_game_event(0, &event, &entity_map).is_none());
    }

    // === check_sequence tests ===

    #[test]
    fn test_check_sequence_simple() {
        let expected = vec![
            ExpectedEvent {
                event: "Pickup".to_string(),
                player: None,
                frame_min: None,
                frame_max: None,
                tolerance: 5,
            },
            ExpectedEvent {
                event: "Goal".to_string(),
                player: None,
                frame_min: None,
                frame_max: None,
                tolerance: 5,
            },
        ];

        let captured = vec![
            CapturedEvent {
                frame: 100,
                event_type: "Pickup".to_string(),
                player: Some("left".to_string()),
            },
            CapturedEvent {
                frame: 200,
                event_type: "Goal".to_string(),
                player: Some("left".to_string()),
            },
        ];

        assert!(check_sequence(&expected, &captured).is_ok());
    }

    #[test]
    fn test_check_sequence_with_player_filter() {
        let expected = vec![ExpectedEvent {
            event: "Pickup".to_string(),
            player: Some("left".to_string()),
            frame_min: None,
            frame_max: None,
            tolerance: 5,
        }];

        let captured = vec![
            CapturedEvent {
                frame: 50,
                event_type: "Pickup".to_string(),
                player: Some("right".to_string()),
            },
            CapturedEvent {
                frame: 100,
                event_type: "Pickup".to_string(),
                player: Some("left".to_string()),
            },
        ];

        assert!(check_sequence(&expected, &captured).is_ok());
    }

    #[test]
    fn test_check_sequence_with_frame_bounds() {
        let expected = vec![ExpectedEvent {
            event: "Goal".to_string(),
            player: None,
            frame_min: Some(50),
            frame_max: Some(150),
            tolerance: 5,
        }];

        // Within bounds
        let captured = vec![CapturedEvent {
            frame: 100,
            event_type: "Goal".to_string(),
            player: None,
        }];
        assert!(check_sequence(&expected, &captured).is_ok());

        // Too early
        let captured_early = vec![CapturedEvent {
            frame: 30,
            event_type: "Goal".to_string(),
            player: None,
        }];
        assert!(check_sequence(&expected, &captured_early).is_err());

        // Too late
        let captured_late = vec![CapturedEvent {
            frame: 200,
            event_type: "Goal".to_string(),
            player: None,
        }];
        assert!(check_sequence(&expected, &captured_late).is_err());
    }

    #[test]
    fn test_check_sequence_missing_event() {
        let expected = vec![ExpectedEvent {
            event: "Goal".to_string(),
            player: None,
            frame_min: None,
            frame_max: None,
            tolerance: 5,
        }];

        let captured = vec![CapturedEvent {
            frame: 100,
            event_type: "Pickup".to_string(),
            player: None,
        }];

        let result = check_sequence(&expected, &captured);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("not found"));
    }

    #[test]
    fn test_check_sequence_multiple_events() {
        let expected = vec![
            ExpectedEvent {
                event: "Pickup".to_string(),
                player: Some("left".to_string()),
                frame_min: None,
                frame_max: None,
                tolerance: 5,
            },
            ExpectedEvent {
                event: "ShotStart".to_string(),
                player: Some("left".to_string()),
                frame_min: None,
                frame_max: None,
                tolerance: 5,
            },
            ExpectedEvent {
                event: "ShotRelease".to_string(),
                player: Some("left".to_string()),
                frame_min: None,
                frame_max: None,
                tolerance: 5,
            },
            ExpectedEvent {
                event: "Goal".to_string(),
                player: Some("left".to_string()),
                frame_min: None,
                frame_max: None,
                tolerance: 5,
            },
        ];

        let captured = vec![
            CapturedEvent {
                frame: 100,
                event_type: "Pickup".to_string(),
                player: Some("left".to_string()),
            },
            CapturedEvent {
                frame: 150,
                event_type: "ShotStart".to_string(),
                player: Some("left".to_string()),
            },
            CapturedEvent {
                frame: 200,
                event_type: "ShotRelease".to_string(),
                player: Some("left".to_string()),
            },
            CapturedEvent {
                frame: 300,
                event_type: "Goal".to_string(),
                player: Some("left".to_string()),
            },
        ];

        assert!(check_sequence(&expected, &captured).is_ok());
    }

    // === check_state tests ===

    #[test]
    fn test_check_state_score() {
        let state = WorldState {
            entities: HashMap::new(),
            ball: None,
            score_left: 3,
            score_right: 2,
        };

        let assertion = StateAssertion {
            after_frame: 0,
            checks: vec!["score.left = 3".to_string(), "score.right = 2".to_string()],
        };

        assert!(check_state(&assertion, &state).is_ok());

        let assertion_fail = StateAssertion {
            after_frame: 0,
            checks: vec!["score.left = 5".to_string()],
        };
        assert!(check_state(&assertion_fail, &state).is_err());
    }

    #[test]
    fn test_check_state_entity_position() {
        let mut entities = HashMap::new();
        entities.insert(
            "player1".to_string(),
            EntityState {
                x: 100.0,
                y: -350.0,
                velocity_x: 10.0,
                velocity_y: 0.0,
                holding_ball: false,
                grounded: true,
            },
        );

        let state = WorldState {
            entities,
            ball: None,
            score_left: 0,
            score_right: 0,
        };

        let assertion = StateAssertion {
            after_frame: 0,
            checks: vec![
                "player1.x >= 50".to_string(),
                "player1.y < -300".to_string(),
                "player1.grounded = true".to_string(),
            ],
        };

        assert!(check_state(&assertion, &state).is_ok());
    }

    #[test]
    fn test_check_state_ball() {
        let state = WorldState {
            entities: HashMap::new(),
            ball: Some(BallState {
                x: 0.0,
                y: 50.0,
                velocity_x: 100.0,
                velocity_y: -200.0,
                state: "InFlight".to_string(),
            }),
            score_left: 0,
            score_right: 0,
        };

        let assertion = StateAssertion {
            after_frame: 0,
            checks: vec![
                "ball.x = 0".to_string(),
                "ball.velocity_y < 0".to_string(),
                "ball.state = InFlight".to_string(),
            ],
        };

        assert!(check_state(&assertion, &state).is_ok());
    }

    #[test]
    fn test_check_state_comparisons() {
        let mut entities = HashMap::new();
        entities.insert(
            "test".to_string(),
            EntityState {
                x: 100.0,
                y: 200.0,
                velocity_x: 50.0,
                velocity_y: -30.0,
                holding_ball: true,
                grounded: false,
            },
        );

        let state = WorldState {
            entities,
            ball: None,
            score_left: 0,
            score_right: 0,
        };

        // Test all operators
        let checks = vec![
            ("test.x >= 100", true),
            ("test.x > 99", true),
            ("test.x <= 100", true),
            ("test.x < 101", true),
            ("test.x = 100", true),
            ("test.x != 50", true),
            ("test.holding_ball = true", true),
            ("test.grounded = false", true),
        ];

        for (check, should_pass) in checks {
            let assertion = StateAssertion {
                after_frame: 0,
                checks: vec![check.to_string()],
            };
            let result = check_state(&assertion, &state);
            assert_eq!(result.is_ok(), should_pass, "Check '{}' should {}pass", check, if should_pass { "" } else { "not " });
        }
    }

    #[test]
    fn test_parse_check() {
        assert_eq!(parse_check("x >= 10"), Some(("x", ">=", "10")));
        assert_eq!(parse_check("y <= -5"), Some(("y", "<=", "-5")));
        assert_eq!(parse_check("value = 100"), Some(("value", "=", "100")));
        assert_eq!(parse_check("score.left != 0"), Some(("score.left", "!=", "0")));
        assert_eq!(parse_check("pos > 50"), Some(("pos", ">", "50")));
        assert_eq!(parse_check("pos < 50"), Some(("pos", "<", "50")));
        assert_eq!(parse_check("invalid"), None);
    }
}
