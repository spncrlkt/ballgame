use bevy::prelude::*;

const BACKGROUND_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const FLOOR_COLOR: Color = Color::srgb(0.0, 0.0, 0.0);
const FLOOR_SIZE: Vec2 = Vec2::new(1300.0, 2.0);
const P1_COLOR: Color = Color::srgb(0.3, 0.3, 0.7);
const P_SIZE: Vec2 = Vec2::new(32.0, 64.0);
const P_SPEED: f32 = 500.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_systems(Startup, setup)
        .add_systems(Update, gamepad_log_system)
        .add_systems(
            FixedUpdate,
            (apply_velocity, move_player, check_for_collisions).chain(),
        )
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Ball;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

// Default must be implemented to define this as a required component for the floors
#[derive(Component, Default)]
struct Collider;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite::from_color(P1_COLOR, Vec2::ONE),
        Transform {
            translation: Vec3::new(0.0, -200.0, 0.0),
            scale: P_SIZE.extend(1.0),
            ..default()
        },
        Player,
        Collider,
    ));

    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, Vec2::ONE),
        Transform {
            translation: Vec3::new(0.0, -232.0, 0.0),
            scale: FLOOR_SIZE.extend(1.0),
            ..default()
        },
        Collider,
    ));
}

fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * time.delta_secs();
        transform.translation.y += velocity.y * time.delta_secs();
    }
}

fn move_player(
    gamepads: Query<(Entity, &Gamepad)>,
    mut p_transform: Single<&mut Transform, With<Player>>,
    time: Res<Time>,
) {
    for (entity, gamepad) in &gamepads {
        let left_stick_x = gamepad.get(GamepadAxis::LeftStickX).unwrap();
        if left_stick_x.abs() > 0.05 {
            info!("{} LeftStickX value is {}", entity, left_stick_x);

            // Update the player position with the result of:
            // Calculate the new horizontal player position based on player input
            p_transform.translation.x =
                p_transform.translation.x + left_stick_x * P_SPEED * time.delta_secs();
        }
    }
}

fn check_for_collisions() {}

fn gamepad_log_system(gamepads: Query<(Entity, &Gamepad)>) {
    for (entity, gamepad) in &gamepads {
        if gamepad.just_pressed(GamepadButton::North) {
            info!("{} just pressed North", entity);
        }

        if gamepad.just_pressed(GamepadButton::East) {
            info!("{} just pressed East", entity);
        }

        if gamepad.just_pressed(GamepadButton::South) {
            info!("{} just pressed South", entity);
        }

        if gamepad.just_pressed(GamepadButton::West) {
            info!("{} just pressed West", entity);
        }

        if gamepad.just_pressed(GamepadButton::RightTrigger) {
            info!("{} just pressed R", entity);
        }

        if gamepad.just_pressed(GamepadButton::LeftTrigger) {
            info!("{} just pressed L", entity);
        }

        let left_stick_x = gamepad.get(GamepadAxis::LeftStickX).unwrap();
        if left_stick_x.abs() > 0.05 {
            info!("{} LeftStickX value is {}", entity, left_stick_x);
        }

        let left_stick_y = gamepad.get(GamepadAxis::LeftStickY).unwrap();
        if left_stick_y.abs() > 0.05 {
            info!("{} LeftStickY value is {}", entity, left_stick_y);
        }
    }
}
