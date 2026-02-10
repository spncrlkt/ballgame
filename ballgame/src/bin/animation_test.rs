//! Animation Test Binary
//!
//! Loads all sprite sheets from assets/sprites/ and provides:
//! - Left panel: Compact gallery of all animations looping
//! - Right panel: Interactive character with platforms and state machine
//!
//! Usage: cargo run --bin animation-test
//!
//! Controls (keyboard + controller):
//!   Gallery: Up/Down or DPad = select row, Left/Right or LT/RT = adjust FPS
//!            P = pause, F = flip, R = reset
//!   Character: A/D or Left Stick = move, Space or South = jump
//!              Q or LB = jab, E or RB = ledge
//!   Escape or Start = quit

use bevy::prelude::*;
use std::collections::HashMap;

// --- Layout constants ---

const FRAME_SIZE: u32 = 48;
const GALLERY_SCALE: f32 = 2.0;
const CHARACTER_SCALE: f32 = 4.0;
const CHAR_HALF_H: f32 = FRAME_SIZE as f32 * CHARACTER_SCALE / 2.0;

// Gallery lives on the far left
const GALLERY_X: f32 = -680.0;
const GALLERY_ROW_SPACING: f32 = 55.0;

// Character playground takes up the right ~70% of the screen
const FLOOR_Y: f32 = -340.0;
const FLOOR_WIDTH: f32 = 1200.0;
const FLOOR_CENTER_X: f32 = 150.0; // centered in the right portion

// Floating platform for jump/ledge testing
const PLATFORM_X: f32 = 350.0;
const PLATFORM_Y: f32 = -140.0;
const PLATFORM_WIDTH: f32 = 300.0;
const PLATFORM_THICKNESS: f32 = 8.0;

// Physics
const CHAR_GRAVITY: f32 = 980.0;
const CHAR_MOVE_SPEED: f32 = 200.0;
const CHAR_JUMP_VEL: f32 = 450.0;
const CHAR_RUN_THRESHOLD: f32 = 10.0;
const STICK_DEADZONE: f32 = 0.15;

// --- Animation definitions ---

struct AnimDef {
    name: &'static str,
    file: &'static str,
    frames: u32,
    /// Last frame index to use (0-based). None = use all frames.
    last_frame: Option<usize>,
    default_fps: f32,
    looping: bool,
}

const JAB_LAST_FRAME: usize = 5; // Use first 6 of 10 frames (indices 0-5)

const ANIM_DEFS: &[AnimDef] = &[
    AnimDef { name: "idle",  file: "sprites/idle.png",  frames: 10, last_frame: None,                 default_fps: 8.0,  looping: true },
    AnimDef { name: "run",   file: "sprites/run.png",   frames: 8,  last_frame: None,                 default_fps: 10.0, looping: true },
    AnimDef { name: "jump",  file: "sprites/jump.png",  frames: 6,  last_frame: None,                 default_fps: 8.0,  looping: false },
    AnimDef { name: "jab",   file: "sprites/jab.png",   frames: 10, last_frame: Some(JAB_LAST_FRAME), default_fps: 12.0, looping: false },
    AnimDef { name: "ledge", file: "sprites/ledge.png", frames: 5,  last_frame: None,                 default_fps: 6.0,  looping: true },
];

// --- Components ---

#[derive(Component)]
struct GallerySprite {
    index: usize,
    first_frame: usize,
    last_frame: usize,
    fps: f32,
    looping: bool,
}

#[derive(Component)]
struct GalleryTimer(Timer);

#[derive(Component)]
struct GalleryInfoText(usize);

#[derive(Component)]
struct GalleryLabel(#[allow(dead_code)] usize);

#[derive(Component)]
struct SelectionIndicator;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CharState {
    Idle,
    Run,
    Jump,
    Fall,
    Jab,
    Ledge,
}

#[derive(Component)]
struct Character {
    state: CharState,
    velocity: Vec2,
    grounded: bool,
    facing_right: bool,
    jab_timer: f32,
    ledge_timer: f32,
}

#[derive(Component)]
struct CharAnimTimer(Timer);

#[derive(Component)]
struct CharCurrentAnim {
    state: CharState,
    first_frame: usize,
    last_frame: usize,
}

/// A solid platform the character can stand on
#[derive(Component)]
struct TestPlatform {
    x: f32,
    y: f32,
    width: f32,
    thickness: f32,
}

#[derive(Component)]
struct ControlsText;

#[derive(Component)]
struct CharStateText;

// --- Resources ---

#[derive(Resource)]
struct GalleryState {
    selected: usize,
    paused: bool,
    flipped: bool,
}

#[derive(Clone)]
struct AnimClip {
    texture: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    first_frame: usize,
    last_frame: usize,
    default_fps: f32,
    #[allow(dead_code)]
    looping: bool,
}

#[derive(Resource)]
struct AnimClips(HashMap<CharState, AnimClip>);

// --- Main ---

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Animation Test".to_string(),
                        resolution: bevy::window::WindowResolution::new(1600, 800)
                            .with_scale_factor_override(1.0),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .insert_resource(GalleryState {
            selected: 0,
            paused: false,
            flipped: false,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                animate_gallery,
                handle_gallery_input,
                handle_character_input,
                update_character_physics,
                update_character_animation,
                animate_character_sprite,
                update_gallery_info_text,
                update_selection_indicator,
                update_char_state_text,
                check_exit,
            ),
        )
        .run();
}

