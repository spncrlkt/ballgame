# Scenario Testing System

## Overview

Deterministic input-script testing for core game mechanics. Tests are TOML files specifying entity setup, frame-based inputs, and expected outcomes (event sequences + state assertions).

## Design Decisions

| Decision | Choice |
|----------|--------|
| RNG handling | Fixed seed for deterministic randomness |
| Test levels | Separate file: `assets/test_levels.txt` |
| Event coverage | Use state assertions for movement/physics (no new events) |
| Test file format | TOML |
| Invocation | Separate binary: `cargo run --bin test-scenarios` |
| Output | Pass/fail with error message |

## File Structure

```
tests/
└── scenarios/
    ├── movement/
    │   ├── walk_right.toml
    │   ├── walk_left.toml
    │   ├── jump_basic.toml
    │   ├── jump_onto_platform.toml
    │   └── air_control.toml
    ├── ball/
    │   ├── pickup_stationary.toml
    │   ├── pickup_while_moving.toml
    │   ├── drop_on_jump.toml
    │   ├── bounce_floor.toml
    │   └── rolling_stops.toml
    ├── shooting/
    │   ├── shoot_basic.toml
    │   ├── shoot_max_charge.toml
    │   ├── shoot_aim_left.toml
    │   └── shoot_while_jumping.toml
    ├── scoring/
    │   ├── score_basic.toml
    │   └── score_increments.toml
    ├── stealing/
    │   ├── steal_in_range.toml
    │   ├── steal_out_of_range.toml
    │   └── steal_cooldown.toml
    └── collision/
        └── wall_stops_player.toml

assets/
└── test_levels.txt          # Minimal test arenas (separate from game levels)

src/
├── testing/
│   ├── mod.rs               # Public exports, TestResult enum
│   ├── parser.rs            # TOML parsing into TestDefinition
│   ├── runner.rs            # Execute single test, return result
│   ├── assertions.rs        # Event sequence matching, state checking
│   └── input.rs             # ScriptedInputs resource, frame-by-frame injection
└── bin/
    └── test_scenarios.rs    # CLI: discover tests, run all, report results
```

## Test File Format (TOML)

```toml
# Example: tests/scenarios/stealing/steal_while_charging.toml

name = "Steal succeeds against charging player"
description = "Steal has +17% chance when victim is charging a shot"

[setup]
level = "test_flat_floor"
seed = 12345                      # Fixed RNG seed for this test

# Explicit entity spawning - only listed entities spawn
[[setup.entities]]
type = "player"
id = "attacker"
team = "left"
x = 400
y = 220
facing = 1.0

[[setup.entities]]
type = "player"
id = "victim"
team = "right"
x = 500
y = 220
facing = -1.0
holding_ball = true               # Spawns with ball attached

# Frame-based inputs (60 FPS, so frame 60 = 1 second)
[[input]]
frame = 0
victim = { throw_held = true }    # Start charging

[[input]]
frame = 30
attacker = { move_right = true }

[[input]]
frame = 45
attacker = { move_right = false, steal = true }

# Expected event sequence (order matters, timing has tolerance)
[[expect.sequence]]
event = "StealAttempt"
player = "attacker"
frame_min = 40
frame_max = 50

[[expect.sequence]]
event = "StealSuccess"
player = "attacker"
# No frame bounds = just check it happens after previous event

# State assertions after simulation ends
[expect.state]
after_frame = 60
checks = [
    "attacker.holding_ball = true",
    "victim.holding_ball = false",
]
```

## Test Levels Format

Uses existing level parser format. File: `assets/test_levels.txt`

```
# Test Levels - Minimal arenas for scenario testing
# These are separate from game levels and use debug mode.

# FLAT FLOOR - Just floor, walls, baskets. No platforms.
# Use for: movement, basic ball physics, pickup, basic shooting
level: test_flat_floor
basket_height: 250
basket_push_in: 60
steps: 0
debug: true

# SINGLE PLATFORM - One center platform at jumpable height
# Use for: platform collision, landing, jumping onto platforms
level: test_single_platform
basket_height: 250
basket_push_in: 60
steps: 0
center: 150 300
debug: true

# SHOOTING RANGE - Low, close baskets for easy scoring
# Use for: scoring tests, shot trajectory validation
level: test_shooting_range
basket_height: 150
basket_push_in: 40
steps: 0
debug: true

# STEAL ARENA - Flat floor, wide spacing
# Use for: steal mechanics, player-to-player interactions
level: test_steal_arena
basket_height: 300
basket_push_in: 80
steps: 0
debug: true
```

## Core Types

