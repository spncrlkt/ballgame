use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};
use rand::Rng;

// Visual constants
const BACKGROUND_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const FLOOR_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);
const PLATFORM_COLOR: Color = Color::srgb(0.5, 0.3, 0.3);
const PLAYER_COLOR: Color = Color::srgb(0.3, 0.3, 0.7);
const BALL_COLOR: Color = Color::srgb(0.9, 0.5, 0.1); // Orange basketball-ish

// Size constants
const PLAYER_SIZE: Vec2 = Vec2::new(32.0, 64.0);
const BALL_SIZE: Vec2 = Vec2::new(24.0, 24.0);

// Physics constants
const GRAVITY_RISE: f32 = 980.0; // Gravity while rising
const GRAVITY_FALL: f32 = 1400.0; // Gravity while falling (fast fall)
const JUMP_VELOCITY: f32 = 650.0; // Full jump height (hold button)
const JUMP_CUT_MULTIPLIER: f32 = 0.4; // Velocity multiplier when releasing jump early
const MOVE_SPEED: f32 = 300.0;
const COLLISION_EPSILON: f32 = 0.5; // Skin width for collision detection

// Game feel constants
const COYOTE_TIME: f32 = 0.1; // Seconds after leaving ground you can still jump
const JUMP_BUFFER_TIME: f32 = 0.1; // Seconds before landing that jump input is remembered

// Ball physics
const BALL_GRAVITY: f32 = 800.0;
const BALL_BOUNCE: f32 = 0.7; // Coefficient of restitution (0 = no bounce, 1 = perfect bounce)
const BALL_FRICTION: f32 = 0.98; // Horizontal velocity retention per frame
const BALL_MIN_BOUNCE_VEL: f32 = 50.0; // Below this, ball stops bouncing
const BALL_PICKUP_RADIUS: f32 = 100.0; // How close player must be to pick up ball (forgiving)
const BALL_FREE_SPEED: f32 = 100.0; // Ball becomes Free when speed drops below this

// Shooting - heights: tap=2x player height (128), full=6x player height (384)
// Using h = v_y²/(2g), v_y = sqrt(2*g*h): tap needs v_y≈452, full needs v_y≈784
const SHOT_MIN_POWER: f32 = 380.0; // Minimum throw velocity (tap shot: 380*1.2=456 → h≈130)
const SHOT_MAX_POWER: f32 = 660.0; // Maximum throw velocity (full charge: 660*1.2=792 → h≈392)
const SHOT_CHARGE_TIME: f32 = 2.0; // Seconds to reach full charge
const SHOT_BASE_ARC: f32 = 1.2; // Upward component multiplier
const SHOT_MAX_RANDOMNESS: f32 = 0.4; // Max angle/power variance at zero charge (40%)
const SHOT_AIR_ACCURACY_PENALTY: f32 = 0.2; // Additional randomness when airborne (20%)
const SHOT_AIR_POWER_PENALTY: f32 = 0.7; // Power multiplier when airborne

// Ball-player collision
const BALL_PLAYER_DRAG: f32 = 0.7; // Velocity multiplier when ball passes through player
const BALL_KICK_STRENGTH: f32 = 100.0; // How much velocity player imparts to stationary ball
const BALL_KICK_THRESHOLD: f32 = 30.0; // Ball speed below this counts as "stationary"

// Steal contest
const STEAL_RANGE: f32 = 60.0; // How close to initiate steal
const STEAL_CONTEST_DURATION: f32 = 0.4; // Seconds to mash buttons
const STEAL_DEFENDER_ADVANTAGE: u32 = 2; // Defender starts with this many "presses"

// Arena dimensions (16:9 aspect ratio friendly)
const ARENA_WIDTH: f32 = 1600.0;
const ARENA_HEIGHT: f32 = 900.0;
const ARENA_FLOOR_Y: f32 = -ARENA_HEIGHT / 2.0 + 20.0; // Floor near bottom

// Baskets
const BASKET_COLOR: Color = Color::srgb(0.8, 0.2, 0.2); // Red
const BASKET_SIZE: Vec2 = Vec2::new(60.0, 80.0);
const BASKET_HEIGHT: f32 = ARENA_FLOOR_Y + 500.0; // Elevated
const LEFT_BASKET_X: f32 = -ARENA_WIDTH / 2.0 + 120.0;
const RIGHT_BASKET_X: f32 = ARENA_WIDTH / 2.0 - 120.0;

