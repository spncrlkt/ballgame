# Code Review Guidelines for Bevy 2D Games

A comprehensive checklist and reference for reviewing Rust/Bevy game code. Covers performance, correctness, and architecture patterns.

---

## Quick Reference Checklists

### Physics & Timing
- [ ] All physics calculations in FixedUpdate (not Update)
- [ ] Time-based values use `* time.delta_secs()` or `.powf(time.delta_secs())`
- [ ] No per-frame multipliers like `velocity *= 0.98`
- [ ] Collision epsilon used for entities resting on surfaces
- [ ] Spiral of death prevention (max physics steps per frame)

### Input Handling
- [ ] `just_pressed()` inputs buffered before FixedUpdate consumption
- [ ] Input buffer flags consumed (set to false) after use
- [ ] Continuous inputs (axes) can overwrite each frame
- [ ] No raw input reads in FixedUpdate

### ECS Queries
- [ ] Queries fetch only needed components
- [ ] `With<T>`/`Without<T>` filters instead of `Option<&T>` where possible
- [ ] No nested loops over large entity sets (O(n^2) collision)
- [ ] Mutable access only when mutation occurs

### Memory & Allocations
- [ ] No `Vec::new()`, `String::new()`, `to_string()` in hot paths
- [ ] Collections pre-allocated and reused via `.clear()`
- [ ] No string formatting in per-frame systems
- [ ] Asset handles cloned, not assets re-added

### System Organization
- [ ] Dependent systems properly ordered with `.after()`/`.before()`/`.chain()`
- [ ] Update systems handle visuals/input, FixedUpdate handles physics
- [ ] No GlobalTransform reads before TransformPropagate
- [ ] System dependencies documented

### Component Design
- [ ] Per-entity data stored in Components, not Resource hashmaps
- [ ] Global singletons stored as Resources, not singleton entities
- [ ] Components small and focused
- [ ] Flag fields preferred over frequent add/remove component

---

## Detailed Guidelines

### 1. Physics & Timing

#### 1.1 FixedUpdate vs Update Separation

**Rule:** Physics in FixedUpdate, visuals in Update.

```rust
// CORRECT - physics in FixedUpdate
app.add_systems(FixedUpdate, (
    apply_gravity,
    apply_velocity,
    check_collisions,
).chain());

// CORRECT - visuals in Update
app.add_systems(Update, (
    update_animations,
    update_ui,
));
```

**Why:** FixedUpdate runs at a consistent rate regardless of frame rate. Physics calculations depend on this consistency. Update runs once per render frame, which varies.

**Symptoms of violation:**
- Physics behaves differently at different frame rates
- Jittery movement
- Objects pass through each other at low FPS

---

#### 1.2 Time-Based Physics Values

**Rule:** All continuous physics must multiply by delta time.

```rust
// WRONG - frame-rate dependent
velocity.y -= GRAVITY;
velocity.x *= 0.98;

// CORRECT - time-based
velocity.y -= GRAVITY * time.delta_secs();
velocity.x *= FRICTION.powf(time.delta_secs());
```

**What needs time scaling:**
| Operation | Pattern |
|-----------|---------|
| Acceleration/gravity | `+= value * delta_secs` |
| Deceleration/friction | `*= value.powf(delta_secs)` |
| Timer countdown | `-= delta_secs` |
| Accumulator | `+= delta_secs` |

**What does NOT need time scaling:**
- One-time impulses (jump, bounce, collision response)
- State changes (pickup, throw)
- Discrete events

---

#### 1.3 Collision Epsilon for Ground Contact

**Rule:** Entities resting on surfaces must overlap by a small epsilon.

```rust
// WRONG - zero overlap, detection fails
transform.y = platform_top + entity_half_height;

// CORRECT - ensures overlap detected next frame
const COLLISION_EPSILON: f32 = 0.001;
transform.y = platform_top + entity_half_height - COLLISION_EPSILON;
```

**Why:** With exact positioning, the overlap is zero. Collision detection checks for overlap, so zero overlap = no collision detected = entity falls next frame.

