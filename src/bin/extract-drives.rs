//! Drive Extractor
//!
//! Extracts player input sequences as "drives" from SQLite event logs.
//! A drive is a sequence from possession start to goal or turnover.
//!
//! Usage:
//!   cargo run --bin extract-drives -- --db training.db --session <SESSION_ID>
//!   cargo run --bin extract-drives -- --db training.db --match <MATCH_ID> --output ghost_trials/

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use rusqlite::params;

use ballgame::events::{parse_event, GameEvent, PlayerId};
use ballgame::simulation::SimDatabase;

/// Input sample at a single tick
#[derive(Debug, Clone)]
struct InputSample {
    tick: u32,
    move_x: f32,
    jump: bool,
    throw: bool,
    pickup: bool,
}

/// A drive is a sequence of inputs from possession start to goal/turnover
#[derive(Debug, Clone)]
struct Drive {
    /// Source identifier
    source_file: String,
    /// Drive number within match
    drive_num: u32,
    /// Level number
    level: u32,
    /// Level name
    level_name: String,
    /// Who had possession at start (L or R)
    initial_possession: char,
    /// Start tick
    start_tick: u32,
    /// End tick
    end_tick: u32,
    /// How the drive ended
    end_reason: DriveEndReason,
    /// Player L's inputs throughout the drive
    l_inputs: Vec<InputSample>,
    /// Player R's inputs throughout the drive
    r_inputs: Vec<InputSample>,
    /// Did player L score?
    l_scored: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DriveEndReason {
    Goal,
    Turnover,
    MatchEnd,
}

impl std::fmt::Display for DriveEndReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriveEndReason::Goal => write!(f, "goal"),
            DriveEndReason::Turnover => write!(f, "turnover"),
            DriveEndReason::MatchEnd => write!(f, "match_end"),
        }
    }
}

fn player_char(player: PlayerId) -> char {
    match player {
        PlayerId::L => 'L',
        PlayerId::R => 'R',
    }
}

/// Parse a single match from SQLite and extract drives
fn parse_match_from_db(db: &SimDatabase, match_id: i64) -> Vec<Drive> {
    let (mut level, mut level_name) = db
        .conn()
        .query_row(
            "SELECT level, level_name FROM matches WHERE id = ?1",
            params![match_id],
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?)),
        )
        .unwrap_or((0, String::new()));

    let mut drives = Vec::new();
    let mut current_drive: Option<Drive> = None;
    let mut current_possession: Option<char> = None;
    let mut drive_num = 0u32;
    let source_file = format!("match_{}", match_id);

    let events = match db.get_events(match_id) {
        Ok(events) => events,
        Err(_) => return drives,
    };

    for event in events {
        let Some((_ts, parsed)) = parse_event(&event.data) else {
            continue;
        };
        let tick = event.time_ms;

        match parsed {
            GameEvent::MatchStart { level: lvl, level_name: name, .. } => {
                level = lvl;
                level_name = name;
            }
            GameEvent::Pickup { player } => {
                let player = player_char(player);

                if let Some(ref mut drive) = current_drive {
                    if let Some(prev_poss) = current_possession {
                        if prev_poss != player {
                            drive.end_tick = tick;
                            drive.end_reason = DriveEndReason::Turnover;
                            drives.push(drive.clone());
                            current_drive = None;
                        }
                    }
                }

                if current_drive.is_none() {
                    drive_num += 1;
                    current_drive = Some(Drive {
                        source_file: source_file.clone(),
                        drive_num,
                        level,
                        level_name: level_name.clone(),
                        initial_possession: player,
                        start_tick: tick,
                        end_tick: tick,
                        end_reason: DriveEndReason::MatchEnd,
                        l_inputs: Vec::new(),
                        r_inputs: Vec::new(),
                        l_scored: false,
                    });
                }

                current_possession = Some(player);
            }
            GameEvent::Input {
                player,
                move_x,
                jump,
                throw,
                pickup,
            } => {
                let player = player_char(player);
                let sample = InputSample {
                    tick,
                    move_x,
                    jump,
                    throw,
                    pickup,
                };

                if let Some(ref mut drive) = current_drive {
                    match player {
                        'L' => drive.l_inputs.push(sample),
                        'R' => drive.r_inputs.push(sample),
                        _ => {}
                    }
                }
            }
            GameEvent::Goal { player, .. } => {
                let scorer = player_char(player);
                if let Some(ref mut drive) = current_drive {
                    drive.end_tick = tick;
                    drive.end_reason = DriveEndReason::Goal;
                    drive.l_scored = scorer == 'L';
                    drives.push(drive.clone());
                    current_drive = None;
                    current_possession = None;
                }
            }
            GameEvent::MatchEnd { .. } => {
                if let Some(ref mut drive) = current_drive {
                    drive.end_tick = tick;
                    drive.end_reason = DriveEndReason::MatchEnd;
                    drives.push(drive.clone());
                    current_drive = None;
                }
            }
            _ => {}
        }
    }

    if let Some(mut drive) = current_drive {
        drive.end_reason = DriveEndReason::MatchEnd;
        drives.push(drive);
    }

    drives
}

/// Delay before starting ghost input capture (ms)
/// This gives the ghost time to get into position from spawn
const GHOST_START_DELAY_MS: u32 = 1000;

