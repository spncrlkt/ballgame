use bevy::{
    color::palettes::css::BLACK, math::bounding::Aabb2d, math::bounding::IntersectsVolume,
    prelude::*,
};

const BACKGROUND_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const FLOOR_COLOR: Color = Color::srgb(0.0, 0.0, 0.0);
const FLOOR_SIZE: Vec2 = Vec2::new(1300.0, 200.0);
const PLATFORM_COLOR: Color = Color::srgb(0.7, 0.3, 0.3);
const PLATFORM_SIZE: Vec2 = Vec2::new(100.0, 20.0);
const P1_COLOR: Color = Color::srgb(0.3, 0.3, 0.7);
const P_SIZE: Vec2 = Vec2::new(32.0, 64.0);
const P_SPEED: f32 = 500.0;

const DEBUG_FONT_SIZE: f32 = 33.0;
const DEBUG_TEXT_PADDING: Val = Val::Px(5.0);

#[derive(Resource)]
struct DebugInfo {
    elapsed_time: f64,
}

impl Default for DebugInfo {
    fn default() -> Self {
        Self { elapsed_time: 0.0 }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .insert_resource(DebugInfo::default())
        .add_systems(Startup, setup)
        .add_systems(Update, gamepad_log_system)
        .add_systems(
            FixedUpdate,
            (apply_velocity, move_player, check_for_collisions).chain(),
        )
        .add_systems(Update, debug_update_system)
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct MoveState {
    is_jumping: bool,
    term_vel: u32,
}

#[derive(Component)]
struct Ball;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

// Default must be implemented to define this as a required component for the floors
#[derive(Component, Default)]
struct Collider;

#[derive(Component)]
#[require(Collider)]
struct Floor;

#[derive(Component)]
struct DebugText;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite::from_color(P1_COLOR, Vec2::ONE),
        Transform {
            translation: Vec3::new(0.0, -100.0, 0.0),
            scale: P_SIZE.extend(1.0),
            ..default()
        },
        Player,
        MoveState {
            is_jumping: true,
            term_vel: 0,
        },
        Velocity(Vec2::ZERO),
        Collider,
    ));

    commands.spawn((
        Sprite::from_color(FLOOR_COLOR, Vec2::ONE),
        Transform {
            translation: Vec3::new(0.0, -382.0, 0.0),
            scale: FLOOR_SIZE.extend(1.0),
            ..default()
        },
        Floor,
    ));

    commands.spawn((
        Sprite::from_color(PLATFORM_COLOR, Vec2::ONE),
        Transform {
            translation: Vec3::new(-100.0, -200.0, 1.0),
            scale: PLATFORM_SIZE.extend(1.0),
            ..default()
        },
        Floor,
    ));

    commands.spawn((
        Text::new("XXXXXXXX"),
        DebugText,
        TextFont {
            font_size: 22.0,
            ..default()
        },
        TextColor(BLACK.into()),
    ));

    /*
    commands
        .spawn((
            // Create a Text with multiple child spans.
            Text::new("DEBUG: "),
            TextFont {
                font_size: 42.0,
                ..default()
            },
            TextColor(BLACK.into()),
        ))
        .with_child((
            TextSpan::default(),
            TextFont {
                font_size: 33.0,
                // If no font is specified, the default font (a minimal subset of FiraMono) will be used.
                ..default()
            },
            TextColor(BLACK.into()),
            DebugText,
        ));
    */
}

fn apply_velocity(
    mut debug_info: ResMut<DebugInfo>,
    mut query: Query<(&mut Transform, &mut Velocity)>,
    time: Res<Time>,
) {
    for (mut transform, mut velocity) in &mut query {
        transform.translation.x += velocity.x * time.delta_secs();
        transform.translation.y += velocity.y * time.delta_secs();

        // TODO: apply gravity here i think
        velocity.y -= 200.0 * time.delta_secs();
    }
    debug_info.elapsed_time += 1.
}

fn move_player(
    mut debug_info: ResMut<DebugInfo>,
    gamepads: Query<(Entity, &Gamepad)>,
    player: Single<(&mut Transform, &mut Velocity, &mut MoveState), With<Player>>,
    time: Res<Time>,
) {
    let (mut p_transform, mut p_velocity, mut p_movestate) = player.into_inner();
    for (entity, gamepad) in &gamepads {
        let left_stick_x = gamepad.get(GamepadAxis::LeftStickX).unwrap();
        if left_stick_x.abs() > 0.05 {
            info!("{} LeftStickX value is {}", entity, left_stick_x);

            // Update the player position with the result of:
            // Calculate the new horizontal player position based on player input
            p_transform.translation.x =
                p_transform.translation.x + left_stick_x * P_SPEED * time.delta_secs();
        }

        if gamepad.just_pressed(GamepadButton::South) {
            info!("{} just pressed South jp", entity);
            if !p_movestate.is_jumping {
                p_transform.translation.y += 3.0;
                p_velocity.y += 200.0;
                p_movestate.is_jumping = true;
            }
        }
    }
}

fn check_for_collisions(
    player_query: Single<(&mut Velocity, &Transform, &mut MoveState), With<Player>>,
    collider_query: Query<(Entity, &Transform, Option<&Floor>), With<Collider>>,
) {
    let (mut player_velocity, player_transform, mut player_movestate) = player_query.into_inner();
    for (collider_entity, collider_transform, maybe_floor) in &collider_query {
        let player_box = Aabb2d::new(
            player_transform.translation.truncate(),
            player_transform.scale.truncate() / 2.,
        );
        let collider_box = Aabb2d::new(
            collider_transform.translation.truncate(),
            collider_transform.scale.truncate() / 2.,
        );
        if !player_box.intersects(&collider_box) {
            continue;
        }

        if maybe_floor.is_some() {
            player_velocity.y = 0.;
            let higher = player_transform.translation.y >= collider_transform.translation.y;
            let between = player_transform.translation.x >= collider_box.min.x
                && player_transform.translation.x <= collider_box.max.x;
            if higher && between {
                player_movestate.is_jumping = false;
            };
        }
    }
}

fn gamepad_log_system(gamepads: Query<(Entity, &Gamepad)>) {
    for (entity, gamepad) in &gamepads {
        if gamepad.just_pressed(GamepadButton::North) {
            info!("{} just pressed North", entity);
        }

        if gamepad.just_pressed(GamepadButton::East) {
            info!("{} just pressed East", entity);
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

fn debug_update_system(
    mut debug_info: ResMut<DebugInfo>,
    mut query: Query<&mut Text, With<DebugText>>,
) {
    for mut text in &mut query {
        text.0 = format!(
            "Player Speed: {:.2}\n\
             Collisions: {}\n\
             Last Side: {}\n\
             Frame Time: {:.3} ms",
            debug_info.elapsed_time,
            debug_info.elapsed_time,
            debug_info.elapsed_time,
            debug_info.elapsed_time,
        );
    }
}
