//! Steal system - instant steal attempts with cooldown

use bevy::prelude::*;

/// Steal feedback resource - tracks last attempt result for visual feedback
#[derive(Resource, Default)]
pub struct StealContest {
    /// Whether the last steal attempt failed (for fail flash)
    pub last_attempt_failed: bool,
    /// Timer for fail flash effect (counts down)
    pub fail_flash_timer: f32,
    /// Entity that last failed a steal (for positioning flash)
    pub fail_flash_entity: Option<Entity>,
}

/// Cooldown timer preventing steal spam (seconds remaining)
#[derive(Component, Default)]
pub struct StealCooldown(pub f32);

/// Tick down steal cooldowns and fail flash timer
pub fn steal_cooldown_update(
    time: Res<Time>,
    mut cooldowns: Query<&mut StealCooldown>,
    mut steal_contest: ResMut<StealContest>,
) {
    for mut cooldown in &mut cooldowns {
        if cooldown.0 > 0.0 {
            cooldown.0 -= time.delta_secs();
        }
    }

    // Tick down fail flash timer
    if steal_contest.fail_flash_timer > 0.0 {
        steal_contest.fail_flash_timer -= time.delta_secs();
        if steal_contest.fail_flash_timer <= 0.0 {
            steal_contest.last_attempt_failed = false;
            steal_contest.fail_flash_entity = None;
        }
    }
}