// --- Setup ---

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn(Camera2d);

    let mut clips = HashMap::new();
    let state_map = [
        CharState::Idle,
        CharState::Run,
        CharState::Jump,
        CharState::Jab,
        CharState::Ledge,
    ];

    // --- Gallery (compact, left edge) ---
    let gallery_top = (ANIM_DEFS.len() as f32 - 1.0) / 2.0 * GALLERY_ROW_SPACING;

    for (i, def) in ANIM_DEFS.iter().enumerate() {
        let texture = asset_server.load(def.file);
        let layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(FRAME_SIZE),
            def.frames,
            1,
            None,
            None,
        ));

        let clip = AnimClip {
            texture: texture.clone(),
            layout: layout.clone(),
            first_frame: 0,
            last_frame: def.last_frame.unwrap_or(def.frames as usize - 1),
            default_fps: def.default_fps,
            looping: def.looping,
        };
        clips.insert(state_map[i], clip);

        let row_y = gallery_top - i as f32 * GALLERY_ROW_SPACING;

        // Label
        commands.spawn((
            Text2d::new(def.name.to_uppercase()),
            TextFont { font_size: 12.0, ..default() },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
            Transform::from_xyz(GALLERY_X - 80.0, row_y, 0.0),
            GalleryLabel(i),
        ));

        // Animated sprite (small scale)
        commands.spawn((
            Sprite::from_atlas_image(
                texture,
                TextureAtlas { layout, index: 0 },
            ),
            Transform::from_xyz(GALLERY_X, row_y, 0.0)
                .with_scale(Vec3::splat(GALLERY_SCALE)),
            GallerySprite {
                index: i,
                first_frame: 0,
                last_frame: def.last_frame.unwrap_or(def.frames as usize - 1),
                fps: def.default_fps,
                looping: def.looping,
            },
            GalleryTimer(Timer::from_seconds(1.0 / def.default_fps, TimerMode::Repeating)),
        ));

        // Info text
        let loop_tag = if def.looping { "loop" } else { "once" };
        commands.spawn((
            Text2d::new(format!("0/{}  {}fps  {}", def.frames, def.default_fps, loop_tag)),
            TextFont { font_size: 11.0, ..default() },
            TextColor(Color::srgb(0.5, 0.8, 0.5)),
            Transform::from_xyz(GALLERY_X + 80.0, row_y, 0.0),
            GalleryInfoText(i),
        ));
    }

    // Selection indicator
    commands.spawn((
        Text2d::new(">"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::srgb(1.0, 1.0, 0.0)),
        Transform::from_xyz(GALLERY_X - 110.0, gallery_top, 0.0),
        SelectionIndicator,
    ));

    // --- Divider ---
    commands.spawn((
        Sprite::from_color(Color::srgb(0.3, 0.3, 0.3), Vec2::new(2.0, 800.0)),
        Transform::from_xyz(-530.0, 0.0, 0.0),
    ));

    // --- Character playground ---

    // Ground floor (wide, near bottom)
    spawn_platform(&mut commands, FLOOR_CENTER_X, FLOOR_Y, FLOOR_WIDTH, PLATFORM_THICKNESS);

    // Floating platform (for jump + ledge testing)
    spawn_platform(&mut commands, PLATFORM_X, PLATFORM_Y, PLATFORM_WIDTH, PLATFORM_THICKNESS);

    // Character sprite (starts with idle on ground)
    let idle_clip = clips.get(&CharState::Idle).unwrap();
    let char_start_x = FLOOR_CENTER_X - 100.0;
    let char_start_y = FLOOR_Y + PLATFORM_THICKNESS / 2.0 + CHAR_HALF_H;
    commands.spawn((
        Sprite::from_atlas_image(
            idle_clip.texture.clone(),
            TextureAtlas {
                layout: idle_clip.layout.clone(),
                index: 0,
            },
        ),
        Transform::from_xyz(char_start_x, char_start_y, 1.0)
            .with_scale(Vec3::splat(CHARACTER_SCALE)),
        Character {
            state: CharState::Idle,
            velocity: Vec2::ZERO,
            grounded: true,
            facing_right: true,
            jab_timer: 0.0,
            ledge_timer: 0.0,
        },
        CharAnimTimer(Timer::from_seconds(
            1.0 / idle_clip.default_fps,
            TimerMode::Repeating,
        )),
        CharCurrentAnim {
            state: CharState::Idle,
            first_frame: idle_clip.first_frame,
            last_frame: idle_clip.last_frame,
        },
    ));

    // State/frame debug text (top of window)
    commands.spawn((
        Text2d::new("State: Idle"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::srgb(0.8, 0.8, 0.2)),
        TextLayout::new_with_justify(Justify::Center),
        Transform::from_xyz(150.0, 370.0, 0.0),
        CharStateText,
    ));

    // Controls help text (top of window, above debug)
    commands.spawn((
        Text2d::new(
            "Gallery: Up/Down/DPad  L/R/LT/RT=FPS  P=pause  F=flip  R=reset\n\
             Char: A/D/Stick  Space/A=jump  Q/LB=jab  E/RB=ledge  Esc/Start=quit",
        ),
        TextFont { font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.45, 0.45, 0.45)),
        TextLayout::new_with_justify(Justify::Center),
        Transform::from_xyz(150.0, 390.0, 0.0),
        ControlsText,
    ));

    // Fall uses the second half of the jump sheet (frames 3-5)
    if let Some(jump_clip) = clips.get(&CharState::Jump) {
        let fall_clip = AnimClip {
            texture: jump_clip.texture.clone(),
            layout: jump_clip.layout.clone(),
            first_frame: 3,
            last_frame: 5,
            default_fps: jump_clip.default_fps,
            looping: false,
        };
        clips.insert(CharState::Fall, fall_clip);
    }

    // Jump only uses the first half (frames 0-2)
    if let Some(jump_clip) = clips.get_mut(&CharState::Jump) {
        jump_clip.last_frame = 2;
    }

    commands.insert_resource(AnimClips(clips));
}

