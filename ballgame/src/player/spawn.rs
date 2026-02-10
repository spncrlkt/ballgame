//! Character spawning helpers for 2v2 support
//!
//! Provides functions to spawn characters based on game mode and controller mapping.

use bevy::prelude::*;

use crate::ai::{AiGoal, AiNavState, AiProfileDatabase, AiState, InputState};
use crate::constants::*;
use crate::events::CharacterId;
use crate::input::{GameMode, InputSourceId, AI_SOURCE_ID_START, KEYBOARD_SOURCE_ID};
use crate::palettes::Palette;
use crate::player::animation::{PlayerAnimClips, PlayerAnimState, PlayerAnimTimer, PlayerCurrentAnim, PlayerVisual};
use crate::player::{
    BlockState, Character, ControlledBy, CoyoteTimer, Facing, Grounded, HumanControlled,
    JumpState, Player, Buff, TargetBasket, Team, TurboGauge, Velocity,
};
use crate::shooting::ChargingShot;
use crate::steal::StealCooldown;
use crate::ui::{ChargeGaugeBackground, ChargeGaugeFill, ChargeGaugeOutline};
use crate::world::{Basket, Collider};

/// Get spawn position for a character
pub fn spawn_position(character: CharacterId) -> Vec3 {
    match character {
        CharacterId::L0 => SPAWN_L0,
        CharacterId::L1 => SPAWN_L1,
        CharacterId::R0 => SPAWN_R0,
        CharacterId::R1 => SPAWN_R1,
    }
}

/// Get team for a character
pub fn team_for_character(character: CharacterId) -> Team {
    match character.team() {
        crate::events::TeamId::Left => Team::Left,
        crate::events::TeamId::Right => Team::Right,
    }
}

/// Get target basket for a character (team scores in opposite basket)
pub fn target_basket_for_character(character: CharacterId) -> Basket {
    match character.team() {
        crate::events::TeamId::Left => Basket::Right,  // Left team shoots at right basket
        crate::events::TeamId::Right => Basket::Left,  // Right team shoots at left basket
    }
}

/// Get initial facing direction for a character
pub fn initial_facing(character: CharacterId) -> f32 {
    match character.team() {
        crate::events::TeamId::Left => 1.0,   // Left team faces right
        crate::events::TeamId::Right => -1.0, // Right team faces left
    }
}

/// Get player color from palette for a character
/// Slot 0 players (L0, R0) use the main team color
/// Slot 1 players (L1, R1) use a significantly darker version (40% brightness)
pub fn color_for_character(character: CharacterId, palette: &Palette) -> Color {
    let base_color = match character.team() {
        crate::events::TeamId::Left => palette.left,
        crate::events::TeamId::Right => palette.right,
    };

    if character.slot() == 1 {
        // Create a noticeably darker shade - 40% of original brightness
        let srgba = base_color.to_srgba();
        Color::srgb(
            srgba.red * 0.4,
            srgba.green * 0.4,
            srgba.blue * 0.4,
        )
    } else {
        base_color
    }
}

/// Configuration for spawning a character
pub struct CharacterSpawnConfig {
    /// Which character to spawn
    pub character: CharacterId,
    /// Input source controlling this character (None = AI with default profile)
    pub controller: Option<InputSourceId>,
    /// AI profile ID (for AI-controlled characters)
    pub ai_profile_id: String,
    /// Whether to start in Idle goal (for debug/regression levels)
    pub start_idle: bool,
    /// Whether this is the human-controlled character (for HumanControlled marker)
    pub is_human_controlled: bool,
    /// Super-ability for this character (defaults to Speed)
    pub ability: Buff,
    /// Override spawn position (None = default for CharacterId)
    pub position_override: Option<Vec3>,
    /// Override facing direction (None = default for team)
    pub facing_override: Option<f32>,
    /// Override initial AI goal (None = Idle if start_idle, else AiGoal::default())
    pub initial_goal_override: Option<AiGoal>,
}

