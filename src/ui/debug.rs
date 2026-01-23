//! Debug UI components and systems

use bevy::prelude::*;

use crate::ball::{Ball, BallStyleType, BallTextures};
use crate::constants::VIEWPORT_PRESETS;
use crate::levels::LevelDatabase;
use crate::palettes::PaletteDatabase;
use crate::player::{Player, Team};
use crate::scoring::CurrentLevel;
use crate::shooting::LastShotInfo;
use crate::steal::StealContest;
use crate::world::{Basket, BasketRim, CornerRamp, LevelPlatform, Platform};

// =============================================================================
// CYCLE SYSTEM - Unified controller cycling for debug/test options
// =============================================================================

/// What category is currently selected for cycling with RT/LT
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CycleTarget {
    #[default]
    Level,
    Viewport,
    Palette,
    BallStyle,
}

impl CycleTarget {
    pub const ALL: [CycleTarget; 4] = [
        CycleTarget::Level,
        CycleTarget::Viewport,
        CycleTarget::Palette,
        CycleTarget::BallStyle,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            CycleTarget::Level => "Level",
            CycleTarget::Viewport => "Viewport",
            CycleTarget::Palette => "Palette",
            CycleTarget::BallStyle => "Ball Style",
        }
    }

    pub fn next(&self) -> Self {
        let idx = Self::ALL.iter().position(|t| t == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }
}

/// Tracks which cycle target is selected and provides brief feedback
#[derive(Resource)]
pub struct CycleSelection {
    pub target: CycleTarget,
    pub display_timer: f32, // Show indicator briefly after changing
}

impl Default for CycleSelection {
    fn default() -> Self {
        Self {
            target: CycleTarget::Level,
            display_timer: 0.0,
        }
    }
}

impl CycleSelection {
    pub const DISPLAY_DURATION: f32 = 1.5; // How long to show the indicator

    pub fn select_next(&mut self) {
        self.target = self.target.next();
        self.display_timer = Self::DISPLAY_DURATION;
    }

