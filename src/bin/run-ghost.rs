//! Ghost Trial Runner
//!
//! Plays back recorded human inputs (ghost trials) against AI to test defensive capability.
//!
//! Usage:
//!   cargo run --bin run-ghost training_logs/session_*/game_1.evlog
//!   cargo run --bin run-ghost training_logs/session_*/ --profile v3_Rush_Smart
//!   cargo run --bin run-ghost ghost_trials/ --summary

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use ballgame::ai::{
    AiNavState, AiProfileDatabase, AiState, InputState, NavGraph,
    ai_navigation_update, mark_nav_dirty_on_level_change, rebuild_nav_graph,
};
use ballgame::ball::{
    Ball, BallState, CurrentPalette, apply_velocity, ball_collisions,
    ball_follow_holder, ball_gravity, ball_player_collision, ball_spin, ball_state_update,
    pickup_ball,
};
use ballgame::constants::*;
use ballgame::levels::LevelDatabase;
use ballgame::palettes::PaletteDatabase;
use ballgame::player::{
    HoldingBall, Player, Team, apply_gravity, apply_input, check_collisions,
};
use ballgame::scoring::{CurrentLevel, Score, check_scoring};
use ballgame::shooting::{throw_ball, update_shot_charge, LastShotInfo};
use ballgame::simulation::{
    GhostOutcome, GhostPlaybackState, GhostTrial, GhostTrialResult, SimConfig, SimControl,
    ghost_input_system, load_ghost_trial, max_tick, sim_setup,
};
use ballgame::steal::{StealContest, StealTracker, steal_cooldown_update};
use ballgame::ui::PhysicsTweaks;

/// Run a single ghost trial
fn run_ghost_trial(
    trial: &GhostTrial,
    ai_profile: &str,
    level_db: &LevelDatabase,
    profile_db: &AiProfileDatabase,
    verbose: bool,
) -> GhostTrialResult {
    let mut app = App::new();

    // Minimal plugins for headless operation
    app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
        Duration::from_secs_f32(1.0 / 60.0),
    )));
    app.add_plugins(bevy::transform::TransformPlugin);

    // Fixed timestep
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_secs_f32(
        1.0 / 60.0,
    )));

    // Game resources
    app.insert_resource(level_db.clone());
    app.insert_resource(profile_db.clone());
    app.init_resource::<Score>();
    app.insert_resource(CurrentLevel(trial.level));
    app.init_resource::<StealContest>();
    app.init_resource::<StealTracker>();
    app.init_resource::<NavGraph>();
    app.init_resource::<PhysicsTweaks>();
    app.init_resource::<LastShotInfo>();
    app.insert_resource(CurrentPalette(0));
    app.init_resource::<PaletteDatabase>();

    // Ghost resources
    app.insert_resource(trial.clone());
    app.insert_resource(GhostPlaybackState::default());

    // Simulation control
    app.insert_resource(SimControl {
        config: SimConfig {
            duration_limit: 30.0, // 30 second max per trial
            level: Some(trial.level),
            left_profile: "Ghost".to_string(),
            right_profile: ai_profile.to_string(),
            ..Default::default()
        },
        should_exit: false,
        current_seed: 0,
    });

    // Startup
    app.add_systems(Startup, sim_setup);
    app.add_systems(PostStartup, |mut nav_graph: ResMut<NavGraph>| {
        nav_graph.dirty = true;
    });

    // Configure AI profile for right player
    let profile_index = profile_db.index_of(ai_profile).unwrap_or(0);
    app.add_systems(
        PostStartup,
        move |mut players: Query<(&Team, &mut AiState), With<Player>>| {
            for (team, mut ai_state) in &mut players {
                if *team == Team::Right {
                    ai_state.profile_index = profile_index;
                }
            }
        },
    );

    // AI systems (only for right player - left uses ghost input)
    app.add_systems(
        Update,
        (
            mark_nav_dirty_on_level_change,
            rebuild_nav_graph,
            ai_navigation_update,
            // Custom AI update that skips the left (ghost) player
            ai_decision_for_right_only,
        )
            .chain(),
    );

    // Ghost input system (runs for left player)
    app.add_systems(Update, ghost_input_system);

    // Steal cooldown and end condition check
    app.add_systems(Update, (steal_cooldown_update, ghost_check_end));

    // Physics
    app.add_systems(
        FixedUpdate,
        (
            apply_input,
            apply_gravity,
            ball_gravity,
            ball_spin,
            apply_velocity,
            check_collisions,
            ball_collisions,
            ball_state_update,
            ball_player_collision,
            ball_follow_holder,
            pickup_ball,
            steal_cooldown_update,
            update_shot_charge,
            throw_ball,
            check_scoring,
        )
            .chain(),
    );

    // Initialize app
    app.finish();
    app.cleanup();
    app.update();

    // Run simulation
    let fixed_dt = Duration::from_secs_f32(1.0 / 60.0);

    loop {
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .advance_by(fixed_dt);
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(fixed_dt);
        app.world_mut()
            .resource_mut::<Time<Fixed>>()
            .advance_by(fixed_dt);

        app.world_mut().run_schedule(Update);
        app.world_mut().run_schedule(FixedUpdate);

        let control = app.world().resource::<SimControl>();
        if control.should_exit {
            break;
        }
    }

    // Extract result
    let playback = app.world().resource::<GhostPlaybackState>();
    let outcome = playback.outcome.unwrap_or(GhostOutcome::TimeLimit);
    let end_tick = playback.end_tick;
    let total_ticks = max_tick(trial);

    let result = GhostTrialResult {
        source_file: trial.source_file.clone(),
        level: trial.level,
        level_name: trial.level_name.clone(),
        ai_profile: ai_profile.to_string(),
        outcome,
        duration: end_tick as f32 / 1000.0,
        end_tick,
        total_ticks,
        originally_scored: trial.originally_scored,
    };

    if verbose {
        let defended = if result.ai_defended() { "YES" } else { "NO" };
        println!(
            "  {} | {} | {:.1}s | defended: {}",
            trial.source_file, outcome, result.duration, defended
        );
    }

    result
}

