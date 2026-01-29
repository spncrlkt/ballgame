//! Random point reachability validation
//!
//! Samples random test points from exploration data and verifies that
//! NavGraph can path to positions that players have actually reached.

use bevy::prelude::*;
use rusqlite::Connection;

use crate::ai::navigation::NavGraph;
use crate::ai::pathfinding::find_path;
use crate::ai::heatmaps::HeatmapBundle;

/// Result of a reachability test for a single level
#[derive(Debug, Clone)]
pub struct ReachabilityTestResult {
    pub level_name: String,
    pub level_id: String,
    pub db_samples: u32,
    pub samples_tested: u32,
    pub navgraph_success: u32,
    pub reachability_nonzero: u32,
    pub failures: Vec<ReachabilityFailure>,
}

/// A failed reachability test case
#[derive(Debug, Clone)]
pub struct ReachabilityFailure {
    pub test_point: Vec2,
    pub nearest_node_dist: f32,
    pub path_found: bool,
    pub heatmap_value: f32,
}

impl ReachabilityTestResult {
    /// Check if all tests passed (at least 95% success rate)
    pub fn passed(&self) -> bool {
        if self.samples_tested == 0 {
            return false;
        }
        let success_rate = self.navgraph_success as f32 / self.samples_tested as f32;
        success_rate >= 0.95
    }

    /// Check if test was skipped due to no data
    pub fn skipped(&self) -> bool {
        self.db_samples == 0
    }

    /// Format result for console output
    pub fn format(&self) -> String {
        if self.skipped() {
            format!(
                "REACHABILITY_TEST: {} (0 samples in DB)\n  SKIP: No exploration data for this level",
                self.level_name
            )
        } else if self.passed() {
            format!(
                "REACHABILITY_TEST: {} ({} samples in DB)\n  PASS: NavGraph {}/{} ({:.0}%), Heatmap {}/{} ({:.0}%)",
                self.level_name,
                self.db_samples,
                self.navgraph_success,
                self.samples_tested,
                self.navgraph_success as f32 / self.samples_tested as f32 * 100.0,
                self.reachability_nonzero,
                self.samples_tested,
                self.reachability_nonzero as f32 / self.samples_tested as f32 * 100.0
            )
        } else {
            let failures: Vec<String> = self
                .failures
                .iter()
                .take(5) // Only show first 5 failures
                .map(|f| {
                    format!(
                        "  FAILURE: ({:.0}, {:.0}) - nearest_node: {:.0}px, path: {}, heatmap: {:.2}",
                        f.test_point.x, f.test_point.y,
                        f.nearest_node_dist, f.path_found, f.heatmap_value
                    )
                })
                .collect();

            let more_failures = if self.failures.len() > 5 {
                format!("\n  ... and {} more failures", self.failures.len() - 5)
            } else {
                String::new()
            };

            format!(
                "REACHABILITY_TEST: {} ({} samples in DB)\n  FAIL: NavGraph {}/{} ({:.0}%)\n{}{}",
                self.level_name,
                self.db_samples,
                self.navgraph_success,
                self.samples_tested,
                self.navgraph_success as f32 / self.samples_tested as f32 * 100.0,
                failures.join("\n"),
                more_failures
            )
        }
    }
}

/// Load exploration positions from the database for a specific level
pub fn load_exploration_positions(
    db_path: &str,
    level_id: &str,
    human_only: bool,
) -> Result<Vec<Vec2>, String> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;

    let query = if human_only {
        "SELECT pos_x, pos_y FROM debug_events WHERE level_id = ?1 AND human_controlled = 1"
    } else {
        "SELECT pos_x, pos_y FROM debug_events WHERE level_id = ?1"
    };

    let mut stmt = conn.prepare(query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let positions: Vec<Vec2> = stmt
        .query_map([level_id], |row| {
            let x: f64 = row.get(0)?;
            let y: f64 = row.get(1)?;
            Ok(Vec2::new(x as f32, y as f32))
        })
        .map_err(|e| format!("Failed to query positions: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(positions)
}

/// Sample random positions from a list
pub fn sample_positions(positions: &[Vec2], count: usize, seed: u64) -> Vec<Vec2> {
    if positions.is_empty() {
        return Vec::new();
    }

    // Use a simple deterministic sampling based on seed
    let step = (positions.len() as f64 / count as f64).max(1.0) as usize;
    let offset = (seed as usize) % step.max(1);

    positions
        .iter()
        .skip(offset)
        .step_by(step.max(1))
        .take(count)
        .copied()
        .collect()
}

/// Run reachability test for a single level
pub fn run_reachability_test(
    nav_graph: &NavGraph,
    heatmaps: &HeatmapBundle,
    level_name: &str,
    level_id: &str,
    db_path: &str,
    sample_count: u32,
) -> ReachabilityTestResult {
    let mut result = ReachabilityTestResult {
        level_name: level_name.to_string(),
        level_id: level_id.to_string(),
        db_samples: 0,
        samples_tested: 0,
        navgraph_success: 0,
        reachability_nonzero: 0,
        failures: Vec::new(),
    };

    // Load exploration positions from database
    let positions = match load_exploration_positions(db_path, level_id, true) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Warning: Failed to load positions for {}: {}", level_name, e);
            return result;
        }
    };

    result.db_samples = positions.len() as u32;

    if positions.is_empty() {
        return result;
    }

    // Sample random positions
    let samples = sample_positions(&positions, sample_count as usize, 0);

    // Find floor node for pathfinding start
    let floor_node = match nav_graph.find_floor_node() {
        Some(idx) => idx,
        None => return result,
    };
    let floor_pos = nav_graph.nodes[floor_node].center;

    // Test each sample point
    for test_point in samples {
        result.samples_tested += 1;

        // Find nearest NavNode to the test point
        let (nearest_node_idx, nearest_dist) = nav_graph
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (i, n.center.distance(test_point)))
            .min_by(|(_, d1), (_, d2)| d1.partial_cmp(d2).unwrap())
            .unwrap_or((0, f32::MAX));

        // Try to find path from floor to nearest node
        let path_result = find_path(nav_graph, floor_pos, nav_graph.nodes[nearest_node_idx].center);
        let path_found = path_result.is_some();

        // Sample heatmap reachability at test point
        let heatmap_value = heatmaps.reachability_at(test_point);
        if heatmap_value > 0.0 {
            result.reachability_nonzero += 1;
        }

        if path_found {
            result.navgraph_success += 1;
        } else {
            // Record failure
            result.failures.push(ReachabilityFailure {
                test_point,
                nearest_node_dist: nearest_dist,
                path_found,
                heatmap_value,
            });
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_positions_deterministic() {
        let positions: Vec<Vec2> = (0..100)
            .map(|i| Vec2::new(i as f32, i as f32))
            .collect();

        let sample1 = sample_positions(&positions, 10, 42);
        let sample2 = sample_positions(&positions, 10, 42);

        assert_eq!(sample1.len(), sample2.len());
        for (a, b) in sample1.iter().zip(sample2.iter()) {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_sample_positions_empty() {
        let positions: Vec<Vec2> = Vec::new();
        let sample = sample_positions(&positions, 10, 42);
        assert!(sample.is_empty());
    }

    #[test]
    fn test_result_format_skip() {
        let result = ReachabilityTestResult {
            level_name: "TestLevel".to_string(),
            level_id: "test123".to_string(),
            db_samples: 0,
            samples_tested: 0,
            navgraph_success: 0,
            reachability_nonzero: 0,
            failures: Vec::new(),
        };

        let output = result.format();
        assert!(output.contains("SKIP"));
    }
}