fn spawn_platform(commands: &mut Commands, x: f32, y: f32, width: f32, thickness: f32) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.3, 0.3, 0.4), Vec2::new(width, thickness)),
        Transform::from_xyz(x, y, 0.0),
        TestPlatform { x, y, width, thickness },
    ));
}

// --- Gallery systems ---

fn animate_gallery(
    time: Res<Time>,
    gallery_state: Res<GalleryState>,
    mut query: Query<(&GallerySprite, &mut GalleryTimer, &mut Sprite)>,
) {
    if gallery_state.paused {
        return;
    }

    for (gallery, mut timer, mut sprite) in &mut query {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            if let Some(ref mut atlas) = sprite.texture_atlas {
                if gallery.looping {
                    if atlas.index >= gallery.last_frame {
                        atlas.index = gallery.first_frame;
                    } else {
                        atlas.index += 1;
                    }
                } else if atlas.index < gallery.last_frame {
                    atlas.index += 1;
                }
            }
        }
        sprite.flip_x = gallery_state.flipped;
    }
}

fn handle_gallery_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut gallery_state: ResMut<GalleryState>,
    mut query: Query<(&mut GallerySprite, &mut GalleryTimer, &mut Sprite)>,
) {
    let count = ANIM_DEFS.len();

    let up = keyboard.just_pressed(KeyCode::ArrowUp)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::DPadUp));
    let down = keyboard.just_pressed(KeyCode::ArrowDown)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::DPadDown));

    if up {
        gallery_state.selected = if gallery_state.selected == 0 {
            count - 1
        } else {
            gallery_state.selected - 1
        };
    }
    if down {
        gallery_state.selected = (gallery_state.selected + 1) % count;
    }

    if keyboard.just_pressed(KeyCode::KeyP) {
        gallery_state.paused = !gallery_state.paused;
    }
    if keyboard.just_pressed(KeyCode::KeyF) {
        gallery_state.flipped = !gallery_state.flipped;
    }

    let reset = keyboard.just_pressed(KeyCode::KeyR)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::DPadLeft));
    if reset {
        for (gallery, _, mut sprite) in &mut query {
            if let Some(ref mut atlas) = sprite.texture_atlas {
                atlas.index = gallery.first_frame;
            }
        }
    }

    let fps_up = keyboard.just_pressed(KeyCode::ArrowRight)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::RightTrigger2));
    let fps_down = keyboard.just_pressed(KeyCode::ArrowLeft)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::LeftTrigger2));

    let fps_delta = if fps_up {
        1.0
    } else if fps_down {
        -1.0
    } else {
        0.0
    };

    if fps_delta != 0.0 {
        for (mut gallery, mut timer, _) in &mut query {
            if gallery.index == gallery_state.selected {
                gallery.fps = (gallery.fps + fps_delta).clamp(1.0, 30.0);
                timer.0 = Timer::from_seconds(1.0 / gallery.fps, TimerMode::Repeating);
            }
        }
    }
}