/// Output a drive as a ghost trial file
fn write_ghost_trial(drive: &Drive, output_dir: &Path) -> std::io::Result<PathBuf> {
    let filename = format!(
        "trial_{}_drive{:02}_{}_L{}.ghost",
        drive.source_file,
        drive.drive_num,
        drive.level_name.to_lowercase().replace(' ', "_"),
        if drive.l_scored { "_scored" } else { "" }
    );

    let path = output_dir.join(&filename);
    let mut file = File::create(&path)?;

    let effective_start = drive.start_tick + GHOST_START_DELAY_MS;

    writeln!(file, "# Ghost Trial")?;
    writeln!(file, "# Source: {}", drive.source_file)?;
    writeln!(file, "# Drive: {}", drive.drive_num)?;
    writeln!(file, "# Level: {} ({})", drive.level, drive.level_name)?;
    writeln!(file, "# Original ticks: {} -> {}", drive.start_tick, drive.end_tick)?;
    writeln!(file, "# Effective start: {} (+{}ms delay)", effective_start, GHOST_START_DELAY_MS)?;
    writeln!(file, "# End: {}", drive.end_reason)?;
    writeln!(file, "# L scored: {}", drive.l_scored)?;
    writeln!(file, "#")?;
    writeln!(file, "# Format: tick|move_x|flags (J=jump T=throw P=pickup)")?;
    writeln!(file)?;

    writeln!(file, "level:{}", drive.level)?;
    writeln!(file, "level_name:{}", drive.level_name)?;
    writeln!(file)?;

    for sample in &drive.l_inputs {
        if sample.tick < effective_start {
            continue;
        }

        let mut flags = String::new();
        if sample.jump {
            flags.push('J');
        }
        if sample.throw {
            flags.push('T');
        }
        if sample.pickup {
            flags.push('P');
        }
        if flags.is_empty() {
            flags.push('-');
        }

        let rel_tick = sample.tick.saturating_sub(effective_start);
        writeln!(file, "{}|{:.2}|{}", rel_tick, sample.move_x, flags)?;
    }

    Ok(path)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} --db <path> [--session <id> | --match <id>] [--output <dir>]", args[0]);
        eprintln!();
        eprintln!("Extracts player input sequences from SQLite events as ghost trials.");
        std::process::exit(1);
    }

    let mut db_path = PathBuf::from("db/training.db");
    let mut session_id: Option<String> = None;
    let mut match_id: Option<i64> = None;
    let mut output_dir: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--db" if i + 1 < args.len() => {
                db_path = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--session" if i + 1 < args.len() => {
                session_id = Some(args[i + 1].clone());
                i += 2;
            }
            "--match" if i + 1 < args.len() => {
                match_id = args[i + 1].parse::<i64>().ok();
                i += 2;
            }
            "--output" if i + 1 < args.len() => {
                output_dir = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    let output_dir = output_dir.unwrap_or_else(|| PathBuf::from("ghost_trials"));
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    let db = SimDatabase::open(&db_path).expect("Failed to open database");

    let match_ids: Vec<i64> = if let Some(match_id) = match_id {
        vec![match_id]
    } else if let Some(session_id) = session_id {
        db.get_session_matches(&session_id)
            .map(|matches| matches.into_iter().map(|m| m.id).collect())
            .unwrap_or_default()
    } else {
        db.get_latest_session()
            .ok()
            .flatten()
            .and_then(|session_id| {
                db.get_session_matches(&session_id)
                    .ok()
                    .map(|matches| matches.into_iter().map(|m| m.id).collect())
            })
            .unwrap_or_default()
    };

    if match_ids.is_empty() {
        eprintln!("No matches found in {}", db_path.display());
        std::process::exit(1);
    }

    println!("Found {} matches", match_ids.len());
    println!("Output directory: {}", output_dir.display());
    println!();

    let mut total_drives = 0;
    let mut scoring_drives = 0;
    let mut turnover_drives = 0;

    for match_id in &match_ids {
        println!("Processing match {}", match_id);
        let drives = parse_match_from_db(&db, *match_id);

        for drive in &drives {
            let l_had_possession = drive.initial_possession == 'L';
            let is_interesting = l_had_possession || drive.l_scored;

            if !is_interesting {
                continue;
            }

            match write_ghost_trial(drive, &output_dir) {
                Ok(_path) => {
                    total_drives += 1;
                    if drive.l_scored {
                        scoring_drives += 1;
                    }
                    if drive.end_reason == DriveEndReason::Turnover {
                        turnover_drives += 1;
                    }

                    println!(
                        "  Drive {} ({} -> {}): {} samples, end={}, L_scored={}",
                        drive.drive_num,
                        drive.start_tick,
                        drive.end_tick,
                        drive.l_inputs.len(),
                        drive.end_reason,
                        drive.l_scored
                    );
                }
                Err(e) => {
                    eprintln!("  Failed to write drive {}: {}", drive.drive_num, e);
                }
            }
        }
    }

    println!();
    println!("=== Summary ===");
    println!("Total drives extracted: {}", total_drives);
    println!("  Scoring drives: {}", scoring_drives);
    println!("  Turnover drives: {}", turnover_drives);
    println!("Ghost trials written to: {}", output_dir.display());
}
