//! Player physics systems

use bevy::prelude::*;
use rand::Rng;

use crate::ai::{AiGoal, AiProfileDatabase, AiState, InputState};
use crate::ball::{
    Ball, BallPlayerContact, BallPulse, BallRolling, BallShotGrace, BallSpin, BallState, BallStyle,
    BallTextures, CurrentPalette,
};
use crate::constants::*;
use crate::helpers::*;
use crate::levels::{LevelDatabase, reload_level_geometry};
use crate::palettes::PaletteDatabase;
use crate::player::components::*;
use crate::player::spawn::spawn_position;
use crate::scoring::CurrentLevel;
use crate::tuning::PhysicsTweaks;
use crate::world::{Basket, BasketRim, CornerRamp, LevelPlatform, Platform};

/// Runs in FixedUpdate to apply captured input to physics.
/// All players read from their InputState component (human input is copied there).
pub fn apply_input(
    tweaks: Res<PhysicsTweaks>,
    mut players: Query<
        (
            &mut Velocity,
            &mut CoyoteTimer,
            &mut JumpState,
            &mut Facing,
            &Grounded,
            &mut InputState,
            &TurboGauge,
            &BlockState,
        ),
        With<Player>,
    >,
    time: Res<Time>,
) {
    // Use a minimum dt for headless mode where time.delta_secs() returns 0 or tiny values
    // In windowed mode, this will use the actual delta. In headless, it enforces 60Hz behavior.
    let dt = time.delta_secs().max(1.0 / 60.0);

    for (mut velocity, mut coyote, mut jump_state, mut facing, grounded, mut input, turbo, block) in
        &mut players
    {
        let move_x = input.move_x;
        let jump_buffer_timer = input.jump_buffer_timer;
        let jump_held = input.jump_held;

        // Calculate turbo speed multiplier: active if holding turbo and has gauge
        let turbo_active = input.turbo_held && turbo.can_use();
        let speed_mult = if turbo_active {
            TURBO_SPEED_MULTIPLIER
        } else {
            1.0
        };

        // Apply block slowdown when blocking
        let block_mult = if block.active {
            BLOCK_HORIZONTAL_SLOW_FACTOR
        } else {
            1.0
        };

        // Acceleration-based horizontal movement with turbo and block multipliers
        let target_speed = move_x * tweaks.move_speed * speed_mult * block_mult;
        let current_speed = velocity.0.x;

        // Determine if accelerating (toward input) or decelerating (stopping/reversing)
        let has_input = move_x.abs() > STICK_DEADZONE;
        let same_direction =
            target_speed.signum() == current_speed.signum() || current_speed.abs() < 1.0;
        let is_accelerating = has_input && same_direction;

        // Select appropriate acceleration rate based on ground state and direction
        let rate = if grounded.0 {
            if is_accelerating {
                tweaks.ground_accel
            } else {
                tweaks.ground_decel
            }
        } else {
            if is_accelerating {
                tweaks.air_accel
            } else {
                tweaks.air_decel
            }
        };

        velocity.0.x = move_toward(current_speed, target_speed, rate * dt);

        // Update facing direction based on input (not velocity, so turning feels responsive)
        if move_x > STICK_DEADZONE {
            facing.0 = 1.0;
        } else if move_x < -STICK_DEADZONE {
            facing.0 = -1.0;
        }

        // Update coyote timer
        if grounded.0 {
            coyote.0 = COYOTE_TIME;
            jump_state.is_jumping = false; // Reset jump state when grounded
        } else {
            coyote.0 = (coyote.0 - dt).max(0.0);
        }

        // Can jump if grounded OR within coyote time
        let can_jump = grounded.0 || coyote.0 > 0.0;

        // Jump if we have buffered input and can jump
        if jump_buffer_timer > 0.0 && can_jump {
            velocity.0.y = tweaks.jump_velocity;
            // Consume the buffered jump
            input.jump_buffer_timer = 0.0;
            coyote.0 = 0.0; // Consume coyote time so we can't double jump
            jump_state.is_jumping = true; // Mark that we're in a jump
        }

        // Variable jump height: cut velocity if button released while rising
        // Check: in a jump + rising + button NOT held = cut velocity
        if jump_state.is_jumping && velocity.0.y > 0.0 && !jump_held {
            velocity.0.y *= JUMP_CUT_MULTIPLIER;
            jump_state.is_jumping = false; // Only cut once per jump
        }
    }
}

