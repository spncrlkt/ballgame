//! Headless App Builder
//!
//! Provides a reusable builder for creating headless Bevy apps for simulation.
//! Used by simulation runner, testing, and parallel execution.

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use std::time::Duration;

use crate::ai::{AiCapabilities, AiProfileDatabase, HeatmapBundle, NavGraph};
use crate::ball::CurrentPalette;
use crate::events::EventBus;
use crate::levels::LevelDatabase;
use crate::palettes::PaletteDatabase;
use crate::scoring::{CurrentLevel, Score};
use crate::shooting::LastShotInfo;
use crate::steal::{StealContest, StealTracker};
use crate::tuning::{self, PhysicsTweaks};

/// Builder for creating headless Bevy apps
pub struct HeadlessAppBuilder {
    level_id: Option<String>,
    level_db: Option<LevelDatabase>,
    profile_db: Option<AiProfileDatabase>,
    fps: f32,
    minimal_threads: bool,
    include_ai: bool,
}

impl HeadlessAppBuilder {
    /// Create a new builder with default settings
    pub fn new() -> Self {
        Self {
            level_id: None,
            level_db: None,
            profile_db: None,
            fps: 60.0,
            minimal_threads: false,
            include_ai: false,
        }
    }

    /// Create a new builder for the given level ID
    pub fn for_level(level_id: String) -> Self {
        Self {
            level_id: Some(level_id),
            level_db: None,
            profile_db: None,
            fps: 60.0,
            minimal_threads: false,
            include_ai: false,
        }
    }

    /// Set the level ID
    pub fn with_level(mut self, level_id: &str) -> Self {
        self.level_id = Some(level_id.to_string());
        self
    }

    /// Set the level database
    pub fn with_level_db(mut self, level_db: LevelDatabase) -> Self {
        self.level_db = Some(level_db);
        self
    }

    /// Set the AI profile database
    pub fn with_profile_db(mut self, profile_db: AiProfileDatabase) -> Self {
        self.profile_db = Some(profile_db);
        self
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

    /// Include AI-related resources (NavGraph, HeatmapBundle, etc.)
    pub fn with_ai(mut self) -> Self {
        self.include_ai = true;
        self
    }

    /// Build the app with minimal plugins and common resources
    ///
    /// The returned app has:
    /// - MinimalPlugins with ScheduleRunnerPlugin
    /// - TransformPlugin for collision detection
    /// - Common game resources (Score, CurrentLevel, PhysicsTweaks, etc.)
    /// - LevelDatabase and AiProfileDatabase if provided
    /// - AI resources (NavGraph, HeatmapBundle) if with_ai() was called
    ///
    /// Callers should add:
    /// - Startup/Update/FixedUpdate systems
    /// - Any additional resources
    pub fn build(self) -> App {
        use crate::ai::{
            ai_decision_update, ai_navigation_update, load_heatmaps_on_level_change,
            mark_nav_dirty_on_level_change, rebuild_nav_graph,
        };

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
        let level_id = self.level_id.unwrap_or_default();
        app.insert_resource(CurrentLevel(level_id));
        app.init_resource::<StealContest>();
        app.init_resource::<StealTracker>();
        app.init_resource::<PhysicsTweaks>();
        let _ = tuning::apply_global_tuning(&mut app.world_mut().resource_mut::<PhysicsTweaks>());
        app.init_resource::<LastShotInfo>();
        app.insert_resource(CurrentPalette(0));
        app.init_resource::<PaletteDatabase>();

        // Optional databases
        if let Some(level_db) = self.level_db {
            app.insert_resource(level_db);
        }
        if let Some(profile_db) = self.profile_db {
            app.insert_resource(profile_db);
        }

        // AI resources
        if self.include_ai {
            app.init_resource::<NavGraph>();
            app.init_resource::<AiCapabilities>();
            app.init_resource::<HeatmapBundle>();
            app.insert_resource(EventBus::new());

            // Add AI systems
            app.add_systems(
                Update,
                (
                    mark_nav_dirty_on_level_change,
                    load_heatmaps_on_level_change,
                    rebuild_nav_graph,
                    ai_navigation_update,
                    ai_decision_update,
                )
                    .chain(),
            );

            // Mark nav graph dirty after first frame
            app.add_systems(PostStartup, |mut nav_graph: ResMut<NavGraph>| {
                nav_graph.dirty = true;
            });
        }

        app
    }

    /// Build with a LevelDatabase included (legacy method)
    pub fn build_with_levels(self, level_db: LevelDatabase) -> App {
        let builder = Self {
            level_db: Some(level_db),
            ..self
        };
        builder.build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creates_app() {
        let app = HeadlessAppBuilder::for_level("test_level".to_string()).build();
        // Just verify it doesn't panic and has expected resources
        assert!(app.world().contains_resource::<Score>());
        assert!(app.world().contains_resource::<CurrentLevel>());
    }

    #[test]
    fn test_minimal_threads_creates_app() {
        // Verify minimal_threads mode creates an app without panicking
        let app = HeadlessAppBuilder::for_level("test_level".to_string())
            .with_minimal_threads()
            .build();
        assert!(app.world().contains_resource::<Score>());
    }

    #[test]
    fn test_builder_with_new_api() {
        let app = HeadlessAppBuilder::new()
            .with_level("test_level")
            .build();
        assert!(app.world().contains_resource::<Score>());
        assert!(app.world().contains_resource::<CurrentLevel>());
    }
}
