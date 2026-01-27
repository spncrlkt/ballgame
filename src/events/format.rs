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

use super::types::{ControllerSource, GameConfig, GameEvent, PlayerId};

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

    #[test]
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
}