/// Apply gravity to player
pub fn apply_gravity(
    tweaks: Res<PhysicsTweaks>,
    mut query: Query<(&mut Velocity, &Grounded), With<Player>>,
    time: Res<Time>,
) {
    // Use minimum dt for headless mode compatibility
    let dt = time.delta_secs().max(1.0 / 60.0);

    for (mut velocity, grounded) in &mut query {
        if !grounded.0 {
            // Fast fall: use higher gravity when falling than rising
            let gravity = if velocity.0.y > 0.0 {
                tweaks.gravity_rise
            } else {
                tweaks.gravity_fall
            };
            velocity.0.y -= gravity * dt;
        }
    }
}

/// Update turbo gauge: drain while held, refill when released
/// Runs in FixedUpdate for consistent behavior
pub fn turbo_update(
    mut players: Query<(&mut TurboGauge, &InputState), With<Player>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs().max(1.0 / 60.0);

    for (mut turbo, input) in &mut players {
        if input.turbo_held {
            turbo.drain(dt);
        } else {
            turbo.refill(dt);
        }
    }
}

/// Update block state: handle activation, timer countdown, and cooldown
/// Runs in FixedUpdate for consistent behavior
pub fn block_update(
    mut players: Query<(&mut BlockState, &mut InputState, Option<&HoldingBall>), With<Player>>,
    time: Res<Time>,
) {
    let dt = time.delta_secs().max(1.0 / 60.0);

    for (mut block, mut input, holding_ball) in &mut players {
        // Can only initiate block if:
        // - Block button pressed
        // - Not holding a ball (modal: RB is shoot when holding, block when not)
        // - Block is not on cooldown
        if input.block_pressed {
            input.block_pressed = false; // Consume the input

            // Only block if not holding ball and can block
            if holding_ball.is_none() && block.can_block() {
                block.start_block(BLOCK_DURATION);
            }
        }

        // Update block timers
        block.update(dt, BLOCK_COOLDOWN);
    }
}

/// Check player collisions with platforms
pub fn check_collisions(
    mut player_query: Query<(&mut Transform, &mut Velocity, &mut Grounded, &Sprite), With<Player>>,
    platform_query: Query<
        (&Transform, &Sprite),
        (With<Platform>, Without<Player>, Without<BasketRim>),
    >,
) {
    for (mut player_transform, mut player_velocity, mut grounded, player_sprite) in
        &mut player_query
    {
        let player_size = player_sprite.custom_size.unwrap_or(PLAYER_SIZE);
        let player_half = player_size / 2.0;

        // Assume not grounded until we find a floor beneath us
        grounded.0 = false;

        for (platform_transform, platform_sprite) in &platform_query {
            let platform_size = platform_sprite
                .custom_size
                .unwrap_or(Vec2::new(100.0, 20.0));
            let platform_half = platform_size / 2.0;

            let player_pos = player_transform.translation.truncate();
            let platform_pos = platform_transform.translation.truncate();

            // Calculate overlap
            let diff = player_pos - platform_pos;
            let overlap_x = player_half.x + platform_half.x - diff.x.abs();
            let overlap_y = player_half.y + platform_half.y - diff.y.abs();

            // No collision
            if overlap_x <= 0.0 || overlap_y <= 0.0 {
                continue;
            }

            // Resolve collision along the smallest overlap axis
            if overlap_y < overlap_x {
                // Vertical collision
                if diff.y > 0.0 {
                    // Player is above - land on platform
                    // Position slightly inside (EPSILON) so next frame still detects collision
                    player_transform.translation.y =
                        platform_pos.y + platform_half.y + player_half.y - COLLISION_EPSILON;
                    if player_velocity.0.y <= 0.0 {
                        player_velocity.0.y = 0.0;
                        grounded.0 = true;
                    }
                } else {
                    // Player hit ceiling
                    player_transform.translation.y =
                        platform_pos.y - platform_half.y - player_half.y + COLLISION_EPSILON;
                    if player_velocity.0.y > 0.0 {
                        player_velocity.0.y = 0.0;
                    }
                }
            } else {
                // Horizontal collision - push player out
                if diff.x > 0.0 {
                    player_transform.translation.x =
                        platform_pos.x + platform_half.x + player_half.x - COLLISION_EPSILON;
                } else {
                    player_transform.translation.x =
                        platform_pos.x - platform_half.x - player_half.x + COLLISION_EPSILON;
                }
                // Don't zero horizontal velocity - let player slide along walls
            }
        }
    }
}