// Spawn
const PLAYER_SPAWN: Vec3 = Vec3::new(-200.0, ARENA_FLOOR_Y + 100.0, 0.0);
const BALL_SPAWN: Vec3 = Vec3::new(0.0, ARENA_FLOOR_Y + 50.0, 0.0); // Center

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin::default()))
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .init_resource::<PlayerInput>()
        .init_resource::<DebugSettings>()
        .init_resource::<StealContest>()
        .init_resource::<Score>()
        .add_systems(Startup, setup)
        .add_systems(Update, (capture_input, respawn_player, toggle_debug, update_debug_text))
        .add_systems(
            FixedUpdate,
            (
                apply_input,
                apply_gravity,
                ball_gravity,
                apply_velocity,
                check_collisions,
                ball_collisions,
                ball_state_update,
                ball_player_collision,
                ball_follow_holder,
                pickup_ball,
                steal_contest_update,
                update_shot_charge,
                throw_ball,
                check_scoring,
            )
                .chain(),
        )
        .run();
}

// Resources

#[derive(Resource, Default)]
struct PlayerInput {
    move_x: f32,
    jump_buffer_timer: f32, // Time remaining in jump buffer
    jump_held: bool,        // Is jump button currently held
    pickup_pressed: bool,   // West button - pick up ball
    throw_held: bool,       // R shoulder - charging throw
    throw_released: bool,   // R shoulder released - execute throw
}

#[derive(Resource)]
struct DebugSettings {
    visible: bool,
}

impl Default for DebugSettings {
    fn default() -> Self {
        Self { visible: true }
    }
}

#[derive(Resource, Default)]
struct StealContest {
    active: bool,
    attacker: Option<Entity>,
    defender: Option<Entity>,
    attacker_presses: u32,
    defender_presses: u32,
    timer: f32,
}

#[derive(Resource, Default)]
struct Score {
    left: u32,  // Team scoring in LEFT basket
    right: u32, // Team scoring in RIGHT basket
}

// Components

#[derive(Component)]
struct Player;

#[derive(Component, Default)]
struct Velocity(Vec2);

#[derive(Component)]
struct Grounded(bool);

#[derive(Component, Default)]
struct CoyoteTimer(f32); // Time remaining where jump is still allowed after leaving ground

#[derive(Component, Default)]
struct JumpState {
    is_jumping: bool, // Currently in a jump (for variable height)
}

#[derive(Component, Default)]
struct Collider;

#[derive(Component)]
#[require(Collider)]
struct Platform;

// Player state
#[derive(Component)]
struct Facing(f32); // -1.0 = left, 1.0 = right

impl Default for Facing {
    fn default() -> Self {
        Self(1.0) // Default facing right
    }
}

#[derive(Component)]
struct HoldingBall(Entity); // Reference to held ball

#[derive(Component, Default)]
struct ChargingShot {
    charge_time: f32, // How long throw button has been held
}

// Ball components
#[derive(Component)]
struct Ball;

#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
enum BallState {
    #[default]
    Free,
    Held(Entity),                              // Entity = player holding it
    InFlight { shooter: Entity, power: f32 },  // Who shot it and how hard
}

#[derive(Component, Default)]
struct BallPlayerContact {
    overlapping: bool, // Track if currently overlapping to apply effects once on entry
}

#[derive(Component, Clone, Copy, PartialEq)]
enum Basket {
    Left,
    Right,
}

#[derive(Component)]
struct DebugText;

// Systems