**Symptoms of violation:**
- Grounded state flickers
- Ball "falls through" ground detection
- Entity floats slightly above surface

---

#### 1.4 Spiral of Death Prevention

**Rule:** Cap maximum physics steps per frame.

```rust
// WRONG - can spiral if frame takes too long
while accumulator >= FIXED_DT {
    simulate(FIXED_DT);
    accumulator -= FIXED_DT;
}

// CORRECT - cap prevents spiral
const MAX_STEPS: u32 = 4;
let mut steps = 0;
while accumulator >= FIXED_DT && steps < MAX_STEPS {
    simulate(FIXED_DT);
    accumulator -= FIXED_DT;
    steps += 1;
}
```

**Why:** If a frame takes too long, accumulator grows. Without a cap, catching up requires more simulation, which takes more time, which increases accumulator further.

---

### 2. Input Handling

#### 2.1 Input Buffering Pattern

**Rule:** Buffer `just_pressed()` inputs for FixedUpdate consumption.

```rust
// Resource holds buffered input
#[derive(Resource, Default)]
pub struct PlayerInput {
    pub jump_pressed: bool,      // Buffered press
    pub movement: Vec2,          // Continuous (no buffer needed)
}

// Update: ACCUMULATE, never set false
fn capture_input(mut input: ResMut<PlayerInput>, keyboard: Res<ButtonInput<KeyCode>>) {
    if keyboard.just_pressed(KeyCode::Space) {
        input.jump_pressed = true;  // Only set true
    }
    input.movement.x = /* read axis */;  // Continuous can overwrite
}

// FixedUpdate: CONSUME after reading
fn apply_input(mut input: ResMut<PlayerInput>) {
    if input.jump_pressed {
        input.jump_pressed = false;  // Consume immediately
        // ... apply jump
    }
}
```

**Why:** `just_pressed()` returns true for only one Update frame. If FixedUpdate doesn't run that exact frame, the input is lost. Buffering holds the input until consumed.

**Common mistake:** Setting buffer to false in Update instead of FixedUpdate.

---

#### 2.2 Jump Buffering (Coyote Time)

**Rule:** Allow jumps slightly after leaving ground (coyote time) and slightly before landing (jump buffer).

```rust
#[derive(Component)]
pub struct JumpBuffer {
    pub coyote_timer: f32,    // Time since last grounded
    pub buffer_timer: f32,    // Time since jump pressed
}

fn jump_system(mut query: Query<(&mut JumpBuffer, &Grounded, &mut Velocity)>) {
    for (mut jump, grounded, mut vel) in &mut query {
        // Coyote: can jump shortly after leaving ground
        if grounded.0 {
            jump.coyote_timer = COYOTE_TIME;
        } else {
            jump.coyote_timer -= time.delta_secs();
        }

        // Buffer: remember jump intent
        if input.jump_pressed {
            jump.buffer_timer = JUMP_BUFFER_TIME;
        } else {
            jump.buffer_timer -= time.delta_secs();
        }

        // Execute jump if either timer valid
        let can_jump = grounded.0 || jump.coyote_timer > 0.0;
        let wants_jump = jump.buffer_timer > 0.0;

        if can_jump && wants_jump {
            vel.y = JUMP_VELOCITY;
            jump.coyote_timer = 0.0;
            jump.buffer_timer = 0.0;
        }
    }
}
```

---

### 3. ECS Queries

#### 3.1 Query Component Selection

**Rule:** Fetch only what you need, filter by what you don't.

```rust
// WRONG - fetches unused components
fn system(query: Query<(&Transform, &Velocity, &Health, &Name)>) {
    for (transform, _, _, _) in &query {
        // Only uses transform
    }
}

// CORRECT - filter instead of fetch
fn system(query: Query<&Transform, (With<Velocity>, With<Health>)>) {
    for transform in &query {
        // Efficient: only fetches Transform
    }
}
```

**Why:** Each fetched component increases memory bandwidth. Filters narrow the archetype set without loading data.

---