/// Handle player-to-player collisions (2v2 mode)
///
/// Collision rules:
/// - Same team: full pass-through, no collision
/// - Single opponent: soft collision with proportional drag, can pass through
/// - Two overlapping opponents: hard block at their intersection, eject if inside
pub fn player_player_collision(
    mut players: Query<(Entity, &mut Transform, &mut Velocity, &Sprite, &Team), With<Player>>,
    time: Res<Time>,
) {
    // Use minimum dt for headless mode compatibility
    let dt = time.delta_secs().max(1.0 / 60.0);

    // Collect all player data first to avoid borrow issues
    let player_data: Vec<(Entity, Vec3, Vec2, Vec2, Team)> = players
        .iter()
        .map(|(e, t, v, s, team)| {
            (
                e,
                t.translation,
                v.0,
                s.custom_size.unwrap_or(PLAYER_SIZE),
                *team,
            )
        })
        .collect();

    // For each player, find overlapping opponents and apply collision rules
    for i in 0..player_data.len() {
        let (entity_i, pos_i, _vel_i, size_i, team_i) = &player_data[i];
        let half_i = *size_i / 2.0;

        // Find all opponents overlapping with this player
        let mut overlapping_opponents: Vec<(usize, f32, f32)> = Vec::new(); // (index, overlap_x, overlap_y)

        for (j, (_, pos_j, _, size_j, team_j)) in player_data.iter().enumerate() {
            if i == j {
                continue;
            }

            // Skip same team - full pass-through
            if team_i == team_j {
                continue;
            }

            let half_j = *size_j / 2.0;
            let diff = pos_i.truncate() - pos_j.truncate();
            let overlap_x = half_i.x + half_j.x - diff.x.abs();
            let overlap_y = half_i.y + half_j.y - diff.y.abs();

            // Check for collision (only horizontal matters for our rules)
            if overlap_x > 0.0 && overlap_y > 0.0 {
                overlapping_opponents.push((j, overlap_x, overlap_y));
            }
        }

        // No opponents overlapping - nothing to do
        if overlapping_opponents.is_empty() {
            continue;
        }

        // Check if two opponents overlap each other (creating a hard block zone)
        let mut hard_block = false;
        let mut hard_block_push_dir = 0.0f32;

        if overlapping_opponents.len() >= 2 {
            // Check if the two opponents overlap each other
            let (idx_a, _, _) = overlapping_opponents[0];
            let (idx_b, _, _) = overlapping_opponents[1];

            let (_, pos_a, _, size_a, _) = &player_data[idx_a];
            let (_, pos_b, _, size_b, _) = &player_data[idx_b];

            let half_a = *size_a / 2.0;
            let half_b = *size_b / 2.0;

            let diff_ab = pos_a.truncate() - pos_b.truncate();
            let overlap_ab_x = half_a.x + half_b.x - diff_ab.x.abs();
            let overlap_ab_y = half_a.y + half_b.y - diff_ab.y.abs();

            if overlap_ab_x > 0.0 && overlap_ab_y > 0.0 {
                // Two opponents overlap - compute their intersection rectangle
                let left_a = pos_a.x - half_a.x;
                let right_a = pos_a.x + half_a.x;
                let left_b = pos_b.x - half_b.x;
                let right_b = pos_b.x + half_b.x;

                let intersection_left = left_a.max(left_b);
                let intersection_right = right_a.min(right_b);

                // Check if player_i overlaps with this intersection
                let left_i = pos_i.x - half_i.x;
                let right_i = pos_i.x + half_i.x;

                let player_in_intersection =
                    right_i > intersection_left && left_i < intersection_right;

                if player_in_intersection {
                    hard_block = true;
                    // Determine push direction: push player out of intersection
                    let player_center = pos_i.x;
                    let intersection_center = (intersection_left + intersection_right) / 2.0;
                    hard_block_push_dir = if player_center > intersection_center {
                        1.0
                    } else {
                        -1.0
                    };
                }
            }
        }

        if hard_block {
            // Hard block: eject player from the intersection
            // Find the nearest edge of the intersection and push player there
            let (idx_a, _, _) = overlapping_opponents[0];
            let (idx_b, _, _) = overlapping_opponents[1];

            let (_, pos_a, _, size_a, _) = &player_data[idx_a];
            let (_, pos_b, _, size_b, _) = &player_data[idx_b];

            let half_a = *size_a / 2.0;
            let half_b = *size_b / 2.0;

            let left_a = pos_a.x - half_a.x;
            let right_a = pos_a.x + half_a.x;
            let left_b = pos_b.x - half_b.x;
            let right_b = pos_b.x + half_b.x;

            let intersection_left = left_a.max(left_b);
            let intersection_right = right_a.min(right_b);

            // Calculate how far to push
            let target_x = if hard_block_push_dir > 0.0 {
                intersection_right + half_i.x + COLLISION_EPSILON
            } else {
                intersection_left - half_i.x - COLLISION_EPSILON
            };

            if let Ok((_, mut trans_i, mut vel_i, _, _)) = players.get_mut(*entity_i) {
                trans_i.translation.x = target_x;
                // Stop horizontal velocity when hitting hard block
                vel_i.0.x = 0.0;
            }
        } else {
            // Soft collision: apply drag proportional to overlap depth
            // Calculate total drag based on deepest overlap with any single opponent
            let mut max_overlap_ratio = 0.0f32;

            for (idx_j, overlap_x, overlap_y) in &overlapping_opponents {
                // Only apply horizontal collision (vertical pass-through allowed)
                if *overlap_x >= *overlap_y {
                    continue;
                }

                let (_, _, _, size_j, _) = &player_data[*idx_j];
                let half_j = *size_j / 2.0;

                // Overlap ratio: how deep into opponent (0 = edge, 1 = center)
                let max_possible_overlap = half_i.x + half_j.x;
                let overlap_ratio = (*overlap_x / max_possible_overlap).min(1.0);
                max_overlap_ratio = max_overlap_ratio.max(overlap_ratio);
            }

            if max_overlap_ratio > 0.0 {
                // Apply drag: 1.0 at edge, PLAYER_SOFT_COLLISION_DRAG at full overlap
                // drag_factor = 1.0 - (1.0 - base_drag) * overlap_ratio
                // For frame-rate independence, use powf(dt)
                let base_drag = PLAYER_SOFT_COLLISION_DRAG;
                let drag_per_second = 1.0 - (1.0 - base_drag) * max_overlap_ratio;
                let drag_factor = drag_per_second.powf(dt);

                // Apply drag to this player
                if let Ok((_, _, mut vel_i, _, _)) = players.get_mut(*entity_i) {
                    vel_i.0.x *= drag_factor;
                }

                // Apply drag to overlapping opponents too
                for (idx_j, overlap_x, overlap_y) in &overlapping_opponents {
                    if *overlap_x >= *overlap_y {
                        continue;
                    }
                    let (entity_j, _, _, _, _) = &player_data[*idx_j];
                    if let Ok((_, _, mut vel_j, _, _)) = players.get_mut(*entity_j) {
                        vel_j.0.x *= drag_factor;
                    }
                }
            }
        }
    }
}