/// AI decision update that only runs for the right player (left is ghost-controlled)
fn ai_decision_for_right_only(
    _time: Res<Time>,
    profile_db: Res<AiProfileDatabase>,
    _nav_graph: Res<NavGraph>,
    mut ai_query: Query<
        (
            Entity,
            &Transform,
            &Team,
            &mut InputState,
            &mut AiState,
            &AiNavState,
            &ballgame::player::TargetBasket,
            Option<&HoldingBall>,
            &ballgame::player::Grounded,
        ),
        With<Player>,
    >,
    all_players: Query<
        (
            Entity,
            &Transform,
            Option<&HoldingBall>,
        ),
        With<Player>,
    >,
    ball_query: Query<(&Transform, &BallState), With<Ball>>,
    basket_query: Query<(&Transform, &ballgame::world::Basket)>,
) {
    // Filter to only process right player
    for (entity, transform, team, mut input, ai_state, _nav_state, target_basket, holding, grounded)
        in &mut ai_query
    {
        if *team != Team::Right {
            continue;
        }

        // Call the regular AI decision logic for this player
        // (Simplified version - just basic behavior)
        let profile = profile_db.get(ai_state.profile_index);
        let ai_pos = transform.translation.truncate();

        // Get ball info
        let Some((ball_transform, ball_state)) = ball_query.iter().next() else {
            continue;
        };
        let ball_pos = ball_transform.translation.truncate();

        // Check if AI has ball
        let ai_has_ball = holding.is_some();

        // Find opponent (left player)
        let opponent_pos = all_players
            .iter()
            .find(|(e, _, _)| *e != entity)
            .map(|(_, t, _)| t.translation.truncate());

        // Simple AI behavior: chase ball or defend
        input.move_x = 0.0;
        input.jump_held = false;
        input.pickup_pressed = false;
        input.throw_held = false;

        if ai_has_ball {
            // Has ball - move toward basket and shoot
            let target = basket_query
                .iter()
                .find(|(_, b)| **b == target_basket.0)
                .map(|(t, _)| t.translation.truncate())
                .unwrap_or(Vec2::new(-600.0, 0.0));

            let dx = target.x - ai_pos.x;
            if dx.abs() > profile.position_tolerance {
                input.move_x = dx.signum();
            }
        } else if matches!(ball_state, BallState::Held(_)) {
            // Opponent has ball - chase them
            if let Some(opp_pos) = opponent_pos {
                let dx = opp_pos.x - ai_pos.x;
                if dx.abs() > profile.position_tolerance {
                    input.move_x = dx.signum();
                }

                // Try to steal
                let dist = ai_pos.distance(opp_pos);
                if dist < profile.steal_range {
                    input.pickup_pressed = true;
                }
            }
        } else {
            // Ball is free - chase it
            let dx = ball_pos.x - ai_pos.x;
            if dx.abs() > profile.position_tolerance {
                input.move_x = dx.signum();
            }

            // Pick up if close
            let dist = ai_pos.distance(ball_pos);
            if dist < BALL_PICKUP_RADIUS {
                input.pickup_pressed = true;
            }
        }

        // Jump if ball/opponent is above
        let target_y = if ai_has_ball {
            0.0
        } else if let Some(opp) = opponent_pos {
            opp.y
        } else {
            ball_pos.y
        };

        if target_y > ai_pos.y + PLAYER_SIZE.y && grounded.0 {
            input.jump_buffer_timer = JUMP_BUFFER_TIME;
            input.jump_held = true;
        }
    }
}

