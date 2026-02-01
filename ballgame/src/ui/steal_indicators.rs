//! Steal visual indicators - cooldown gauge and fail flash

use bevy::prelude::*;

use crate::constants::{PLAYER_SIZE, STEAL_COOLDOWN, STEAL_INDICATOR_SIZE};
use crate::player::Player;
use crate::shooting::ChargingShot;
use crate::steal::{StealContest, StealCooldown};

// =============================================================================
// COMPONENTS
// =============================================================================

/// Marker for cooldown indicator (shows remaining cooldown centered on player)
#[derive(Component)]
pub struct StealCooldownIndicator;

/// Marker for fail flash indicator (brief red flash when steal fails)
#[derive(Component)]
pub struct StealFailFlash;

/// Marker for out-of-range flash indicator (orange flash when steal attempt is too far)
#[derive(Component)]
pub struct StealOutOfRangeFlash;

/// Marker for vulnerable indicator (shows when player is charging and exposed)
#[derive(Component)]
pub struct VulnerableIndicator;

/// Marker for cooldown-blocked flash (shows when steal pressed but on cooldown)
#[derive(Component)]
pub struct CooldownBlockedFlash;

// =============================================================================
// COLORS
// =============================================================================

const COOLDOWN_COLOR: Color = Color::srgba(0.4, 0.4, 0.5, 0.8);
const FAIL_FLASH_COLOR: Color = Color::srgba(0.9, 0.2, 0.2, 0.7); // Red - steal failed
const OUT_OF_RANGE_COLOR: Color = Color::srgba(1.0, 0.5, 0.0, 0.6); // Orange - too far
const VULNERABLE_COLOR: Color = Color::srgba(1.0, 0.8, 0.0, 0.6); // Yellow warning
const COOLDOWN_BLOCKED_COLOR: Color = Color::srgba(0.3, 0.6, 0.9, 0.7); // Blue - on cooldown

// =============================================================================
// SPAWNING
// =============================================================================

/// Spawn steal indicator children for a player entity
pub fn spawn_steal_indicators(commands: &mut Commands, player_entity: Entity, _facing: f32) {
    // Cooldown indicator: centered above player
    let cooldown_indicator = commands
        .spawn((
            Sprite::from_color(COOLDOWN_COLOR, Vec2::new(STEAL_INDICATOR_SIZE, 0.0)),
            Transform::from_xyz(0.0, PLAYER_SIZE.y / 2.0 + 8.0, 0.4),
            Visibility::Hidden,
            StealCooldownIndicator,
        ))
        .id();
    commands.entity(player_entity).add_child(cooldown_indicator);

    // Fail flash indicator: brief red outline when steal fails
    let fail_flash = commands
        .spawn((
            Sprite::from_color(
                FAIL_FLASH_COLOR,
                Vec2::new(PLAYER_SIZE.x + 6.0, PLAYER_SIZE.y + 6.0),
            ),
            Transform::from_xyz(0.0, 0.0, -0.1),
            Visibility::Hidden,
            StealFailFlash,
        ))
        .id();
    commands.entity(player_entity).add_child(fail_flash);

    // Out-of-range flash: orange outline when steal attempted but too far
    let out_of_range = commands
        .spawn((
            Sprite::from_color(
                OUT_OF_RANGE_COLOR,
                Vec2::new(PLAYER_SIZE.x + 10.0, PLAYER_SIZE.y + 10.0),
            ),
            Transform::from_xyz(0.0, 0.0, -0.15),
            Visibility::Hidden,
            StealOutOfRangeFlash,
        ))
        .id();
    commands.entity(player_entity).add_child(out_of_range);

    // Vulnerable indicator: outline around player when charging and exposed
    let vulnerable = commands
        .spawn((
            Sprite::from_color(
                VULNERABLE_COLOR,
                Vec2::new(PLAYER_SIZE.x + 8.0, PLAYER_SIZE.y + 8.0),
            ),
            Transform::from_xyz(0.0, 0.0, -0.1),
            Visibility::Hidden,
            VulnerableIndicator,
        ))
        .id();
    commands.entity(player_entity).add_child(vulnerable);

    // Cooldown-blocked flash: blue pulse when steal pressed but on cooldown
    let cooldown_blocked = commands
        .spawn((
            Sprite::from_color(
                COOLDOWN_BLOCKED_COLOR,
                Vec2::new(PLAYER_SIZE.x + 12.0, PLAYER_SIZE.y + 12.0),
            ),
            Transform::from_xyz(0.0, 0.0, -0.2),
            Visibility::Hidden,
            CooldownBlockedFlash,
        ))
        .id();
    commands.entity(player_entity).add_child(cooldown_blocked);
}