#### 3.2 Avoid Option<&T> Overuse

**Rule:** Use `With<T>`/`Without<T>` filters instead of `Option<&T>` when you're checking presence, not reading data.

```rust
// WRONG - broadens archetype matching
fn system(query: Query<(Option<&Shield>, Option<&Health>)>) {
    for (shield, health) in &query {
        if shield.is_some() { /* ... */ }
    }
}

// CORRECT - narrows to specific archetypes
fn shielded(query: Query<&Health, With<Shield>>) { }
fn unshielded(query: Query<&Health, Without<Shield>>) { }
```

**Why:** Queries with only `Option<T>` can match ALL entities in the world, iterating far more than intended.

---

#### 3.3 Change Detection False Positives

**Rule:** Only request `&mut` when you actually mutate.

```rust
// WRONG - marks all as changed via DerefMut
fn system(mut query: Query<&mut Transform>) {
    for mut transform in &mut query {
        if some_condition {
            transform.translation.x += 1.0;
        }
        // Even entities not modified are marked Changed
    }
}

// CORRECT - only access mut when needed
fn system(mut query: Query<&mut Transform>) {
    for mut transform in &mut query {
        if some_condition {
            transform.translation.x += 1.0;
            // Only this entity marked Changed
        }
    }
}
```

**Why:** `Changed<T>` triggers on `DerefMut`, not actual value changes. Accessing `&mut` even without writing marks it changed.

---

#### 3.4 Avoid O(n^2) Collision Loops

**Rule:** Use spatial partitioning for collision between large entity sets.

```rust
// PROBLEMATIC - O(n*m) for n entities and m platforms
for entity in entity_query.iter() {
    for platform in platform_query.iter() {
        if collides(entity, platform) { /* ... */ }
    }
}

// BETTER - spatial partitioning
// Only check entities in same/adjacent grid cells
let cell = spatial_grid.get_cell(entity_pos);
for platform in spatial_grid.get_nearby(cell) {
    if collides(entity, platform) { /* ... */ }
}
```

**When this matters:**
- Acceptable: 2 balls × 20 platforms = 40 iterations
- Problematic: 100 enemies × 100 bullets = 10,000 iterations

---

### 4. Memory & Allocations

#### 4.1 Pre-allocate and Reuse Collections

**Rule:** Don't allocate in hot paths. Clear and reuse.

```rust
// WRONG - allocates every frame
fn system() {
    let nearby: Vec<Entity> = query.iter()
        .filter(|e| in_range(e))
        .collect();
    process(nearby);
}

// CORRECT - reuse buffer
#[derive(Resource)]
struct ScratchBuffers {
    nearby: Vec<Entity>,
}

fn system(mut buffers: ResMut<ScratchBuffers>) {
    buffers.nearby.clear();
    for e in query.iter() {
        if in_range(e) {
            buffers.nearby.push(e);
        }
    }
    process(&buffers.nearby);
}
```

---

#### 4.2 String Allocation Hotspots

**Rule:** Avoid `to_string()`, `format!()`, `String::from()` in per-frame code.

```rust
// WRONG - allocates string every frame
fn update_debug(mut text: Query<&mut Text>) {
    text.single_mut().0 = format!("Score: {}", score);
}

// BETTER - only update when changed
fn update_debug(score: Res<Score>, mut text: Query<&mut Text>) {
    if score.is_changed() {
        text.single_mut().0 = format!("Score: {}", score.0);
    }
}

// BEST - pre-format where possible, use integers
```

**Common culprits:**
- Debug text updated every frame
- Logging in hot paths
- String keys for lookups

---

#### 4.3 Asset Handle Reuse

**Rule:** Clone handles, don't re-add assets.

```rust
// WRONG - creates duplicate asset
for _ in 0..100 {
    let mesh = meshes.add(Mesh::from(shape::Quad::default()));
    commands.spawn(Mesh2d(mesh));
}

// CORRECT - clone handle, reuse asset
let mesh = meshes.add(Mesh::from(shape::Quad::default()));
for _ in 0..100 {
    commands.spawn(Mesh2d(mesh.clone()));
}
```