fn setup(mut commands: Commands) {
    // Camera - orthographic, shows entire arena
    // Using scale to zoom out and show full arena (default is 1.0 = 1 pixel per unit)
    // With 1600x900 arena, we need to scale so it fits in typical window sizes
    commands.spawn((
        Camera2d,
        Transform::from_xyz(0.0, 0.0, 0.0),
        Projection::Orthographic(OrthographicProjection {
            scale: 1.5, // Zoom out to show more of the arena
            ..OrthographicProjection::default_2d()
        }),
    ));

    // Player - spawns above the floor
    commands.spawn((
        Sprite::from_color(PLAYER_COLOR, PLAYER_SIZE),
        Transform::from_translation(PLAYER_SPAWN),
        Player,
        Velocity::default(),
        Grounded(false),
        CoyoteTimer::default(),
        JumpState::default(),
        Facing::default(),
        ChargingShot::default(),
        Collider,
    ));

    // Ball
    commands.spawn((
        Sprite::from_color(BALL_COLOR, BALL_SIZE),
        Transform::from_translation(BALL_SPAWN),
        Ball,
        BallState::default(),
        Velocity::default(),
        BallPlayerContact::default(),
    ));

    // Arena floor (spans most of the arena width)
    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, Vec2::new(ARENA_WIDTH - 100.0, 40.0)),
        Transform::from_xyz(0.0, ARENA_FLOOR_Y, 0.0),
        Platform,
    ));

    // Left wall
    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, Vec2::new(40.0, ARENA_HEIGHT)),
        Transform::from_xyz(-ARENA_WIDTH / 2.0 + 20.0, 0.0, 0.0),
        Platform,
    ));

    // Right wall
    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, Vec2::new(40.0, ARENA_HEIGHT)),
        Transform::from_xyz(ARENA_WIDTH / 2.0 - 20.0, 0.0, 0.0),
        Platform,
    ));

    // Ceiling
    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, Vec2::new(ARENA_WIDTH - 100.0, 40.0)),
        Transform::from_xyz(0.0, ARENA_HEIGHT / 2.0 - 20.0, 0.0),
        Platform,
    ));

    // Floating platforms - spread across arena
    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(200.0, 20.0)),
        Transform::from_xyz(-400.0, ARENA_FLOOR_Y + 150.0, 0.0),
        Platform,
    ));

    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(200.0, 20.0)),
        Transform::from_xyz(400.0, ARENA_FLOOR_Y + 150.0, 0.0),
        Platform,
    ));

    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(250.0, 20.0)),
        Transform::from_xyz(0.0, ARENA_FLOOR_Y + 300.0, 0.0),
        Platform,
    ));

    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(150.0, 20.0)),
        Transform::from_xyz(-500.0, ARENA_FLOOR_Y + 400.0, 0.0),
        Platform,
    ));

    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(150.0, 20.0)),
        Transform::from_xyz(500.0, ARENA_FLOOR_Y + 400.0, 0.0),
        Platform,
    ));

    // Left basket platform
    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(120.0, 20.0)),
        Transform::from_xyz(LEFT_BASKET_X, BASKET_HEIGHT - BASKET_SIZE.y / 2.0 - 10.0, 0.0),
        Platform,
    ));

    // Left basket (scoring zone)
    commands.spawn((
        Sprite::from_color(BASKET_COLOR, BASKET_SIZE),
        Transform::from_xyz(LEFT_BASKET_X, BASKET_HEIGHT, 0.0),
        Basket::Left,
    ));

    // Right basket platform
    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::new(120.0, 20.0)),
        Transform::from_xyz(RIGHT_BASKET_X, BASKET_HEIGHT - BASKET_SIZE.y / 2.0 - 10.0, 0.0),
        Platform,
    ));

    // Right basket (scoring zone)
    commands.spawn((
        Sprite::from_color(BASKET_COLOR, BASKET_SIZE),
        Transform::from_xyz(RIGHT_BASKET_X, BASKET_HEIGHT, 0.0),
        Basket::Right,
    ));

    // Debug UI
    commands.spawn((
        Text::new("Debug"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::BLACK),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        DebugText,
    ));
}

