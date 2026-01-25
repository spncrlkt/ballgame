//! Drive Extractor
//!
//! Parses evlog files and extracts player input sequences as "drives" -
//! sequences from possession start to goal or turnover.
//!
//! Usage:
//!   cargo run --bin extract-drives training_logs/session_20260125_000742/
//!   cargo run --bin extract-drives training_logs/session_20260125_000742/ --output ghost_trials/

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

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
    /// Source file
    source_file: String,
    /// Drive number within game
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

/// Parse a single evlog file and extract drives
fn parse_evlog(path: &Path) -> Vec<Drive> {
    let file = File::open(path).expect("Failed to open evlog");
    let reader = BufReader::new(file);

    let mut drives = Vec::new();
    let mut current_drive: Option<Drive> = None;
    let mut level = 0u32;
    let mut level_name = String::new();
    let mut current_possession: Option<char> = None;
    let mut drive_num = 0u32;

    let source_file = path.file_name().unwrap().to_string_lossy().to_string();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 2 {
            continue;
        }

        // Parse tick from T:XXXXX format
        let tick = if let Some(tick_str) = parts[0].strip_prefix("T:") {
            tick_str.parse::<u32>().unwrap_or(0)
        } else {
            continue;
        };

        let event_type = parts[1];

        match event_type {
            "MS" if parts.len() >= 4 => {
                // Match Setup: T:00000|MS|level|name|...
                level = parts[2].parse().unwrap_or(0);
                level_name = parts[3].to_string();
            }
            "PU" if parts.len() >= 3 => {
                // Pickup: T:XXXXX|PU|player
                let player = parts[2].chars().next().unwrap_or('?');

                // If we have a current drive and possession changed, end it
                if let Some(ref mut drive) = current_drive {
                    if let Some(prev_poss) = current_possession {
                        if prev_poss != player {
                            // Turnover - end current drive
                            drive.end_tick = tick;
                            drive.end_reason = DriveEndReason::Turnover;
                            drives.push(drive.clone());
                            current_drive = None;
                        }
                    }
                }

                // Start new drive if none active
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
            "S+" if parts.len() >= 3 => {
                // Steal success: T:XXXXX|S+|player
                // This causes a possession change, handled by the PU event that follows
            }
            "I" if parts.len() >= 5 => {
                // Input: T:XXXXX|I|player|move_x|flags
                let player = parts[2].chars().next().unwrap_or('?');
                let move_x: f32 = parts[3].parse().unwrap_or(0.0);
                let flags = parts[4];

                let sample = InputSample {
                    tick,
                    move_x,
                    jump: flags.contains('J'),
                    throw: flags.contains('T'),
                    pickup: flags.contains('P'),
                };

                if let Some(ref mut drive) = current_drive {
                    match player {
                        'L' => drive.l_inputs.push(sample),
                        'R' => drive.r_inputs.push(sample),
                        _ => {}
                    }
                }
            }
            "G" if parts.len() >= 4 => {
                // Goal: T:XXXXX|G|scorer|score1|score2
                let scorer = parts[2].chars().next().unwrap_or('?');

                if let Some(ref mut drive) = current_drive {
                    drive.end_tick = tick;
                    drive.end_reason = DriveEndReason::Goal;
                    drive.l_scored = scorer == 'L';
                    drives.push(drive.clone());
                    current_drive = None;
                    current_possession = None;
                }
            }
            "ME" => {
                // Match End: T:XXXXX|ME|score1|score2|duration
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

    // Handle any remaining drive
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
        drive.source_file.replace(".evlog", ""),
        drive.drive_num,
        drive.level_name.to_lowercase().replace(' ', "_"),
        if drive.l_scored { "_scored" } else { "" }
    );

    let path = output_dir.join(&filename);
    let mut file = File::create(&path)?;

    // Effective start is 1 second after possession
    let effective_start = drive.start_tick + GHOST_START_DELAY_MS;

    // Write header
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

    // Write level info
    writeln!(file, "level:{}", drive.level)?;
    writeln!(file, "level_name:{}", drive.level_name)?;
    writeln!(file)?;

    // Write L's inputs (the human player), skipping first second
    for sample in &drive.l_inputs {
        // Skip inputs before the delay period
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

        // Normalize tick to start from 0 (after the delay)
        let rel_tick = sample.tick.saturating_sub(effective_start);
        writeln!(file, "{}|{:.2}|{}", rel_tick, sample.move_x, flags)?;
    }

    Ok(path)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <session_dir> [--output <output_dir>]", args[0]);
        eprintln!();
        eprintln!("Extracts player input sequences from evlog files as ghost trials.");
        eprintln!("Ghost trials end at turnovers or goals.");
        std::process::exit(1);
    }

    let session_dir = PathBuf::from(&args[1]);
    let output_dir = if args.len() >= 4 && args[2] == "--output" {
        PathBuf::from(&args[3])
    } else {
        session_dir.join("ghost_trials")
    };

    // Create output directory
    fs::create_dir_all(&output_dir).expect("Failed to create output directory");

    // Find all evlog files
    let evlogs: Vec<PathBuf> = fs::read_dir(&session_dir)
        .expect("Failed to read session directory")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "evlog"))
        .collect();

    if evlogs.is_empty() {
        eprintln!("No .evlog files found in {}", session_dir.display());
        std::process::exit(1);
    }

    println!("Found {} evlog files", evlogs.len());
    println!("Output directory: {}", output_dir.display());
    println!();

    let mut total_drives = 0;
    let mut scoring_drives = 0;
    let mut turnover_drives = 0;

    // Process each evlog
    for evlog in &evlogs {
        println!("Processing: {}", evlog.file_name().unwrap().to_string_lossy());

        let drives = parse_evlog(evlog);

        for drive in &drives {
            // Only save drives where L (human) had possession or L scored
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
