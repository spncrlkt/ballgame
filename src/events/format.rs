//! Compact text format for game event serialization
//!
//! Format: `T:NNNNN|CODE|data...`
//! - T:NNNNN = timestamp in milliseconds (5 digits, wraps at 99999)
//! - CODE = 2-char event type code
//! - data = pipe-separated values specific to event type
//!
//! Examples:
//! ```text
//! T:00000|MS|1|Open Floor|Balanced|Balanced|12345678
//! T:00150|PU|L
//! T:00320|SS|L|-200.5,-418.2|0.47
//! T:00850|SR|L|0.65|62.3|720.5
//! T:01200|G|L|1|0
//! T:01500|ME|1|0|45.5
//! ```
//!
//! Tick events (sampled every 50ms / 20 Hz):
//! ```text
//! T:00050|T|1|-200.5,-418.2|50.0,0.0|300.2,-418.2|-30.0,0.0|0.0,50.5|0.0,-200.0|F
//!          ^frame|left_pos|left_vel|right_pos|right_vel|ball_pos|ball_vel|state
//! ```

use super::types::{CharacterId, CharacterTickData, ControllerSource, GameConfig, GameEvent, PlayerId};

/// Format a float with fixed precision (1 decimal)
fn fmt_f1(v: f32) -> String {
    format!("{:.1}", v)
}

/// Format a position tuple
fn fmt_pos(pos: (f32, f32)) -> String {
    format!("{:.1},{:.1}", pos.0, pos.1)
}