/// Runs in Update to capture input state before it's cleared
fn capture_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut input: ResMut<PlayerInput>,
    time: Res<Time>,
) {
    // Horizontal movement (continuous - overwrite each frame)
    let mut move_x = 0.0;

    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        move_x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        move_x += 1.0;
    }

    for gamepad in &gamepads {
        if let Some(stick_x) = gamepad.get(GamepadAxis::LeftStickX) {
            if stick_x.abs() > 0.1 {
                move_x += stick_x;
            }
        }
    }

    input.move_x = move_x.clamp(-1.0, 1.0);

    // Jump button state
    let jump_pressed = keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::KeyW)
        || keyboard.just_pressed(KeyCode::ArrowUp)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::South));

    input.jump_held = keyboard.pressed(KeyCode::Space)
        || keyboard.pressed(KeyCode::KeyW)
        || keyboard.pressed(KeyCode::ArrowUp)
        || gamepads.iter().any(|gp| gp.pressed(GamepadButton::South));

    // Jump buffering - reset timer on press, count down otherwise
    if jump_pressed {
        input.jump_buffer_timer = JUMP_BUFFER_TIME;
    } else {
        input.jump_buffer_timer = (input.jump_buffer_timer - time.delta_secs()).max(0.0);
    }

    // Pickup (West button / E key)
    input.pickup_pressed = keyboard.just_pressed(KeyCode::KeyE)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::West));

    // Throw (R shoulder / F key)
    let throw_held_now = keyboard.pressed(KeyCode::KeyF)
        || gamepads.iter().any(|gp| gp.pressed(GamepadButton::RightTrigger));

    // Accumulate throw_released until consumed (like jump buffering)
    if input.throw_held && !throw_held_now {
        input.throw_released = true;
    }
    input.throw_held = throw_held_now;
}

fn respawn_player(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut commands: Commands,
    mut player: Query<(Entity, &mut Transform, &mut Velocity, Option<&HoldingBall>), With<Player>>,
    mut ball: Query<(&mut Transform, &mut Velocity, &mut BallState), (With<Ball>, Without<Player>)>,
) {
    let respawn_pressed = keyboard.just_pressed(KeyCode::KeyR)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::Start));

    if respawn_pressed {
        // Reset player
        if let Ok((player_entity, mut p_transform, mut p_velocity, holding)) = player.single_mut() {
            p_transform.translation = PLAYER_SPAWN;
            p_velocity.0 = Vec2::ZERO;

            // Drop ball if holding
            if holding.is_some() {
                commands.entity(player_entity).remove::<HoldingBall>();
            }
        }

        // Reset ball
        if let Ok((mut b_transform, mut b_velocity, mut b_state)) = ball.single_mut() {
            b_transform.translation = BALL_SPAWN;
            b_velocity.0 = Vec2::ZERO;
            *b_state = BallState::Free;
        }
    }
}

/// Runs in FixedUpdate to apply captured input to physics
fn apply_input(
    mut input: ResMut<PlayerInput>,
    mut player: Query<
        (&mut Velocity, &mut CoyoteTimer, &mut JumpState, &mut Facing, &Grounded),
        With<Player>,
    >,
    time: Res<Time>,
) {
    let Ok((mut velocity, mut coyote, mut jump_state, mut facing, grounded)) =
        player.single_mut()
    else {
        return;
    };

    velocity.0.x = input.move_x * MOVE_SPEED;

    // Update facing direction based on movement
    if input.move_x > 0.1 {
        facing.0 = 1.0;
    } else if input.move_x < -0.1 {
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
    if input.jump_buffer_timer > 0.0 && can_jump {
        velocity.0.y = JUMP_VELOCITY;
        input.jump_buffer_timer = 0.0; // Consume the buffered jump
        coyote.0 = 0.0; // Consume coyote time so we can't double jump
        jump_state.is_jumping = true; // Mark that we're in a jump
    }

    // Variable jump height: cut velocity if button released while rising
    // Check: in a jump + rising + button NOT held = cut velocity
    if jump_state.is_jumping && velocity.0.y > 0.0 && !input.jump_held {
        velocity.0.y *= JUMP_CUT_MULTIPLIER;
        jump_state.is_jumping = false; // Only cut once per jump
    }
}

fn apply_gravity(mut query: Query<(&mut Velocity, &Grounded)>, time: Res<Time>) {
    for (mut velocity, grounded) in &mut query {
        if !grounded.0 {
            // Fast fall: use higher gravity when falling than rising
            let gravity = if velocity.0.y > 0.0 {
                GRAVITY_RISE
            } else {
                GRAVITY_FALL
            };
            velocity.0.y -= gravity * time.delta_secs();
        }
    }
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * time.delta_secs();
        transform.translation.y += velocity.0.y * time.delta_secs();
    }
}

