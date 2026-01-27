//! Test execution engine

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

use crate::ai::InputState;
use crate::ball::{
    Ball, BallPlayerContact, BallPulse, BallRolling, BallShotGrace, BallSpin, BallState, BallStyle,
    CurrentPalette, Velocity, apply_velocity, ball_collisions, ball_follow_holder, ball_gravity,
    ball_player_collision, ball_spin, ball_state_update, pickup_ball,
};
use crate::constants::*;
use crate::events::EventBus;
use crate::levels::LevelDatabase;
use crate::palettes::PaletteDatabase;
use crate::player::{
    CoyoteTimer, Facing, Grounded, HoldingBall, JumpState, Player, TargetBasket, Team,
    apply_gravity, apply_input, check_collisions,
};
use crate::scoring::{CurrentLevel, Score, check_scoring};
use crate::shooting::{ChargingShot, LastShotInfo, throw_ball, update_shot_charge};
use crate::steal::{StealContest, StealCooldown, StealTracker, steal_cooldown_update};
use crate::tuning::{self, PhysicsTweaks};
use crate::world::{Basket, Collider, Platform, spawn_baskets, spawn_floor, spawn_walls};

use super::TEST_LEVELS_FILE;
use super::assertions::{
    AssertionError, BallState as AssertionBallState, CapturedEvent, EntityState, WorldState,
    check_sequence, check_state,
};
use super::input::{ScriptedInputs, TestEntityId};
use super::parser::{EntityDef, TestDefinition};

/// Result of running a test
#[derive(Debug)]
pub enum TestResult {
    Pass { frames: u64 },
    Fail { error: AssertionError },
    Error { message: String },
}

/// Resource to track test state
#[derive(Resource)]
struct TestControl {
    should_exit: bool,
    max_frame: u64,
    current_frame: u64,
    /// Frames at which to check state assertions (sorted)
    state_check_frames: Vec<u64>,
    /// Index of next state check to perform
    next_state_check: usize,
}

/// Resource to capture events during test
#[derive(Resource, Default)]
struct EventCapture {
    events: Vec<CapturedEvent>,
    entity_map: HashMap<Entity, String>,
    // Track state changes for event detection
    prev_score_left: u32,
    prev_score_right: u32,
    prev_ball_holder: Option<Entity>,
    prev_charging: HashMap<Entity, bool>,
    prev_steal_cooldowns: HashMap<Entity, f32>,
}

/// Resource to store state assertion error (if any)
#[derive(Resource, Default)]
struct StateAssertionResult {
    error: Option<AssertionError>,
}