    pub fn flash(&mut self) {
        self.display_timer = Self::DISPLAY_DURATION;
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
#[derive(Resource, Default)]
pub struct ViewportScale {
    pub preset_index: usize,
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
        self.preset_index = (self.preset_index + VIEWPORT_PRESETS.len() - 1) % VIEWPORT_PRESETS.len();
    }
}

/// Debug text component
#[derive(Component)]
pub struct DebugText;

/// Toggle debug UI visibility (Tab / D-pad Up)
pub fn toggle_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut settings: ResMut<DebugSettings>,
    mut text_query: Query<&mut Visibility, With<DebugText>>,
) {
    let pressed = keyboard.just_pressed(KeyCode::Tab)
        || gamepads
            .iter()
            .any(|gp| gp.just_pressed(GamepadButton::DPadUp));

    if pressed {
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

/// Update debug text display
pub fn update_debug_text(
    debug_settings: Res<DebugSettings>,
    shot_info: Res<LastShotInfo>,
    steal_contest: Res<StealContest>,
    mut text_query: Query<&mut Text2d, With<DebugText>>,
) {
    if !debug_settings.visible {
        return;
    }

    let Ok(mut text) = text_query.single_mut() else {
        return;
    };

    let steal_str = if steal_contest.active {
        format!(
            " | Steal: A:{} D:{} ({:.1}s)",
            steal_contest.attacker_presses, steal_contest.defender_presses, steal_contest.timer
        )
    } else {
        String::new()
    };

    // Show last shot info
    if shot_info.target.is_some() {
        let target_str = match shot_info.target {
            Some(Basket::Left) => "Left",
            Some(Basket::Right) => "Right",
            None => "?",
        };
        **text = format!(
            "Last Shot: {:.0}deg {:.0}u/s | Variance: base {:.0}% + air {:.0}% + move {:.0}% + dist {:.0}% = {:.0}% | Req speed: {:.0} | Target: {}{}",
            shot_info.angle_degrees,
            shot_info.speed,
            shot_info.base_variance * 100.0,
            shot_info.air_penalty * 100.0,
            shot_info.move_penalty * 100.0,
            shot_info.distance_variance * 100.0,
            shot_info.total_variance * 100.0,
            shot_info.required_speed,
            target_str,
            steal_str,
        );
    } else {
        **text = format!("No shots yet{}", steal_str);
    }
}

/// Cycle through viewport scale presets (V key only - controller uses unified cycle)
pub fn cycle_viewport(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut viewport_scale: ResMut<ViewportScale>,
    mut window_query: Query<&mut Window>,
    mut camera_query: Query<&mut Projection, With<Camera2d>>,
) {
    if keyboard.just_pressed(KeyCode::KeyV) {
        viewport_scale.cycle_next();
        apply_viewport(&viewport_scale, &mut window_query, &mut camera_query);
    }
}

/// Apply current viewport scale to window and camera
fn apply_viewport(
    viewport_scale: &ViewportScale,
    window_query: &mut Query<&mut Window>,
    camera_query: &mut Query<&mut Projection, With<Camera2d>>,
) {
    let (width, height, label) = viewport_scale.current();

    // Change window size - use scale_factor_override 1.0 for consistent HiDPI behavior
    if let Ok(mut window) = window_query.single_mut() {
        window.resolution = bevy::window::WindowResolution::new(width as u32, height as u32)
            .with_scale_factor_override(1.0);
    }

    // Adjust camera scale to keep full arena visible
    // Native is 1600x900, so scale = 1600/width to show same world area
    let camera_scale = crate::constants::ARENA_WIDTH / width;
    if let Ok(mut projection) = camera_query.single_mut() {
        if let Projection::Orthographic(ref mut ortho) = *projection {
            ortho.scale = camera_scale;
        }
    }

    info!("Viewport: {} (camera scale {:.2}x)", label, camera_scale);
}

/// Marker for cycle indicator text
#[derive(Component)]
pub struct CycleIndicator;

/// Unified cycle system - D-pad Down selects target, RT/LT cycle values
pub fn unified_cycle_system(
    gamepads: Query<&Gamepad>,
    mut cycle_selection: ResMut<CycleSelection>,
    mut current_level: ResMut<CurrentLevel>,
    mut current_palette: ResMut<crate::ball::CurrentPalette>,
    mut viewport_scale: ResMut<ViewportScale>,
    level_db: Res<LevelDatabase>,
    ball_textures: Res<BallTextures>,
    time: Res<Time>,
    mut window_query: Query<&mut Window>,
    mut camera_query: Query<&mut Projection, With<Camera2d>>,
    mut ball_query: Query<(&mut BallStyleType, &mut Sprite), With<Ball>>,
) {
    // Decay display timer
    if cycle_selection.display_timer > 0.0 {
        cycle_selection.display_timer -= time.delta_secs();
    }

    // D-pad Down selects next cycle target
    let select_pressed = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::DPadDown));

    if select_pressed {
        cycle_selection.select_next();
        info!("Cycle target: {}", cycle_selection.target.name());
    }

    // RT cycles forward, LT cycles backward
    let cycle_next = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::RightTrigger2));
    let cycle_prev = gamepads
        .iter()
        .any(|gp| gp.just_pressed(GamepadButton::LeftTrigger2));

    if !cycle_next && !cycle_prev {
        return;
    }

    cycle_selection.flash();

    match cycle_selection.target {
        CycleTarget::Level => {
            let num_levels = level_db.len() as u32;
            if cycle_next {
                current_level.0 = if current_level.0 >= num_levels {
                    1
                } else {
                    current_level.0 + 1
                };
            } else if cycle_prev {
                current_level.0 = if current_level.0 <= 1 {
                    num_levels
                } else {
                    current_level.0 - 1
                };
            }
            info!("Level: {}", current_level.0);
        }
        CycleTarget::Viewport => {
            if cycle_next {
                viewport_scale.cycle_next();
            } else if cycle_prev {
                viewport_scale.cycle_prev();
            }
            apply_viewport(&viewport_scale, &mut window_query, &mut camera_query);
        }
        CycleTarget::Palette => {
            // Just change the index - apply_palette_colors system handles the visuals
            let num_palettes = crate::palettes::NUM_PALETTES;
            if cycle_next {
                current_palette.0 = (current_palette.0 + 1) % num_palettes;
            } else if cycle_prev {
                current_palette.0 = (current_palette.0 + num_palettes - 1) % num_palettes;
            }
            info!("Palette: {}", current_palette.0);
        }
        CycleTarget::BallStyle => {
            // Cycle all balls to the next/prev style
            for (mut style, mut sprite) in &mut ball_query {
                let current_idx = BallStyleType::ALL
                    .iter()
                    .position(|s| *s == *style)
                    .unwrap_or(0);
                let num_styles = BallStyleType::ALL.len();

                let new_idx = if cycle_next {
                    (current_idx + 1) % num_styles
                } else {
                    (current_idx + num_styles - 1) % num_styles
                };

                *style = BallStyleType::ALL[new_idx];

                // Update sprite texture
                let textures = ball_textures.get(*style);
                sprite.image = textures.textures[current_palette.0].clone();
            }

            // Log the new style (use first ball's style)
            if let Some((style, _)) = ball_query.iter().next() {
                info!("Ball Style: {}", style.name());
            }
        }
    }
}

