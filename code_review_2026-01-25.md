# Code Review: 2026-01-25

**Commit:** `7462671` - "ghost replay clean up and ai tuning"
**Codebase Size:** ~15,000 lines across 50+ files
**Reviewer:** Claude Code (deep analysis session)

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Best Practices Library](#best-practices-library)
3. [Anti-Patterns Analysis](#anti-patterns-analysis)
4. [Codebase Deep Dive](#codebase-deep-dive)
5. [Game Design Fundamentals](#game-design-fundamentals)
6. [Review Process Gaps](#review-process-gaps)
7. [Iterative Improvement Plan](#iterative-improvement-plan)
8. [Resources & References](#resources--references)

---

## Executive Summary

### Overall Assessment

| Category | Grade | Notes |
|----------|-------|-------|
| Physics/Timing | A+ | Excellent frame-rate independence, proper FixedUpdate usage |
| Input Handling | A+ | Buffering pattern well-implemented throughout |
| ECS Architecture | A | Good component design, some query complexity |
| AI Decision System | B+ | Works but has scaling concerns, could use refactoring |
| Performance | A- | Some hot-path allocations, RNG instantiation spread |
| Game Design | B+ | Solid foundation, needs balance iteration |

### Key Findings Summary

**Strengths:**
- Frame-rate independent physics using `time.delta_secs()` consistently
- Proper input buffering for all `just_pressed()` events
- Good separation between Update (visuals) and FixedUpdate (physics)
- Comprehensive constants management in `constants.rs`
- Solid testing infrastructure with scenario tests

**Areas for Improvement:**
- AI decision system in single 1195-line file (`ai/decision.rs`)
- 21+ `thread_rng()` instantiations scattered (should consolidate)
- Some very large functions (see complexity section)
- Limited spatial partitioning (O(n*m) collisions acceptable for now)

---

## Best Practices Library

### 1. Physics & Timing

#### 1.1 The "Fix Your Timestep" Pattern
**Source:** [Gaffer on Games - Fix Your Timestep!](https://gafferongames.com/post/fix_your_timestep/)

The fundamental principle: decouple physics simulation rate from render rate.

```
┌─────────────────────────────────────────┐
│ Frame                                    │
│  ├── Accumulate time (dt)               │
│  ├── While accumulator >= FIXED_DT:     │
│  │    └── Physics step (FIXED_DT)       │
│  │    └── Decrement accumulator         │
│  └── Render (interpolate if needed)     │
└─────────────────────────────────────────┘
```

**Your implementation:** Bevy handles this via `FixedUpdate` schedule. You correctly use it for all physics. **Grade: A+**

**Spiral of Death Prevention:** Bevy caps physics steps per frame by default, but monitor with `bevy/trace_tracy` if you add expensive physics.

#### 1.2 Time-Based vs Per-Frame Physics

| Operation | Correct Pattern | Why |
|-----------|-----------------|-----|
| Gravity | `velocity.y -= GRAVITY * dt` | Accumulates correctly over time |
| Friction | `velocity.x *= FRICTION.powf(dt)` | Exponential decay needs `.powf()` |
| Timer | `timer -= dt` | Linear countdown |
| One-shot impulse | `velocity.y = JUMP_VELOCITY` | No `dt` - happens once |

**Anti-pattern found:** None in your codebase. All physics are time-based correctly.

#### 1.3 Collision Epsilon

**The Problem:** When entity A rests on platform B with zero overlap, the next frame's collision check sees no overlap and A falls.

```
Wrong:  entity.y = platform.top + entity.half_height     // Zero overlap
Right:  entity.y = platform.top + entity.half_height - ε // Guaranteed overlap
```

**Your implementation:** Uses `COLLISION_EPSILON = 0.5` consistently. **Grade: A+**

---

### 2. Input Handling

#### 2.1 Input Buffering Pattern

**The Problem:** `just_pressed()` returns true for ONE Update frame. If FixedUpdate doesn't run that frame, input is lost.

```rust
// WRONG - Input lost if FixedUpdate skips a frame
fn fixed_update(keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::Space) { /* may never fire */ }
}

// CORRECT - Buffer in Update, consume in FixedUpdate
fn update(mut input: ResMut<InputBuffer>, keyboard: Res<..>) {
    if keyboard.just_pressed(KeyCode::Space) {
        input.jump_pressed = true;  // Only set true, never false
    }
}

fn fixed_update(mut input: ResMut<InputBuffer>) {
    if input.jump_pressed {
        input.jump_pressed = false;  // Consume after reading
        // ... apply jump
    }
}
```

**Your implementation:** `PlayerInput` resource with proper accumulation/consumption. **Grade: A+**

#### 2.2 Coyote Time & Jump Buffering

Two complementary techniques for responsive platforming:

| Technique | What It Does | Typical Value |
|-----------|--------------|---------------|
| Coyote Time | Allow jump shortly after leaving platform | 100-150ms |
| Jump Buffer | Remember jump input before landing | 100-150ms |

**Your implementation:** Both implemented with `COYOTE_TIME = 0.1` and `JUMP_BUFFER_TIME = 0.1`. **Grade: A+**

---

### 3. ECS Architecture

#### 3.1 Component vs Resource Decision Tree

```
Is this data per-entity?
├── YES → Component
│   └── Multiple entities with Health? → Component
│   └── Single player position? → Component (on player entity)
└── NO → Resource
    └── Global score? → Resource
    └── Current level? → Resource
    └── Input state? → Resource (or Component if per-player)
```

**Anti-pattern:** Entity data in HashMap inside Resource

```rust
// WRONG
#[derive(Resource)]
struct EntityHealth(HashMap<Entity, f32>);

// RIGHT
#[derive(Component)]
struct Health(f32);
```

**Your implementation:** Good separation. `Score`, `CurrentLevel`, `PhysicsTweaks` as Resources; player-specific data as Components. **Grade: A**

#### 3.2 Query Optimization

**Filter vs Fetch:**
```rust
// Fetches unused data
Query<(&Transform, &Velocity, &Health)>  // 3 components fetched

// Better - filter without fetching
Query<&Transform, (With<Velocity>, With<Health>)>  // 1 component fetched
```

**Option<&T> Pitfall:**
```rust
// Matches EVERY entity that has or doesn't have Shield
Query<(Option<&Shield>, Option<&Health>)>  // Very broad

// Matches only entities with Health, checks Shield
Query<&Health, With<Shield>>  // Narrow, efficient
```

**Your implementation:** Generally good, but some queries in `ui/debug.rs` and `simulation/runner.rs` are complex (13+ parameters). Consider splitting.

#### 3.3 System Ordering

```rust
// Ambiguous - may run in any order
app.add_systems(Update, (system_a, system_b));

// Explicit dependency
app.add_systems(Update, (system_a, system_b.after(system_a)));

// Strict chain
app.add_systems(FixedUpdate, (
    apply_input,
    apply_gravity,
    apply_velocity,
    check_collisions,
).chain());
```

**Your implementation:** Good use of `.chain()` for FixedUpdate physics. Update systems have explicit ordering where needed. **Grade: A**

---

### 4. AI Decision Systems

#### 4.1 State Machines vs Behavior Trees

**Source:** [FSM vs Behavior Tree Analysis](https://medium.com/@abdullahahmetaskin/finite-state-machine-and-behavior-tree-fusion-3fcce33566)

| Aspect | State Machine | Behavior Tree |
|--------|---------------|---------------|
| Complexity | O(n²) transitions | O(n) depth |
| Scalability | Poor (>20 states = spaghetti) | Good (modular subtrees) |
| Reactivity | Explicit transitions | Natural interrupts |
| Debugging | Easy (current state) | Harder (tree traversal) |
| Performance | Lower overhead | Higher but manageable |

**Recommendation for your game:**

Your current goal-based system (`AiGoal` enum with ~10 goals) is a lightweight state machine. This is appropriate for 1v1/2v2 sports AI because:
- Limited state space (chase, shoot, defend, steal)
- Fast transitions needed (reactive gameplay)
- Performance is critical (multiple AI per frame)

**When to consider Behavior Trees:**
- If goals exceed 15-20 states
- If you need complex sequencing (defend THEN steal THEN retreat)
- If AI feels "robotic" and needs more nuanced behavior

#### 4.2 Goal Prioritization

**Current approach:** Large if-else chain in `ai_decision_update`

**Better approach:** Utility AI / Priority Queue

```rust
struct GoalCandidate {
    goal: AiGoal,
    priority: f32,
    conditions_met: bool,
}

// Each tick, evaluate all candidates
let candidates = vec![
    evaluate_chase_ball(state),
    evaluate_defend(state),
    evaluate_shoot(state),
    evaluate_steal(state),
];

// Pick highest priority that's valid
let best = candidates.iter()
    .filter(|c| c.conditions_met)
    .max_by(|a, b| a.priority.partial_cmp(&b.priority));
```

**Benefits:**
- Each goal's logic isolated in its own function
- Easy to add new goals without modifying others
- Priority tuning via AI profiles

---

### 5. Performance Patterns

#### 5.1 Avoiding Allocations in Hot Paths

**Hot path:** Code that runs every frame (Update, FixedUpdate)

```rust
// WRONG - allocates every frame
fn per_frame_system() {
    let results: Vec<Entity> = query.iter().collect();  // ALLOCATION
    let message = format!("Score: {}", score);          // ALLOCATION
}

// BETTER - reuse buffers
#[derive(Resource)]
struct ScratchBuffers {
    entity_buffer: Vec<Entity>,
}

fn per_frame_system(mut buffers: ResMut<ScratchBuffers>) {
    buffers.entity_buffer.clear();  // Reuse capacity
    for entity in query.iter() {
        buffers.entity_buffer.push(entity);
    }
}
```

**Your implementation:** Some `format!()` calls in debug text, but guarded by `is_changed()` checks. **Grade: A-**

#### 5.2 RNG Consolidation

**Current:** 21+ `thread_rng()` calls scattered across codebase

```rust
// Creates new RNG each call (syscall overhead)
let angle = rand::thread_rng().gen_range(-0.1..0.1);
```

**Better:** Resource-based RNG

```rust
#[derive(Resource)]
struct GameRng(rand::rngs::StdRng);

fn system(mut rng: ResMut<GameRng>) {
    let angle = rng.0.gen_range(-0.1..0.1);  // No syscall
}
```

**Benefits:**
- Deterministic replays (seed the RNG)
- Better performance (no `getrandom` syscalls)
- Easier testing

#### 5.3 String Key Lookups

**Current:** Ball textures keyed by `String`

```rust
ball_textures.get(style.name())  // String lookup
```

**Better:** Enum or index-based

```rust
ball_textures.get(BallStyleId::Wedges)  // No string allocation/comparison
```

**Impact:** Low for current usage (only on style change), but would matter for particle systems.

---

## Anti-Patterns Analysis

### Found in Codebase

#### 1. God Function: `ai_decision_update` (1195 lines)

**File:** `src/ai/decision.rs`
**Problem:** Single function handling all AI decision logic
**Impact:** Hard to test, hard to modify, hard to understand

**Suggested refactor:**
```
ai/
├── decision.rs      # Coordinator, goal selection
├── goals/
│   ├── chase_ball.rs
│   ├── defend.rs
│   ├── shoot.rs
│   └── steal.rs
└── evaluation.rs    # Shot quality, position scoring
```

#### 2. Monolithic Cycle System: `unified_cycle_system`

**File:** `src/ui/debug.rs:299-597` (300 lines)
**Problem:** Handles 4 different D-pad directions, each with different logic
**Impact:** Hard to add new cycle targets, high cognitive load

**Suggested refactor:** Split into per-direction handlers or data-driven approach.

#### 3. Scattered RNG Instantiation

**Files:** Multiple (see grep results above)
**Problem:** 21+ `thread_rng()` calls, each a syscall
**Impact:** Performance overhead, non-deterministic replays

#### 4. Complex Query Signatures

**File:** `src/simulation/runner.rs:331-339`
```rust
fn metrics_update(
    mut metrics: ResMut<SimMetrics>,
    players: Query<(Entity, &Transform, &Team, &AiState, &JumpState,
                   &AiNavState, Option<&HoldingBall>, &TargetBasket), With<Player>>,
    balls: Query<(&Transform, &BallState), With<Ball>>,
    baskets: Query<(&Transform, &Basket)>,
    score: Res<Score>,
)
```

**Impact:** 8-tuple query is hard to read and modify

**Suggested:** Use `#[derive(QueryData)]` for custom query structs (Bevy 0.17+)

### Not Found (Good!)

- **Raw input in FixedUpdate** - All input properly buffered
- **Frame-rate dependent physics** - All uses `delta_secs()`
- **Missing collision epsilon** - Consistently used
- **Unbounded physics steps** - Bevy handles this

---

## Codebase Deep Dive

### File-by-File Analysis

#### High Priority Files

| File | Lines | Complexity | Issues |
|------|-------|------------|--------|
| `ai/decision.rs` | 1195 | High | God function, nested conditionals |
| `simulation/runner.rs` | 1391 | High | 4 separate App builds, duplication |
| `bin/training.rs` | 1092 | Medium | Similar to runner.rs |
| `ui/debug.rs` | 857 | High | Mixed concerns (cycle, debug, viewport) |

#### Duplication Patterns

1. **App setup** duplicated across `main.rs`, `simulation/runner.rs`, `bin/training.rs`
   - Each builds similar plugin/system sets
   - Consider: shared `GamePlugins` bundle

2. **Ball spawning** in `main.rs:setup()` and `player/physics.rs:spawn_balls()`
   - Similar entity composition
   - Consider: `ball/spawning.rs` with `spawn_ball()` helper

3. **Level geometry** in `respawn_player` and `config_watcher`
   - Both rebuild platforms on level change
   - Consider: `levels/geometry.rs` with `rebuild_level()`

### Metrics

| Metric | Value | Assessment |
|--------|-------|------------|
| Total .rs files | 50+ | Reasonable modularization |
| Largest file | 1391 lines | Should be split |
| Average file | ~300 lines | Good |
| Constants centralized | Yes | Excellent |
| Magic numbers found | 2-3 | Minor (STICK_DEADZONE) |
| `thread_rng()` calls | 21 | Should consolidate |
| `to_string()` calls | ~164 | Many not in hot paths |
| Clippy warnings | ~90 | Mostly style (type_complexity) |

---

## Game Design Fundamentals

### 2D Arcade Sports Games

**Reference games:** NBA Jam, Windjammers, Lethal League, Duck Game

**Source:** [Sports Game Design Principles](https://gamedesignskills.com/game-design/sports/)

#### Core Loop Analysis

Your game's core loop:
```
[Get Ball] → [Navigate to Position] → [Shoot/Score] → [Defend/Steal] → [Repeat]
```

**NBA Jam's approach:**
- Simple inputs (shoot, pass, turbo)
- Exaggerated physics (dunks from halfcourt)
- Clear feedback (fire effects, announcer)

**Your potential improvements:**
1. **Juice/Feedback:** Score effects, hit stop, screen shake
2. **Risk/Reward:** Longer charge = better shot but stealable
3. **Momentum:** Winning team gets harder to stop, losing gets help

#### Balance Considerations for 2v2

| Element | 1v1 Balance | 2v2 Implications |
|---------|-------------|------------------|
| Steal | 25% success | May need lower (2 defenders) |
| Shot charge | 1.6s full | Teammate can cover |
| Positioning | Solo decision | Passing lanes matter |
| AI Goals | Chase/Shoot/Defend | Need "Support Teammate" goal |

**Key 2v2 questions:**
- Should passes exist?
- Can teammates block shots?
- Do defenders zone or man-mark?

### Movement & Physics Feel

**Source:** [Gaffer on Games Physics Series](https://gafferongames.com/)

**Your current values:**
- `GRAVITY_RISE = 980` (lighter going up)
- `GRAVITY_FALL = 1400` (heavier coming down)
- `JUMP_VELOCITY = 650`
- `MOVE_SPEED = 300`

**This creates:** Mario-style variable jump height with fast falls

**Tuning dimensions:**
1. **Snappiness:** Ground accel/decel ratio
2. **Air Control:** Air accel vs ground accel
3. **Commitment:** Jump cut multiplier
4. **Weight:** Gravity values

**Tip:** Your `PhysicsTweaks` panel is excellent for tuning. Consider saving "feel presets" for different game modes.

---

## Review Process Gaps

### What `code_review_guidelines.md` Lacks

1. **AI-specific patterns**
   - No guidance on state machine design
   - No goal prioritization patterns
   - No AI debugging techniques

2. **Game design validation**
   - No balance testing methodology
   - No player feedback loop
   - No feel/juice checklist

3. **Multiplayer considerations**
   - No determinism requirements
   - No input synchronization patterns
   - No rollback/lockstep guidance

4. **Profiling workflow**
   - Tools listed but no workflow
   - No baseline metrics defined
   - No regression thresholds

### Suggested Additions to `code_review_guidelines.md`

```markdown
### 7. AI Systems

#### 7.1 Goal-Based AI Review
- [ ] Each goal has clear entry/exit conditions
- [ ] No goals with overlapping conditions (ambiguity)
- [ ] Hysteresis prevents goal flickering
- [ ] Goal transitions logged for debugging

#### 7.2 AI Debugging
- Add `AiGoalChanged` event for logging transitions
- Use simulation mode to test AI in isolation
- Compare AI behavior across profile presets

### 8. Game Feel

#### 8.1 Juice Checklist
- [ ] Score events have visual feedback
- [ ] Steal attempts have feedback (success/fail distinct)
- [ ] Charge meter has audio/visual feedback
- [ ] Movement has acceleration curves (not instant)

#### 8.2 Balance Testing
- Run `cargo run --bin simulate -- --tournament 5`
- Check win rate variance across AI profiles
- Target: no profile >60% or <40% win rate
```

---

## Iterative Improvement Plan

### Short-Term (This Week)

| Priority | Task | Impact | Effort |
|----------|------|--------|--------|
| P0 | Consolidate RNG into Resource | Determinism, performance | 2h |
| P1 | Split `ai/decision.rs` into modules | Maintainability | 4h |
| P2 | Extract shared app setup | Reduces duplication | 2h |
| P3 | Add AI goal logging events | Debugging | 1h |

### Medium-Term (This Month)

| Priority | Task | Impact | Effort |
|----------|------|--------|--------|
| P4 | Refactor cycle system to data-driven | Maintainability | 4h |
| P5 | Add balance testing to CI | Catch regressions | 4h |
| P6 | Implement utility-based goal selection | Better AI | 8h |
| P7 | Add "juice" system (screenshake, etc) | Game feel | 8h |

### Long-Term (Future)

- **Behavior tree experiment:** Try for one AI profile to compare
- **Netcode foundation:** Deterministic simulation validation
- **Profiling baseline:** Establish frame budget targets

---

## Resources & References

### Essential Reading

| Resource | Topic | Link |
|----------|-------|------|
| Fix Your Timestep! | Physics timing | [gafferongames.com](https://gafferongames.com/post/fix_your_timestep/) |
| Game Programming Patterns | Design patterns | [gameprogrammingpatterns.com](https://gameprogrammingpatterns.com/) |
| Bevy Best Practices | Bevy-specific | [GitHub](https://github.com/tbillington/bevy_best_practices) |
| ECS FAQ | ECS concepts | [GitHub](https://github.com/SanderMertens/ecs-faq) |

### AI Design

| Resource | Topic | Link |
|----------|-------|------|
| FSM vs BT Comparison | Decision systems | [Medium](https://medium.com/@abdullahahmetaskin/finite-state-machine-and-behavior-tree-fusion-3fcce33566) |
| Behavior Trees Survey | Academic overview | [ScienceDirect](https://www.sciencedirect.com/science/article/pii/S0921889022000513) |
| Game AI Decision Making | Lecture notes | [UMD CS](https://www.cs.umd.edu/class/spring2018/cmsc425/Lects/lect21-ai-dec-making.pdf) |

### Game Design

| Resource | Topic | Link |
|----------|-------|------|
| Sports Game Design | Genre fundamentals | [gamedesignskills.com](https://gamedesignskills.com/game-design/sports/) |
| Arcade Game Design | Core loops | [gamedesignskills.com](https://gamedesignskills.com/game-design/arcade/) |
| Fundamentals of Action Games | Book | [O'Reilly](https://www.oreilly.com/library/view/fundamentals-of-action/9780133812503/) |

### Bevy-Specific

| Resource | Topic | Link |
|----------|-------|------|
| Unofficial Bevy Cheat Book | Comprehensive guide | [bevy-cheatbook.github.io](https://bevy-cheatbook.github.io/) |
| Bevy 0.17 Release | Latest features | [bevy.org](https://bevy.org/news/bevy-0-17/) |
| This Week in Bevy | News/updates | [thisweekinbevy.com](https://thisweekinbevy.com/) |

### Anti-Pattern Catalogs

| Resource | Topic | Link |
|----------|-------|------|
| Game-Specific Anti-Patterns | Academic paper | [ResearchGate](https://www.researchgate.net/publication/342408679_A_Catalogue_of_Game-Specific_Anti-Patterns) |
| ECS Design Decisions | Practical guide | [arielcoppes.dev](https://arielcoppes.dev/2023/07/13/design-decisions-when-building-games-using-ecs.html) |

---

## Appendix: Code Samples

### A. Consolidated RNG Resource

```rust
// src/rng.rs
use bevy::prelude::*;
use rand::{Rng, SeedableRng, rngs::StdRng};

#[derive(Resource)]
pub struct GameRng(pub StdRng);

impl Default for GameRng {
    fn default() -> Self {
        Self(StdRng::from_entropy())
    }
}

impl GameRng {
    pub fn from_seed(seed: u64) -> Self {
        Self(StdRng::seed_from_u64(seed))
    }
}

// Usage:
fn bounce_system(mut rng: ResMut<GameRng>) {
    let deflection = rng.0.gen_range(-0.1..0.1);
}
```

### B. Data-Driven Cycle System

```rust
// src/ui/cycle.rs
#[derive(Clone)]
struct CycleOption {
    name: &'static str,
    apply: fn(&mut World, direction: CycleDirection),
    format: fn(&World) -> String,
}

const CYCLE_OPTIONS: &[CycleOption] = &[
    CycleOption {
        name: "Viewport",
        apply: apply_viewport_cycle,
        format: format_viewport,
    },
    CycleOption {
        name: "Level",
        apply: apply_level_cycle,
        format: format_level,
    },
    // ...
];

fn unified_cycle_system(world: &mut World, direction: CycleDirection) {
    let idx = get_current_option_index(world, direction);
    CYCLE_OPTIONS[idx].apply(world, direction);
}
```

### C. AI Goal Evaluation Pattern

```rust
// src/ai/goals/mod.rs
pub trait GoalEvaluator {
    fn evaluate(&self, state: &AiState, world: &GameState) -> GoalScore;
    fn execute(&self, state: &mut AiState, input: &mut InputState);
}

pub struct GoalScore {
    pub priority: f32,     // 0.0 - 1.0
    pub urgency: f32,      // Time-sensitive bonus
    pub can_execute: bool, // Preconditions met?
}

fn select_goal(evaluators: &[Box<dyn GoalEvaluator>], state: &AiState) -> Option<AiGoal> {
    evaluators.iter()
        .map(|e| (e, e.evaluate(state, world)))
        .filter(|(_, score)| score.can_execute)
        .max_by(|(_, a), (_, b)| {
            (a.priority * a.urgency).partial_cmp(&(b.priority * b.urgency)).unwrap()
        })
        .map(|(evaluator, _)| evaluator.goal())
}
```

---

## Final Summary & Action Items

### Immediate Priorities (When Resuming)

| Priority | Task | File(s) | Est. Effort |
|----------|------|---------|-------------|
| **P0** | Consolidate RNG to `GameRng` resource | New `src/rng.rs`, ~21 files | 2h |
| **P1** | Split `ai/decision.rs` into modules | `src/ai/goals/*.rs` | 4h |
| **P2** | Extract shared app setup | `src/app_builder.rs` | 2h |
| **P3** | Add `AiGoalChanged` events | `src/events/`, `src/ai/` | 1h |

### Files to Review First

1. `src/ai/decision.rs` - 1195 lines, main refactor target
2. `src/ui/debug.rs` - 857 lines, cycle system complexity
3. `src/simulation/runner.rs` - 1391 lines, app setup duplication

### Grep Commands for Finding Issues

```bash
# Find all thread_rng() calls
rg "thread_rng\(\)" src/

# Find large functions (>100 lines)
rg -l "^pub fn" src/ | xargs wc -l | sort -n

# Find complex queries (many parameters)
rg "Query<\(" src/ -A 3 | grep -E "\(.{80,}"
```

### Session Paused

**Status:** Review complete, documentation written, no code changes made
**Resume with:** Pick a priority from P0-P3 above, or read through findings first

---

*Generated by Claude Code deep analysis session, 2026-01-25*