/// Spawn a single character entity
/// Returns the entity ID
///
/// When `anim_clips` is `Some`, the character uses animated atlas sprites.
/// When `None`, falls back to a solid-color rectangle (headless/simulation).
pub fn spawn_character(
    commands: &mut Commands,
    config: CharacterSpawnConfig,
    palette: &Palette,
    anim_clips: Option<&PlayerAnimClips>,
) -> Entity {
    let position = config
        .position_override
        .unwrap_or_else(|| spawn_position(config.character));
    let team = team_for_character(config.character);
    let target_basket = target_basket_for_character(config.character);
    let facing = config
        .facing_override
        .unwrap_or_else(|| initial_facing(config.character));
    let color = color_for_character(config.character, palette);

    let initial_goal = if let Some(goal) = config.initial_goal_override {
        goal
    } else if config.start_idle {
        AiGoal::Idle
    } else {
        AiGoal::default()
    };

    // Determine controller source ID
    let controller_id = config.controller.unwrap_or_else(|| {
        // AI-controlled: use a unique AI source ID
        // In practice this should be set properly by the caller
        AI_SOURCE_ID_START + config.character.slot() as u32
            + if config.character.team() == crate::events::TeamId::Right {
                10
            } else {
                0
            }
    });

    // Build visual sprite as a child entity, offset upward so the character's feet
    // align with the parent's collision box bottom. The parent entity has no sprite —
    // all collision uses PLAYER_SIZE directly.
    let visual_offset_y = PLAYER_SPRITE_OFFSET_Y;

    // Prepare visual sprite + optional animation components
    let (visual_sprite, anim_timer, anim_current) = if let Some(clips) = anim_clips {
        if let Some(idle_clip) = clips.0.get(&PlayerAnimState::Idle) {
            let mut s = Sprite::from_atlas_image(
                idle_clip.texture.clone(),
                TextureAtlas {
                    layout: idle_clip.layout.clone(),
                    index: 0,
                },
            );
            s.custom_size = Some(PLAYER_SPRITE_SIZE);
            let timer = PlayerAnimTimer(Timer::from_seconds(
                1.0 / idle_clip.fps,
                TimerMode::Repeating,
            ));
            let current = PlayerCurrentAnim {
                state: PlayerAnimState::Idle,
                first_frame: idle_clip.first_frame,
                last_frame: idle_clip.last_frame,
            };
            (s, Some(timer), Some(current))
        } else {
            (Sprite::from_color(color, PLAYER_SPRITE_SIZE), None, None)
        }
    } else {
        (Sprite::from_color(color, PLAYER_SIZE), None, None)
    };

    // Split into smaller bundles to avoid hitting Bevy's tuple size limit
    let mut entity_commands = commands.spawn((
        Transform::from_translation(position),
        Visibility::default(),
        Player,
        Character(config.character),
        ControlledBy(controller_id),
        Velocity::default(),
        Grounded(false),
        CoyoteTimer::default(),
    ));

    entity_commands.insert((
        JumpState::default(),
        Facing(facing),
        ChargingShot::default(),
        TargetBasket(target_basket),
        Collider,
        team,
    ));

    entity_commands.insert((
        InputState::default(),
        AiState {
            current_goal: initial_goal,
            profile_id: config.ai_profile_id,
            ..default()
        },
        AiNavState::default(),
        StealCooldown::default(),
    ));

    // Create turbo gauge, applying Turbo ability bonus if applicable
    let turbo_gauge = if config.ability == Buff::Turbo {
        use crate::constants::*;
        TurboGauge {
            current: TURBO_MAX_GAUGE * BUFF_TURBO_GAUGE,
            max: TURBO_MAX_GAUGE * BUFF_TURBO_GAUGE,
            drain_rate: TURBO_DRAIN_RATE,
            refill_rate: TURBO_REFILL_RATE * BUFF_TURBO_REFILL,
        }
    } else {
        TurboGauge::default()
    };

    // Turbo, block, and ability components for new mechanics
    entity_commands.insert((turbo_gauge, BlockState::default(), config.ability));

    // Add HumanControlled marker if this is the human-controlled character
    if config.is_human_controlled {
        entity_commands.insert(HumanControlled);
    }

    // Spawn visual sprite as a child entity, offset upward
    let player_entity = entity_commands.id();
    let mut visual_commands = commands.spawn((
        visual_sprite,
        Transform::from_xyz(0.0, visual_offset_y, 0.0),
        PlayerVisual,
    ));
    // Add animation components to the visual child if clips were provided
    if let (Some(timer), Some(current)) = (anim_timer, anim_current) {
        visual_commands.insert((timer, current));
    }
    let visual_entity = visual_commands.id();
    commands.entity(player_entity).add_child(visual_entity);

    player_entity
}