/// Serialize a GameEvent to compact text format
pub fn serialize_event(time_ms: u32, event: &GameEvent) -> String {
    let ts = format!("T:{:05}", time_ms % 100000);
    let code = event.type_code();

    let data = match event {
        GameEvent::SessionStart {
            session_id,
            timestamp,
        } => {
            format!("{}|{}", session_id, timestamp)
        }
        GameEvent::Config(config) => {
            // Serialize config as compact JSON for easy parsing
            serde_json::to_string(config).unwrap_or_else(|_| "{}".to_string())
        }
        GameEvent::MatchStart {
            level,
            level_name,
            left_profile,
            right_profile,
            seed,
        } => {
            format!(
                "{}|{}|{}|{}|{}",
                level, level_name, left_profile, right_profile, seed
            )
        }
        GameEvent::MatchEnd {
            score_left,
            score_right,
            duration,
        } => {
            format!("{}|{}|{}", score_left, score_right, fmt_f1(*duration))
        }
        GameEvent::Goal {
            player,
            score_left,
            score_right,
        } => {
            format!("{}|{}|{}", player, score_left, score_right)
        }
        GameEvent::Pickup { player } => player.to_string(),
        GameEvent::Drop { player } => player.to_string(),
        GameEvent::ShotStart {
            player,
            pos,
            quality,
        } => {
            format!("{}|{}|{:.2}", player, fmt_pos(*pos), quality)
        }
        GameEvent::ShotRelease {
            player,
            charge,
            angle,
            power,
        } => {
            format!("{}|{:.2}|{:.1}|{:.1}", player, charge, angle, power)
        }
        GameEvent::StealAttempt { attacker } => attacker.to_string(),
        GameEvent::StealSuccess { attacker } => attacker.to_string(),
        GameEvent::StealFail { attacker } => attacker.to_string(),
        GameEvent::StealOutOfRange { attacker } => attacker.to_string(),
        GameEvent::Jump { player } => player.to_string(),
        GameEvent::Land { player } => player.to_string(),
        GameEvent::AiGoal { player, goal } => {
            format!("{}|{}", player, goal)
        }
        GameEvent::NavStart { player, target } => {
            format!("{}|{}", player, fmt_pos(*target))
        }
        GameEvent::NavComplete { player } => player.to_string(),
        GameEvent::Input {
            player,
            move_x,
            jump,
            throw,
            pickup,
        } => {
            // Compact input encoding: player|move_x|flags
            // flags: J=jump, T=throw, P=pickup
            let mut flags = String::new();
            if *jump {
                flags.push('J');
            }
            if *throw {
                flags.push('T');
            }
            if *pickup {
                flags.push('P');
            }
            if flags.is_empty() {
                flags.push('-');
            }
            format!("{}|{:.1}|{}", player, move_x, flags)
        }
        GameEvent::Tick {
            frame,
            left_pos,
            left_vel,
            right_pos,
            right_vel,
            ball_pos,
            ball_vel,
            ball_state,
        } => {
            format!(
                "{}|{}|{}|{}|{}|{}|{}|{}",
                frame,
                fmt_pos(*left_pos),
                fmt_pos(*left_vel),
                fmt_pos(*right_pos),
                fmt_pos(*right_vel),
                fmt_pos(*ball_pos),
                fmt_pos(*ball_vel),
                ball_state
            )
        }
        GameEvent::ControllerInput {
            player,
            source,
            move_x,
            jump,
            jump_pressed,
            throw,
            throw_released,
            pickup,
        } => {
            // Compact encoding: player|source|move_x|jump|jump_pressed|throw|throw_released|pickup
            format!(
                "{}|{}|{:.2}|{}|{}|{}|{}|{}",
                player,
                source,
                move_x,
                if *jump { 1 } else { 0 },
                if *jump_pressed { 1 } else { 0 },
                if *throw { 1 } else { 0 },
                if *throw_released { 1 } else { 0 },
                if *pickup { 1 } else { 0 }
            )
        }
        GameEvent::ControlSwap {
            from_player,
            to_player,
        } => {
            let from = from_player
                .map(|p| p.to_string())
                .unwrap_or_else(|| "_".to_string());
            let to = to_player
                .map(|p| p.to_string())
                .unwrap_or_else(|| "_".to_string());
            format!("{}|{}", from, to)
        }
        GameEvent::ResetAiState { player } => player.to_string(),
        GameEvent::ResetScores => String::new(),
        GameEvent::ResetBall => String::new(),
        GameEvent::LevelChange { level_id } => level_id.clone(),
        // === 2v2 Events with CharacterId ===
        GameEvent::Goal2 {
            character,
            score_left,
            score_right,
        } => {
            format!("{}|{}|{}", character, score_left, score_right)
        }
        GameEvent::Pickup2 { character } => character.to_string(),
        GameEvent::Drop2 { character } => character.to_string(),
        GameEvent::ShotStart2 {
            character,
            pos,
            quality,
        } => {
            format!("{}|{}|{:.2}", character, fmt_pos(*pos), quality)
        }
        GameEvent::ShotRelease2 {
            character,
            charge,
            angle,
            power,
        } => {
            format!("{}|{:.2}|{:.1}|{:.1}", character, charge, angle, power)
        }
        GameEvent::Pass { from, to } => {
            format!("{}|{}", from, to)
        }
        GameEvent::StealAttempt2 { attacker } => attacker.to_string(),
        GameEvent::StealSuccess2 { attacker } => attacker.to_string(),
        GameEvent::StealFail2 { attacker } => attacker.to_string(),
        GameEvent::StealOutOfRange2 { attacker } => attacker.to_string(),
        GameEvent::Jump2 { character } => character.to_string(),
        GameEvent::Land2 { character } => character.to_string(),
        GameEvent::AiGoal2 { character, goal } => {
            format!("{}|{}", character, goal)
        }
        GameEvent::NavStart2 { character, target } => {
            format!("{}|{}", character, fmt_pos(*target))
        }
        GameEvent::NavComplete2 { character } => character.to_string(),
        GameEvent::Input2 {
            character,
            source,
            move_x,
            jump,
            throw,
            pickup,
            pass,
        } => {
            let mut flags = String::new();
            if *jump {
                flags.push('J');
            }
            if *throw {
                flags.push('T');
            }
            if *pickup {
                flags.push('P');
            }
            if *pass {
                flags.push('X'); // X for pass to avoid conflict
            }
            if flags.is_empty() {
                flags.push('-');
            }
            format!("{}|{}|{:.1}|{}", character, source, move_x, flags)
        }
        GameEvent::Tick2 {
            frame,
            characters,
            ball_pos,
            ball_vel,
            ball_state,
        } => {
            // Format: frame|char_count|c1_id:pos,vel,ctrl|c2_id:...|ball_pos|ball_vel|state
            let char_data: Vec<String> = characters
                .iter()
                .map(|c| {
                    format!(
                        "{}:{},{},{},{},{}",
                        c.id, c.pos.0, c.pos.1, c.vel.0, c.vel.1, c.controller
                    )
                })
                .collect();
            format!(
                "{}|{}|{}|{}|{}|{}",
                frame,
                characters.len(),
                char_data.join("|"),
                fmt_pos(*ball_pos),
                fmt_pos(*ball_vel),
                ball_state
            )
        }
        GameEvent::ControllerInput2 {
            character,
            source_id,
            move_x,
            jump,
            jump_pressed,
            throw,
            throw_released,
            pickup,
            pass,
        } => {
            format!(
                "{}|{}|{:.2}|{}|{}|{}|{}|{}|{}",
                character,
                source_id,
                move_x,
                if *jump { 1 } else { 0 },
                if *jump_pressed { 1 } else { 0 },
                if *throw { 1 } else { 0 },
                if *throw_released { 1 } else { 0 },
                if *pickup { 1 } else { 0 },
                if *pass { 1 } else { 0 }
            )
        }
        GameEvent::ControllerAssign {
            character,
            source_id,
            descriptor,
        } => {
            format!("{}|{}|{}", character, source_id, descriptor)
        }
        GameEvent::ControllerSwap2 {
            character,
            old_source,
            new_source,
        } => {
            format!("{}|{}|{}", character, old_source, new_source)
        }
        GameEvent::ResetAiState2 { character } => character.to_string(),
    };

    format!("{}|{}|{}", ts, code, data)
}