/// Check ghost trial end conditions
fn ghost_check_end(
    score: Res<Score>,
    mut playback: ResMut<GhostPlaybackState>,
    trial: Res<GhostTrial>,
    players: Query<(&Team, Option<&HoldingBall>), With<Player>>,
    mut control: ResMut<SimControl>,
) {
    if playback.outcome.is_some() {
        control.should_exit = true;
        return;
    }

    // Grace period
    if playback.elapsed_ms < 500 {
        return;
    }

    // Inputs exhausted
    if playback.current_index >= trial.inputs.len() && !playback.finished {
        playback.outcome = Some(GhostOutcome::InputsExhausted);
        playback.end_tick = playback.elapsed_ms;
        playback.finished = true;
        control.should_exit = true;
        return;
    }

    // Ghost (left) scored
    if score.left > 0 {
        playback.outcome = Some(GhostOutcome::GhostScored);
        playback.end_tick = playback.elapsed_ms;
        playback.finished = true;
        control.should_exit = true;
        return;
    }

    // AI (right) has ball - stole/intercepted
    for (team, holding) in &players {
        if *team == Team::Right && holding.is_some() {
            playback.outcome = Some(GhostOutcome::AiStole);
            playback.end_tick = playback.elapsed_ms;
            playback.finished = true;
            control.should_exit = true;
            return;
        }
    }

    // Time limit
    let elapsed_secs = playback.elapsed_ms as f32 / 1000.0;
    if elapsed_secs > control.config.duration_limit {
        playback.outcome = Some(GhostOutcome::TimeLimit);
        playback.end_tick = playback.elapsed_ms;
        playback.finished = true;
        control.should_exit = true;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Ghost Trial Runner");
        eprintln!();
        eprintln!("Usage:");
        eprintln!("  {} <file.evlog>                    Run single training session", args[0]);
        eprintln!("  {} <dir/> [--profile <name>]       Run all .evlog/.ghost files in directory", args[0]);
        eprintln!("  {} <dir/> --summary                Show summary only", args[0]);
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --profile <name>   AI profile to test (default: v2_Balanced)");
        eprintln!("  --summary          Only show summary stats");
        eprintln!("  --verbose          Show each trial result");
        std::process::exit(1);
    }

    let input_path = PathBuf::from(&args[1]);

    // Parse options
    let mut ai_profile = "v2_Balanced".to_string();
    let mut summary_only = false;
    let mut verbose = false;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--profile" if i + 1 < args.len() => {
                ai_profile = args[i + 1].clone();
                i += 2;
            }
            "--summary" => {
                summary_only = true;
                i += 1;
            }
            "--verbose" => {
                verbose = true;
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    // Load databases
    let level_db = LevelDatabase::load_from_file("assets/levels.txt");
    let profile_db = AiProfileDatabase::load_from_file("assets/ai_profiles.txt");

    // Verify AI profile exists
    if profile_db.index_of(&ai_profile).is_none() {
        eprintln!("ERROR: AI profile '{}' not found", ai_profile);
        eprintln!("Available profiles:");
        for p in profile_db.profiles() {
            eprintln!("  {}", p.name);
        }
        std::process::exit(1);
    }

    // Collect trials to run
    let trials: Vec<GhostTrial> = if input_path.is_dir() {
        fs::read_dir(&input_path)
            .expect("Failed to read directory")
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().map_or(false, |ext| ext == "ghost" || ext == "evlog")
            })
            .filter_map(|p| load_ghost_trial(&p).ok())
            .collect()
    } else {
        match load_ghost_trial(&input_path) {
            Ok(trial) => vec![trial],
            Err(e) => {
                eprintln!("Failed to load trial: {}", e);
                std::process::exit(1);
            }
        }
    };

    if trials.is_empty() {
        eprintln!("No ghost trials found");
        std::process::exit(1);
    }

    println!("Ghost Trial Runner");
    println!("==================");
    println!("AI Profile: {}", ai_profile);
    println!("Trials: {}", trials.len());
    println!();

    // Run trials
    let mut results = Vec::new();
    for trial in &trials {
        if !summary_only {
            print!("Running {}... ", trial.source_file);
            std::io::Write::flush(&mut std::io::stdout()).ok();
        }

        let result = run_ghost_trial(trial, &ai_profile, &level_db, &profile_db, verbose);

        if !summary_only && !verbose {
            let defended = if result.ai_defended() { "defended" } else { "scored" };
            println!("{} ({})", result.outcome, defended);
        }

        results.push(result);
    }

    // Summary
    println!();
    println!("=== Summary ===");

    let total = results.len();
    let defended = results.iter().filter(|r| r.ai_defended()).count();
    let ghost_scored = results
        .iter()
        .filter(|r| r.outcome == GhostOutcome::GhostScored)
        .count();
    let ai_stole = results
        .iter()
        .filter(|r| r.outcome == GhostOutcome::AiStole)
        .count();
    let exhausted = results
        .iter()
        .filter(|r| r.outcome == GhostOutcome::InputsExhausted)
        .count();
    let timeout = results
        .iter()
        .filter(|r| r.outcome == GhostOutcome::TimeLimit)
        .count();

    let defense_rate = if total > 0 {
        defended as f32 / total as f32 * 100.0
    } else {
        0.0
    };

    println!("Defense rate: {:.1}% ({}/{})", defense_rate, defended, total);
    println!();
    println!("Outcomes:");
    println!("  Ghost scored:      {} ({:.1}%)", ghost_scored, ghost_scored as f32 / total as f32 * 100.0);
    println!("  AI stole:          {} ({:.1}%)", ai_stole, ai_stole as f32 / total as f32 * 100.0);
    println!("  Inputs exhausted:  {} ({:.1}%)", exhausted, exhausted as f32 / total as f32 * 100.0);
    println!("  Time limit:        {} ({:.1}%)", timeout, timeout as f32 / total as f32 * 100.0);

    // Originally scoring trials
    let originally_scored: Vec<_> = results.iter().filter(|r| r.originally_scored).collect();
    if !originally_scored.is_empty() {
        let defended_scoring = originally_scored.iter().filter(|r| r.ai_defended()).count();
        println!();
        println!(
            "Originally-scoring drives defended: {}/{} ({:.1}%)",
            defended_scoring,
            originally_scored.len(),
            defended_scoring as f32 / originally_scored.len() as f32 * 100.0
        );
    }
}