/// Handle Start button: opens pause menu (menu handles closing)
pub fn check_pause_toggle(
    gamepads: Query<&Gamepad>,
    mut game_paused: ResMut<crate::scoring::GamePaused>,
) {
    // Only open pause menu, not close it (menu handles that)
    // Also skip if game was just unpaused this frame (prevents re-pause on same Start press)
    if game_paused.0 || game_paused.is_changed() {
        return;
    }

    // Check for Start button press
    let start_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::Start));

    if start_pressed {
        game_paused.0 = true;
        info!("Game PAUSED");
    }
}

/// Handle Escape key to quit the game
pub fn check_quit(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        info!("Quit requested via Escape");
        app_exit.write(AppExit::Success);
    }
}

/// Handle player respawn and level changes
#[allow(clippy::too_many_arguments)]
pub fn respawn_player(
    mut commands: Commands,
    level_db: Res<LevelDatabase>,
    palette_db: Res<PaletteDatabase>,
    profile_db: Res<AiProfileDatabase>,
    mut current_level: ResMut<CurrentLevel>,
    current_palette: Res<CurrentPalette>,
    mut score: ResMut<crate::scoring::Score>,
    ball_textures: Res<BallTextures>,
    mut restart_requested: ResMut<crate::scoring::RestartRequested>,
    mut countdown: Option<ResMut<crate::countdown::MatchCountdown>>,
    mut players: Query<
        (
            Entity,
            &mut Transform,
            &mut Velocity,
            Option<&HoldingBall>,
            &Character,
        ),
        With<Player>,
    >,
    mut ai_players: Query<&mut AiState, With<Player>>,
    ball_query: Query<Entity, With<Ball>>,
    level_platforms: Query<Entity, With<LevelPlatform>>,
    corner_ramps: Query<Entity, With<CornerRamp>>,
    mut baskets: Query<&mut Transform, (With<Basket>, Without<Player>, Without<Ball>)>,
) {
    // Reset current level (triggered by Restart Level from pause menu)
    let reset_pressed = restart_requested.0;
    if reset_pressed {
        restart_requested.0 = false; // Consume the flag
        // Restart countdown if it exists
        if let Some(ref mut cd) = countdown {
            cd.start();
        }
    }

    // Level cycling handled by unified cycle system (controller only)
    let next_level_pressed = false;
    let prev_level_pressed = false;

    // Detect if level was changed externally (by unified cycle system)
    let level_changed_externally = current_level.is_changed() && !reset_pressed;

    // Handle level cycling with IDs
    let level_ids: Vec<String> = level_db.all().iter().map(|l| l.id.clone()).collect();
    let current_idx = level_ids
        .iter()
        .position(|id| *id == current_level.0)
        .unwrap_or(0);
    let num_levels = level_ids.len();

    if next_level_pressed {
        let next_idx = (current_idx + 1) % num_levels;
        current_level.0 = level_ids[next_idx].clone();
    } else if prev_level_pressed {
        let prev_idx = if current_idx == 0 {
            num_levels - 1
        } else {
            current_idx - 1
        };
        current_level.0 = level_ids[prev_idx].clone();
    }

    let level_changed = next_level_pressed || prev_level_pressed || level_changed_externally;

    // Reset: just reset positions and score, keep current palette/level
    if reset_pressed {
        // Reset score
        score.left = 0;
        score.right = 0;

        // Reset player positions
        for (player_entity, mut p_transform, mut p_velocity, holding, character) in &mut players {
            // Use Character component to determine spawn position
            p_transform.translation = spawn_position(character.0);
            p_velocity.0 = Vec2::ZERO;

            // Drop ball if holding
            if holding.is_some() {
                commands.entity(player_entity).remove::<HoldingBall>();
            }
        }

        // Reset ball positions (despawn and respawn at starting positions)
        for ball_entity in &ball_query {
            commands.entity(ball_entity).despawn();
        }

        let level_data = level_db.get_by_id(&current_level.0);
        let is_debug = level_data.map(|l| l.debug).unwrap_or(false);
        let ball_start = level_data.and_then(|l| l.ball_start);

        spawn_balls(
            &mut commands,
            &ball_textures,
            current_palette.0,
            is_debug,
            ball_start,
        );

        // Randomize AI profile on reset
        let profiles: Vec<String> = profile_db.profiles().iter().map(|p| p.id.clone()).collect();
        for mut ai_state in &mut ai_players {
            let random_idx = rand::thread_rng().gen_range(0..profiles.len());
            ai_state.profile_id = profiles[random_idx].clone();
            if let Some(profile) = profile_db.get_by_id(&ai_state.profile_id) {
                info!("AI reset with profile: {}", profile.name);
            }
        }
    }

    // Level change: update geometry and reset positions
    if level_changed {
        // Reset score on level change
        score.left = 0;
        score.right = 0;

        // Get palette for new geometry colors
        let palette = palette_db
            .get(current_palette.0)
            .expect("Palette index out of bounds");

        // Reset player positions
        for (player_entity, mut p_transform, mut p_velocity, holding, character) in &mut players {
            p_transform.translation = spawn_position(character.0);
            p_velocity.0 = Vec2::ZERO;

            if holding.is_some() {
                commands.entity(player_entity).remove::<HoldingBall>();
            }
        }

        // Respawn balls for new level
        for ball_entity in &ball_query {
            commands.entity(ball_entity).despawn();
        }

        let new_level_data = level_db.get_by_id(&current_level.0);
        let is_debug = new_level_data.map(|l| l.debug).unwrap_or(false);
        let ball_start = new_level_data.and_then(|l| l.ball_start);

        spawn_balls(
            &mut commands,
            &ball_textures,
            current_palette.0,
            is_debug,
            ball_start,
        );

        // Reload level geometry (platforms + corner ramps)
        if let Some((left_x, right_x, basket_y)) = reload_level_geometry(
            &mut commands,
            &level_db,
            &current_level.0,
            palette.platforms,
            level_platforms.iter(),
            corner_ramps.iter(),
        ) {
            // Update basket positions
            for mut basket_transform in &mut baskets {
                // Determine which basket by X position
                if basket_transform.translation.x < 0.0 {
                    basket_transform.translation.x = left_x;
                } else {
                    basket_transform.translation.x = right_x;
                }
                basket_transform.translation.y = basket_y;
            }

            // Update AI goals based on debug status
            let is_debug = level_db
                .get_by_id(&current_level.0)
                .map(|l| l.debug)
                .unwrap_or(false);
            let new_goal = if is_debug {
                AiGoal::Idle
            } else {
                AiGoal::default()
            };
            for mut ai_state in &mut ai_players {
                ai_state.current_goal = new_goal;
            }
        }
    }
}

