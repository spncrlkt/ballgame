//! Debug UI components and resources
//!
//! This module contains resources and types used by the debug menu system.
//! The old cycle indicator, debug text, and tweak panel have been replaced
//! by the unified debug menu in debug_menu.rs.

use bevy::prelude::*;

use crate::ball::{Ball, BallStyle, BallTextures};
use crate::constants::{DEFAULT_VIEWPORT_INDEX, VIEWPORT_PRESETS};
use crate::palettes::PaletteDatabase;
use crate::player::{Character, Player};
use crate::ui::hud::ScoreLevelText;
use crate::world::{Basket, BasketRim, CornerRamp, LevelPlatform, Platform};

// =============================================================================
// CYCLE SELECTION - Tracks saved settings state
// =============================================================================

/// Which D-pad direction is currently active for value cycling
/// (kept for settings persistence compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CycleDirection {
    #[default]
    Up,
    Down,
    Left,
    Right,
}

impl CycleDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            CycleDirection::Up => "Up",
            CycleDirection::Down => "Down",
            CycleDirection::Left => "Left",
            CycleDirection::Right => "Right",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Up" => CycleDirection::Up,
            "Left" => CycleDirection::Left,
            "Right" => CycleDirection::Right,
            _ => CycleDirection::Down,
        }
    }
}

/// Options available for D-pad Down (presets)
/// (kept for settings persistence compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DownOption {
    #[default]
    Composite,
    Movement,
    Ball,
    Shooting,
}

impl DownOption {
    pub fn next(&self) -> Self {
        match self {
            DownOption::Composite => DownOption::Movement,
            DownOption::Movement => DownOption::Ball,
            DownOption::Ball => DownOption::Shooting,
            DownOption::Shooting => DownOption::Composite,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            DownOption::Composite => "Composite",
            DownOption::Movement => "Movement",
            DownOption::Ball => "Ball",
            DownOption::Shooting => "Shooting",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Movement" => DownOption::Movement,
            "Ball" => DownOption::Ball,
            "Shooting" => DownOption::Shooting,
            _ => DownOption::Composite,
        }
    }
}

/// Options available for D-pad Right (visual/level settings + character)
/// (kept for settings persistence compatibility)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightOption {
    #[default]
    Character,
    Level,
    Palette,
    BallStyle,
}

impl RightOption {
    pub fn next(&self) -> Self {
        match self {
            RightOption::Character => RightOption::Level,
            RightOption::Level => RightOption::Palette,
            RightOption::Palette => RightOption::BallStyle,
            RightOption::BallStyle => RightOption::Character,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            RightOption::Character => "Character",
            RightOption::Level => "Level",
            RightOption::Palette => "Palette",
            RightOption::BallStyle => "BallStyle",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "Character" => RightOption::Character,
            "Palette" => RightOption::Palette,
            "BallStyle" => RightOption::BallStyle,
            _ => RightOption::Level,
        }
    }
}

/// Tracks cycle state (kept for settings persistence compatibility)
#[derive(Resource)]
pub struct CycleSelection {
    pub active_direction: CycleDirection,
    pub down_option: DownOption,
    pub right_option: RightOption,
    pub ai_player_index: usize,
    pub menu_enabled: bool,
}

impl Default for CycleSelection {
    fn default() -> Self {
        Self {
            active_direction: CycleDirection::Down,
            down_option: DownOption::Composite,
            right_option: RightOption::Level,
            ai_player_index: 0,
            menu_enabled: false,
        }
    }
}

/// Debug settings resource
#[derive(Resource)]
pub struct DebugSettings {
    pub visible: bool,
}

impl Default for DebugSettings {
    fn default() -> Self {
        Self { visible: true }
    }
}

/// Current viewport scale preset index
#[derive(Resource)]
pub struct ViewportScale {
    pub preset_index: usize,
}

impl Default for ViewportScale {
    fn default() -> Self {
        Self {
            preset_index: DEFAULT_VIEWPORT_INDEX,
        }
    }
}

impl ViewportScale {
    /// Get current preset (width, height, label)
    pub fn current(&self) -> (f32, f32, &'static str) {
        VIEWPORT_PRESETS[self.preset_index]
    }

    /// Cycle to next preset
    pub fn cycle_next(&mut self) {
        self.preset_index = (self.preset_index + 1) % VIEWPORT_PRESETS.len();
    }

    /// Cycle to previous preset
    pub fn cycle_prev(&mut self) {
        self.preset_index =
            (self.preset_index + VIEWPORT_PRESETS.len() - 1) % VIEWPORT_PRESETS.len();
    }
}

