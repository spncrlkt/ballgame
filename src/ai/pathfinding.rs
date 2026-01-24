//! A* pathfinding for AI navigation
//!
//! Finds optimal paths through the navigation graph using A* search.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use bevy::prelude::*;

use crate::ai::navigation::{EdgeType, NavAction, NavGraph};
use crate::constants::*;

/// Node in the A* search priority queue
#[derive(Clone)]
#[allow(dead_code)] // Fields used for path reconstruction stored in came_from
struct SearchNode {
    /// Index into nav graph nodes
    node_index: usize,
    /// Cost from start to this node (g-score)
    g_cost: f32,
    /// Estimated total cost (f-score = g + h)
    f_cost: f32,
    /// Parent node for path reconstruction
    parent: Option<usize>,
    /// Edge type used to reach this node
    edge_type: Option<EdgeType>,
    /// Jump parameters for this edge
    jump_from_x: f32,
    land_on_x: f32,
    jump_hold_duration: f32,
}

impl PartialEq for SearchNode {
    fn eq(&self, other: &Self) -> bool {
        self.node_index == other.node_index
    }
}

impl Eq for SearchNode {}

impl PartialOrd for SearchNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SearchNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order for min-heap (lower f_cost = higher priority)
        other
            .f_cost
            .partial_cmp(&self.f_cost)
            .unwrap_or(Ordering::Equal)
    }
}

/// Result of pathfinding
pub struct PathResult {
    /// Sequence of actions to reach goal
    pub actions: Vec<NavAction>,
    /// Total path cost
    pub total_cost: f32,
    /// Final node reached
    pub goal_node: usize,
}

/// Find a path from current position to target position using A*
pub fn find_path(
    nav_graph: &NavGraph,
    start_pos: Vec2,
    target_pos: Vec2,
) -> Option<PathResult> {
    if nav_graph.nodes.is_empty() {
        return None;
    }

    // Find start and goal nodes
    let start_node = nav_graph.find_node_at(start_pos, NAV_POSITION_TOLERANCE)?;
    let goal_node = nav_graph.find_closest_node(target_pos)?;

    // If already at goal, no path needed
    if start_node == goal_node {
        return Some(PathResult {
            actions: Vec::new(),
            total_cost: 0.0,
            goal_node,
        });
    }

    // A* search
    let mut open_set = BinaryHeap::new();
    let mut came_from: Vec<Option<(usize, EdgeType, f32, f32, f32)>> = vec![None; nav_graph.nodes.len()];
    let mut g_scores = vec![f32::INFINITY; nav_graph.nodes.len()];

    let goal_pos = nav_graph.nodes[goal_node].center;
    let h_start = heuristic(&nav_graph.nodes[start_node].center, &goal_pos);

    g_scores[start_node] = 0.0;
    open_set.push(SearchNode {
        node_index: start_node,
        g_cost: 0.0,
        f_cost: h_start,
        parent: None,
        edge_type: None,
        jump_from_x: 0.0,
        land_on_x: 0.0,
        jump_hold_duration: 0.0,
    });

    while let Some(current) = open_set.pop() {
        if current.node_index == goal_node {
            // Reconstruct path
            return Some(reconstruct_path(
                nav_graph,
                &came_from,
                start_node,
                goal_node,
                start_pos,
                target_pos,
                current.g_cost,
            ));
        }

        // Skip if we've found a better path to this node
        if current.g_cost > g_scores[current.node_index] + 0.01 {
            continue;
        }

        // Explore neighbors
        for edge in &nav_graph.edges[current.node_index] {
            let tentative_g = current.g_cost + edge.cost;

            if tentative_g < g_scores[edge.to_node] {
                g_scores[edge.to_node] = tentative_g;
                let h = heuristic(&nav_graph.nodes[edge.to_node].center, &goal_pos);

                came_from[edge.to_node] = Some((
                    current.node_index,
                    edge.edge_type,
                    edge.jump_from_x,
                    edge.land_on_x,
                    edge.jump_hold_duration,
                ));

                open_set.push(SearchNode {
                    node_index: edge.to_node,
                    g_cost: tentative_g,
                    f_cost: tentative_g + h,
                    parent: Some(current.node_index),
                    edge_type: Some(edge.edge_type),
                    jump_from_x: edge.jump_from_x,
                    land_on_x: edge.land_on_x,
                    jump_hold_duration: edge.jump_hold_duration,
                });
            }
        }
    }

    // No path found
    None
}

/// Find path to a position from which we can shoot at the target.
/// Takes `min_shot_quality` to filter out positions where shot quality is too low.
pub fn find_path_to_shoot(
    nav_graph: &NavGraph,
    start_pos: Vec2,
    target_basket_pos: Vec2,
    shoot_range: f32,
    min_shot_quality: f32,
) -> Option<PathResult> {
    if nav_graph.nodes.is_empty() {
        return None;
    }

    // Find a node within shooting range that meets quality threshold
    let goal_node = nav_graph.find_shooting_node(target_basket_pos, shoot_range, min_shot_quality)?;
    let goal_pos = nav_graph.nodes[goal_node].center;

    find_path(nav_graph, start_pos, goal_pos)
}