// =============================================================================
// UPDATE SYSTEM
// =============================================================================

/// Update steal indicator visuals based on game state
#[allow(clippy::type_complexity)]
pub fn update_steal_indicators(
    steal_contest: Res<StealContest>,
    player_query: Query<(Entity, &StealCooldown, &ChargingShot, &Children), With<Player>>,
    mut cooldown_query: Query<
        (&mut Sprite, &mut Transform, &mut Visibility),
        (
            With<StealCooldownIndicator>,
            Without<StealFailFlash>,
            Without<StealOutOfRangeFlash>,
            Without<VulnerableIndicator>,
            Without<CooldownBlockedFlash>,
        ),
    >,
    mut fail_flash_query: Query<
        &mut Visibility,
        (
            With<StealFailFlash>,
            Without<StealCooldownIndicator>,
            Without<StealOutOfRangeFlash>,
            Without<VulnerableIndicator>,
            Without<CooldownBlockedFlash>,
        ),
    >,
    mut out_of_range_query: Query<
        &mut Visibility,
        (
            With<StealOutOfRangeFlash>,
            Without<StealCooldownIndicator>,
            Without<StealFailFlash>,
            Without<VulnerableIndicator>,
            Without<CooldownBlockedFlash>,
        ),
    >,
    mut vulnerable_query: Query<
        &mut Visibility,
        (
            With<VulnerableIndicator>,
            Without<StealCooldownIndicator>,
            Without<StealFailFlash>,
            Without<StealOutOfRangeFlash>,
            Without<CooldownBlockedFlash>,
        ),
    >,
    mut cooldown_blocked_query: Query<
        &mut Visibility,
        (
            With<CooldownBlockedFlash>,
            Without<StealCooldownIndicator>,
            Without<StealFailFlash>,
            Without<StealOutOfRangeFlash>,
            Without<VulnerableIndicator>,
        ),
    >,
) {
    for (player_entity, cooldown, charging, children) in &player_query {
        for child in children.iter() {
            // Update cooldown indicator
            if let Ok((mut sprite, mut transform, mut visibility)) = cooldown_query.get_mut(child) {
                if cooldown.0 > 0.0 {
                    *visibility = Visibility::Inherited;
                    let fill_pct = cooldown.0 / STEAL_COOLDOWN;
                    let height = STEAL_INDICATOR_SIZE * fill_pct;
                    sprite.custom_size = Some(Vec2::new(STEAL_INDICATOR_SIZE, height));
                    // Centered above player, anchor at bottom
                    transform.translation.y = PLAYER_SIZE.y / 2.0 + 8.0 + height / 2.0;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }

            // Update fail flash indicator
            if let Ok(mut visibility) = fail_flash_query.get_mut(child) {
                // Show when this player just failed a steal
                if steal_contest.last_attempt_failed
                    && steal_contest.fail_flash_entity == Some(player_entity)
                {
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }

            // Update out-of-range flash indicator
            if let Ok(mut visibility) = out_of_range_query.get_mut(child) {
                // Show when this player attempted steal but was too far
                if steal_contest.out_of_range_timer > 0.0
                    && steal_contest.out_of_range_entity == Some(player_entity)
                {
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }

            // Update vulnerable indicator
            if let Ok(mut visibility) = vulnerable_query.get_mut(child) {
                // Show when charging (vulnerable to steal)
                if charging.charge_time > 0.0 {
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }

            // Update cooldown-blocked flash indicator
            if let Ok(mut visibility) = cooldown_blocked_query.get_mut(child) {
                // Show when this player pressed steal but was on cooldown
                if steal_contest.cooldown_blocked_timer > 0.0
                    && steal_contest.cooldown_blocked_entity == Some(player_entity)
                {
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }
}
