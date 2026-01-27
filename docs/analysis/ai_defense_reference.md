# AI Defense Reference (Current Flow + Test Inputs/Outputs)

## Scope
- Code paths: `src/ai/decision.rs`, `src/ai/navigation.rs`, `src/ai/mod.rs`, `src/ai/world_model.rs`, `src/ai/capabilities.rs`.
- Contexts: main game, training, simulation runner, run-ghost, scenario tests.

## Data Flow (Current)

**Update**
1) `input::capture_input` -> `PlayerInput`
2) `ai::copy_human_input` -> `InputState` (human)
3) `ai::mark_nav_dirty_on_level_change`
4) `ai::rebuild_nav_graph`
5) `ai::ai_navigation_update` -> `AiNavState`
6) `ai::ai_decision_update` -> `InputState` (AI)

**FixedUpdate**
1) `player::apply_input`
2) physics/collisions/ball/scoring

**Key state/resources**
- `InputState` (physics input buffer)
- `AiState` (goal, timers, nav target)
- `AiCapabilities` (jump reach/clearance)
- `NavGraph` (nodes/edges, shot quality, platform roles)

## Defensive Goal Selection (Current)

Per AI in `ai_decision_update`:
1) Determine ball ownership.
2) If opponent has ball:
   - `AttemptSteal` if distance < `profile.steal_range`
   - `PressureDefense` if distance < `profile.pressure_distance * (1.5 + (1 - aggression))`
   - `InterceptDefense` otherwise
3) Hysteresis 0.4s.
4) `AttemptSteal` commitment timer 0.5s.

**Behavior summary**
- `AttemptSteal`: chase, jump if elevated, attempt pickup when reaction/cooldown allow.
- `InterceptDefense`: intercept line; route to ramps if opponent elevated.
- `PressureDefense`: close tracking with steal attempts; less direct intercept.

## Navigation Targeting (Current)

- `InterceptDefense`/`PressureDefense`: target line from opponent to defended basket.
- Opponent detection prefers `HumanControlled`, so AI-vs-AI can fall back to generic defense.

## Defensive Vision Direction (Planned)

**Zone model** (derived from level geometry)
- Zones: rim, paint, midcourt, wing L/R, corner ramps, perimeter.
- Select zone first, then target point; zones shift with basket height/platforms.
- Compute bounds from level config; world-space coordinates.

**Zone geometry formulas (proposed)**

- **Basket center**  
  - `wall_inner = ARENA_WIDTH / 2 - WALL_THICKNESS`  
  - `basket_x = if side == Left { -wall_inner + basket_push_in } else { wall_inner - basket_push_in }`  
  - `basket_y = ARENA_FLOOR_Y + level.basket_height`

- **Rim Zone**  
  - `rim_x_min = basket_x - (BASKET_SIZE.x / 2 + rim_margin)`  
  - `rim_x_max = basket_x + (BASKET_SIZE.x / 2 + rim_margin)`  
  - `rim_y_min = basket_y - (BASKET_SIZE.y / 2 + rim_margin)`  
  - `rim_y_max = basket_y + (BASKET_SIZE.y / 2 + rim_margin)`

- **Paint Zone**  
  - `paint_x_min = basket_x - paint_width / 2`  
  - `paint_x_max = basket_x + paint_width / 2`  
  - `paint_y_min = ARENA_FLOOR_Y`  
  - `paint_y_max = basket_y - BASKET_SIZE.y / 2`

- **Wing Zones**  
  - `wing_y_min = basket_y - wing_height / 2`  
  - `wing_y_max = basket_y + wing_height / 2`  
  - Left: `x in [basket_x + wing_inner_offset, basket_x + wing_outer_offset]`  
  - Right: `x in [basket_x - wing_outer_offset, basket_x - wing_inner_offset]`

- **Midcourt Zone**  
  - `mid_x_min = -midcourt_width / 2`  
  - `mid_x_max = midcourt_width / 2`  
  - `mid_y_min = ARENA_FLOOR_Y`  
  - `mid_y_max = ARENA_FLOOR_Y + ARENA_HEIGHT`