/// Update cycle indicator display
pub fn update_cycle_indicator(
    cycle_selection: Res<CycleSelection>,
    current_level: Res<CurrentLevel>,
    current_palette: Res<crate::ball::CurrentPalette>,
    viewport_scale: Res<ViewportScale>,
    level_db: Res<LevelDatabase>,
    ball_query: Query<&BallStyleType, With<Ball>>,
    mut query: Query<(&mut Text2d, &mut Visibility), With<CycleIndicator>>,
) {
    let Ok((mut text, mut visibility)) = query.single_mut() else {
        return;
    };

    if cycle_selection.display_timer <= 0.0 {
        *visibility = Visibility::Hidden;
        return;
    }

    *visibility = Visibility::Inherited;

    // Show current target and its value
    let value_str = match cycle_selection.target {
        CycleTarget::Level => format!("{}/{}", current_level.0, level_db.len()),
        CycleTarget::Viewport => {
            let (_, _, label) = viewport_scale.current();
            label.to_string()
        }
        CycleTarget::Palette => format!("{}/{}", current_palette.0 + 1, crate::palettes::NUM_PALETTES),
        CycleTarget::BallStyle => {
            // Get style from first ball
            ball_query
                .iter()
                .next()
                .map(|s| s.name().to_string())
                .unwrap_or_else(|| "None".to_string())
        }
    };

    **text = format!("[{}] {}", cycle_selection.target.name(), value_str);
}

/// Apply palette colors when CurrentPalette changes
#[allow(clippy::too_many_arguments)]
pub fn apply_palette_colors(
    current_palette: Res<crate::ball::CurrentPalette>,
    palette_db: Res<PaletteDatabase>,
    ball_textures: Res<BallTextures>,
    mut clear_color: ResMut<ClearColor>,
    mut player_query: Query<(&mut Sprite, &Team), (With<Player>, Without<Ball>, Without<Basket>)>,
    mut basket_query: Query<
        (&mut Sprite, &Basket, Option<&Children>),
        (Without<Player>, Without<Ball>),
    >,
    mut rim_query: Query<
        &mut Sprite,
        (With<BasketRim>, Without<Player>, Without<Ball>, Without<Basket>),
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
        ),
    >,
    mut ball_query: Query<(&BallStyleType, &mut Sprite), With<Ball>>,
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

    // Players
    for (mut sprite, team) in &mut player_query {
        sprite.color = match team {
            Team::Left => palette.left,
            Team::Right => palette.right,
        };
    }

    // Baskets and rims
    for (mut sprite, basket, children) in &mut basket_query {
        sprite.color = match basket {
            Basket::Left => palette.left,
            Basket::Right => palette.right,
        };

        // Update rim colors (children)
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(mut rim_sprite) = rim_query.get_mut(child) {
                    rim_sprite.color = match basket {
                        Basket::Left => palette.right_rim,
                        Basket::Right => palette.left_rim,
                    };
                }
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
        let textures = ball_textures.get(*style);
        sprite.image = textures.textures[current_palette.0].clone();
    }
}