**Why:** `Assets::add()` creates a new copy. Handles are cheap to clone.

---

### 5. System Organization

#### 5.1 System Ordering

**Rule:** Explicitly order dependent systems.

```rust
// WRONG - ambiguous order
app.add_systems(Update, (update_velocity, apply_velocity));

// CORRECT - explicit dependency
app.add_systems(Update, (
    update_velocity,
    apply_velocity.after(update_velocity),
));

// OR use chain for strict sequence
app.add_systems(Update, (
    capture_input,
    process_input,
    apply_movement,
).chain());
```

**When to use each:**
- `.after()`/`.before()`: Specific dependencies between systems
- `.chain()`: Strict sequential execution for a group
- No ordering: Independent systems (can parallelize)

---

#### 5.2 GlobalTransform Timing

**Rule:** Read GlobalTransform after TransformPropagate, or use TransformHelper.

```rust
// WRONG - GlobalTransform may be stale
app.add_systems(Update, my_system);  // Runs before propagation

// CORRECT - schedule after propagation
app.add_systems(PostUpdate,
    my_system.after(TransformSystem::TransformPropagate)
);

// OR use TransformHelper for immediate calculation
fn my_system(transform_helper: TransformHelper, query: Query<Entity>) {
    let global = transform_helper.compute_global_transform(entity);
}
```

**Why:** GlobalTransform updates in PostUpdate. Reading it in Update gives last frame's value.

---

### 6. Component & Resource Design

#### 6.1 Component vs Resource

**Rule:** Per-entity data = Component. Global singleton = Resource.

```rust
// WRONG - entity data in Resource
#[derive(Resource)]
struct EntityHealth(HashMap<Entity, f32>);

// CORRECT - use Component
#[derive(Component)]
struct Health(f32);

// WRONG - singleton as entity
#[derive(Component)]
struct GameSettings { /* ... */ }
fn system(query: Query<&GameSettings>) {
    let settings = query.single();
}

// CORRECT - use Resource
#[derive(Resource)]
struct GameSettings { /* ... */ }
fn system(settings: Res<GameSettings>) { }
```

**Why:** Components get ECS benefits (parallelism, cache efficiency, automatic cleanup). Resources have O(1) access.

---

#### 6.2 Avoid Frequent Component Add/Remove

**Rule:** Prefer flag fields over adding/removing components.

```rust
// PROBLEMATIC - causes archetype moves
if should_glow {
    commands.entity(e).insert(Glowing);
} else {
    commands.entity(e).remove::<Glowing>();
}

// BETTER - flag in existing component
#[derive(Component)]
struct Visual {
    glowing: bool,
}

visual.glowing = should_glow;
```

**Why:** Adding/removing components moves the entity between archetypes, which is expensive.

**Exception:** Use `#[component(storage = "SparseSet")]` for components that legitimately toggle frequently.

---

## Performance Profiling Guide

### When to Profile

Profile when you observe:
- Frame time > 16.6ms (below 60 FPS)
- Inconsistent frame pacing (stutters)
- Memory usage growing over time
- Performance degrading with more entities

### Profiling Tools

#### Tracy Profiler (Recommended)

```bash
# Enable tracy feature
cargo run --release --features bevy/trace_tracy
```

- Real-time frame breakdown
- System timing per frame
- Memory allocation tracking
- Requires Tracy profiler application

#### Chrome Tracing

```bash
cargo run --release --features bevy/trace_chrome
```

- Generates JSON trace file
- Open in chrome://tracing
- Good for one-off analysis

#### Built-in Frame Diagnostics

```rust
// Add to app
app.add_plugins(FrameTimeDiagnosticsPlugin)
   .add_plugins(LogDiagnosticsPlugin::default());
```

- Logs FPS and frame time to console
- Low overhead, always-on option

### What to Look For

