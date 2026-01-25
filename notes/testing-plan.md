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

## Physics Testing Strategies

For testing physics interactions, we have two complementary approaches:

### 1. High-Level (Player Actions)

Use scenario tests with scripted player inputs for end-to-end verification:

```toml
# Example: Player shoots, ball should score
[[input]]
frame = 10
shooter = { throw_held = true }

[[input]]
frame = 100
shooter = { throw_held = false }  # Release shot

[[expect.state]]
after_frame = 200
checks = ["score.left = 1"]
```

Best for:
- Complete player actions (shooting, stealing, pickup)
- Testing the full stack from input → physics → outcome
- Shot accuracy testing (`cargo run --bin simulate -- --shot-test`)

### 2. Low-Level (Direct Physics)

Spawn objects with specific positions and velocities, then verify physics behavior:

```toml
# Example: Ball bounces off rim correctly
[[setup.entities]]
type = "ball"
x = 740.0
y = -200.0
velocity_x = -50.0
velocity_y = -100.0

[[expect.state]]
after_frame = 60
checks = [
    "ball.velocity_y > 0",  # Should bounce upward
    "ball.x < 740.0",       # Should deflect left
]
```

Best for:
- Testing 2+ body physics interactions (ball-rim, ball-wall, player-platform)
- Isolating specific collision behaviors
- Verifying bounce coefficients, friction, gravity
- Rim rejection/acceptance scenarios
- Testing edge cases without complex input sequences

### Statistical Shot Testing

For shot accuracy tuning, use the simulate binary:

```bash
cargo run --bin simulate -- --shot-test 30 --level 3
```

This fires shots from multiple positions and tracks:
- **Goal**: Ball enters basket
- **Overshoot**: Ball's peak height exceeds basket Y (missed high)
- **Undershoot**: Ball's peak height below basket Y (missed low)

Target: 40-60% overshoot/undershoot ratio for balanced feel.

### Physics Parameters to Test

| Interaction | Direct Test Approach |
|-------------|---------------------|
| Ball-rim bounce | Set ball velocity toward rim, check deflection angle |
| Ball-floor friction | Set ball rolling, measure deceleration |
| Ball gravity | Set ball at height, measure fall time |
| Player-wall collision | Move player into wall, verify position clamped |
| Platform landing | Drop player onto platform, verify grounded state |
| Shot trajectory | Use `--shot-test` for statistical coverage |

## Balance Simulation Suite

Statistical simulations for game balance testing. These tests run many iterations to detect bias and regressions in game mechanics.

**Pattern:**
1. Run headless simulation with controlled inputs
2. Collect outcome statistics (goals, misses, steals, etc.)
3. Compare ratios against target thresholds
4. Report PASS/FAIL for automated checking

**Current simulations:**

| Test | Command | Target | Measures |
|------|---------|--------|----------|
| Shot accuracy | `--shot-test 30 --level 3` | 40-60% over/under | Overshoot vs undershoot ratio |

**Planned simulations:**

| Test | Target | Measures |
|------|--------|----------|
| Steal success rate | ~33% base | Successful steals / attempts |
| AI profile balance | <60% win rate | Any profile dominating |
| Ball physics consistency | Low variance | Bounce heights, friction |

**Usage:**
```bash
# Quick balance check (in audit checklist)
cargo run --bin simulate -- --shot-test 30 --level 3

# Full tournament for AI balance
cargo run --bin simulate -- --tournament 5
```

**Adding new simulations:**
1. Add `SimMode` variant in `src/simulation/config.rs`
2. Add CLI flag parsing
3. Implement test runner in `src/simulation/runner.rs`
4. Define pass/fail thresholds
5. Add to audit checklist in CLAUDE.md

## Feature Coverage Map

*Last updated: 2026-01-24*
*Current tests: 33 passing*

### Legend
- ✅ = Has test(s)
- ⚠️ = Partial coverage
- ❌ = No test coverage

---

### Movement (8 tests)
| Feature | Status | Test(s) | Notes |
|---------|--------|---------|-------|
| Walk left | ✅ | `movement/walk_left` | |
| Walk right | ✅ | `movement/walk_right` | |
| Basic jump | ✅ | `movement/jump_basic` | |
| Max jump height | ✅ | `movement/jump_max_height` | |
| Air control | ✅ | `movement/air_control` | |
| Coyote time | ✅ | `movement/coyote_time` | Jump after leaving platform edge |
| Jump buffer | ✅ | `movement/jump_buffer` | Jump pressed while airborne before landing |
| Jump while moving | ✅ | `movement/jump_while_moving` | Horizontal movement during jump |
| Variable jump height | ❌ | - | Hold vs tap jump |
| Acceleration curve | ❌ | - | Movement ramp-up speed |

