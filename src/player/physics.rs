//! Player physics systems

use bevy::prelude::*;
use rand::Rng;

use crate::ai::{AiGoal, AiProfileDatabase, AiState, InputState};
use crate::ball::{
    Ball, BallPlayerContact, BallPulse, BallRolling, BallShotGrace, BallSpin, BallState,
    BallStyle, BallTextures, CurrentPalette,
};
use crate::palettes::PaletteDatabase;
use crate::constants::*;
use crate::helpers::*;
use crate::levels::LevelDatabase;
use crate::levels::{spawn_corner_ramps, spawn_level_platforms};
use crate::player::components::*;
use crate::scoring::CurrentLevel;
use crate::ui::PhysicsTweaks;
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
        ),
        With<Player>,
    >,
    time: Res<Time>,
) {
    for (mut velocity, mut coyote, mut jump_state, mut facing, grounded, mut input) in &mut players
    {
        let move_x = input.move_x;
        let jump_buffer_timer = input.jump_buffer_timer;
        let jump_held = input.jump_held;

        // Acceleration-based horizontal movement
        let target_speed = move_x * tweaks.move_speed;
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

        velocity.0.x = move_toward(current_speed, target_speed, rate * time.delta_secs());

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
            coyote.0 = (coyote.0 - time.delta_secs()).max(0.0);
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
    for (mut velocity, grounded) in &mut query {
        if !grounded.0 {
            // Fast fall: use higher gravity when falling than rising
            let gravity = if velocity.0.y > 0.0 {
                tweaks.gravity_rise
            } else {
                tweaks.gravity_fall
            };
            velocity.0.y -= gravity * time.delta_secs();
        }
    }
}

/// Check player collisions with platforms
pub fn check_collisions(
    mut player_query: Query<(&mut Transform, &mut Velocity, &mut Grounded, &Sprite), With<Player>>,
    platform_query: Query<
        (&GlobalTransform, &Sprite),
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

        for (platform_global, platform_sprite) in &platform_query {
            let platform_size = platform_sprite
                .custom_size
                .unwrap_or(Vec2::new(100.0, 20.0));
            let platform_half = platform_size / 2.0;

            let player_pos = player_transform.translation.truncate();
            let platform_pos = platform_global.translation().truncate();

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

/// Handle player respawn and level changes
#[allow(clippy::too_many_arguments)]
pub fn respawn_player(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut commands: Commands,
    level_db: Res<LevelDatabase>,
    palette_db: Res<PaletteDatabase>,
    profile_db: Res<AiProfileDatabase>,
    mut current_level: ResMut<CurrentLevel>,
    current_palette: Res<CurrentPalette>,
    mut score: ResMut<crate::scoring::Score>,
    ball_textures: Res<BallTextures>,
    mut players: Query<
        (Entity, &mut Transform, &mut Velocity, Option<&HoldingBall>, &Team),
        With<Player>,
    >,
    mut ai_players: Query<&mut AiState, (With<Player>, Without<HumanControlled>)>,
    ball_query: Query<Entity, With<Ball>>,
    level_platforms: Query<Entity, With<LevelPlatform>>,
    corner_ramps: Query<Entity, With<CornerRamp>>,
    mut baskets: Query<&mut Transform, (With<Basket>, Without<Player>, Without<Ball>)>,
) {
    // Reset current level (R / Start) - resets positions and score only
    let reset_pressed = keyboard.just_pressed(KeyCode::KeyR)
        || gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::Start));

    // Cycle to next level (] key only - controller uses unified cycle system)
    let next_level_pressed = keyboard.just_pressed(KeyCode::BracketRight);

    // Cycle to previous level ([ key only - controller uses unified cycle system)
    let prev_level_pressed = keyboard.just_pressed(KeyCode::BracketLeft);

    // Detect if level was changed externally (by unified cycle system)
    let num_levels = level_db.len() as u32;
    let level_changed_externally = current_level.is_changed() && !reset_pressed;

    if next_level_pressed {
        current_level.0 = (current_level.0 % num_levels) + 1;
    } else if prev_level_pressed {
        current_level.0 = if current_level.0 <= 1 {
            num_levels
        } else {
            current_level.0 - 1
        };
    }

    let level_changed = next_level_pressed || prev_level_pressed || level_changed_externally;

    // Reset: just reset positions and score, keep current palette/level
    if reset_pressed {
        // Reset score
        score.left = 0;
        score.right = 0;

        // Reset player positions
        for (player_entity, mut p_transform, mut p_velocity, holding, team) in &mut players {
            // Use Team component to determine spawn position
            p_transform.translation = match team {
                Team::Left => PLAYER_SPAWN_LEFT,
                Team::Right => PLAYER_SPAWN_RIGHT,
            };
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

        let level_index = (current_level.0 - 1) as usize;
        let is_debug = level_db
            .get(level_index)
            .map(|l| l.debug)
            .unwrap_or(false);

        spawn_balls(
            &mut commands,
            &ball_textures,
            current_palette.0,
            is_debug,
        );

        // Randomize AI profile on reset
        let num_profiles = profile_db.len();
        for mut ai_state in &mut ai_players {
            ai_state.profile_index = rand::thread_rng().gen_range(0..num_profiles);
            let profile = profile_db.get(ai_state.profile_index);
            info!("AI reset with profile: {}", profile.name);
        }
    }

    // Level change: update geometry and reset positions
    if level_changed {
        let level_index = (current_level.0 - 1) as usize;

        // Get palette for new geometry colors
        let palette = palette_db
            .get(current_palette.0)
            .expect("Palette index out of bounds");

        // Reset player positions
        for (player_entity, mut p_transform, mut p_velocity, holding, team) in &mut players {
            p_transform.translation = match team {
                Team::Left => PLAYER_SPAWN_LEFT,
                Team::Right => PLAYER_SPAWN_RIGHT,
            };
            p_velocity.0 = Vec2::ZERO;

            if holding.is_some() {
                commands.entity(player_entity).remove::<HoldingBall>();
            }
        }

        // Respawn balls for new level
        for ball_entity in &ball_query {
            commands.entity(ball_entity).despawn();
        }

        let is_debug = level_db
            .get(level_index)
            .map(|l| l.debug)
            .unwrap_or(false);

        spawn_balls(
            &mut commands,
            &ball_textures,
            current_palette.0,
            is_debug,
        );

        // Despawn old level platforms and spawn new ones
        for entity in &level_platforms {
            commands.entity(entity).despawn();
        }
        spawn_level_platforms(&mut commands, &level_db, level_index, palette.platforms);

        // Update basket positions for new level
        if let Some(level) = level_db.get(level_index) {
            let basket_y = ARENA_FLOOR_Y + level.basket_height;
            let (left_x, right_x) = basket_x_from_offset(level.basket_push_in);

            for mut basket_transform in &mut baskets {
                // Determine which basket by X position
                if basket_transform.translation.x < 0.0 {
                    basket_transform.translation.x = left_x;
                } else {
                    basket_transform.translation.x = right_x;
                }
                basket_transform.translation.y = basket_y;
            }

            // Despawn old corner ramps and spawn new ones
            for entity in &corner_ramps {
                commands.entity(entity).despawn();
            }
            spawn_corner_ramps(
                &mut commands,
                level.step_count,
                level.corner_height,
                level.corner_width,
                level.step_push_in,
                palette.platforms,
            );

            // Update AI goals based on debug status
            let new_goal = if level.debug {
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

/// Helper to spawn balls at starting positions
fn spawn_balls(
    commands: &mut Commands,
    ball_textures: &BallTextures,
    palette_index: usize,
    is_debug: bool,
) {
    if is_debug {
        // Debug level: spawn ALL ball styles dynamically
        let num_styles = ball_textures.len();
        let total_width = 1200.0;
        let spacing = if num_styles > 1 {
            total_width / (num_styles - 1) as f32
        } else {
            0.0
        };
        let start_x = -total_width / 2.0;

        for (i, style_name) in ball_textures.style_order.iter().enumerate() {
            let x = start_x + (i as f32 * spacing);
            if let Some(textures) = ball_textures.get(style_name) {
                if let Some(texture) = textures.textures.get(palette_index) {
                    commands.spawn((
                        Sprite {
                            image: texture.clone(),
                            custom_size: Some(BALL_SIZE),
                            ..default()
                        },
                        Transform::from_xyz(x, ARENA_FLOOR_Y + BALL_SIZE.y / 2.0 + 20.0, 2.0),
                        Ball,
                        BallState::default(),
                        Velocity::default(),
                        BallPlayerContact::default(),
                        BallPulse::default(),
                        BallRolling::default(),
                        BallShotGrace::default(),
                        BallSpin::default(),
                        BallStyle::new(style_name),
                    ));
                }
            }
        }
    } else {
        // Normal level: spawn single ball with first style
        let default_style = ball_textures.default_style().cloned().unwrap_or_else(|| "wedges".to_string());
        if let Some(textures) = ball_textures.get(&default_style) {
            if let Some(texture) = textures.textures.get(palette_index) {
                commands.spawn((
                    Sprite {
                        image: texture.clone(),
                        custom_size: Some(BALL_SIZE),
                        ..default()
                    },
                    Transform::from_translation(BALL_SPAWN),
                    Ball,
                    BallState::default(),
                    Velocity::default(),
                    BallPlayerContact::default(),
                    BallPulse::default(),
                    BallRolling::default(),
                    BallShotGrace::default(),
                    BallSpin::default(),
                    BallStyle::new(&default_style),
                ));
            }
        }
    }
}