/// Parse a line back into timestamp and event (optional, for replay)
pub fn parse_event(line: &str) -> Option<(u32, GameEvent)> {
    let parts: Vec<&str> = line.split('|').collect();
    if parts.len() < 3 {
        return None;
    }

    // Parse timestamp
    let ts_str = parts[0].strip_prefix("T:")?;
    let time_ms: u32 = ts_str.parse().ok()?;

    let code = parts[1];
    let data = &parts[2..];

    let event = match code {
        "SE" if data.len() >= 2 => GameEvent::SessionStart {
            session_id: data[0].to_string(),
            timestamp: data[1].to_string(),
        },
        "CF" if !data.is_empty() => {
            // Config is serialized as JSON, rejoin with | in case JSON contains |
            let json_str = data.join("|");
            let config: GameConfig = serde_json::from_str(&json_str).ok()?;
            GameEvent::Config(config)
        }
        "MS" if data.len() >= 5 => GameEvent::MatchStart {
            level: data[0].parse().ok()?,
            level_name: data[1].to_string(),
            left_profile: data[2].to_string(),
            right_profile: data[3].to_string(),
            seed: data[4].parse().ok()?,
        },
        "ME" if data.len() >= 3 => GameEvent::MatchEnd {
            score_left: data[0].parse().ok()?,
            score_right: data[1].parse().ok()?,
            duration: data[2].parse().ok()?,
        },
        "G" if data.len() >= 3 => GameEvent::Goal {
            player: parse_player(data[0])?,
            score_left: data[1].parse().ok()?,
            score_right: data[2].parse().ok()?,
        },
        "PU" if !data.is_empty() => GameEvent::Pickup {
            player: parse_player(data[0])?,
        },
        "DR" if !data.is_empty() => GameEvent::Drop {
            player: parse_player(data[0])?,
        },
        "SS" if data.len() >= 3 => GameEvent::ShotStart {
            player: parse_player(data[0])?,
            pos: parse_pos(data[1])?,
            quality: data[2].parse().ok()?,
        },
        "SR" if data.len() >= 4 => GameEvent::ShotRelease {
            player: parse_player(data[0])?,
            charge: data[1].parse().ok()?,
            angle: data[2].parse().ok()?,
            power: data[3].parse().ok()?,
        },
        "SA" if !data.is_empty() => GameEvent::StealAttempt {
            attacker: parse_player(data[0])?,
        },
        "S+" if !data.is_empty() => GameEvent::StealSuccess {
            attacker: parse_player(data[0])?,
        },
        "S-" if !data.is_empty() => GameEvent::StealFail {
            attacker: parse_player(data[0])?,
        },
        "SO" if !data.is_empty() => GameEvent::StealOutOfRange {
            attacker: parse_player(data[0])?,
        },
        "J" if !data.is_empty() => GameEvent::Jump {
            player: parse_player(data[0])?,
        },
        "LD" if !data.is_empty() => GameEvent::Land {
            player: parse_player(data[0])?,
        },
        "AG" if data.len() >= 2 => GameEvent::AiGoal {
            player: parse_player(data[0])?,
            goal: data[1].to_string(),
        },
        "NS" if data.len() >= 2 => GameEvent::NavStart {
            player: parse_player(data[0])?,
            target: parse_pos(data[1])?,
        },
        "NC" if !data.is_empty() => GameEvent::NavComplete {
            player: parse_player(data[0])?,
        },
        "I" if data.len() >= 3 => GameEvent::Input {
            player: parse_player(data[0])?,
            move_x: data[1].parse().ok()?,
            jump: data[2].contains('J'),
            throw: data[2].contains('T'),
            pickup: data[2].contains('P'),
        },
        "T" if data.len() >= 8 => GameEvent::Tick {
            frame: data[0].parse().ok()?,
            left_pos: parse_pos(data[1])?,
            left_vel: parse_pos(data[2])?,
            right_pos: parse_pos(data[3])?,
            right_vel: parse_pos(data[4])?,
            ball_pos: parse_pos(data[5])?,
            ball_vel: parse_pos(data[6])?,
            ball_state: data[7].chars().next()?,
        },
        "CI" if data.len() >= 8 => GameEvent::ControllerInput {
            player: parse_player(data[0])?,
            source: parse_source(data[1])?,
            move_x: data[2].parse().ok()?,
            jump: data[3] == "1",
            jump_pressed: data[4] == "1",
            throw: data[5] == "1",
            throw_released: data[6] == "1",
            pickup: data[7] == "1",
        },
        "CS" if data.len() >= 2 => GameEvent::ControlSwap {
            from_player: if data[0] == "_" {
                None
            } else {
                parse_player(data[0])
            },
            to_player: if data[1] == "_" {
                None
            } else {
                parse_player(data[1])
            },
        },
        "RA" if !data.is_empty() => GameEvent::ResetAiState {
            player: parse_player(data[0])?,
        },
        "RS" => GameEvent::ResetScores,
        "RB" => GameEvent::ResetBall,
        "LC" if !data.is_empty() => GameEvent::LevelChange {
            level_id: data[0].to_string(),
        },
        // === 2v2 Event Parsing ===
        "G2" if data.len() >= 3 => GameEvent::Goal2 {
            character: parse_character(data[0])?,
            score_left: data[1].parse().ok()?,
            score_right: data[2].parse().ok()?,
        },
        "P2" if !data.is_empty() => GameEvent::Pickup2 {
            character: parse_character(data[0])?,
        },
        "D2" if !data.is_empty() => GameEvent::Drop2 {
            character: parse_character(data[0])?,
        },
        "S2" if data.len() >= 3 => GameEvent::ShotStart2 {
            character: parse_character(data[0])?,
            pos: parse_pos(data[1])?,
            quality: data[2].parse().ok()?,
        },
        "R2" if data.len() >= 4 => GameEvent::ShotRelease2 {
            character: parse_character(data[0])?,
            charge: data[1].parse().ok()?,
            angle: data[2].parse().ok()?,
            power: data[3].parse().ok()?,
        },
        "PA" if data.len() >= 2 => GameEvent::Pass {
            from: parse_character(data[0])?,
            to: parse_character(data[1])?,
        },
        "A2" if !data.is_empty() => GameEvent::StealAttempt2 {
            attacker: parse_character(data[0])?,
        },
        "+2" if !data.is_empty() => GameEvent::StealSuccess2 {
            attacker: parse_character(data[0])?,
        },
        "-2" if !data.is_empty() => GameEvent::StealFail2 {
            attacker: parse_character(data[0])?,
        },
        "O2" if !data.is_empty() => GameEvent::StealOutOfRange2 {
            attacker: parse_character(data[0])?,
        },
        "J2" if !data.is_empty() => GameEvent::Jump2 {
            character: parse_character(data[0])?,
        },
        "L2" if !data.is_empty() => GameEvent::Land2 {
            character: parse_character(data[0])?,
        },
        "AI" if data.len() >= 2 => GameEvent::AiGoal2 {
            character: parse_character(data[0])?,
            goal: data[1].to_string(),
        },
        "N2" if data.len() >= 2 => GameEvent::NavStart2 {
            character: parse_character(data[0])?,
            target: parse_pos(data[1])?,
        },
        "C2" if !data.is_empty() => GameEvent::NavComplete2 {
            character: parse_character(data[0])?,
        },
        "I2" if data.len() >= 4 => GameEvent::Input2 {
            character: parse_character(data[0])?,
            source: data[1].parse().ok()?,
            move_x: data[2].parse().ok()?,
            jump: data[3].contains('J'),
            throw: data[3].contains('T'),
            pickup: data[3].contains('P'),
            pass: data[3].contains('X'),
        },
        "T2" if data.len() >= 3 => {
            let frame: u64 = data[0].parse().ok()?;
            let char_count: usize = data[1].parse().ok()?;
            // Parse character data starting at data[2] for char_count entries
            let mut characters = Vec::with_capacity(char_count);
            for i in 0..char_count {
                if data.len() <= 2 + i {
                    return None;
                }
                let char_str = data[2 + i];
                // Format: id:x,y,vx,vy,ctrl
                let parts: Vec<&str> = char_str.split(':').collect();
                if parts.len() != 2 {
                    return None;
                }
                let id = parse_character(parts[0])?;
                let nums: Vec<&str> = parts[1].split(',').collect();
                if nums.len() != 5 {
                    return None;
                }
                characters.push(CharacterTickData {
                    id,
                    pos: (nums[0].parse().ok()?, nums[1].parse().ok()?),
                    vel: (nums[2].parse().ok()?, nums[3].parse().ok()?),
                    controller: nums[4].parse().ok()?,
                });
            }
            // After characters, we have ball_pos|ball_vel|ball_state
            let ball_idx = 2 + char_count;
            if data.len() < ball_idx + 3 {
                return None;
            }
            GameEvent::Tick2 {
                frame,
                characters,
                ball_pos: parse_pos(data[ball_idx])?,
                ball_vel: parse_pos(data[ball_idx + 1])?,
                ball_state: data[ball_idx + 2].chars().next()?,
            }
        }
        "CA" if data.len() >= 3 => GameEvent::ControllerAssign {
            character: parse_character(data[0])?,
            source_id: data[1].parse().ok()?,
            descriptor: data[2].to_string(),
        },
        "W2" if data.len() >= 3 => GameEvent::ControllerSwap2 {
            character: parse_character(data[0])?,
            old_source: data[1].parse().ok()?,
            new_source: data[2].parse().ok()?,
        },
        "X2" if data.len() >= 9 => GameEvent::ControllerInput2 {
            character: parse_character(data[0])?,
            source_id: data[1].parse().ok()?,
            move_x: data[2].parse().ok()?,
            jump: data[3] == "1",
            jump_pressed: data[4] == "1",
            throw: data[5] == "1",
            throw_released: data[6] == "1",
            pickup: data[7] == "1",
            pass: data[8] == "1",
        },
        "Z2" if !data.is_empty() => GameEvent::ResetAiState2 {
            character: parse_character(data[0])?,
        },
        _ => return None,
    };

    Some((time_ms, event))
}