/// Toggle debug UI visibility (no-op - kept for compatibility)
pub fn toggle_debug(
    _keyboard: Res<ButtonInput<KeyCode>>,
    _settings: ResMut<DebugSettings>,
) {
    // No-op - debug menu now handles visibility
}

// =============================================================================
// PALETTE COLORS
// =============================================================================

/// Apply palette colors when CurrentPalette changes
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
pub fn apply_palette_colors(
    current_palette: Res<crate::ball::CurrentPalette>,
    palette_db: Res<PaletteDatabase>,
    ball_textures: Res<BallTextures>,
    mut clear_color: ResMut<ClearColor>,
    mut player_query: Query<
        (&mut Sprite, &Character),
        (With<Player>, Without<Ball>, Without<Basket>),
    >,
    basket_query: Query<(&Basket, &Children), Without<Player>>,
    mut stripe_query: Query<
        (&mut Sprite, &crate::world::BasketStripe),
        (
            Without<BasketRim>,
            Without<Player>,
            Without<Ball>,
            Without<Platform>,
        ),
    >,
    mut rim_query: Query<
        &mut Sprite,
        (
            With<BasketRim>,
            Without<Player>,
            Without<Ball>,
            Without<Basket>,
        ),
    >,
    mut floor_query: Query<
        &mut Sprite,
        (
            With<Platform>,
            Without<LevelPlatform>,
            Without<CornerRamp>,
            Without<Player>,
            Without<Ball>,
            Without<Basket>,
            Without<BasketRim>,
            Without<crate::world::BasketStripe>,
        ),
    >,
    mut level_platform_query: Query<
        &mut Sprite,
        (
            With<LevelPlatform>,
            Without<CornerRamp>,
            Without<Player>,
            Without<Ball>,
            Without<Basket>,
            Without<BasketRim>,
            Without<crate::world::BasketStripe>,
        ),
    >,
    mut corner_ramp_query: Query<
        &mut Sprite,
        (
            With<CornerRamp>,
            Without<LevelPlatform>,
            Without<Player>,
            Without<Ball>,
            Without<Basket>,
            Without<BasketRim>,
            Without<crate::world::BasketStripe>,
        ),
    >,
    mut ball_query: Query<(&BallStyle, &mut Sprite), With<Ball>>,
    mut score_text_query: Query<&mut TextColor, With<ScoreLevelText>>,
) {
    // Only run when palette actually changes
    if !current_palette.is_changed() {
        return;
    }

    let palette = palette_db
        .get(current_palette.0)
        .expect("Palette index out of bounds");

    // Background
    clear_color.0 = palette.background;

    // Players - use character-specific colors (slot 1 players are darker)
    for (mut sprite, character) in &mut player_query {
        sprite.color = crate::player::color_for_character(character.0, palette);
    }

    // Baskets - update stripe colors and rims
    for (basket, children) in &basket_query {
        // Get team colors for this basket
        let (color1, color2, rim_color) = match basket {
            Basket::Left => (
                palette.left,
                crate::player::color_for_character(crate::events::CharacterId::L1, palette),
                palette.right_rim,
            ),
            Basket::Right => (
                palette.right,
                crate::player::color_for_character(crate::events::CharacterId::R1, palette),
                palette.left_rim,
            ),
        };

        // Update stripe and rim colors (children)
        for child in children.iter() {
            if let Ok((mut stripe_sprite, stripe)) = stripe_query.get_mut(child) {
                stripe_sprite.color = if stripe.index % 2 == 0 { color1 } else { color2 };
            }
            if let Ok(mut rim_sprite) = rim_query.get_mut(child) {
                rim_sprite.color = rim_color;
            }
        }
    }

    // Floor and walls
    for mut sprite in &mut floor_query {
        sprite.color = palette.platforms;
    }

    // Level platforms (same color as floor)
    for mut sprite in &mut level_platform_query {
        sprite.color = palette.platforms;
    }

    // Corner ramps
    for mut sprite in &mut corner_ramp_query {
        sprite.color = palette.platforms;
    }

    // Ball textures
    for (style, mut sprite) in &mut ball_query {
        if let Some(textures) = ball_textures.get(style.name()) {
            if let Some(texture) = textures.textures.get(current_palette.0) {
                sprite.image = texture.clone();
            }
        }
    }

    // Text colors (score/level text)
    for mut text_color in &mut score_text_query {
        text_color.0 = palette.text;
    }
}
