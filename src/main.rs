use bevy::prelude::*;

// Visual constants
const BACKGROUND_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const FLOOR_COLOR: Color = Color::srgb(0.2, 0.2, 0.2);
const PLATFORM_COLOR: Color = Color::srgb(0.5, 0.3, 0.3);
const PLAYER_COLOR: Color = Color::srgb(0.3, 0.3, 0.7);

// Size constants
const PLAYER_SIZE: Vec2 = Vec2::new(32.0, 64.0);
const FLOOR_SIZE: Vec2 = Vec2::new(800.0, 40.0);
const PLATFORM_SIZE: Vec2 = Vec2::new(150.0, 20.0);

// Physics constants
const GRAVITY: f32 = 980.0;
const JUMP_VELOCITY: f32 = 400.0;
const MOVE_SPEED: f32 = 300.0;
const COLLISION_EPSILON: f32 = 0.5; // Skin width for collision detection

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .init_resource::<PlayerInput>()
        .add_systems(Startup, setup)
        .add_systems(Update, (capture_input, update_debug_text))
        .add_systems(
            FixedUpdate,
            (
                apply_input,
                apply_gravity,
                apply_velocity,
                check_collisions,
            )
                .chain(),
        )
        .run();
}

// Resources

#[derive(Resource, Default)]
struct PlayerInput {
    move_x: f32,
    jump: bool,
    // Debug tracking
    jump_pressed_frame: u32,
    jump_consumed_frame: u32,
    frame_counter: u32,
}

// Components

#[derive(Component)]
struct Player;

#[derive(Component, Default)]
struct Velocity(Vec2);

#[derive(Component)]
struct Grounded(bool);

#[derive(Component, Default)]
struct Collider;

#[derive(Component)]
#[require(Collider)]
struct Platform;

#[derive(Component)]
struct DebugText;

// Systems

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn(Camera2d);

    // Player - spawns above the floor
    commands.spawn((
        Sprite::from_color(PLAYER_COLOR, PLAYER_SIZE),
        Transform::from_xyz(0.0, 100.0, 0.0),
        Player,
        Velocity::default(),
        Grounded(false),
        Collider,
    ));

    // Main floor
    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, FLOOR_SIZE),
        Transform::from_xyz(0.0, -200.0, 0.0),
        Platform,
    ));

    // Floating platforms
    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, PLATFORM_SIZE),
        Transform::from_xyz(-200.0, -50.0, 0.0),
        Platform,
    ));

    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, PLATFORM_SIZE),
        Transform::from_xyz(150.0, 50.0, 0.0),
        Platform,
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
) {
    input.frame_counter += 1;

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

    // Jump (edge-triggered - accumulate until consumed)
    let jump_pressed = keyboard.just_pressed(KeyCode::Space)
        || keyboard.just_pressed(KeyCode::KeyW)
        || keyboard.just_pressed(KeyCode::ArrowUp)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::South));

    if jump_pressed {
        input.jump = true;
        input.jump_pressed_frame = input.frame_counter;
    }
}

/// Runs in FixedUpdate to apply captured input to physics
fn apply_input(
    mut input: ResMut<PlayerInput>,
    mut player: Query<(&mut Velocity, &Grounded), With<Player>>,
) {
    let Ok((mut velocity, grounded)) = player.single_mut() else {
        return;
    };

    velocity.0.x = input.move_x * MOVE_SPEED;

    if input.jump {
        if grounded.0 {
            velocity.0.y = JUMP_VELOCITY;
        }
        // Consume the jump input so it only fires once
        input.jump = false;
        input.jump_consumed_frame = input.frame_counter;
    }
}

fn apply_gravity(mut query: Query<(&mut Velocity, &Grounded)>, time: Res<Time>) {
    for (mut velocity, grounded) in &mut query {
        if !grounded.0 {
            velocity.0.y -= GRAVITY * time.delta_secs();
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
        let platform_size = platform_sprite.custom_size.unwrap_or(PLATFORM_SIZE);
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

fn update_debug_text(
    input: Res<PlayerInput>,
    player: Query<(&Velocity, &Grounded), With<Player>>,
    gamepads: Query<&Gamepad>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut text_query: Query<&mut Text, With<DebugText>>,
) {
    let Ok((velocity, grounded)) = player.single() else {
        return;
    };
    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let frames_since_press = input.frame_counter.saturating_sub(input.jump_pressed_frame);
    let frames_since_consume = input.frame_counter.saturating_sub(input.jump_consumed_frame);

    // Check raw button states
    let mut gp_south_pressed = false;
    let mut gp_south_just_pressed = false;
    for gamepad in &gamepads {
        gp_south_pressed |= gamepad.pressed(GamepadButton::South);
        gp_south_just_pressed |= gamepad.just_pressed(GamepadButton::South);
    }
    let kb_space_pressed = keyboard.pressed(KeyCode::Space);
    let kb_space_just_pressed = keyboard.just_pressed(KeyCode::Space);

    text.0 = format!(
        "Frame: {}\n\
         Input: move_x={:.2} jump_buffered={}\n\
         Player: vel=({:.0}, {:.0}) grounded={}\n\
         Jump pressed: {} frames ago\n\
         Jump consumed: {} frames ago\n\
         ---\n\
         GP South: held={} just={}\n\
         KB Space: held={} just={}",
        input.frame_counter,
        input.move_x,
        input.jump,
        velocity.0.x,
        velocity.0.y,
        grounded.0,
        frames_since_press,
        frames_since_consume,
        gp_south_pressed,
        gp_south_just_pressed,
        kb_space_pressed,
        kb_space_just_pressed,
    );
}
