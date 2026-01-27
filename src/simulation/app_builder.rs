//! Headless App Builder
//!
//! Provides a reusable builder for creating headless Bevy apps for simulation.
//! Used by simulation runner, testing, and parallel execution.

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use std::time::Duration;

use crate::ball::CurrentPalette;
use crate::levels::LevelDatabase;
use crate::palettes::PaletteDatabase;
use crate::scoring::{CurrentLevel, Score};
use crate::shooting::LastShotInfo;
use crate::steal::{StealContest, StealTracker};
use crate::ui::PhysicsTweaks;

/// Builder for creating headless Bevy apps
pub struct HeadlessAppBuilder {
    level_id: String,
    fps: f32,
    minimal_threads: bool,
}

impl HeadlessAppBuilder {
    /// Create a new builder for the given level ID
    pub fn new(level_id: String) -> Self {
        Self {
            level_id,
            fps: 60.0,
            minimal_threads: false,
        }
    }

    /// Set the target FPS (default: 60)
    pub fn with_fps(mut self, fps: f32) -> Self {
        self.fps = fps;
        self
    }

    /// Enable minimal thread mode (task pools = 1)
    ///
    /// Use this when running many apps in parallel to avoid hitting OS thread limits.
    /// Each Bevy app normally spawns multiple threads; this reduces it to 1 per app.
    pub fn with_minimal_threads(mut self) -> Self {
        self.minimal_threads = true;
        self
    }

    /// Build the app with minimal plugins and common resources
    ///
    /// The returned app has:
    /// - MinimalPlugins with ScheduleRunnerPlugin
    /// - TransformPlugin for collision detection
    /// - Common game resources (Score, CurrentLevel, PhysicsTweaks, etc.)
    ///
    /// Callers should add:
    /// - LevelDatabase (required for level loading)
    /// - AiProfileDatabase (for AI simulation)
    /// - Startup/Update/FixedUpdate systems
    /// - Any additional resources
    pub fn build(self) -> App {
        let mut app = App::new();

        // Configure minimal plugins
        // Note: MinimalPlugins includes TaskPoolPlugin by default
        if self.minimal_threads {
            // Reduce Bevy's internal thread pools to minimum
            // This is critical for parallel execution to avoid hitting OS limits
            app.add_plugins(
                MinimalPlugins
                    .set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f32(
                        1.0 / self.fps,
                    )))
                    .set(TaskPoolPlugin {
                        task_pool_options: TaskPoolOptions::with_num_threads(1),
                    }),
            );
        } else {
            // Default threading
            app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
                Duration::from_secs_f32(1.0 / self.fps),
            )));
        }

        // Transform plugin for GlobalTransform propagation (needed for collision)
        app.add_plugins(bevy::transform::TransformPlugin);

        // Common game resources
        app.init_resource::<Score>();
        app.insert_resource(CurrentLevel(self.level_id.clone()));
        app.init_resource::<StealContest>();
        app.init_resource::<StealTracker>();
        app.init_resource::<PhysicsTweaks>();
        app.init_resource::<LastShotInfo>();
        app.insert_resource(CurrentPalette(0));
        app.init_resource::<PaletteDatabase>();

        app
    }

    /// Build with a LevelDatabase included
    pub fn build_with_levels(self, level_db: LevelDatabase) -> App {
        let mut app = self.build();
        app.insert_resource(level_db);
        app
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creates_app() {
        let app = HeadlessAppBuilder::new("test_level".to_string()).build();
        // Just verify it doesn't panic and has expected resources
        assert!(app.world().contains_resource::<Score>());
        assert!(app.world().contains_resource::<CurrentLevel>());
    }

    #[test]
    fn test_minimal_threads_creates_app() {
        // Verify minimal_threads mode creates an app without panicking
        let app = HeadlessAppBuilder::new("test_level".to_string())
            .with_minimal_threads()
            .build();
        assert!(app.world().contains_resource::<Score>());
    }
}