| Symptom | Likely Cause | Check |
|---------|--------------|-------|
| Consistent low FPS | Expensive system | Tracy system timings |
| Periodic stutters | GC/allocations | Memory allocation view |
| FPS drops with entities | O(n^2) algorithm | Query iteration counts |
| Long ApplyDeferred | Many commands | Batch spawns, reduce commands |
| High GPU wait | Draw call bound | Sprite batching, atlases |

### Profiling Checklist

1. **Always profile in release mode** - Debug is 10-100x slower
2. **Profile representative scenarios** - Menu, gameplay, worst-case entity count
3. **Measure before optimizing** - Don't guess at bottlenecks
4. **Profile after changes** - Verify improvement

### Release Build Configuration

```toml
# Cargo.toml - optimize dependencies even in debug
[profile.dev.package."*"]
opt-level = 3

# Full release optimization
[profile.release]
lto = "thin"
codegen-units = 1
```

---

## Project-Specific Notes

### Patterns Established in This Codebase

These patterns are already documented in CLAUDE.md and must be maintained:

1. **Input buffering** - All `just_pressed()` buffered in `PlayerInput`
2. **Physics time-scaling** - All physics uses `time.delta_secs()` or `.powf()`
3. **Collision epsilon** - All ground contacts use `- COLLISION_EPSILON`
4. **Constants in constants.rs** - No magic numbers in code
5. **Child entities for attachments** - Gauges, indicators as player children

### Known Scaling Concerns

Monitor these as the game grows:

| Area | Current State | Watch For |
|------|---------------|-----------|
| Collision loops | O(balls × platforms) ~40 | Adding more physics objects |
| String allocations | ~164 to_string() calls | Ball style cycling frequency |
| RNG instantiation | 23 thread_rng() calls | Adding particle systems |
| HashMap lookups | String keys for styles | Per-frame texture changes |

### Codebase Quality Scores

Last audit scores for reference:

| Category | Score |
|----------|-------|
| Physics Correctness | A+ |
| Input Handling | A+ |
| System Organization | A |
| Component Design | A |
| Query Patterns | A |
| Constants Management | A+ |

---

## Review Workflow

### For Every PR/Change

Run through Quick Reference Checklists at top of document.

### For Physics Changes

1. Verify FixedUpdate placement
2. Check time scaling on all velocity/timer changes
3. Test at different frame rates if possible
4. Verify collision epsilon if adding ground-contact entities

### For New Input Actions

1. Add buffer field to PlayerInput
2. Accumulate in Update (never set false)
3. Consume in FixedUpdate (set false after read)
4. Document in CLAUDE.md input section

### For New Systems

1. Determine Update vs FixedUpdate placement
2. Add ordering constraints if dependent on other systems
3. Add to documented system chain in CLAUDE.md
4. Profile impact with many entities

### For New Components

1. Ensure per-entity data (not global)
2. Keep component small and focused
3. Consider SparseSet if frequently added/removed
4. Update CLAUDE.md ECS Structure section

---

## 7. AI Systems

### 7.1 Goal-Based AI Review

When reviewing AI decision code, check:

- [ ] Each goal has clear entry conditions (when to switch TO this goal)
- [ ] Each goal has clear exit conditions (when to switch AWAY)
- [ ] No goals with overlapping conditions (causes flickering)
- [ ] Hysteresis present (stay in goal slightly longer than entry threshold)
- [ ] Goal transitions logged for debugging

**Hysteresis Example:**
```rust
// BAD - flickers when distance hovers around 100
if distance < 100.0 { goal = Chase; }

// GOOD - hysteresis prevents flickering
if goal != Chase && distance < 100.0 { goal = Chase; }
if goal == Chase && distance > 120.0 { goal = Patrol; }  // 20% buffer
```

### 7.2 AI Debugging Checklist

- [ ] Add `AiGoalChanged` event for logging transitions
- [ ] Each AI profile tested in isolation (simulation mode)
- [ ] Goal time distribution checked (no goal dominates >80%)
- [ ] Edge cases tested (corner positions, empty ball, multiple steals)

### 7.3 State Machine vs Behavior Tree Decision