/// Heuristic function for A* (Euclidean distance)
fn heuristic(from: &Vec2, to: &Vec2) -> f32 {
    // Weight vertical distance more since jumps are "expensive"
    let dx = (from.x - to.x).abs();
    let dy = (from.y - to.y).abs();

    // Penalize height differences more than horizontal distance
    dx + dy * 1.5
}

/// Reconstruct path from came_from map
fn reconstruct_path(
    nav_graph: &NavGraph,
    came_from: &[Option<(usize, EdgeType, f32, f32, f32)>],
    start_node: usize,
    goal_node: usize,
    start_pos: Vec2,
    target_pos: Vec2,
    total_cost: f32,
) -> PathResult {
    let mut actions = Vec::new();
    let mut current = goal_node;

    // Build path in reverse
    let mut path_nodes = vec![current];
    while let Some((parent, _, _, _, _)) = came_from[current] {
        path_nodes.push(parent);
        current = parent;
        if current == start_node {
            break;
        }
    }
    path_nodes.reverse();

    // Convert path nodes to actions
    let mut current_x = start_pos.x;

    for i in 0..path_nodes.len() - 1 {
        let _from_node = path_nodes[i];
        let to_node = path_nodes[i + 1];

        if let Some((_, edge_type, jump_from_x, land_on_x, hold_duration)) = came_from[to_node] {
            // Walk to jump/drop point if needed
            if (current_x - jump_from_x).abs() > NAV_POSITION_TOLERANCE {
                actions.push(NavAction::WalkTo { x: jump_from_x });
            }

            // Perform the transition
            match edge_type {
                EdgeType::Walk => {
                    // Just walk to the landing point
                    actions.push(NavAction::WalkTo { x: land_on_x });
                }
                EdgeType::Jump => {
                    // Jump with appropriate hold duration
                    actions.push(NavAction::JumpAt {
                        x: jump_from_x,
                        hold_duration,
                    });
                    // Walk to landing position while in air (controlled by AI)
                }
                EdgeType::Drop => {
                    // Walk off edge
                    let direction = if land_on_x > jump_from_x { 1.0 } else { -1.0 };
                    actions.push(NavAction::WalkOffEdge { direction });
                }
            }

            current_x = land_on_x;
        }
    }

    // Add final walk to exact target position if not at goal node center
    let goal = &nav_graph.nodes[goal_node];
    // For narrow platforms, use center; otherwise clamp with tolerance
    let target_x = if goal.right_x - goal.left_x <= NAV_POSITION_TOLERANCE * 2.0 {
        goal.center.x // Platform too narrow for tolerance, use center
    } else {
        target_pos.x.clamp(
            goal.left_x + NAV_POSITION_TOLERANCE,
            goal.right_x - NAV_POSITION_TOLERANCE,
        )
    };

    if (current_x - target_x).abs() > NAV_POSITION_TOLERANCE {
        actions.push(NavAction::WalkTo { x: target_x });
    }

    PathResult {
        actions,
        total_cost,
        goal_node,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::navigation::{NavNode, PlatformRole};

    fn create_test_graph() -> NavGraph {
        // Simple test: floor and one platform above
        let nodes = vec![
            NavNode {
                id: 0,
                center: Vec2::new(0.0, -430.0),
                left_x: -780.0,
                right_x: 780.0,
                top_y: -430.0,
                platform_entity: None,
                is_floor: true,
                shot_quality_left: 0.5,
                shot_quality_right: 0.5,
                platform_role: PlatformRole::Floor,
            },
            NavNode {
                id: 1,
                center: Vec2::new(0.0, -230.0),
                left_x: -100.0,
                right_x: 100.0,
                top_y: -230.0,
                platform_entity: None,
                is_floor: false,
                shot_quality_left: 0.6,
                shot_quality_right: 0.6,
                platform_role: PlatformRole::ShotPosition,
            },
        ];

        let mut edges = vec![Vec::new(); 2];
        // Floor to platform (jump)
        edges[0].push(crate::ai::navigation::NavEdge {
            to_node: 1,
            edge_type: EdgeType::Jump,
            cost: 200.0,
            jump_from_x: 0.0,
            land_on_x: 0.0,
            jump_hold_duration: 0.8,
        });
        // Platform to floor (drop)
        edges[1].push(crate::ai::navigation::NavEdge {
            to_node: 0,
            edge_type: EdgeType::Drop,
            cost: 60.0,
            jump_from_x: 0.0,
            land_on_x: 0.0,
            jump_hold_duration: 0.0,
        });

        NavGraph {
            nodes,
            edges,
            dirty: false,
            built_for_level: 0,
        }
    }

    #[test]
    fn test_find_path_same_node() {
        let graph = create_test_graph();
        let result = find_path(&graph, Vec2::new(0.0, -430.0), Vec2::new(50.0, -430.0));
        assert!(result.is_some());
        // Should have minimal or no actions (same platform)
    }
}