/// Spawn all characters for a game mode
/// Returns a map of character ID to entity
pub fn spawn_characters_for_mode(
    commands: &mut Commands,
    mode: GameMode,
    palette: &Palette,
    left_profile: &str,
    right_profile: &str,
    human_controlled: Option<CharacterId>,
    start_idle: bool,
    profile_db: &AiProfileDatabase,
    anim_clips: Option<&PlayerAnimClips>,
) -> Vec<(CharacterId, Entity)> {
    let characters = mode.characters();
    let mut results = Vec::with_capacity(characters.len());

    for &character in characters {
        let profile_id = match character.team() {
            crate::events::TeamId::Left => left_profile.to_string(),
            crate::events::TeamId::Right => right_profile.to_string(),
        };

        // Look up the AI profile to get preferred_buff
        let ability = profile_db
            .get_by_id(&profile_id)
            .map(|p| p.preferred_buff)
            .unwrap_or(Buff::Speed);

        let is_human = human_controlled == Some(character);
        let controller = if is_human {
            Some(KEYBOARD_SOURCE_ID)
        } else {
            None // Will use AI source ID
        };

        let config = CharacterSpawnConfig {
            character,
            controller,
            ai_profile_id: profile_id,
            start_idle,
            is_human_controlled: is_human,
            ability,
            position_override: None,
            facing_override: None,
            initial_goal_override: None,
        };

        let entity = spawn_character(commands, config, palette, anim_clips);
        results.push((character, entity));
    }

    results
}

/// Spawn charge gauge UI elements as children of a player entity.
///
/// The gauge is a horizontal bar centered above the player's head.
pub fn spawn_charge_gauge(commands: &mut Commands, player_entity: Entity, _facing: f32) {
    // Gauge centered horizontally, positioned above player's head
    let gauge_y = CHARGE_GAUGE_Y_OFFSET;
    let outline_thickness = 2.0;

    // Black outline (slightly larger, behind everything)
    let gauge_outline = commands
        .spawn((
            Sprite::from_color(
                Color::BLACK,
                Vec2::new(
                    CHARGE_GAUGE_WIDTH + outline_thickness * 2.0,
                    CHARGE_GAUGE_HEIGHT + outline_thickness * 2.0,
                ),
            ),
            Transform::from_xyz(0.0, gauge_y, 0.4),
            ChargeGaugeOutline,
        ))
        .id();
    commands.entity(player_entity).add_child(gauge_outline);

    // White background
    let gauge_bg = commands
        .spawn((
            Sprite::from_color(
                Color::WHITE,
                Vec2::new(CHARGE_GAUGE_WIDTH, CHARGE_GAUGE_HEIGHT),
            ),
            Transform::from_xyz(0.0, gauge_y, 0.5),
            ChargeGaugeBackground,
        ))
        .id();
    commands.entity(player_entity).add_child(gauge_bg);

    // Fill (green->red, scales with charge) - starts invisible (scale.x = 0)
    let gauge_fill = commands
        .spawn((
            Sprite::from_color(
                Color::srgb(0.0, 0.8, 0.0),
                Vec2::new(CHARGE_GAUGE_WIDTH - 2.0, CHARGE_GAUGE_HEIGHT - 2.0),
            ),
            Transform::from_xyz(0.0, gauge_y, 0.6).with_scale(Vec3::new(0.0, 1.0, 1.0)),
            ChargeGaugeFill,
        ))
        .id();
    commands.entity(player_entity).add_child(gauge_fill);
}