fn update_gallery_info_text(
    gallery_query: Query<(&GallerySprite, &Sprite)>,
    mut text_query: Query<(&GalleryInfoText, &mut Text2d)>,
    gallery_state: Res<GalleryState>,
) {
    for (info, mut text) in &mut text_query {
        for (gallery, sprite) in &gallery_query {
            if gallery.index == info.0 {
                let frame_idx = sprite.texture_atlas.as_ref().map_or(0, |a| a.index);
                let clip_len = gallery.last_frame - gallery.first_frame + 1;
                let frame_in_clip = frame_idx - gallery.first_frame + 1;
                let selected = if gallery.index == gallery_state.selected { " *" } else { "" };
                let paused = if gallery_state.paused { " [P]" } else { "" };
                let loop_tag = if gallery.looping { "loop" } else { "once" };
                **text = format!(
                    "{}/{}  {}fps  {}{}{}",
                    frame_in_clip, clip_len, gallery.fps, loop_tag, selected, paused,
                );
            }
        }
    }
}

fn update_selection_indicator(
    gallery_state: Res<GalleryState>,
    mut query: Query<&mut Transform, With<SelectionIndicator>>,
) {
    let gallery_top = (ANIM_DEFS.len() as f32 - 1.0) / 2.0 * GALLERY_ROW_SPACING;
    for mut transform in &mut query {
        transform.translation.y = gallery_top - gallery_state.selected as f32 * GALLERY_ROW_SPACING;
    }
}

// --- Character systems ---

fn handle_character_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut query: Query<&mut Character>,
) {
    for mut character in &mut query {
        // Horizontal movement
        let mut move_x = 0.0;
        if keyboard.pressed(KeyCode::KeyA) {
            move_x -= 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            move_x += 1.0;
        }
        for gamepad in &gamepads {
            if let Some(stick_x) = gamepad.get(GamepadAxis::LeftStickX) {
                if stick_x.abs() > STICK_DEADZONE {
                    move_x += stick_x;
                }
            }
        }
        move_x = move_x.clamp(-1.0, 1.0);
        character.velocity.x = move_x * CHAR_MOVE_SPEED;

        if move_x > 0.0 {
            character.facing_right = true;
        } else if move_x < 0.0 {
            character.facing_right = false;
        }

        // Jump
        let jump = keyboard.just_pressed(KeyCode::Space)
            || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::South));
        if jump && character.grounded {
            character.velocity.y = CHAR_JUMP_VEL;
            character.grounded = false;
        }

        // Jab
        let jab = keyboard.just_pressed(KeyCode::KeyQ)
            || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::LeftTrigger));
        if jab && character.grounded {
            let jab_def = &ANIM_DEFS[3];
            let jab_frames =
                (jab_def.last_frame.unwrap_or(jab_def.frames as usize - 1) + 1) as f32;
            character.jab_timer = jab_frames / jab_def.default_fps;
        }

        // Ledge
        let ledge = keyboard.just_pressed(KeyCode::KeyE)
            || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::RightTrigger));
        if ledge {
            character.ledge_timer = ANIM_DEFS[4].frames as f32 / ANIM_DEFS[4].default_fps;
            character.velocity = Vec2::ZERO;
            character.grounded = false;
        }
    }
}