| Use State Machine When | Use Behavior Tree When |
|------------------------|------------------------|
| < 15 distinct states | > 20 states |
| Reactive gameplay (sports) | Complex sequences |
| Performance critical | Rich behavior needed |
| Simple transitions | Parallel behaviors |

---

## 8. Game Feel & Balance

### 8.1 Juice Checklist

For each game event, verify feedback exists:

| Event | Visual | Audio | Haptic |
|-------|--------|-------|--------|
| Score | Flash, particles | Sound | Rumble (optional) |
| Steal success | Color change | Sound | - |
| Steal fail | Red flash | Fail sound | - |
| Charge complete | Gauge full + glow | Ding | - |
| Pickup ball | Pulse stops | Pickup sound | - |

### 8.2 Balance Testing Workflow

```bash
# Run tournament to check AI profile balance
cargo run --bin simulate -- --tournament 5 --parallel 8

# Target: All profiles between 40-60% win rate
# Red flag: Any profile >65% or <35%

# Check shot success rates
cargo run --bin simulate -- --shot-test 30 --level 3
# Target: 40-60% over/under ratio

# Generate heatmap for scoring positions
cargo run --bin heatmap -- score
```

### 8.3 Movement Feel Tuning

| Parameter | Snappy Feel | Floaty Feel | Realistic |
|-----------|-------------|-------------|-----------|
| Ground Accel | 3000+ | 1200 | 2000 |
| Ground Decel | 2500+ | 800 | 1500 |
| Air Control | 80%+ of ground | 30% of ground | 50% |
| Gravity Rise | 800 | 600 | 980 |
| Gravity Fall | 1600 | 900 | 980 |

---

## 9. Determinism & Multiplayer Prep

### 9.1 Determinism Requirements

For future netcode (rollback/lockstep), verify:

- [ ] All RNG calls use seeded source (not `thread_rng()`)
- [ ] Physics uses fixed timestep (FixedUpdate)
- [ ] No floating point order-dependence
- [ ] Input processed in deterministic order

### 9.2 RNG Consolidation Pattern

```rust
// AVOID - non-deterministic
let angle = rand::thread_rng().gen_range(-0.1..0.1);

// PREFER - seeded resource
#[derive(Resource)]
pub struct GameRng(pub rand::rngs::StdRng);

fn system(mut rng: ResMut<GameRng>) {
    let angle = rng.0.gen_range(-0.1..0.1);
}
```

---

## References

### Bevy Resources
- [Unofficial Bevy Cheat Book - Performance](https://bevy-cheatbook.github.io/setup/perf.html)
- [Bevy ECS Best Practices](https://bevy-cheatbook.github.io/programming/ecs-intro.html)
- [Bevy Best Practices GitHub](https://github.com/tbillington/bevy_best_practices)

### Game Development
- [Fix Your Timestep - Gaffer on Games](https://gafferongames.com/post/fix_your_timestep/)
- [Game Programming Patterns](https://gameprogrammingpatterns.com/)
- [ECS FAQ](https://github.com/SanderMertens/ecs-faq)

### AI Design
- [FSM vs Behavior Tree](https://medium.com/@abdullahahmetaskin/finite-state-machine-and-behavior-tree-fusion-3fcce33566)
- [Behavior Trees Survey](https://www.sciencedirect.com/science/article/pii/S0921889022000513)

### Game Design
- [Sports Game Design](https://gamedesignskills.com/game-design/sports/)
- [Arcade Game Design](https://gamedesignskills.com/game-design/arcade/)

### Anti-Patterns
- [Game-Specific Anti-Patterns Catalog](https://www.researchgate.net/publication/342408679_A_Catalogue_of_Game-Specific_Anti-Patterns)
- [ECS Design Decisions](https://arielcoppes.dev/2023/07/13/design-decisions-when-building-games-using-ecs.html)

### This Project
- `CLAUDE.md` - Architecture and patterns
- `code_review_audits.md` - Previous audit results
- `code_review_2026-01-25.md` - Deep analysis with resources
- `audit_record.md` - Change history