/// Helper to spawn a single playable ball at the starting position.
/// Debug level shelf displays are handled separately in main.rs setup.
fn spawn_balls(
    commands: &mut Commands,
    ball_textures: &BallTextures,
    palette_index: usize,
    is_debug: bool,
    ball_start: Option<Vec2>,
) {
    // Pick ball style: random for debug level, default for normal levels
    let style_name = if is_debug {
        let num_styles = ball_textures.len();
        let random_idx = rand::random::<usize>() % num_styles;
        ball_textures.style_order.get(random_idx).cloned()
    } else {
        ball_textures.default_style().cloned()
    }
    .unwrap_or_else(|| "wedges".to_string());

    // Calculate spawn position from level config (if set) or use default
    let spawn_pos = ball_start
        .map(|pos| Vec3::new(pos.x, ARENA_FLOOR_Y + pos.y, BALL_SPAWN.z))
        .unwrap_or(BALL_SPAWN);

    if let Some(textures) = ball_textures.get(&style_name) {
        if let Some(texture) = textures.textures.get(palette_index) {
            commands.spawn((
                Sprite {
                    image: texture.clone(),
                    custom_size: Some(BALL_SIZE),
                    ..default()
                },
                Transform::from_translation(spawn_pos),
                Ball,
                BallState::default(),
                Velocity::default(),
                BallPlayerContact::default(),
                BallPulse::default(),
                BallRolling::default(),
                BallShotGrace::default(),
                BallSpin::default(),
                BallStyle::new(&style_name),
            ));
        }
    }
}