### Ball Physics (7 tests)
| Feature | Status | Test(s) | Notes |
|---------|--------|---------|-------|
| Pickup stationary | ✅ | `ball/pickup_stationary` | |
| Pickup while moving | ✅ | `ball/pickup_while_moving` | |
| Drop on jump | ✅ | `ball/drop_on_jump` | Ball doesn't stick during jump |
| Bounce floor | ✅ | `ball/bounce_floor` | |
| Bounce rim | ✅ | `ball/bounce_rim` | |
| Rolling stops | ✅ | `ball/rolling_stops` | Friction slows ball |
| Ball near own basket | ✅ | `ball/pass_through_own_basket` | Ball behaves correctly near defensive basket |
| Ball follow holder | ❌ | - | Ball position relative to player |
| Ball shot grace | ❌ | - | 100ms no-friction after shot |
| Ball spin visual | ❌ | - | Angular velocity/rotation |
| Ball-wall bounce | ❌ | - | Side wall collision |
| Ball gravity | ❌ | - | Isolated gravity test |

### Shooting (5 tests)
| Feature | Status | Test(s) | Notes |
|---------|--------|---------|-------|
| Basic shot | ✅ | `shooting/shoot_basic` | Charge and release |
| Max charge | ✅ | `shooting/shoot_max_charge` | Full power shot |
| Aim left | ✅ | `shooting/shoot_aim_left` | Target basket switching |
| Shoot while jumping | ✅ | `shooting/shoot_while_jumping` | |
| Shot from elevation | ✅ | `shooting/shoot_from_elevation` | Shooting from platform |
| Shot variance | ❌ | - | RNG spread on shots |
| Minimum charge threshold | ❌ | - | Shot requires min charge |
| Shot accuracy (statistical) | ⚠️ | `--shot-test` flag | Separate binary, not scenario test |

### Scoring (4 tests)
| Feature | Status | Test(s) | Notes |
|---------|--------|---------|-------|
| Basic score | ✅ | `scoring/score_basic` | Ball enters basket |
| Score increments | ✅ | `scoring/score_increments` | Multiple goals counted |
| Ball respawn after score | ✅ | `scoring/ball_respawn` | Ball resets to center |
| Own goal | ✅ | `scoring/own_goal` | Shooting toward own basket |
| Score attribution | ❌ | - | Verify correct team gets point |

### Stealing (6 tests)
| Feature | Status | Test(s) | Notes |
|---------|--------|---------|-------|
| Steal in range | ✅ | `stealing/steal_in_range` | Attempt when close |
| Steal out of range | ✅ | `stealing/steal_out_of_range` | No attempt when far |
| Steal cooldown | ✅ | `stealing/steal_cooldown` | Can't spam steal |
| Steal while charging | ✅ | `stealing/steal_while_charging` | Attempt triggers during victim charge |
| Steal knockback | ✅ | `stealing/steal_knockback` | Multiple attempts, verifies no crash |
| No-stealback cooldown | ✅ | `stealing/no_stealback_cooldown` | Victim gets 1s cooldown |
| Steal success/fail ratio | ❌ | - | Statistical test (33% base) |

### Collision (3 tests)
| Feature | Status | Test(s) | Notes |
|---------|--------|---------|-------|
| Wall stops player | ✅ | `collision/wall_stops_player` | |
| Platform collision | ✅ | `collision/platform_landing` | Player lands on platform |
| Platform head bonk | ✅ | `collision/platform_head_bonk` | Hit platform from below |
| Player-basket collision | ❌ | - | Can't walk through baskets |
| Ball-basket rim collision | ⚠️ | `ball/bounce_rim` | Basic bounce only |
| Corner step collision | ❌ | - | Step platform shape |

### AI Behavior (0 tests)
| Feature | Status | Test(s) | Notes |
|---------|--------|---------|-------|
| AI navigation | ❌ | - | Pathfinding to targets |
| AI decision making | ❌ | - | Goal state machine |
| AI shooting | ❌ | - | When/where AI shoots |
| AI defending | ❌ | - | Positioning, stealing |
| AI jump capability | ❌ | - | Knows what it can reach |

### Input System (0 tests)
| Feature | Status | Test(s) | Notes |
|---------|--------|---------|-------|
| Input buffering | ❌ | - | Buffered press inputs |
| Gamepad support | ❌ | - | Not testable in headless |

---

### Priority Test Gaps

**High Priority (MVP blockers):** All done!
**Medium Priority (Quality):** All done!
**Low Priority (mostly done):** Most done!

**Remaining gaps:**
1. `movement/variable_jump_height` - Hold vs tap jump
2. `scoring/score_attribution` - Verify correct team gets point
3. Statistical tests (steal ratio, shot variance) - Requires seeded RNG

---

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