fn update_character_physics(
    time: Res<Time>,
    platforms: Query<&TestPlatform>,
    mut query: Query<(&mut Character, &mut Transform)>,
) {
    let dt = time.delta_secs();

    for (mut character, mut transform) in &mut query {
        // Ledge hold freezes physics
        if character.ledge_timer > 0.0 {
            character.ledge_timer -= dt;
            continue;
        }

        // Jab timer countdown (movement continues during jab)
        if character.jab_timer > 0.0 {
            character.jab_timer -= dt;
        }

        // Gravity
        if !character.grounded {
            character.velocity.y -= CHAR_GRAVITY * dt;
        }

        // Apply velocity
        transform.translation.x += character.velocity.x * dt;
        transform.translation.y += character.velocity.y * dt;

        // Platform collision (only when falling)
        character.grounded = false;
        if character.velocity.y <= 0.0 {
            for plat in &platforms {
                let plat_top = plat.y + plat.thickness / 2.0;
                let plat_left = plat.x - plat.width / 2.0;
                let plat_right = plat.x + plat.width / 2.0;
                let char_bottom = transform.translation.y - CHAR_HALF_H;
                let prev_bottom = char_bottom - character.velocity.y * dt; // where we were

                // Horizontally overlapping and falling through the top surface
                if transform.translation.x > plat_left
                    && transform.translation.x < plat_right
                    && char_bottom <= plat_top
                    && prev_bottom >= plat_top - 2.0 // was above (with small epsilon)
                {
                    transform.translation.y = plat_top + CHAR_HALF_H;
                    character.velocity.y = 0.0;
                    character.grounded = true;
                    break;
                }
            }
        }

        // Clamp to playground area
        let min_x = FLOOR_CENTER_X - FLOOR_WIDTH / 2.0 + 20.0;
        let max_x = FLOOR_CENTER_X + FLOOR_WIDTH / 2.0 - 20.0;
        transform.translation.x = transform.translation.x.clamp(min_x, max_x);
    }
}

fn update_character_animation(
    clips: Res<AnimClips>,
    mut query: Query<(
        &mut Character,
        &mut Sprite,
        &mut CharAnimTimer,
        &mut CharCurrentAnim,
    )>,
) {
    for (mut character, mut sprite, mut anim_timer, mut current) in &mut query {
        let desired = if character.ledge_timer > 0.0 {
            CharState::Ledge
        } else if character.jab_timer > 0.0 {
            CharState::Jab
        } else if !character.grounded && character.velocity.y > 0.0 {
            CharState::Jump
        } else if !character.grounded && character.velocity.y <= 0.0 {
            CharState::Fall
        } else if character.velocity.x.abs() > CHAR_RUN_THRESHOLD {
            CharState::Run
        } else {
            CharState::Idle
        };

        character.state = desired;
        sprite.flip_x = !character.facing_right;

        if current.state != desired {
            if let Some(clip) = clips.0.get(&desired) {
                sprite.image = clip.texture.clone();
                if let Some(ref mut atlas) = sprite.texture_atlas {
                    atlas.layout = clip.layout.clone();
                    atlas.index = clip.first_frame;
                }
                anim_timer.0 =
                    Timer::from_seconds(1.0 / clip.default_fps, TimerMode::Repeating);
                anim_timer.0.reset();
                current.state = desired;
                current.first_frame = clip.first_frame;
                current.last_frame = clip.last_frame;
            }
        }
    }
}

fn animate_character_sprite(
    time: Res<Time>,
    mut query: Query<(&Character, &mut CharAnimTimer, &CharCurrentAnim, &mut Sprite)>,
) {
    for (character, mut timer, current, mut sprite) in &mut query {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            if let Some(ref mut atlas) = sprite.texture_atlas {
                let is_looping =
                    matches!(character.state, CharState::Idle | CharState::Run | CharState::Ledge);
                if is_looping {
                    if atlas.index >= current.last_frame {
                        atlas.index = current.first_frame;
                    } else {
                        atlas.index += 1;
                    }
                } else if atlas.index < current.last_frame {
                    atlas.index += 1;
                }
            }
        }
    }
}

fn update_char_state_text(
    character_query: Query<(&Character, &Sprite, &CharCurrentAnim)>,
    mut text_query: Query<&mut Text2d, With<CharStateText>>,
) {
    for (character, sprite, current) in &character_query {
        let frame = sprite.texture_atlas.as_ref().map_or(0, |a| a.index);
        let frame_in_clip = frame - current.first_frame + 1;
        let clip_len = current.last_frame - current.first_frame + 1;
        for mut text in &mut text_query {
            **text = format!(
                "State: {:?}  Frame: {}/{}  Vel: ({:.0}, {:.0})  Grounded: {}  Facing: {}",
                character.state,
                frame_in_clip,
                clip_len,
                character.velocity.x,
                character.velocity.y,
                character.grounded,
                if character.facing_right { "Right" } else { "Left" },
            );
        }
    }
}

fn check_exit(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut exit: MessageWriter<AppExit>,
) {
    let quit = keyboard.just_pressed(KeyCode::Escape)
        || gamepads.iter().any(|gp| gp.just_pressed(GamepadButton::Start));
    if quit {
        exit.write(AppExit::Success);
    }
}