/// Manage debug level display entities when changing levels.
/// Despawns when leaving debug level, spawns when entering debug level.
pub fn manage_debug_display(
    mut commands: Commands,
    current_level: Res<CurrentLevel>,
    level_db: Res<LevelDatabase>,
    ball_textures: Res<BallTextures>,
    current_palette: Res<CurrentPalette>,
    display_balls: Query<Entity, With<crate::ball::DisplayBall>>,
    ball_labels: Query<Entity, With<crate::ball::BallLabel>>,
) {
    // Only run when level changes
    if !current_level.is_changed() {
        return;
    }

    let is_debug = level_db
        .get_by_id(&current_level.0)
        .map(|l| l.debug)
        .unwrap_or(false);
    let has_display_balls = !display_balls.is_empty();

    if is_debug && !has_display_balls {
        // Entering debug level: spawn display balls and labels
        spawn_debug_display(&mut commands, &ball_textures, current_palette.0);
    } else if !is_debug && has_display_balls {
        // Leaving debug level: despawn all display balls and labels
        for entity in &display_balls {
            commands.entity(entity).despawn();
        }
        for entity in &ball_labels {
            commands.entity(entity).despawn();
        }
    }
}

/// Spawn all ball styles on shelf platforms for debug level display
pub fn spawn_debug_display(
    commands: &mut Commands,
    ball_textures: &BallTextures,
    palette_index: usize,
) {
    use crate::ball::{BallLabel, DisplayBall, DisplayBallSpin};

    let shelf_heights = [380.0, 480.0, 580.0, 680.0, 780.0];
    let num_shelves = shelf_heights.len();
    let num_styles = ball_textures.len();
    let balls_per_shelf = (num_styles + num_shelves - 1) / num_shelves;
    let shelf_width = 1100.0;

    for (i, style_name) in ball_textures.style_order.iter().enumerate() {
        let shelf_idx = i / balls_per_shelf;
        let pos_in_shelf = i % balls_per_shelf;
        let balls_this_shelf = if shelf_idx == num_shelves - 1 {
            num_styles - shelf_idx * balls_per_shelf
        } else {
            balls_per_shelf
        };

        if shelf_idx >= num_shelves {
            break;
        }

        let spacing = if balls_this_shelf > 1 {
            shelf_width / (balls_this_shelf - 1) as f32
        } else {
            0.0
        };
        let x = -shelf_width / 2.0 + pos_in_shelf as f32 * spacing;
        let y = ARENA_FLOOR_Y + shelf_heights[shelf_idx] + BALL_SIZE.y / 2.0 + 10.0;

        if let Some(textures) = ball_textures.get(style_name) {
            if let Some(texture) = textures.textures.get(palette_index) {
                // Spawn display ball
                commands.spawn((
                    Sprite {
                        image: texture.clone(),
                        custom_size: Some(BALL_SIZE),
                        ..default()
                    },
                    Transform::from_xyz(x, y, 2.0),
                    DisplayBall {
                        row: shelf_idx,
                        col: pos_in_shelf,
                        total_rows: num_shelves,
                    },
                    DisplayBallSpin::default(),
                    BallStyle::new(style_name),
                ));

                // Spawn label above ball
                commands.spawn((
                    Text2d::new(style_name.clone()),
                    TextFont {
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(TEXT_SECONDARY),
                    Transform::from_xyz(x, y + BALL_SIZE.y / 2.0 + 8.0, 3.0),
                    BallLabel,
                ));
            }
        }
    }
}