```rust
// src/testing/parser.rs

#[derive(Debug, Deserialize)]
pub struct TestDefinition {
    pub name: String,
    pub description: Option<String>,
    pub setup: TestSetup,
    pub input: Vec<FrameInput>,
    pub expect: TestExpectations,
}

#[derive(Debug, Deserialize)]
pub struct TestSetup {
    pub level: String,
    pub seed: Option<u64>,           // RNG seed, defaults to 0
    pub entities: Vec<EntityDef>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum EntityDef {
    #[serde(rename = "player")]
    Player {
        id: String,
        team: String,                // "left" or "right"
        x: f32,
        y: f32,
        facing: Option<f32>,         // default 1.0
        holding_ball: Option<bool>,  // default false
    },
    #[serde(rename = "ball")]
    Ball {
        x: f32,
        y: f32,
        velocity: Option<(f32, f32)>,
    },
}

#[derive(Debug, Deserialize)]
pub struct FrameInput {
    pub frame: u64,
    #[serde(flatten)]
    pub inputs: HashMap<String, InputSnapshot>,
}

#[derive(Debug, Deserialize)]
pub struct InputSnapshot {
    pub move_x: Option<f32>,
    pub jump: Option<bool>,
    pub pickup: Option<bool>,
    pub steal: Option<bool>,
    pub throw_held: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct TestExpectations {
    pub sequence: Vec<ExpectedEvent>,
    pub state: Option<StateAssertion>,
}

#[derive(Debug, Deserialize)]
pub struct ExpectedEvent {
    pub event: String,               // "Pickup", "ShotRelease", "Goal", etc.
    pub player: Option<String>,      // entity id
    pub frame_min: Option<u64>,
    pub frame_max: Option<u64>,
    #[serde(default = "default_tolerance")]
    pub tolerance: u64,              // default ±5 frames
}

#[derive(Debug, Deserialize)]
pub struct StateAssertion {
    pub after_frame: u64,
    pub checks: Vec<String>,         // e.g., "attacker.holding_ball = true"
}
```

## Test Runner Flow

1. Discover all `.toml` files in `tests/scenarios/`
2. For each test:
   a. Parse TOML into `TestDefinition`
   b. Load test level from `assets/test_levels.txt`
   c. Initialize headless simulation with fixed seed
   d. Spawn entities as specified
   e. Run simulation, injecting inputs at specified frames
   f. Capture events to buffer
   g. After final frame, check sequence assertions
   h. Check state assertions
   i. Return pass/fail with error details
3. Report summary

## CLI Usage

```bash
# Run all tests
cargo run --bin test-scenarios

# Run single category
cargo run --bin test-scenarios -- movement/

# Run single test
cargo run --bin test-scenarios -- shooting/shoot_basic

# Verbose mode (show event log on failure)
cargo run --bin test-scenarios -- --verbose
```

## Output Format

```
Scenario Tests
==============

movement/
  walk_right ..................... PASS (47 frames)
  walk_left ...................... PASS (47 frames)
  jump_basic ..................... PASS (82 frames)

ball/
  pickup_stationary .............. PASS (38 frames)
  pickup_while_moving ............ FAIL
    Expected: Pickup event at frame 40-50
    Actual: No Pickup event found

shooting/
  shoot_basic .................... PASS (95 frames)

==============
Results: 18 passed, 1 failed
```

## Implementation Order

1. **Infrastructure** (`src/testing/mod.rs`, `parser.rs`)
   - TOML parsing with serde
   - TestDefinition structs
   - Level name resolution

2. **Input injection** (`src/testing/input.rs`)
   - `ScriptedInputs` resource
   - Modify simulation to read from scripts instead of AI
   - Entity ID tracking with `TestEntityId` component

3. **Test runner** (`src/testing/runner.rs`)
   - Spawn test entities
   - Run simulation loop with input injection
   - Capture events

4. **Assertions** (`src/testing/assertions.rs`)
   - Sequence matching (ordered events with timing tolerance)
   - State checking (query world after simulation)

5. **Test levels** (`assets/test_levels.txt`)
   - Minimal arenas for each test category

6. **CLI binary** (`src/bin/test_scenarios.rs`)
   - Test discovery
   - Parallel or sequential execution
   - Summary reporting

7. **Initial test suite**
   - One test per category to validate system
   - Expand to full 20 tests

## Events Available for Assertions

From existing event logger:
- `Pickup` - player picked up ball
- `Drop` - player dropped ball
- `ShotStart` - player began charging
- `ShotRelease` - player released shot
- `StealAttempt` - player attempted steal
- `StealSuccess` - steal succeeded
- `StealFail` - steal failed
- `Goal` - ball entered basket

## State Assertions Available

Query world state after simulation:
- `{entity_id}.x`, `{entity_id}.y` - position
- `{entity_id}.velocity_x`, `{entity_id}.velocity_y` - velocity
- `{entity_id}.holding_ball` - bool
- `{entity_id}.grounded` - bool
- `ball.state` - "Free", "Held", "InFlight"
- `ball.x`, `ball.y` - ball position
- `score.left`, `score.right` - team scores

## Milestone Todo

Add to `milestones.md` under V1/Beyond:

```markdown
**CI/Automation:**
- [ ] Automated build + test workflow
  - `cargo check` and `cargo clippy` for compilation
  - `cargo run --bin test-scenarios` for functional tests
  - `./scripts/regression.sh` for visual regression
  - Local script or GitHub Actions
  - Pre-commit hook option
```
