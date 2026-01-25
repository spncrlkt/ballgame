# Simulation Code Consolidation Plan

## Overview

Consolidate simulation infrastructure across binaries, add parallel execution, and implement SQLite for results persistence.

**Current state:** Physics is already centralized. Issues are duplicated app setup, event emission, and scattered file-based results.

---

## Research Findings & Best Practices

### Bevy Headless Mode
- Use `MinimalPlugins` or configure `RenderPlugin` with `backends: None` for headless execution
- Set `synchronous_pipeline_compilation: true` for deterministic test behavior
- Task pool configuration matters - set sizes to 1 for minimal threading overhead
- Reference: [Tainted Coders - Headless Mode](https://taintedcoders.com/bevy/how-to/headless-mode)

### Parallel Bevy Apps - Critical Concern
Each Bevy App spawns multiple threads. Running many parallel apps can hit OS thread limits.
- **Solution 1:** Use `App::update()` for manual stepping instead of `App::run()`
- **Solution 2:** Set all task pool sizes to 1 to minimize threading
- **Solution 3:** Consider fewer threads, each owning multiple Apps
- Reference: [Bevy Discussion #5580](https://github.com/bevyengine/bevy/discussions/5580)

### Rayon + Bevy
Bevy replaced Rayon with `bevy_tasks` for internal parallelism. Using Rayon externally (to run multiple apps) is fine, but must manage thread counts carefully.
- Configure Rayon thread pool size + Bevy task pools to stay under OS limits
- `rayon::ThreadPoolBuilder::new().num_threads(n).build_global()`
- Reference: [Red Hat - Rayon Data Parallelism](https://developers.redhat.com/blog/2021/04/30/how-rust-makes-rayons-data-parallelism-magical)

### Deterministic Simulation
- Use FixedUpdate schedule for all physics/game logic
- Fixed timestep clock follows `Time<Virtual>` - can pause/control for testing
- Seed RNG with match seed for reproducible results
- Reference: [Bevy Cheat Book - Fixed Timestep](https://bevy-cheatbook.github.io/fundamentals/fixed-timestep.html)

### Event Sourcing for Replay
- Events are append-only, timestamped, sequentially ordered
- Support "complete rebuild" by replaying all events
- Consider snapshots every N events for faster replay startup
- Reference: [Martin Fowler - Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html)

### SQLite Concurrency
- Default: one writer at a time, multiple readers
- WAL mode: concurrent reads even during writes
- Use transactions for rollback capability on errors
- Reference: [rusqlite docs](https://docs.rs/rusqlite/latest/rusqlite/)

---

## Phase 1: Headless App Builder

**Goal:** Extract common Bevy app setup into a reusable builder.

### Files to create:
- `src/simulation/app_builder.rs` - New builder module

### Pattern:
```rust
pub struct HeadlessAppBuilder {
    level: u32,
    seed: u64,
    left_profile: String,
    right_profile: String,
    event_logging: Option<PathBuf>,
    metrics: bool,
    minimal_threads: bool,  // For parallel execution
}

impl HeadlessAppBuilder {
    pub fn new(level: u32) -> Self;
    pub fn with_seed(self, seed: u64) -> Self;
    pub fn with_profiles(self, left: &str, right: &str) -> Self;
    pub fn with_event_logging(self, dir: PathBuf) -> Self;
    pub fn with_metrics(self) -> Self;
    pub fn with_minimal_threads(self) -> Self;  // Task pools = 1
    pub fn build(self) -> App;
}

// Reusable system chains
pub fn physics_system_chain() -> impl IntoSystemConfigs;
pub fn ai_system_chain() -> impl IntoSystemConfigs;
```

### Task Pool Configuration (for parallel execution):
```rust
// Minimize thread spawning when running many apps
use bevy::core::TaskPoolOptions;

app.insert_resource(TaskPoolOptions {
    min_total_threads: 1,
    max_total_threads: 1,
    ..default()
});
```

### Files to modify:
- `src/simulation/runner.rs` - Use builder instead of manual setup (lines 70-220)
- `src/testing/runner.rs` - Use builder (lines 119-189)
- `src/simulation/mod.rs` - Export new module

---

## Phase 2: Event Emission Consolidation

**Goal:** Unify duplicated 200+ line event emission systems.

### Current duplication:
- `src/simulation/runner.rs:626-851` - `emit_simulation_events`
- `src/bin/training.rs:853-1071` - `emit_training_events`

### Files to create:
- `src/events/emitter.rs` - Shared emission logic

### Pattern:
```rust
pub struct EventEmitterState {
    pub prev_score: [u32; 2],
    pub prev_ball_holder: Option<Entity>,
    pub prev_charging: [bool; 2],
    pub prev_ai_goals: [Option<String>; 2],
    pub prev_steal_cooldowns: [f32; 2],
    pub last_tick_time: f32,
    pub tick_frame_count: u64,
}

pub fn emit_game_events(
    state: &mut EventEmitterState,
    buffer: &mut EventBuffer,
    elapsed: f32,
    // ... query params
);
```

### Files to modify:
- `src/simulation/runner.rs` - Replace inline emission with shared function
- `src/bin/training.rs` - Replace inline emission with shared function
- `src/events/mod.rs` - Export emitter

---

## Phase 3: Runner Modularization

**Goal:** Break up 1662-line `runner.rs` into focused modules.

### New file structure:
```
src/simulation/
├── mod.rs              # Public API
├── app_builder.rs      # Phase 1
├── runner.rs           # Slimmed: run_match() only (~200 lines)
├── setup.rs            # Entity spawning (extract from runner)
├── control.rs          # SimControl, SimEventBuffer resources
├── modes.rs            # Tournament, LevelSweep, MultiMatch logic
├── shot_test.rs        # Shot accuracy testing (~300 lines)
├── parallel.rs         # Phase 4
├── db.rs               # Phase 5
├── config.rs           # Existing
└── metrics.rs          # Existing
```

### Key extractions from runner.rs:
| Lines | Content | New Location |
|-------|---------|--------------|
| 37-67 | SimControl, SimEventBuffer | `control.rs` |
| 277-486 | sim_setup() | `setup.rs` (use world.rs helpers) |
| 888-1115 | Mode orchestration | `modes.rs` |
| 1196-1662 | Shot test code | `shot_test.rs` |

### Fix: Use existing world.rs helpers
`sim_setup` should call `spawn_floor()`, `spawn_walls()`, `spawn_baskets()` from `src/world/mod.rs` instead of duplicating.

---

## Phase 4: Parallel Simulation

**Goal:** Run multiple simulations concurrently using Rayon.

### Critical: Thread Management

Each Bevy App spawns internal threads. Running N parallel apps naively creates N × M threads (where M = Bevy's default pool size). This can hit OS limits (Linux default ~1024).

**Solution:** Use `with_minimal_threads()` from Phase 1 + configure Rayon conservatively.

### Files to create:
- `src/simulation/parallel.rs`

### Dependencies to add:
```toml
rayon = "1.10"
```

### Pattern:
```rust
use rayon::prelude::*;

pub struct ParallelRunner {
    thread_count: usize,
}

impl ParallelRunner {
    pub fn new(threads: usize) -> Self {
        // Configure Rayon global pool ONCE at startup
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .expect("Failed to build Rayon thread pool");
        Self { thread_count: threads }
    }

    pub fn run_batch(&self, configs: Vec<MatchConfig>) -> Vec<MatchResult> {
        configs.into_par_iter()
            .map(|config| {
                // CRITICAL: minimal_threads reduces Bevy's internal threading
                let app = HeadlessAppBuilder::new(config.level)
                    .with_seed(config.seed)
                    .with_profiles(&config.left, &config.right)
                    .with_minimal_threads()  // Task pools = 1
                    .build();
                run_match_with_app(app, &config)
            })
            .collect()
    }
}

fn run_match_with_app(mut app: App, config: &MatchConfig) -> MatchResult {
    // Use App::update() for manual stepping (not App::run())
    for _ in 0..config.max_frames {
        app.update();
        if match_complete(&app) { break; }
    }
    extract_result(&app)
}
```

### Thread Budget Example:
| Rayon threads | Bevy pools per app | Total threads |
|---------------|-------------------|---------------|
| 8 | 4 (default) | 8 × 4 = 32 (too many) |
| 8 | 1 (minimal) | 8 × 1 = 8 |
| 16 | 1 (minimal) | 16 × 1 = 16 |

### CLI extension:
```bash
cargo run --bin simulate -- --tournament 5 --parallel 8
```

---

## Phase 5: SQLite Database

**Goal:** Persistent queryable storage for simulation results.

### Dependencies to add:
```toml
rusqlite = { version = "0.31", features = ["bundled"] }
```

### Concurrency Configuration:
```rust
// Enable WAL mode for concurrent reads during writes
conn.execute_batch("PRAGMA journal_mode=WAL;")?;

// Optional: busy timeout for parallel writers
conn.busy_timeout(std::time::Duration::from_secs(5))?;
```

### Files to create:
- `src/simulation/db.rs` - Database wrapper
- `src/bin/migrate_to_db.rs` - Migration tool for existing logs

### Schema:
```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    created_at TEXT,
    session_type TEXT,
    config_json TEXT
);

CREATE TABLE matches (
    id INTEGER PRIMARY KEY,
    session_id TEXT REFERENCES sessions(id),
    seed INTEGER,
    level INTEGER,
    level_name TEXT,
    left_profile TEXT,
    right_profile TEXT,
    score_left INTEGER,
    score_right INTEGER,
    duration_secs REAL,
    winner TEXT
);

CREATE TABLE player_stats (
    id INTEGER PRIMARY KEY,
    match_id INTEGER REFERENCES matches(id),
    side TEXT,
    goals INTEGER,
    shots_attempted INTEGER,
    shots_made INTEGER,
    steals_attempted INTEGER,
    steals_successful INTEGER,
    possession_time REAL,
    distance_traveled REAL
);

CREATE TABLE events (
    id INTEGER PRIMARY KEY,
    match_id INTEGER REFERENCES matches(id),
    timestamp_ms INTEGER,
    event_type TEXT,
    event_json TEXT
);
```

### API:
```rust
pub struct SimDatabase {
    conn: Connection,
}

impl SimDatabase {
    pub fn open(path: &Path) -> Result<Self>;
    pub fn insert_match(&self, session: &str, result: &MatchResult) -> Result<i64>;
    pub fn get_profile_stats(&self, profile: &str) -> Result<ProfileStats>;
    pub fn query_matches(&self, filter: MatchFilter) -> Result<Vec<MatchSummary>>;
}
```

---

## Phase 6: Shot Test Refactor

**Goal:** Fix anti-pattern of creating mini-apps per shot.

### Current problem (runner.rs:1355-1475):
Each shot creates a new Bevy App - wasteful and slow.

### Solution:
Reuse single app, reset world state between shots:

```rust
pub struct ShotTestRunner {
    app: App,
    player_entity: Entity,
    ball_entity: Entity,
}

impl ShotTestRunner {
    pub fn new(level: u32) -> Self;
    pub fn test_shot(&mut self, x_position: f32) -> ShotOutcome {
        self.reset_entities();
        self.position_player(x_position);
        self.run_shot_sequence()
    }
}
```

---

## Phase 7: Unified Evlog Parser

**Goal:** Consolidate duplicated .evlog parsing between replay and analytics systems.

### Event Sourcing Principles Applied:
- **Append-only log:** .evlog is immutable, chronologically ordered
- **Complete rebuild:** Replay reconstructs full game state from events
- **Temporal query:** Analytics queries specific event types across timeline
- **Single source of truth:** One parser, multiple consumers

### Current duplication:
- `src/replay/loader.rs` - Parses .evlog for playback (tick events, positions, velocities)
- `src/analytics/parser.rs` - Parses .evlog for metrics (goals, steals, possession changes)

Both traverse the same file format independently, duplicating ~40-50% of parsing logic.

### Files to create:
- `src/events/evlog_parser.rs` - Unified parser module

### Pattern:
```rust
pub struct EvlogParser {
    events: Vec<ParsedEvent>,
    metadata: MatchMetadata,
}

pub struct ParsedEvent {
    pub timestamp_ms: u64,
    pub event_type: EventType,
    pub data: EventData,
}

pub enum EventData {
    Tick(TickData),           // For replay: positions, velocities
    Goal(GoalData),           // For analytics: scorer, team
    Steal(StealData),         // For analytics: attacker, victim
    Shot(ShotData),           // For both: shooter, outcome
    // ... other event types
}

impl EvlogParser {
    pub fn parse(path: &Path) -> Result<Self>;

    // Replay-focused access
    pub fn iter_ticks(&self) -> impl Iterator<Item = &TickData>;
    pub fn get_tick_at(&self, time_ms: u64) -> Option<&TickData>;

    // Analytics-focused access
    pub fn iter_goals(&self) -> impl Iterator<Item = &GoalData>;
    pub fn iter_steals(&self) -> impl Iterator<Item = &StealData>;
    pub fn compute_possession_timeline(&self) -> PossessionTimeline;
}
```

### Files to modify:
- `src/replay/loader.rs` - Use unified parser instead of custom parsing
- `src/analytics/parser.rs` - Use unified parser instead of custom parsing
- `src/events/mod.rs` - Export parser module

---

## Phase 8: Analytics + SQLite Integration

**Goal:** Connect analytics system to SQLite for historical queries across sessions.

### Current state:
- Analytics reads individual .evlog files
- No cross-session analysis capability
- Results computed on-demand, not persisted

### Enhancement:
Allow analytics to query aggregated data from SQLite database (populated by Phase 5).

### Files to modify:
- `src/analytics/mod.rs` - Add database query option

### New capabilities:
```rust
// Existing: analyze single match
pub fn analyze_match(evlog_path: &Path) -> MatchAnalysis;

// New: analyze from database
pub fn analyze_profile(db: &SimDatabase, profile: &str) -> ProfileAnalysis {
    // Query all matches for profile
    // Compute aggregate stats: win rate, avg goals, shot accuracy
}

pub fn compare_profiles(db: &SimDatabase, profiles: &[&str]) -> ComparisonReport {
    // Side-by-side profile comparison
    // Identify strengths/weaknesses
}

pub fn trend_analysis(db: &SimDatabase, profile: &str, days: u32) -> TrendReport {
    // Performance over time
    // Identify improvement/regression
}
```

### CLI extension:
```bash
# Existing: single match analysis
cargo run --bin analyze -- training_logs/session_xyz/game_1.evlog

# New: database queries
cargo run --bin analyze -- --db sim_results.db --profile Aggressive
cargo run --bin analyze -- --db sim_results.db --compare Aggressive,Passive,Balanced
cargo run --bin analyze -- --db sim_results.db --trend Aggressive --days 7
```

---

## Effort Estimates

| Phase | Description | Effort | Notes |
|-------|-------------|--------|-------|
| 2 | Event emission consolidation | 1-2 hrs | Low risk, moving existing code |
| 1 | Headless App Builder | 2-3 hrs | New pattern, extract setup logic |
| 3 | Runner modularization | 3-4 hrs | Large file split, mechanical |
| 6 | Shot test refactor | 1-2 hrs | Fix anti-pattern |
| 4 | Parallel simulation | 2-3 hrs | New feature, thread management |
| 5 | SQLite database | 4-6 hrs | New dependency, schema, API |
| 7 | Unified evlog parser | 2-3 hrs | Consolidate two parsers |
| 8 | Analytics + SQLite | 2-3 hrs | Extend analytics with DB queries |

**Total: ~18-26 hours of focused work**

### Suggested Sprint Groupings:
- **Sprint 1** (5-7 hrs): Phases 2, 1, 6 — Core consolidation, no new deps
- **Sprint 2** (3-4 hrs): Phase 3 — Runner modularization cleanup
- **Sprint 3** (6-9 hrs): Phases 4, 5 — Parallel + SQLite (major features)
- **Sprint 4** (4-6 hrs): Phases 7, 8 — Evlog + Analytics integration

---

## Implementation Order

1. **Phase 2 first** - Event emission consolidation (lowest risk, high dedup value)
2. **Phase 1** - App builder (enables phases 4 and 6)
3. **Phase 3** - Runner modularization (cleanup)
4. **Phase 6** - Shot test refactor (uses app builder)
5. **Phase 4** - Parallel simulation (uses app builder)
6. **Phase 5** - SQLite (can be done independently)
7. **Phase 7** - Unified evlog parser (after Phase 5, before Phase 8)
8. **Phase 8** - Analytics + SQLite integration (depends on Phase 5 and 7)

---

## Verification

### After Phase 1-3:
```bash
cargo run --bin simulate -- --level 3  # Single match still works
cargo run --bin test-scenarios         # Tests still pass
cargo run --bin training               # Training mode works
```

### After Phase 4:
```bash
cargo run --bin simulate -- --tournament 5 --parallel 4
# Should complete ~4x faster than sequential
```

### After Phase 5:
```bash
cargo run --bin simulate -- --tournament 5 --db sim_results.db
sqlite3 sim_results.db "SELECT left_profile, COUNT(*), AVG(score_left) FROM matches GROUP BY left_profile"
```

### After Phase 6:
```bash
cargo run --bin simulate -- --shot-test 100
# Should be noticeably faster than current implementation
```

### After Phase 7:
```bash
# Replay still works (uses unified parser)
cargo run -- --replay training_logs/session_xyz/game_1.evlog

# Analytics still works (uses unified parser)
cargo run --bin analyze -- training_logs/session_xyz/game_1.evlog
```

### After Phase 8:
```bash
# Profile analysis from database
cargo run --bin analyze -- --db sim_results.db --profile Aggressive

# Compare multiple profiles
cargo run --bin analyze -- --db sim_results.db --compare Aggressive,Passive

# Trend analysis
cargo run --bin analyze -- --db sim_results.db --trend Aggressive --days 7
```

---

## Files Summary

### New files:
- `src/simulation/app_builder.rs`
- `src/simulation/control.rs`
- `src/simulation/setup.rs`
- `src/simulation/modes.rs`
- `src/simulation/shot_test.rs`
- `src/simulation/parallel.rs`
- `src/simulation/db.rs`
- `src/events/emitter.rs`
- `src/events/evlog_parser.rs`
- `src/bin/migrate_to_db.rs`

### Modified files:
- `src/simulation/runner.rs` - Slim down significantly
- `src/simulation/mod.rs` - Add exports
- `src/bin/training.rs` - Use shared event emitter
- `src/events/mod.rs` - Add emitter and parser exports
- `src/replay/loader.rs` - Use unified evlog parser
- `src/analytics/parser.rs` - Use unified evlog parser
- `src/analytics/mod.rs` - Add database query functions
- `Cargo.toml` - Add rayon, rusqlite