fn check_collisions(
    mut player_query: Query<(&mut Transform, &mut Velocity, &mut Grounded, &Sprite), With<Player>>,
    platform_query: Query<(&Transform, &Sprite), (With<Platform>, Without<Player>)>,
) {
    let Ok((mut player_transform, mut player_velocity, mut grounded, player_sprite)) =
        player_query.single_mut()
    else {
        return;
    };

    let player_size = player_sprite.custom_size.unwrap_or(PLAYER_SIZE);
    let player_half = player_size / 2.0;

    // Assume not grounded until we find a floor beneath us
    grounded.0 = false;

    for (platform_transform, platform_sprite) in &platform_query {
        let platform_size = platform_sprite.custom_size.unwrap_or(Vec2::new(100.0, 20.0));
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

fn ball_gravity(mut query: Query<(&mut Velocity, &BallState), With<Ball>>, time: Res<Time>) {
    for (mut velocity, state) in &mut query {
        // Only apply gravity when ball is free or in flight
        match state {
            BallState::Free | BallState::InFlight { .. } => {
                velocity.0.y -= BALL_GRAVITY * time.delta_secs();
                // Apply friction to horizontal movement
                velocity.0.x *= BALL_FRICTION;
            }
            BallState::Held(_) => {
                // Ball follows player, no gravity
                velocity.0 = Vec2::ZERO;
            }
        }
    }
}

fn ball_collisions(
    mut ball_query: Query<(&mut Transform, &mut Velocity, &BallState, &Sprite), With<Ball>>,
    platform_query: Query<(&Transform, &Sprite), (With<Platform>, Without<Ball>)>,
) {
    for (mut ball_transform, mut ball_velocity, state, ball_sprite) in &mut ball_query {
        // Skip collision for held balls
        if matches!(state, BallState::Held(_)) {
            continue;
        }

        let ball_size = ball_sprite.custom_size.unwrap_or(BALL_SIZE);
        let ball_half = ball_size / 2.0;

        for (platform_transform, platform_sprite) in &platform_query {
            let platform_size = platform_sprite.custom_size.unwrap_or(Vec2::new(100.0, 20.0));
            let platform_half = platform_size / 2.0;

            let ball_pos = ball_transform.translation.truncate();
            let platform_pos = platform_transform.translation.truncate();

            let diff = ball_pos - platform_pos;
            let overlap_x = ball_half.x + platform_half.x - diff.x.abs();
            let overlap_y = ball_half.y + platform_half.y - diff.y.abs();

            if overlap_x <= 0.0 || overlap_y <= 0.0 {
                continue;
            }

            // Resolve collision with bounce
            if overlap_y < overlap_x {
                // Vertical collision
                if diff.y > 0.0 {
                    // Ball above platform
                    ball_transform.translation.y =
                        platform_pos.y + platform_half.y + ball_half.y;
                    if ball_velocity.0.y < 0.0 {
                        // Bounce with energy loss
                        if ball_velocity.0.y.abs() > BALL_MIN_BOUNCE_VEL {
                            ball_velocity.0.y = -ball_velocity.0.y * BALL_BOUNCE;
                        } else {
                            ball_velocity.0.y = 0.0;
                        }
                    }
                } else {
                    // Ball below platform (hit ceiling)
                    ball_transform.translation.y =
                        platform_pos.y - platform_half.y - ball_half.y;
                    if ball_velocity.0.y > 0.0 {
                        ball_velocity.0.y = -ball_velocity.0.y * BALL_BOUNCE;
                    }
                }
            } else {
                // Horizontal collision - bounce off walls
                if diff.x > 0.0 {
                    ball_transform.translation.x =
                        platform_pos.x + platform_half.x + ball_half.x;
                } else {
                    ball_transform.translation.x =
                        platform_pos.x - platform_half.x - ball_half.x;
                }
                ball_velocity.0.x = -ball_velocity.0.x * BALL_BOUNCE;
            }
        }
    }
}

fn ball_state_update(mut ball_query: Query<(&Velocity, &mut BallState), With<Ball>>) {
    for (velocity, mut state) in &mut ball_query {
        // InFlight balls become Free when they slow down enough
        if matches!(*state, BallState::InFlight { .. }) {
            let speed = velocity.0.length();
            if speed < BALL_FREE_SPEED {
                *state = BallState::Free;
            }
        }
    }
}

fn ball_player_collision(
    mut ball_query: Query<
        (&Transform, &mut Velocity, &mut BallPlayerContact, &BallState, &Sprite),
        With<Ball>,
    >,
    mut player_query: Query<(&Transform, &mut Velocity, &Sprite), (With<Player>, Without<Ball>)>,
) {
    for (ball_transform, mut ball_velocity, mut contact, ball_state, ball_sprite) in &mut ball_query
    {
        // Skip held balls
        if matches!(ball_state, BallState::Held(_)) {
            contact.overlapping = false;
            continue;
        }

        let ball_size = ball_sprite.custom_size.unwrap_or(BALL_SIZE);
        let ball_half = ball_size / 2.0;
        let ball_pos = ball_transform.translation.truncate();

        let mut is_overlapping = false;

        for (player_transform, mut player_velocity, player_sprite) in &mut player_query {
            let player_size = player_sprite.custom_size.unwrap_or(PLAYER_SIZE);
            let player_half = player_size / 2.0;
            let player_pos = player_transform.translation.truncate();

            let diff = ball_pos - player_pos;
            let overlap_x = ball_half.x + player_half.x - diff.x.abs();
            let overlap_y = ball_half.y + player_half.y - diff.y.abs();

            if overlap_x > 0.0 && overlap_y > 0.0 {
                is_overlapping = true;

                // Only apply effects on first frame of contact
                if !contact.overlapping {
                    let ball_speed = ball_velocity.0.length();
                    let player_speed = player_velocity.0.length();

                    // Both slow down when passing through each other
                    ball_velocity.0 *= BALL_PLAYER_DRAG;
                    player_velocity.0 *= BALL_PLAYER_DRAG;

                    // If ball is slow/stationary and player is moving, kick the ball
                    if ball_speed < BALL_KICK_THRESHOLD && player_speed > BALL_KICK_THRESHOLD {
                        let kick_dir = if player_velocity.0.x > 0.0 { 1.0 } else { -1.0 };
                        ball_velocity.0.x += kick_dir * BALL_KICK_STRENGTH;
                        ball_velocity.0.y += BALL_KICK_STRENGTH * 0.3; // Small upward nudge
                    }
                }
            }
        }

        contact.overlapping = is_overlapping;
    }
}

fn ball_follow_holder(
    mut ball_query: Query<(&mut Transform, &BallState), With<Ball>>,
    player_query: Query<(&Transform, &Facing), (With<Player>, Without<Ball>)>,
) {
    for (mut ball_transform, state) in &mut ball_query {
        if let BallState::Held(holder_entity) = state {
            if let Ok((player_transform, facing)) = player_query.get(*holder_entity) {
                // Position ball in front of player based on facing direction
                ball_transform.translation.x =
                    player_transform.translation.x + facing.0 * (PLAYER_SIZE.x / 2.0 + BALL_SIZE.x / 2.0 + 5.0);
                ball_transform.translation.y =
                    player_transform.translation.y + PLAYER_SIZE.y / 4.0; // Slightly above center
            }
        }
    }
}

fn pickup_ball(
    input: Res<PlayerInput>,
    mut commands: Commands,
    mut steal_contest: ResMut<StealContest>,
    non_holding_players: Query<(Entity, &Transform), (With<Player>, Without<HoldingBall>)>,
    holding_players: Query<(Entity, &Transform, &HoldingBall), With<Player>>,
    mut ball_query: Query<(Entity, &Transform, &mut BallState), With<Ball>>,
) {
    // If steal contest is active, count presses
    if steal_contest.active {
        if input.pickup_pressed {
            // For now, attribute press to attacker (single player)
            // In multiplayer, check which player pressed
            steal_contest.attacker_presses += 1;
        }
        return; // Don't allow pickup during contest
    }

    if !input.pickup_pressed {
        return;
    }

    // Check each non-holding player
    for (player_entity, player_transform) in &non_holding_players {
        let player_pos = player_transform.translation.truncate();

        // First, try to pick up a free ball
        for (ball_entity, ball_transform, mut ball_state) in &mut ball_query {
            if *ball_state != BallState::Free {
                continue;
            }

            let distance = player_pos.distance(ball_transform.translation.truncate());

            if distance < BALL_PICKUP_RADIUS {
                *ball_state = BallState::Held(player_entity);
                commands.entity(player_entity).insert(HoldingBall(ball_entity));
                return; // Done - picked up ball
            }
        }

        // If no free ball nearby, check for steal opportunity
        for (defender_entity, defender_transform, _holding) in &holding_players {
            let distance = player_pos.distance(defender_transform.translation.truncate());

            if distance < STEAL_RANGE {
                // Initiate steal contest
                steal_contest.active = true;
                steal_contest.attacker = Some(player_entity);
                steal_contest.defender = Some(defender_entity);
                steal_contest.attacker_presses = 1; // Count the initiating press
                steal_contest.defender_presses = STEAL_DEFENDER_ADVANTAGE;
                steal_contest.timer = STEAL_CONTEST_DURATION;
                return;
            }
        }
    }
}

fn steal_contest_update(
    mut commands: Commands,
    mut steal_contest: ResMut<StealContest>,
    time: Res<Time>,
    mut ball_query: Query<&mut BallState, With<Ball>>,
    holding_query: Query<&HoldingBall>,
) {
    if !steal_contest.active {
        return;
    }

    steal_contest.timer -= time.delta_secs();

    // TODO: In multiplayer, check defender's button presses here
    // For now, defender gets occasional "presses" to simulate resistance
    if steal_contest.timer > 0.0 && steal_contest.timer % 0.1 < time.delta_secs() {
        steal_contest.defender_presses += 1;
    }

    if steal_contest.timer <= 0.0 {
        // Contest ended - resolve it
        let attacker = steal_contest.attacker.unwrap();
        let defender = steal_contest.defender.unwrap();

        if steal_contest.attacker_presses > steal_contest.defender_presses {
            // Attacker wins - steal the ball
            if let Ok(holding) = holding_query.get(defender) {
                let ball_entity = holding.0;
                if let Ok(mut ball_state) = ball_query.get_mut(ball_entity) {
                    *ball_state = BallState::Held(attacker);
                    commands.entity(defender).remove::<HoldingBall>();
                    commands.entity(attacker).insert(HoldingBall(ball_entity));
                }
            }
        }
        // If defender wins or tie, they keep the ball (no action needed)

        // Reset contest
        *steal_contest = StealContest::default();
    }
}

fn update_shot_charge(
    input: Res<PlayerInput>,
    time: Res<Time>,
    mut player_query: Query<&mut ChargingShot, With<Player>>,
) {
    for mut charging in &mut player_query {
        if input.throw_held {
            charging.charge_time += time.delta_secs();
        }
        // Don't reset here - let throw_ball reset after using the charge
        // Otherwise charge resets to 0 before throw_ball can read it
    }
}

fn throw_ball(
    mut input: ResMut<PlayerInput>,
    mut commands: Commands,
    mut player_query: Query<
        (Entity, &Facing, &Grounded, &mut ChargingShot, Option<&HoldingBall>),
        With<Player>,
    >,
    mut ball_query: Query<(&mut Velocity, &mut BallState), With<Ball>>,
) {
    if !input.throw_released {
        return;
    }

    // Consume the throw_released flag immediately
    input.throw_released = false;

    let Ok((player_entity, facing, grounded, mut charging, holding)) = player_query.single_mut()
    else {
        return;
    };

    let Some(holding_ball) = holding else {
        // Not holding a ball - reset charge since they released the button
        charging.charge_time = 0.0;
        return;
    };

    let Ok((mut ball_velocity, mut ball_state)) = ball_query.get_mut(holding_ball.0) else {
        return;
    };

    // Calculate charge percentage (0.0 to 1.0)
    let charge_pct = (charging.charge_time / SHOT_CHARGE_TIME).min(1.0);

    // Power scales with charge
    let mut power = SHOT_MIN_POWER + (SHOT_MAX_POWER - SHOT_MIN_POWER) * charge_pct;

    // Calculate randomness (less charge = more randomness)
    let mut randomness = SHOT_MAX_RANDOMNESS * (1.0 - charge_pct);

    // Air shot penalties
    if !grounded.0 {
        power *= SHOT_AIR_POWER_PENALTY;
        randomness += SHOT_AIR_ACCURACY_PENALTY;
    }

    // Apply randomness to power and angle (only if randomness > 0)
    let (power_variance, angle_variance) = if randomness > 0.01 {
        let mut rng = rand::thread_rng();
        (
            1.0 + rng.gen_range(-randomness..randomness),
            rng.gen_range(-randomness..randomness),
        )
    } else {
        (1.0, 0.0) // Full charge = perfect accuracy
    };

    let final_power = power * power_variance;
    let arc = SHOT_BASE_ARC + angle_variance;

    // Set ball velocity in arc
    ball_velocity.0.x = facing.0 * final_power;
    ball_velocity.0.y = final_power * arc;

    *ball_state = BallState::InFlight {
        shooter: player_entity,
        power: final_power,
    };

    // Reset charge and release ball
    charging.charge_time = 0.0;
    commands.entity(player_entity).remove::<HoldingBall>();
}

fn check_scoring(
    mut score: ResMut<Score>,
    mut ball_query: Query<(&mut Transform, &mut Velocity, &mut BallState, &Sprite), With<Ball>>,
    basket_query: Query<(&Transform, &Basket, &Sprite), Without<Ball>>,
) {
    for (mut ball_transform, mut ball_velocity, mut ball_state, _ball_sprite) in &mut ball_query {
        // Only score with free or in-flight balls (not held)
        if matches!(*ball_state, BallState::Held(_)) {
            continue;
        }

        let ball_pos = ball_transform.translation.truncate();

        for (basket_transform, basket, basket_sprite) in &basket_query {
            let basket_size = basket_sprite.custom_size.unwrap_or(BASKET_SIZE);
            let basket_pos = basket_transform.translation.truncate();
            let basket_half = basket_size / 2.0;

            // Check if ball center is inside basket
            let in_basket = ball_pos.x > basket_pos.x - basket_half.x
                && ball_pos.x < basket_pos.x + basket_half.x
                && ball_pos.y > basket_pos.y - basket_half.y
                && ball_pos.y < basket_pos.y + basket_half.y;

            if in_basket {
                // Score!
                match basket {
                    Basket::Left => score.left += 1,
                    Basket::Right => score.right += 1,
                }

                // Reset ball to center
                ball_transform.translation = BALL_SPAWN;
                ball_velocity.0 = Vec2::ZERO;
                *ball_state = BallState::Free;

                info!("SCORE! Left: {} Right: {}", score.left, score.right);
            }
        }
    }
}

fn toggle_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<DebugSettings>,
    mut text_query: Query<&mut Visibility, With<DebugText>>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        settings.visible = !settings.visible;
        if let Ok(mut visibility) = text_query.single_mut() {
            *visibility = if settings.visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn update_debug_text(
    debug_settings: Res<DebugSettings>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    steal_contest: Res<StealContest>,
    score: Res<Score>,
    player: Query<(&Transform, &Facing, &Grounded, &ChargingShot, Option<&HoldingBall>), With<Player>>,
    ball_query: Query<&BallState, With<Ball>>,
    mut text_query: Query<&mut Text, With<DebugText>>,
) {
    if !debug_settings.visible {
        return;
    }

    let Ok((transform, facing, grounded, charging, holding)) = player.single() else {
        return;
    };
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let fps = diagnostics
        .get(&bevy::diagnostic::FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let pos = transform.translation;
    let facing_str = if facing.0 > 0.0 { "R" } else { "L" };
    let holding_str = if holding.is_some() { "Yes" } else { "No" };

    let charge_pct = ((charging.charge_time / SHOT_CHARGE_TIME) * 100.0).min(100.0);

    let ball_state_str = ball_query
        .iter()
        .next()
        .map(|s| match s {
            BallState::Free => "Free".to_string(),
            BallState::Held(_) => "Held".to_string(),
            BallState::InFlight { power, .. } => format!("Flight({:.0})", power),
        })
        .unwrap_or("None".to_string());

    let steal_str = if steal_contest.active {
        format!(
            "A:{} D:{} ({:.1}s)",
            steal_contest.attacker_presses,
            steal_contest.defender_presses,
            steal_contest.timer
        )
    } else {
        "-".to_string()
    };

    text.0 = format!(
        "SCORE: {} - {}\n\
         FPS: {:.0} | Pos: ({:.0}, {:.0})\n\
         Face: {} | Ground: {} | Hold: {}\n\
         Ball: {} | Charge: {:.0}%\n\
         Steal: {}\n\
         \n\
         [E] pickup [F] charge/throw [Tab] hide",
        score.left,
        score.right,
        fps,
        pos.x,
        pos.y,
        facing_str,
        grounded.0,
        holding_str,
        ball_state_str,
        charge_pct,
        steal_str,
    );
}