/// Run a single test and return the result
pub fn run_test(test: &TestDefinition) -> TestResult {
    // Load test levels
    let level_db = LevelDatabase::load_from_file(TEST_LEVELS_FILE);

    // Find the level by name
    let level_name = &test.setup.level;
    let level_idx = level_db
        .levels
        .iter()
        .position(|l| l.name == *level_name || l.name == level_name.replace("test_", ""))
        .or_else(|| {
            // Try matching without prefix
            level_db
                .levels
                .iter()
                .position(|l| l.name.to_lowercase().replace(' ', "_") == level_name.to_lowercase())
        });

    let level_idx = match level_idx {
        Some(idx) => idx,
        None => {
            return TestResult::Error {
                message: format!(
                    "Level '{}' not found. Available: {:?}",
                    level_name,
                    level_db.levels.iter().map(|l| &l.name).collect::<Vec<_>>()
                ),
            };
        }
    };

    // Create scripted inputs
    let mut scripted_inputs = ScriptedInputs::from_inputs(&test.input);

    // Set max frame from state assertions if present (use max of all assertions)
    for state in &test.expect.state {
        scripted_inputs.set_max_frame(state.after_frame);
    }

    // Ensure we run at least some frames
    if scripted_inputs.max_frame == 0 {
        scripted_inputs.max_frame = 60; // Default 1 second
    }

    // Create minimal Bevy app
    let mut app = App::new();

    app.add_plugins(
        MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f32(
            1.0 / 60.0,
        ))),
    );
    app.add_plugins(bevy::transform::TransformPlugin);

    // Get level ID before moving level_db into app
    let level_id = level_db
        .get(level_idx)
        .map(|l| l.id.clone())
        .unwrap_or_default();

    // Resources
    app.insert_resource(level_db);
    app.init_resource::<Score>();
    app.insert_resource(CurrentLevel(level_id));
    app.init_resource::<StealContest>();
    app.init_resource::<StealTracker>();
    app.init_resource::<PhysicsTweaks>();
    let _ = tuning::apply_global_tuning(&mut app.world_mut().resource_mut::<PhysicsTweaks>());
    app.init_resource::<LastShotInfo>();
    app.insert_resource(CurrentPalette(0));
    app.init_resource::<PaletteDatabase>();
    app.insert_resource(EventBus::new());
    // Collect state check frames
    let state_check_frames: Vec<u64> = {
        let mut frames: Vec<u64> = test.expect.state.iter().map(|s| s.after_frame).collect();
        frames.sort();
        frames.dedup();
        frames
    };

    app.insert_resource(scripted_inputs);
    app.insert_resource(TestControl {
        should_exit: false,
        max_frame: 0, // Will be set from ScriptedInputs
        current_frame: 0,
        state_check_frames,
        next_state_check: 0,
    });
    app.init_resource::<EventCapture>();
    app.init_resource::<StateAssertionResult>();

    // Store state assertions for inline checking
    let state_assertions = test.expect.state.clone();

    // Startup
    let entities_clone = test.setup.entities.clone();
    app.add_systems(
        Startup,
        move |commands: Commands,
              level_db: Res<LevelDatabase>,
              current_level: Res<CurrentLevel>,
              mut capture: ResMut<EventCapture>| {
            test_setup(
                commands,
                &level_db,
                &current_level,
                &entities_clone,
                &mut capture,
            );
        },
    );

    // Game systems - Update for event capture and end check
    app.add_systems(Update, (event_capture, test_end_check));

    // FixedUpdate - input injection runs first, then physics
    app.add_systems(
        FixedUpdate,
        (
            input_injection,
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

    // Run simulation with inline state assertion checking
    loop {
        app.update();

        // Check if we need to run state assertions at this frame
        {
            let control = app.world().resource::<TestControl>();
            let current_frame = control.current_frame;
            let next_check = control.next_state_check;

            if next_check < control.state_check_frames.len() {
                let check_frame = control.state_check_frames[next_check];
                if current_frame >= check_frame {
                    // Find all assertions for this frame
                    let assertions_for_frame: Vec<_> = state_assertions
                        .iter()
                        .filter(|a| a.after_frame == check_frame)
                        .collect();

                    // Run state checks
                    let world_state = extract_world_state(app.world_mut());
                    for assertion in assertions_for_frame {
                        if let Err(e) = check_state(assertion, &world_state) {
                            app.world_mut().resource_mut::<StateAssertionResult>().error = Some(e);
                            app.world_mut().resource_mut::<TestControl>().should_exit = true;
                            break;
                        }
                    }

                    // Move to next state check
                    app.world_mut()
                        .resource_mut::<TestControl>()
                        .next_state_check += 1;
                }
            }
        }

        let control = app.world().resource::<TestControl>();
        if control.should_exit {
            break;
        }
    }

    // Check for state assertion errors
    {
        let result = app.world().resource::<StateAssertionResult>();
        if let Some(ref error) = result.error {
            return TestResult::Fail {
                error: error.clone(),
            };
        }
    }

    // Extract results
    let final_frame;
    let captured_events;
    {
        let world = app.world();
        final_frame = world.resource::<TestControl>().current_frame;
        captured_events = world.resource::<EventCapture>().events.clone();
    }

    // Check sequence assertions
    if let Err(e) = check_sequence(&test.expect.sequence, &captured_events) {
        return TestResult::Fail { error: e };
    }

    TestResult::Pass {
        frames: final_frame,
    }
}

/// Setup system for test
fn test_setup(
    mut commands: Commands,
    level_db: &LevelDatabase,
    current_level: &CurrentLevel,
    entities: &[EntityDef],
    capture: &mut EventCapture,
) {
    // Use a default gray color for test arena (no palette needed)
    let arena_color = Color::srgb(0.3, 0.3, 0.3);

    // Spawn arena using shared functions
    spawn_floor(&mut commands, arena_color);
    spawn_walls(&mut commands, arena_color);

    // Level platforms and baskets
    if let Some(level) = level_db.get_by_id(&current_level.0) {
        for platform in &level.platforms {
            match platform {
                crate::levels::PlatformDef::Mirror { x, y, width } => {
                    commands.spawn((
                        Sprite {
                            custom_size: Some(Vec2::new(*width, 20.0)),
                            ..default()
                        },
                        Transform::from_xyz(-x, ARENA_FLOOR_Y + y, 0.0),
                        Platform,
                        Collider,
                    ));
                    commands.spawn((
                        Sprite {
                            custom_size: Some(Vec2::new(*width, 20.0)),
                            ..default()
                        },
                        Transform::from_xyz(*x, ARENA_FLOOR_Y + y, 0.0),
                        Platform,
                        Collider,
                    ));
                }
                crate::levels::PlatformDef::Center { y, width } => {
                    commands.spawn((
                        Sprite {
                            custom_size: Some(Vec2::new(*width, 20.0)),
                            ..default()
                        },
                        Transform::from_xyz(0.0, ARENA_FLOOR_Y + y, 0.0),
                        Platform,
                        Collider,
                    ));
                }
            }
        }

        // Baskets with rims using shared function
        let basket_y = ARENA_FLOOR_Y + level.basket_height;
        let basket_color = Color::srgb(0.5, 0.5, 0.5);
        let rim_color = Color::srgb(0.4, 0.4, 0.4);
        spawn_baskets(
            &mut commands,
            basket_y,
            level.basket_push_in,
            basket_color,
            basket_color,
            rim_color,
            rim_color,
        );
    }

    // Spawn test entities
    let mut ball_holder: Option<(Entity, String)> = None;

    for entity_def in entities {
        match entity_def {
            EntityDef::Player {
                id,
                team,
                x,
                y,
                facing,
                holding_ball,
            } => {
                let team_enum = if team == "left" {
                    Team::Left
                } else {
                    Team::Right
                };
                let target = if team == "left" {
                    Basket::Right
                } else {
                    Basket::Left
                };

                let entity = commands
                    .spawn((
                        Transform::from_translation(Vec3::new(*x, *y, 0.0)),
                        Sprite {
                            custom_size: Some(PLAYER_SIZE),
                            ..default()
                        },
                        Player,
                        Velocity::default(),
                        Grounded(false),
                        CoyoteTimer::default(),
                        JumpState::default(),
                        Facing(*facing),
                        ChargingShot::default(),
                        TargetBasket(target),
                        Collider,
                        team_enum,
                        InputState::default(),
                        StealCooldown::default(),
                        TestEntityId(id.clone()),
                    ))
                    .id();

                capture.entity_map.insert(entity, id.clone());

                if *holding_ball {
                    ball_holder = Some((entity, id.clone()));
                }
            }
            EntityDef::Ball {
                x,
                y,
                velocity_x,
                velocity_y,
            } => {
                commands.spawn((
                    Transform::from_translation(Vec3::new(*x, *y, 0.0)),
                    Sprite {
                        custom_size: Some(BALL_SIZE),
                        ..default()
                    },
                    Ball,
                    BallState::Free,
                    Velocity(Vec2::new(*velocity_x, *velocity_y)),
                    BallPlayerContact::default(),
                    BallPulse::default(),
                    BallRolling::default(),
                    BallShotGrace::default(),
                    BallSpin::default(),
                    BallStyle::new("wedges"),
                ));
            }
        }
    }

    // If a player should hold the ball, spawn it attached
    if let Some((holder_entity, _holder_id)) = ball_holder {
        let ball_entity = commands
            .spawn((
                Transform::default(),
                Sprite {
                    custom_size: Some(BALL_SIZE),
                    ..default()
                },
                Ball,
                BallState::Held(holder_entity),
                Velocity::default(),
                BallPlayerContact::default(),
                BallPulse::default(),
                BallRolling::default(),
                BallShotGrace::default(),
                BallSpin::default(),
                BallStyle::new("wedges"),
            ))
            .id();

        commands
            .entity(holder_entity)
            .insert(HoldingBall(ball_entity));
        capture.prev_ball_holder = Some(holder_entity);
    }
}

/// System to inject scripted inputs
fn input_injection(
    mut scripted: ResMut<ScriptedInputs>,
    mut control: ResMut<TestControl>,
    mut players: Query<(&TestEntityId, &mut InputState)>,
) {
    control.current_frame = scripted.current_frame;
    control.max_frame = scripted.max_frame;

    if !scripted.should_continue() {
        control.should_exit = true;
        return;
    }

    let frame_inputs = scripted.advance_frame();

    for (test_id, mut input_state) in &mut players {
        if let Some(state) = frame_inputs.get(&test_id.0) {
            input_state.move_x = state.move_x;
            if state.jump_pressed {
                input_state.jump_buffer_timer = 0.1;
                input_state.jump_held = true;
            }
            // Only update jump_held if explicitly set, otherwise maintain from jump_pressed
            if !state.jump_pressed {
                input_state.jump_held = state.jump_held;
            }
            input_state.pickup_pressed = state.pickup_pressed;

            // Check throw_released BEFORE updating throw_held
            let was_throwing = input_state.throw_held;
            input_state.throw_held = state.throw_held;
            input_state.throw_released = was_throwing && !state.throw_held;
        } else {
            // Clear one-frame inputs
            input_state.pickup_pressed = false;
            input_state.throw_released = false;
        }
    }
}

/// System to capture events
fn event_capture(
    control: Res<TestControl>,
    mut capture: ResMut<EventCapture>,
    score: Res<Score>,
    steal_contest: Res<StealContest>,
    players: Query<(Entity, &ChargingShot, &StealCooldown, Option<&HoldingBall>), With<Player>>,
    balls: Query<&BallState, With<Ball>>,
) {
    let frame = control.current_frame;

    // Detect score changes (Goal events)
    if score.left > capture.prev_score_left {
        capture.events.push(CapturedEvent {
            frame,
            event_type: "Goal".to_string(),
            player: Some("left".to_string()),
        });
        capture.prev_score_left = score.left;
    }
    if score.right > capture.prev_score_right {
        capture.events.push(CapturedEvent {
            frame,
            event_type: "Goal".to_string(),
            player: Some("right".to_string()),
        });
        capture.prev_score_right = score.right;
    }

    // Detect player events
    for (entity, charging, steal_cooldown, holding) in &players {
        let player_id = capture.entity_map.get(&entity).cloned();

        // Pickup detection
        let is_holding = holding.is_some();
        let was_prev_holder = capture.prev_ball_holder == Some(entity);

        if is_holding && !was_prev_holder && capture.prev_ball_holder.is_none() {
            capture.events.push(CapturedEvent {
                frame,
                event_type: "Pickup".to_string(),
                player: player_id.clone(),
            });
            capture.prev_ball_holder = Some(entity);
        }

        // Shot charging detection
        let is_charging = charging.charge_time > 0.0;
        let was_charging = capture.prev_charging.get(&entity).copied().unwrap_or(false);

        if is_charging && !was_charging {
            capture.events.push(CapturedEvent {
                frame,
                event_type: "ShotStart".to_string(),
                player: player_id.clone(),
            });
        }
        capture.prev_charging.insert(entity, is_charging);

        // Steal detection - detect when cooldown jumps up significantly
        let current_cooldown = steal_cooldown.0;
        let prev_cooldown = capture
            .prev_steal_cooldowns
            .get(&entity)
            .copied()
            .unwrap_or(0.0);

        // Detect out-of-range attempts (shorter cooldown ~0.2s)
        if steal_contest.out_of_range_timer > 0.0
            && steal_contest.out_of_range_entity == Some(entity)
            && current_cooldown > prev_cooldown + 0.1
        {
            capture.events.push(CapturedEvent {
                frame,
                event_type: "StealOutOfRange".to_string(),
                player: player_id.clone(),
            });
        }
        // Detect actual steal attempts (longer cooldown >= 0.25s)
        else if current_cooldown > prev_cooldown + 0.15 && current_cooldown >= 0.25 {
            capture.events.push(CapturedEvent {
                frame,
                event_type: "StealAttempt".to_string(),
                player: player_id.clone(),
            });

            if steal_contest.fail_flash_timer > 0.0 {
                capture.events.push(CapturedEvent {
                    frame,
                    event_type: "StealFail".to_string(),
                    player: player_id.clone(),
                });
            } else {
                capture.events.push(CapturedEvent {
                    frame,
                    event_type: "StealSuccess".to_string(),
                    player: player_id,
                });
            }
        }
        capture
            .prev_steal_cooldowns
            .insert(entity, current_cooldown);
    }

    // Detect shot release (ball becomes InFlight)
    for ball_state in &balls {
        if let BallState::InFlight { shooter, .. } = ball_state {
            if capture.prev_ball_holder.is_some() {
                let player_id = capture.entity_map.get(shooter).cloned();
                capture.events.push(CapturedEvent {
                    frame,
                    event_type: "ShotRelease".to_string(),
                    player: player_id,
                });
                capture.prev_ball_holder = None;
            }
        } else if matches!(ball_state, BallState::Free) && capture.prev_ball_holder.is_some() {
            // Ball was dropped
            let player_id = capture
                .prev_ball_holder
                .and_then(|e| capture.entity_map.get(&e).cloned());
            capture.events.push(CapturedEvent {
                frame,
                event_type: "Drop".to_string(),
                player: player_id,
            });
            capture.prev_ball_holder = None;
        }
    }
}

/// System to check if test should end
fn test_end_check(scripted: Res<ScriptedInputs>, mut control: ResMut<TestControl>) {
    if !scripted.should_continue() {
        control.should_exit = true;
    }
}

/// Extract world state for assertions
fn extract_world_state(world: &mut World) -> WorldState {
    let mut entities = HashMap::new();

    // Query players
    let mut player_query = world.query::<(
        &Transform,
        &Velocity,
        &Grounded,
        Option<&HoldingBall>,
        &TestEntityId,
    )>();

    for (transform, velocity, grounded, holding, test_id) in player_query.iter(world) {
        entities.insert(
            test_id.0.clone(),
            EntityState {
                x: transform.translation.x,
                y: transform.translation.y,
                velocity_x: velocity.0.x,
                velocity_y: velocity.0.y,
                holding_ball: holding.is_some(),
                grounded: grounded.0,
            },
        );
    }

    // Query ball
    let mut ball_query = world.query::<(&Transform, &Velocity, &BallState)>();
    let ball = ball_query
        .iter(world)
        .next()
        .map(|(transform, velocity, state)| AssertionBallState {
            x: transform.translation.x,
            y: transform.translation.y,
            velocity_x: velocity.0.x,
            velocity_y: velocity.0.y,
            state: match state {
                BallState::Free => "Free".to_string(),
                BallState::Held(_) => "Held".to_string(),
                BallState::InFlight { .. } => "InFlight".to_string(),
            },
        });

    // Get score
    let score = world.resource::<Score>();

    WorldState {
        entities,
        ball,
        score_left: score.left,
        score_right: score.right,
    }
}
