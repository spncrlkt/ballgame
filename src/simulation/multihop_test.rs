//! Multi-hop platform reachability test
//!
//! Tests that NavGraph correctly chains edges for platforms only reachable
//! via intermediate hops from the floor.

use bevy::prelude::*;

use crate::ai::navigation::{NavGraph, PlatformRole};
use crate::ai::pathfinding::find_path;
use crate::levels::LevelDatabase;

/// Result of a multi-hop test for a single level
#[derive(Debug, Clone)]
pub struct MultihopTestResult {
    pub level_name: String,
    pub level_id: String,
    pub platforms_tested: u32,
    pub paths_found: u32,
    pub failures: Vec<MultihopFailure>,
}

/// A failed multi-hop test case
#[derive(Debug, Clone)]
pub struct MultihopFailure {
    pub platform_id: usize,
    pub platform_center: Vec2,
    pub reachability_value: f32,
}

impl MultihopTestResult {
    /// Check if all tests passed
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }

    /// Format result for console output
    pub fn format(&self) -> String {
        if self.passed() {
            format!(
                "MULTIHOP_TEST: {}\n  PASS: {}/{} platforms reachable",
                self.level_name, self.paths_found, self.platforms_tested
            )
        } else {
            let failures: Vec<String> = self
                .failures
                .iter()
                .map(|f| {
                    format!(
                        "  FAILURE: platform_{} (center: {:.0}, {:.0}) - no path, reachability: {:.2}",
                        f.platform_id, f.platform_center.x, f.platform_center.y, f.reachability_value
                    )
                })
                .collect();

            format!(
                "MULTIHOP_TEST: {}\n  FAIL: {}/{} platforms reachable\n{}",
                self.level_name,
                self.paths_found,
                self.platforms_tested,
                failures.join("\n")
            )
        }
    }
}

/// Run multi-hop test for a single level
///
/// Tests that all non-floor nodes in the NavGraph are reachable from the floor
/// via A* pathfinding. This validates that the NavGraph edges correctly chain
/// together to allow reaching elevated platforms.
pub fn run_multihop_test(nav_graph: &NavGraph, level_name: &str, level_id: &str) -> MultihopTestResult {
    let mut result = MultihopTestResult {
        level_name: level_name.to_string(),
        level_id: level_id.to_string(),
        platforms_tested: 0,
        paths_found: 0,
        failures: Vec::new(),
    };

    // Find the floor node (starting point for all paths)
    let floor_node = match nav_graph.find_floor_node() {
        Some(idx) => idx,
        None => {
            // No floor node - can't test
            return result;
        }
    };

    let floor_pos = nav_graph.nodes[floor_node].center;

    // Test each non-floor node
    for (idx, node) in nav_graph.nodes.iter().enumerate() {
        // Skip floor and ramp nodes (ramps are intermediate steps, not destinations)
        if node.is_floor || node.platform_role == PlatformRole::Ramp {
            continue;
        }

        result.platforms_tested += 1;

        // Try to find path from floor to this node
        let path_result = find_path(nav_graph, floor_pos, node.center);

        if path_result.is_some() {
            result.paths_found += 1;
        } else {
            // No path found - record failure
            result.failures.push(MultihopFailure {
                platform_id: idx,
                platform_center: node.center,
                reachability_value: node.reachability,
            });
        }
    }

    result
}

/// Run multi-hop test for all levels
pub fn run_multihop_test_all_levels(
    level_db: &LevelDatabase,
) -> Vec<MultihopTestResult> {
    // Note: This requires building NavGraphs for each level, which needs a Bevy World
    // For now, this function is a placeholder - the actual test is run from the simulation runner
    // which has access to the World and can properly build NavGraphs

    let _count = level_db.len();
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multihop_result_format() {
        let result = MultihopTestResult {
            level_name: "TestLevel".to_string(),
            level_id: "test123".to_string(),
            platforms_tested: 5,
            paths_found: 5,
            failures: Vec::new(),
        };

        let output = result.format();
        assert!(output.contains("PASS"));
        assert!(output.contains("5/5"));
    }

    #[test]
    fn test_multihop_failure_format() {
        let result = MultihopTestResult {
            level_name: "TestLevel".to_string(),
            level_id: "test123".to_string(),
            platforms_tested: 5,
            paths_found: 3,
            failures: vec![
                MultihopFailure {
                    platform_id: 2,
                    platform_center: Vec2::new(100.0, 200.0),
                    reachability_value: 0.05,
                },
            ],
        };

        let output = result.format();
        assert!(output.contains("FAIL"));
        assert!(output.contains("3/5"));
        assert!(output.contains("platform_2"));
    }
}
