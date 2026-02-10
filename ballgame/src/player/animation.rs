//! Player sprite animation system
//!
//! Manages animated sprite sheets for player characters. Each animation state
//! (Idle, Run, Jump, Fall, Jab) maps to a clip with texture, atlas layout,
//! frame range, and FPS. Two Update systems drive the animation:
//!   - `update_player_animation_state` picks the correct clip based on game state
//!   - `animate_player_sprites` advances frames on a timer

use bevy::prelude::*;
use std::collections::HashMap;

use crate::constants::*;
use crate::player::{BlockState, Facing, Grounded, Player, Velocity};

// =============================================================================
// Types
// =============================================================================

/// Animation state for player sprites
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerAnimState {
    Idle,
    Run,
    Jump,
    Fall,
    Jab,
}

/// A single animation clip: texture + atlas layout + frame range + timing
#[derive(Clone)]
pub struct PlayerAnimClip {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub first_frame: usize,
    pub last_frame: usize,
    pub fps: f32,
    pub looping: bool,
}

/// Resource holding all loaded animation clips, keyed by state.
#[derive(Resource, Clone)]
pub struct PlayerAnimClips(pub HashMap<PlayerAnimState, PlayerAnimClip>);

/// Marker for the visual sprite child entity.
/// The animated (or colored) sprite lives on this child, offset upward so
/// the character's feet align with the parent's collision-box bottom.
#[derive(Component)]
pub struct PlayerVisual;

/// Per-entity timer that drives frame advance.
#[derive(Component)]
pub struct PlayerAnimTimer(pub Timer);

/// Tracks which animation is currently playing on this entity.
#[derive(Component)]
pub struct PlayerCurrentAnim {
    pub state: PlayerAnimState,
    pub first_frame: usize,
    pub last_frame: usize,
}

// =============================================================================
// Asset loading
// =============================================================================

/// Load all player animation clips from sprite sheets.
///
/// Call once during setup; returns a `PlayerAnimClips` resource.
pub fn load_player_animations(
    asset_server: &AssetServer,
    atlas_layouts: &mut Assets<TextureAtlasLayout>,
) -> PlayerAnimClips {
    let mut clips = HashMap::new();

    // Idle: 10 frames, all used, looping
    let idle_texture = asset_server.load("sprites/idle.png");
    let idle_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(ANIM_FRAME_SIZE), 10, 1, None, None,
    ));
    clips.insert(PlayerAnimState::Idle, PlayerAnimClip {
        texture: idle_texture, layout: idle_layout,
        first_frame: 0, last_frame: 9, fps: ANIM_FPS_IDLE, looping: true,
    });

    // Run: 8 frames, all used, looping
    let run_texture = asset_server.load("sprites/run.png");
    let run_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(ANIM_FRAME_SIZE), 8, 1, None, None,
    ));
    clips.insert(PlayerAnimState::Run, PlayerAnimClip {
        texture: run_texture, layout: run_layout,
        first_frame: 0, last_frame: 7, fps: ANIM_FPS_RUN, looping: true,
    });

    // Jump sheet: 6 frames, split into Jump (0-2) and Fall (3-5)
    let jump_texture: Handle<Image> = asset_server.load("sprites/jump.png");
    let jump_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(ANIM_FRAME_SIZE), 6, 1, None, None,
    ));
    clips.insert(PlayerAnimState::Jump, PlayerAnimClip {
        texture: jump_texture.clone(), layout: jump_layout.clone(),
        first_frame: 0, last_frame: 2, fps: ANIM_FPS_JUMP, looping: false,
    });
    clips.insert(PlayerAnimState::Fall, PlayerAnimClip {
        texture: jump_texture, layout: jump_layout,
        first_frame: 3, last_frame: 5, fps: ANIM_FPS_JUMP, looping: false,
    });

    // Jab: 10-frame sheet, clipped to 0..ANIM_JAB_LAST_FRAME, non-looping
    let jab_texture = asset_server.load("sprites/jab.png");
    let jab_layout = atlas_layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(ANIM_FRAME_SIZE), 10, 1, None, None,
    ));
    clips.insert(PlayerAnimState::Jab, PlayerAnimClip {
        texture: jab_texture, layout: jab_layout,
        first_frame: 0, last_frame: ANIM_JAB_LAST_FRAME, fps: ANIM_FPS_JAB, looping: false,
    });

    PlayerAnimClips(clips)
}

// =============================================================================
// Systems
// =============================================================================

/// Determine desired animation state from gameplay components and swap clips on change.
///
/// Priority: Jab (BlockState.active) > Jump (rising, !grounded) > Fall > Run > Idle.
/// Also sets `sprite.flip_x` based on `Facing`.
/// Queries the `PlayerVisual` child entity and reads gameplay state from its `Parent`.
#[allow(clippy::type_complexity)]
pub fn update_player_animation_state(
    clips: Res<PlayerAnimClips>,
    parent_query: Query<(&Velocity, &Grounded, &Facing, &BlockState), With<Player>>,
    mut visual_query: Query<
        (
            &ChildOf,
            &mut Sprite,
            &mut PlayerAnimTimer,
            &mut PlayerCurrentAnim,
        ),
        With<PlayerVisual>,
    >,
) {
    for (child_of, mut sprite, mut anim_timer, mut current) in &mut visual_query {
        let Ok((velocity, grounded, facing, block)) = parent_query.get(child_of.parent()) else {
            continue;
        };
        // Pick desired state
        let desired = if block.active {
            PlayerAnimState::Jab
        } else if !grounded.0 && velocity.0.y > 0.0 {
            PlayerAnimState::Jump
        } else if !grounded.0 && velocity.0.y <= 0.0 {
            PlayerAnimState::Fall
        } else if velocity.0.x.abs() > ANIM_RUN_THRESHOLD {
            PlayerAnimState::Run
        } else {
            PlayerAnimState::Idle
        };

        // Flip sprite based on facing direction
        sprite.flip_x = facing.0 < 0.0;

        // Swap clip on state change
        if current.state != desired
            && let Some(clip) = clips.0.get(&desired)
        {
            sprite.image = clip.texture.clone();
            if let Some(ref mut atlas) = sprite.texture_atlas {
                atlas.layout = clip.layout.clone();
                atlas.index = clip.first_frame;
            }
            anim_timer.0 = Timer::from_seconds(1.0 / clip.fps, TimerMode::Repeating);
            anim_timer.0.reset();
            current.state = desired;
            current.first_frame = clip.first_frame;
            current.last_frame = clip.last_frame;
        }
    }
}

/// Advance animation frames based on the per-entity timer.
///
/// Looping animations wrap; non-looping hold on last frame.
pub fn animate_player_sprites(
    time: Res<Time>,
    mut query: Query<(&mut PlayerAnimTimer, &PlayerCurrentAnim, &mut Sprite), With<PlayerVisual>>,
) {
    for (mut timer, current, mut sprite) in &mut query {
        timer.0.tick(time.delta());
        if timer.0.just_finished()
            && let Some(ref mut atlas) = sprite.texture_atlas
        {
            let is_looping = matches!(
                current.state,
                PlayerAnimState::Idle | PlayerAnimState::Run
            );
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