- **Corner Ramp Zones** (if `step_count > 0`)  
  - Left: `x in [-wall_inner + step_push_in - corner_width, -wall_inner + step_push_in]`  
  - Right: `x in [wall_inner - step_push_in, wall_inner - step_push_in + corner_width]`  
  - `y in [ARENA_FLOOR_Y, ARENA_FLOOR_Y + corner_height]`

- **Platform Zones**  
  - Use `NavNode` bounds to classify elevated areas (upper wing, catwalk).

Parameter defaults: `rim_margin`, `paint_width`, `wing_height`, `wing_inner_offset`, `wing_outer_offset`, `midcourt_width`.

## Simulation Inputs for Defensive Testing

**Simulation runner**
- `--level <N>`, `--left <PROFILE> --right <PROFILE>`
- `--seed <N>`, `--duration <SECS>`, `--stalemate-timeout <SECS>`
- `--db <PATH>`

**Ghost trials**
- `--ghost <path>` in simulate or `run-ghost`.

**Training**
- Use for full logging with controlled profiles/levels.

## Logging and Measurement (Current)

- `events/emitter.rs`: `GameEvent::AiGoal` + 20 Hz tick snapshots.
- `simulation/metrics.rs`: per-goal time (`goal_time`).
- `training/analysis.rs`: goal transitions/oscillation.
- Signals: goal transitions/time, snapshots (player + ball), `StealContest` events.

## Heatmap Notes (Shot/Zone Context)

- `src/bin/heatmap.rs` now generates per-level heatmaps using `LevelDatabase` (basket height/push-in + platforms).
- Output lives in `showcase/heatmaps/heatmap_<type>_<level>_<uuid>.png` (score adds `_left`/`_right`) with text sidecars for numeric grids.
- Types: speed, score, reachability (full physics), landing_safety, path_cost, line_of_sight, elevation, escape_routes.
- Full bundles: `--full` writes `showcase/heatmaps/heatmap_full_<level>_<uuid>.png`.
- Combined sheets: `showcase/heatmap_<type>_all.png`.
- Change detection: `--check` compares `config/level_hashes.json` to run only new/changed levels.

## Nav + Heatmap Integration (Current Direction)

- Keep `NavGraph` as the connectivity backbone.
- Use heatmaps to score node desirability (shot quality, safety, LOS, elevation, escape).
- AI should choose candidate nodes via nav graph, then rank with heatmap-derived scoring.

## Proposed Heatmap Data Schema (Level-Indexed)

JSON sidecar per level:
```json
{
  "level_index": 4,
  "level_name": "Open Floor",
  "grid": {
    "cell_size": 20,
    "width": 80,
    "height": 45,
    "origin": { "x": -800.0, "y": 450.0 }
  },
  "baskets": {
    "left": { "x": -624.0, "y": -450.0 },
    "right": { "x": 624.0, "y": -450.0 }
  },
  "cells": [
    {
      "x": 0,
      "y": 0,
      "world": { "x": -790.0, "y": 440.0 },
      "shot_quality_left": 0.32,
      "shot_quality_right": 0.41,
      "zone_id": "midcourt",
      "threat": 0.18
    }
  ]
}
```

**Notes**
- `origin` is the grid top-left world coordinate.
- `cells` can be flat or 2D; flat is smaller.
- `threat` is optional.

## Mapping NavGraph Nodes to Heatmap

1) Map `NavNode.center` to grid cell:
   - `cell_x = floor((center.x - origin.x) / cell_size)`
   - `cell_y = floor((origin.y - center.y) / cell_size)`
2) Clamp to bounds; read cell data.
3) Store `shot_quality_left/right` + `zone_id`.
4) Prefer non-dead zones with quality above minimums.

## Expected AI Usage

- Offense: choose reachable node with best `shot_quality_*` above threshold.
- Defense: choose zone via threat ranking (zone weights + ball/opponent state).
- Navigation: prefer paths through higher-priority zones.

## Zone Vocabulary + Color Mapping

