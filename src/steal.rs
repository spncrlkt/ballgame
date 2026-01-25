//! Steal system - instant steal attempts with cooldown

use bevy::prelude::*;

use crate::player::Team;

/// Maximum allowed steal differential between teams.
/// If one team has MAX_STEAL_DIFFERENTIAL more steals than the other,
/// that team cannot steal until the differential is reduced.
pub const MAX_STEAL_DIFFERENTIAL: i32 = 2;

/// Steal feedback resource - tracks last attempt result for visual feedback
#[derive(Resource, Default)]
pub struct StealContest {
    /// Whether the last steal attempt failed (for fail flash)
    pub last_attempt_failed: bool,
    /// Timer for fail flash effect (counts down)
    pub fail_flash_timer: f32,
    /// Entity that last failed a steal (for positioning flash)
    pub fail_flash_entity: Option<Entity>,
    /// Timer for "out of range" feedback (counts down)
    pub out_of_range_timer: f32,
    /// Entity that attempted steal while out of range
    pub out_of_range_entity: Option<Entity>,
}

/// Resource tracking steal attempts and successes per team for differential enforcement
#[derive(Resource, Default)]
pub struct StealTracker {
    /// Steal attempts by Left team
    pub left_attempts: i32,
    /// Steal attempts by Right team
    pub right_attempts: i32,
    /// Successful steals by Left team
    pub left_steals: i32,
    /// Successful steals by Right team
    pub right_steals: i32,
}

impl StealTracker {
    /// Get the current attempt differential (left - right)
    pub fn attempt_differential(&self) -> i32 {
        self.left_attempts - self.right_attempts
    }

    /// Get the current success differential (left - right)
    pub fn success_differential(&self) -> i32 {
        self.left_steals - self.right_steals
    }

    /// Check if a team is allowed to attempt a steal based on SUCCESS differential
    pub fn can_attempt_steal(&self, team: Team) -> bool {
        let diff = self.success_differential();
        match team {
            Team::Left => diff < MAX_STEAL_DIFFERENTIAL,
            Team::Right => diff > -MAX_STEAL_DIFFERENTIAL,
        }
    }

    /// Record a steal attempt by a team
    pub fn record_attempt(&mut self, team: Team) {
        match team {
            Team::Left => self.left_attempts += 1,
            Team::Right => self.right_attempts += 1,
        }
    }

    /// Record a successful steal by a team
    pub fn record_success(&mut self, team: Team) {
        match team {
            Team::Left => self.left_steals += 1,
            Team::Right => self.right_steals += 1,
        }
    }

    /// Reset for a new game
    pub fn reset(&mut self) {
        self.left_attempts = 0;
        self.right_attempts = 0;
        self.left_steals = 0;
        self.right_steals = 0;
    }

    /// Log current state
    pub fn log_state(&self, context: &str) {
        info!(
            "STEAL STATE [{}]: Attempts L{}/R{} (diff={}) | Success L{}/R{} (diff={})",
            context,
            self.left_attempts,
            self.right_attempts,
            self.attempt_differential(),
            self.left_steals,
            self.right_steals,
            self.success_differential()
        );
    }
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
    // Use minimum dt for headless mode compatibility
    let dt = time.delta_secs().max(1.0 / 60.0);

    for mut cooldown in &mut cooldowns {
        if cooldown.0 > 0.0 {
            cooldown.0 -= dt;
        }
    }

    // Tick down fail flash timer
    if steal_contest.fail_flash_timer > 0.0 {
        steal_contest.fail_flash_timer -= dt;
        if steal_contest.fail_flash_timer <= 0.0 {
            steal_contest.last_attempt_failed = false;
            steal_contest.fail_flash_entity = None;
        }
    }

    // Tick down out-of-range timer
    if steal_contest.out_of_range_timer > 0.0 {
        steal_contest.out_of_range_timer -= dt;
        if steal_contest.out_of_range_timer <= 0.0 {
            steal_contest.out_of_range_entity = None;
        }
    }
}
