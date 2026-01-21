use bevy::{diagnostic::FrameTimeDiagnosticsPlugin, prelude::*};

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
const GRAVITY_RISE: f32 = 980.0; // Gravity while rising
const GRAVITY_FALL: f32 = 1400.0; // Gravity while falling (fast fall)
const JUMP_VELOCITY: f32 = 650.0; // Full jump height (hold button)
const JUMP_CUT_MULTIPLIER: f32 = 0.4; // Velocity multiplier when releasing jump early
const MOVE_SPEED: f32 = 300.0;
const COLLISION_EPSILON: f32 = 0.5; // Skin width for collision detection

// Game feel constants
const COYOTE_TIME: f32 = 0.1; // Seconds after leaving ground you can still jump
const JUMP_BUFFER_TIME: f32 = 0.1; // Seconds before landing that jump input is remembered

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin::default()))
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .init_resource::<PlayerInput>()
        .init_resource::<DebugSettings>()
        .add_systems(Startup, setup)
        .add_systems(Update, (capture_input, toggle_debug, update_debug_text))
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
    jump_buffer_timer: f32, // Time remaining in jump buffer
    jump_held: bool,        // Is jump button currently held
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
        CoyoteTimer::default(),
        JumpState::default(),
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
}

/// Runs in FixedUpdate to apply captured input to physics
fn apply_input(
    mut input: ResMut<PlayerInput>,
    mut player: Query<(&mut Velocity, &mut CoyoteTimer, &mut JumpState, &Grounded), With<Player>>,
    time: Res<Time>,
) {
    let Ok((mut velocity, mut coyote, mut jump_state, grounded)) = player.single_mut() else {
        return;
    };

    velocity.0.x = input.move_x * MOVE_SPEED;

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
    input: Res<PlayerInput>,
    player: Query<(&Transform, &Velocity, &Grounded, &CoyoteTimer), With<Player>>,
    mut text_query: Query<&mut Text, With<DebugText>>,
) {
    if !debug_settings.visible {
        return;
    }

    let Ok((transform, velocity, grounded, coyote)) = player.single() else {
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

    text.0 = format!(
        "FPS: {:.0}\n\
         Pos: ({:.0}, {:.0})\n\
         Vel: ({:.0}, {:.0})\n\
         Grounded: {}\n\
         Coyote: {:.0}ms\n\
         JumpBuf: {:.0}ms\n\
         \n\
         [Tab] hide",
        fps,
        pos.x,
        pos.y,
        velocity.0.x,
        velocity.0.y,
        grounded.0,
        coyote.0 * 1000.0,
        input.jump_buffer_timer * 1000.0,
    );
}