**Zone IDs**: `rim`, `paint`, `wing_left`, `wing_right`, `midcourt`, `ramp_left`, `ramp_right`, `perimeter`, `dead`.

**Colors (PNG)**
- `rim` #ff3b30, `paint` #ff9500, `wing_left` #ffcc00, `wing_right` #ffd60a
- `midcourt` #34c759, `ramp_left` #007aff, `ramp_right` #5ac8fa
- `perimeter` #8e8e93, `dead` #1c1c1e

**Precedence**
- Rank zones by expected scoring value (heatmap or match data) to guide defense.

## Defensive Test Scenarios (Inputs → Expected Outputs)

### 1) Ball carrier on floor, medium distance
**Inputs**: flat level, opponent at midcourt, distance > `pressure_threshold`.  
**Expected**: `InterceptDefense`, intercept target on opponent→basket line, steady approach.

### 2) Ball carrier on floor, within pressure range
**Inputs**: distance < `pressure_threshold` but > `steal_range`.  
**Expected**: `PressureDefense`, closes distance, maintains spacing.

### 3) Ball carrier within steal range
**Inputs**: distance < `profile.steal_range`.  
**Expected**: `AttemptSteal`, commitment timer starts; steal attempts after reaction/cooldown.

### 4) Opponent elevated, AI on floor (ramps available)
**Inputs**: ramp level, opponent elevated, AI on floor.  
**Expected**: `InterceptDefense` (or `AttemptSteal` if close); routes to ramp, climbs.

### 5) Opponent elevated, AI already elevated
**Inputs**: both elevated, similar height.  
**Expected**: `InterceptDefense` or `PressureDefense`; direct chase, jump only if needed.

## Defensive Test Matrix (Concrete Runs)

1) **Flat defense baseline**  
   `cargo run --bin simulate -- --level 4 --left v3_Steady_Deep --right v3_Rush_Smart --duration 45 --seed 111 --db logs/ai_defense.db`  
   Expected: high `InterceptDefense`, low ramp usage, low oscillation.

2) **Ramp defense baseline**  
   `cargo run --bin simulate -- --level 8 --left v3_Steady_Deep --right v3_Rush_Smart --duration 45 --seed 112 --db logs/ai_defense.db`  
   Expected: ramp traversal, nav path completions > 0.

3) **Pressure vs steal thresholds**  
   `cargo run --bin simulate -- --level 14 --left v3_Steady_Deep --right v3_Rush_Smart --duration 45 --seed 113 --db logs/ai_defense.db`  
   Expected: `PressureDefense`/`AttemptSteal` transitions, low oscillation.

4) **High basket height influence**  
   `cargo run --bin simulate -- --level 3 --left v3_Steady_Deep --right v3_Rush_Smart --duration 45 --seed 114 --db logs/ai_defense.db`  
   Expected: zones shift upward; intercept points adjust.

5) **No ramps (elevated platforms)**  
   `cargo run --bin simulate -- --level 9 --left v3_Steady_Deep --right v3_Rush_Smart --duration 45 --seed 115 --db logs/ai_defense.db`  
   Expected: more direct chase on platforms; fewer ramp-targeted behaviors.

Level indices from `config/levels.txt` (Debug=1, Regression=2, Arena=3, Open Floor=4, Islands=5, Slopes=6, Tower=7, Skyway=8, Terraces=9, Catwalk=10, Bunker=11, Pit=12, Twin Towers=13, Pursuit Arena=14, Pursuit Arena 2=15).

## Metrics to Validate

- `goal_time` per goal (Intercept/Pressure/AttemptSteal).
- `steals_attempted` / `steals_successful`.
- Distance to opponent over time (snapshots or custom metrics).
- Nav path completion count in elevated scenarios.
- Goal oscillation frequency (hysteresis effectiveness).

## Instrumentation Gaps (for zone testing)

- Zone occupancy not tracked; derive from tick snapshots.
- Threat inputs (zone, basket proximity, height advantage) not logged; derive or emit.

## Notes for Planned Fixes

- `ai_navigation_update` opponent selection should be dynamic + zone-aware.
- Cooldown timing should live in a single schedule (avoid double-ticking).