fn parse_player(s: &str) -> Option<PlayerId> {
    match s {
        "L" => Some(PlayerId::L),
        "R" => Some(PlayerId::R),
        _ => None,
    }
}

fn parse_character(s: &str) -> Option<CharacterId> {
    CharacterId::from_str(s)
}

fn parse_source(s: &str) -> Option<ControllerSource> {
    match s {
        "H" => Some(ControllerSource::Human),
        "A" => Some(ControllerSource::Ai),
        "X" => Some(ControllerSource::External),
        _ => None,
    }
}

fn parse_pos(s: &str) -> Option<(f32, f32)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return None;
    }
    Some((parts[0].parse().ok()?, parts[1].parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Legacy PlayerId Tests (backward compatibility) ===

    #[test]
    #[allow(deprecated)]
    fn test_roundtrip_goal() {
        let event = GameEvent::Goal {
            player: PlayerId::L,
            score_left: 1,
            score_right: 0,
        };
        let line = serialize_event(1500, &event);
        let (ts, parsed) = parse_event(&line).unwrap();
        assert_eq!(ts, 1500);
        if let GameEvent::Goal {
            player,
            score_left,
            score_right,
        } = parsed
        {
            assert_eq!(player, PlayerId::L);
            assert_eq!(score_left, 1);
            assert_eq!(score_right, 0);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    #[allow(deprecated)]
    fn test_roundtrip_shot() {
        let event = GameEvent::ShotRelease {
            player: PlayerId::R,
            charge: 0.75,
            angle: 62.5,
            power: 720.0,
        };
        let line = serialize_event(850, &event);
        let (ts, parsed) = parse_event(&line).unwrap();
        assert_eq!(ts, 850);
        if let GameEvent::ShotRelease { player, charge, .. } = parsed {
            assert_eq!(player, PlayerId::R);
            assert!((charge - 0.75).abs() < 0.01);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    #[allow(deprecated)]
    fn test_roundtrip_tick() {
        let event = GameEvent::Tick {
            frame: 150,
            left_pos: (-200.5, -418.2),
            left_vel: (50.0, 0.0),
            right_pos: (300.2, -418.2),
            right_vel: (-30.0, 0.0),
            ball_pos: (0.0, 50.5),
            ball_vel: (0.0, -200.0),
            ball_state: 'F',
        };
        let line = serialize_event(100, &event);
        assert!(line.contains("|T|"));
        let (_, parsed) = parse_event(&line).unwrap();
        if let GameEvent::Tick {
            frame,
            left_vel,
            right_vel,
            ball_vel,
            ball_state,
            ..
        } = parsed
        {
            assert_eq!(frame, 150);
            assert_eq!(ball_state, 'F');
            assert!((left_vel.0 - 50.0).abs() < 0.1);
            assert!((right_vel.0 - -30.0).abs() < 0.1);
            assert!((ball_vel.1 - -200.0).abs() < 0.1);
        } else {
            panic!("Wrong event type");
        }
    }

    // === 2v2 CharacterId Tests ===

    #[test]
    fn test_roundtrip_goal2() {
        for character in CharacterId::all() {
            let event = GameEvent::Goal2 {
                character,
                score_left: 2,
                score_right: 1,
            };
            let line = serialize_event(1500, &event);
            assert!(line.contains("|G2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 1500);
            if let GameEvent::Goal2 {
                character: c,
                score_left,
                score_right,
            } = parsed
            {
                assert_eq!(c, character);
                assert_eq!(score_left, 2);
                assert_eq!(score_right, 1);
            } else {
                panic!("Wrong event type for {:?}", character);
            }
        }
    }

    #[test]
    fn test_roundtrip_pickup2() {
        for character in CharacterId::all() {
            let event = GameEvent::Pickup2 { character };
            let line = serialize_event(200, &event);
            assert!(line.contains("|P2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 200);
            if let GameEvent::Pickup2 { character: c } = parsed {
                assert_eq!(c, character);
            } else {
                panic!("Wrong event type for {:?}", character);
            }
        }
    }

    #[test]
    fn test_roundtrip_drop2() {
        for character in CharacterId::all() {
            let event = GameEvent::Drop2 { character };
            let line = serialize_event(300, &event);
            assert!(line.contains("|D2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 300);
            if let GameEvent::Drop2 { character: c } = parsed {
                assert_eq!(c, character);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_roundtrip_shotstart2() {
        for character in CharacterId::all() {
            let event = GameEvent::ShotStart2 {
                character,
                pos: (-150.0, -300.5),
                quality: 0.75,
            };
            let line = serialize_event(400, &event);
            assert!(line.contains("|S2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 400);
            if let GameEvent::ShotStart2 {
                character: c,
                pos,
                quality,
            } = parsed
            {
                assert_eq!(c, character);
                assert!((pos.0 - -150.0).abs() < 0.1);
                assert!((pos.1 - -300.5).abs() < 0.1);
                assert!((quality - 0.75).abs() < 0.01);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_roundtrip_shotrelease2() {
        for character in CharacterId::all() {
            let event = GameEvent::ShotRelease2 {
                character,
                charge: 0.85,
                angle: 55.5,
                power: 650.0,
            };
            let line = serialize_event(500, &event);
            assert!(line.contains("|R2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 500);
            if let GameEvent::ShotRelease2 {
                character: c,
                charge,
                angle,
                power,
            } = parsed
            {
                assert_eq!(c, character);
                assert!((charge - 0.85).abs() < 0.01);
                assert!((angle - 55.5).abs() < 0.1);
                assert!((power - 650.0).abs() < 0.1);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_roundtrip_pass() {
        // Test pass from each character to their teammate
        let test_cases = [
            (CharacterId::L0, CharacterId::L1),
            (CharacterId::L1, CharacterId::L0),
            (CharacterId::R0, CharacterId::R1),
            (CharacterId::R1, CharacterId::R0),
        ];
        for (from, to) in test_cases {
            let event = GameEvent::Pass { from, to };
            let line = serialize_event(600, &event);
            assert!(line.contains("|PA|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 600);
            if let GameEvent::Pass { from: f, to: t } = parsed {
                assert_eq!(f, from);
                assert_eq!(t, to);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_roundtrip_steal_attempt2() {
        for character in CharacterId::all() {
            let event = GameEvent::StealAttempt2 { attacker: character };
            let line = serialize_event(700, &event);
            assert!(line.contains("|A2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 700);
            if let GameEvent::StealAttempt2 { attacker } = parsed {
                assert_eq!(attacker, character);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_roundtrip_steal_success2() {
        for character in CharacterId::all() {
            let event = GameEvent::StealSuccess2 { attacker: character };
            let line = serialize_event(800, &event);
            assert!(line.contains("|+2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 800);
            if let GameEvent::StealSuccess2 { attacker } = parsed {
                assert_eq!(attacker, character);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_roundtrip_steal_fail2() {
        for character in CharacterId::all() {
            let event = GameEvent::StealFail2 { attacker: character };
            let line = serialize_event(900, &event);
            assert!(line.contains("|-2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 900);
            if let GameEvent::StealFail2 { attacker } = parsed {
                assert_eq!(attacker, character);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_roundtrip_steal_outofrange2() {
        for character in CharacterId::all() {
            let event = GameEvent::StealOutOfRange2 { attacker: character };
            let line = serialize_event(1000, &event);
            assert!(line.contains("|O2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 1000);
            if let GameEvent::StealOutOfRange2 { attacker } = parsed {
                assert_eq!(attacker, character);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_roundtrip_jump2() {
        for character in CharacterId::all() {
            let event = GameEvent::Jump2 { character };
            let line = serialize_event(1100, &event);
            assert!(line.contains("|J2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 1100);
            if let GameEvent::Jump2 { character: c } = parsed {
                assert_eq!(c, character);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_roundtrip_land2() {
        for character in CharacterId::all() {
            let event = GameEvent::Land2 { character };
            let line = serialize_event(1200, &event);
            assert!(line.contains("|L2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 1200);
            if let GameEvent::Land2 { character: c } = parsed {
                assert_eq!(c, character);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_roundtrip_tick2_single_character() {
        let event = GameEvent::Tick2 {
            frame: 200,
            characters: vec![CharacterTickData {
                id: CharacterId::L0,
                pos: (-100.0, -350.0),
                vel: (25.0, -10.0),
                controller: 0,
            }],
            ball_pos: (50.0, 100.0),
            ball_vel: (0.0, -150.0),
            ball_state: 'F',
        };
        let line = serialize_event(1300, &event);
        assert!(line.contains("|T2|"));
        let (ts, parsed) = parse_event(&line).unwrap();
        assert_eq!(ts, 1300);
        if let GameEvent::Tick2 {
            frame,
            characters,
            ball_pos,
            ball_vel: _,
            ball_state,
        } = parsed
        {
            assert_eq!(frame, 200);
            assert_eq!(characters.len(), 1);
            assert_eq!(characters[0].id, CharacterId::L0);
            assert!((ball_pos.0 - 50.0).abs() < 0.1);
            assert_eq!(ball_state, 'F');
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_roundtrip_tick2_all_characters() {
        let event = GameEvent::Tick2 {
            frame: 300,
            characters: vec![
                CharacterTickData {
                    id: CharacterId::L0,
                    pos: (-200.0, -350.0),
                    vel: (10.0, 0.0),
                    controller: 0,
                },
                CharacterTickData {
                    id: CharacterId::L1,
                    pos: (-100.0, -350.0),
                    vel: (0.0, 0.0),
                    controller: 1000,
                },
                CharacterTickData {
                    id: CharacterId::R0,
                    pos: (100.0, -350.0),
                    vel: (-5.0, 0.0),
                    controller: 1001,
                },
                CharacterTickData {
                    id: CharacterId::R1,
                    pos: (200.0, -350.0),
                    vel: (0.0, -50.0),
                    controller: 1002,
                },
            ],
            ball_pos: (0.0, 50.0),
            ball_vel: (10.0, -100.0),
            ball_state: 'H',
        };
        let line = serialize_event(1400, &event);
        let (ts, parsed) = parse_event(&line).unwrap();
        assert_eq!(ts, 1400);
        if let GameEvent::Tick2 {
            frame,
            characters,
            ball_state,
            ..
        } = parsed
        {
            assert_eq!(frame, 300);
            assert_eq!(characters.len(), 4);
            assert_eq!(characters[0].id, CharacterId::L0);
            assert_eq!(characters[1].id, CharacterId::L1);
            assert_eq!(characters[2].id, CharacterId::R0);
            assert_eq!(characters[3].id, CharacterId::R1);
            assert_eq!(ball_state, 'H');
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_roundtrip_controller_input2() {
        let event = GameEvent::ControllerInput2 {
            character: CharacterId::L0,
            source_id: 1,
            move_x: 0.75,
            jump: true,
            jump_pressed: true,
            throw: false,
            throw_released: false,
            pickup: true,
            pass: false,
        };
        let line = serialize_event(1500, &event);
        assert!(line.contains("|X2|"));
        let (ts, parsed) = parse_event(&line).unwrap();
        assert_eq!(ts, 1500);
        if let GameEvent::ControllerInput2 {
            character,
            source_id,
            move_x,
            jump,
            jump_pressed,
            throw,
            pickup,
            pass,
            ..
        } = parsed
        {
            assert_eq!(character, CharacterId::L0);
            assert_eq!(source_id, 1);
            assert!((move_x - 0.75).abs() < 0.01);
            assert!(jump);
            assert!(jump_pressed);
            assert!(!throw);
            assert!(pickup);
            assert!(!pass);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_roundtrip_controller_swap2() {
        let event = GameEvent::ControllerSwap2 {
            character: CharacterId::R1,
            old_source: 1000,
            new_source: 0,
        };
        let line = serialize_event(1600, &event);
        assert!(line.contains("|W2|"));
        let (ts, parsed) = parse_event(&line).unwrap();
        assert_eq!(ts, 1600);
        if let GameEvent::ControllerSwap2 {
            character,
            old_source,
            new_source,
        } = parsed
        {
            assert_eq!(character, CharacterId::R1);
            assert_eq!(old_source, 1000);
            assert_eq!(new_source, 0);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_roundtrip_controller_assign() {
        let event = GameEvent::ControllerAssign {
            character: CharacterId::L1,
            source_id: 1001,
            descriptor: "ai:Aggressive".to_string(),
        };
        let line = serialize_event(1700, &event);
        assert!(line.contains("|CA|"));
        let (ts, parsed) = parse_event(&line).unwrap();
        assert_eq!(ts, 1700);
        if let GameEvent::ControllerAssign {
            character,
            source_id,
            descriptor,
        } = parsed
        {
            assert_eq!(character, CharacterId::L1);
            assert_eq!(source_id, 1001);
            assert_eq!(descriptor, "ai:Aggressive");
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_roundtrip_input2() {
        let event = GameEvent::Input2 {
            character: CharacterId::R0,
            source: 5,
            move_x: -0.5,
            jump: false,
            throw: true,
            pickup: false,
            pass: true,
        };
        let line = serialize_event(1800, &event);
        assert!(line.contains("|I2|"));
        let (ts, parsed) = parse_event(&line).unwrap();
        assert_eq!(ts, 1800);
        if let GameEvent::Input2 {
            character,
            source,
            move_x,
            jump,
            throw,
            pickup,
            pass,
        } = parsed
        {
            assert_eq!(character, CharacterId::R0);
            assert_eq!(source, 5);
            assert!((move_x - -0.5).abs() < 0.1);
            assert!(!jump);
            assert!(throw);
            assert!(!pickup);
            assert!(pass);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_roundtrip_aigoal2() {
        let event = GameEvent::AiGoal2 {
            character: CharacterId::L0,
            goal: "GetBall".to_string(),
        };
        let line = serialize_event(1900, &event);
        assert!(line.contains("|AI|"));
        let (ts, parsed) = parse_event(&line).unwrap();
        assert_eq!(ts, 1900);
        if let GameEvent::AiGoal2 { character, goal } = parsed {
            assert_eq!(character, CharacterId::L0);
            assert_eq!(goal, "GetBall");
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_roundtrip_navstart2() {
        let event = GameEvent::NavStart2 {
            character: CharacterId::R1,
            target: (150.0, -300.0),
        };
        let line = serialize_event(2000, &event);
        assert!(line.contains("|N2|"));
        let (ts, parsed) = parse_event(&line).unwrap();
        assert_eq!(ts, 2000);
        if let GameEvent::NavStart2 { character, target } = parsed {
            assert_eq!(character, CharacterId::R1);
            assert!((target.0 - 150.0).abs() < 0.1);
            assert!((target.1 - -300.0).abs() < 0.1);
        } else {
            panic!("Wrong event type");
        }
    }

    #[test]
    fn test_roundtrip_navcomplete2() {
        for character in CharacterId::all() {
            let event = GameEvent::NavComplete2 { character };
            let line = serialize_event(2100, &event);
            assert!(line.contains("|C2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 2100);
            if let GameEvent::NavComplete2 { character: c } = parsed {
                assert_eq!(c, character);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_roundtrip_reset_ai_state2() {
        for character in CharacterId::all() {
            let event = GameEvent::ResetAiState2 { character };
            let line = serialize_event(2200, &event);
            assert!(line.contains("|Z2|"));
            let (ts, parsed) = parse_event(&line).unwrap();
            assert_eq!(ts, 2200);
            if let GameEvent::ResetAiState2 { character: c } = parsed {
                assert_eq!(c, character);
            } else {
                panic!("Wrong event type");
            }
        }
    }

    #[test]
    fn test_parse_legacy_as_characterid() {
        // Ensure CharacterId::from_str handles "L" -> L0 and "R" -> R0
        assert_eq!(CharacterId::from_str("L"), Some(CharacterId::L0));
        assert_eq!(CharacterId::from_str("R"), Some(CharacterId::R0));
    }
}
