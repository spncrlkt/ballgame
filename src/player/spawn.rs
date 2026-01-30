//! Character spawning helpers for 2v2 support
//!
//! Provides functions to spawn characters based on game mode and controller mapping.

use bevy::prelude::*;

use crate::ai::{AiGoal, AiNavState, AiState, InputState};
use crate::constants::*;
use crate::events::CharacterId;
use crate::input::{GameMode, InputSourceId, AI_SOURCE_ID_START, KEYBOARD_SOURCE_ID};
use crate::palettes::Palette;
use crate::player::{
    Character, ControlledBy, CoyoteTimer, Facing, Grounded, HumanControlled, JumpState, Player,
    TargetBasket, Team, Velocity,
};
use crate::shooting::ChargingShot;
use crate::steal::StealCooldown;
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
pub fn color_for_character(character: CharacterId, palette: &Palette) -> Color {
    match character.team() {
        crate::events::TeamId::Left => palette.left,
        crate::events::TeamId::Right => palette.right,
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
}

/// Spawn a single character entity
/// Returns the entity ID
pub fn spawn_character(
    commands: &mut Commands,
    config: CharacterSpawnConfig,
    palette: &Palette,
) -> Entity {
    let position = spawn_position(config.character);
    let team = team_for_character(config.character);
    let target_basket = target_basket_for_character(config.character);
    let facing = initial_facing(config.character);
    let color = color_for_character(config.character, palette);

    let initial_goal = if config.start_idle {
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

    // Split into smaller bundles to avoid hitting Bevy's tuple size limit
    let mut entity_commands = commands.spawn((
        Sprite::from_color(color, PLAYER_SIZE),
        Transform::from_translation(position),
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

    // Add HumanControlled marker if this is the human-controlled character
    if config.is_human_controlled {
        entity_commands.insert(HumanControlled);
    }

    entity_commands.id()
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
) -> Vec<(CharacterId, Entity)> {
    let characters = mode.characters();
    let mut results = Vec::with_capacity(characters.len());

    for &character in characters {
        let profile = match character.team() {
            crate::events::TeamId::Left => left_profile.to_string(),
            crate::events::TeamId::Right => right_profile.to_string(),
        };

        let is_human = human_controlled == Some(character);
        let controller = if is_human {
            Some(KEYBOARD_SOURCE_ID)
        } else {
            None // Will use AI source ID
        };

        let config = CharacterSpawnConfig {
            character,
            controller,
            ai_profile_id: profile,
            start_idle,
            is_human_controlled: is_human,
        };

        let entity = spawn_character(commands, config, palette);
        results.push((character, entity));
    }

    results
}
