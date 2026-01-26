# Plan: SQLite-based Training + Ghost Replay System

## Goal
Combine training sessions with ghost replay. Record player input to SQLite, then replay it as a ghost while controlling the other player. Support recording/replaying either player with rewind capability.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        SQLite Database                           │
│  ┌──────────────┐ ┌──────────────┐ ┌─────────────────────────┐  │
│  │   sessions   │ │   matches    │ │        events           │  │
│  │   (existing) │ │   (extended) │ │   (NEW: raw event log)  │  │
│  └──────────────┘ └──────────────┘ └─────────────────────────┘  │
│                                     ┌─────────────────────────┐  │
│                                     │    input_samples        │  │
│                                     │   (NEW: for ghost)      │  │
│                                     └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                    │                           │
          ┌─────────┴─────────┐       ┌────────┴────────┐
          │   Training Mode   │       │   Ghost Mode    │
          │   (writes events) │       │  (reads inputs) │
          └───────────────────┘       └─────────────────┘
```

## Database Schema Changes

### New Tables in `src/simulation/db.rs`

```sql
-- Store raw events for each match (for analysis/replay)
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY,
    match_id INTEGER REFERENCES matches(id) ON DELETE CASCADE,
    time_ms INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    event_data TEXT NOT NULL,  -- JSON or compact format
    UNIQUE(match_id, time_ms, event_type, event_data)
);
CREATE INDEX IF NOT EXISTS idx_events_match ON events(match_id);
CREATE INDEX IF NOT EXISTS idx_events_time ON events(match_id, time_ms);

-- Store input samples for ghost replay (derived from events but optimized)
CREATE TABLE IF NOT EXISTS input_samples (
    id INTEGER PRIMARY KEY,
    match_id INTEGER REFERENCES matches(id) ON DELETE CASCADE,
    player TEXT NOT NULL,  -- 'L' or 'R'
    time_ms INTEGER NOT NULL,
    move_x REAL NOT NULL,
    jump INTEGER NOT NULL,    -- 0 or 1
    throw INTEGER NOT NULL,   -- 0 or 1
    pickup INTEGER NOT NULL,  -- 0 or 1
    UNIQUE(match_id, player, time_ms)
);
CREATE INDEX IF NOT EXISTS idx_input_samples_match ON input_samples(match_id, player);
```

### Extended matches table
```sql
-- Add columns to matches table
ALTER TABLE matches ADD COLUMN level_name TEXT;
ALTER TABLE matches ADD COLUMN config_json TEXT;  -- GameConfig snapshot
```

## Key Components

### 1. SQLite Event Logger (`src/events/db_logger.rs` - NEW)

Replace file-based `EventLogger` with SQLite-based logger:

```rust
pub struct DbEventLogger {
    db: SimDatabase,
    session_id: String,
    match_id: Option<i64>,
    start_time: f32,
    input_sample_interval_ms: u32,  // How often to sample input (e.g., 50ms)
    last_input_sample_time: HashMap<PlayerId, f32>,
}

impl DbEventLogger {
    pub fn new(db: SimDatabase) -> Self;
    pub fn start_session(&mut self, session_type: &str) -> String;
    pub fn start_match(&mut self, match_info: &MatchStartInfo) -> i64;
    pub fn log_event(&mut self, time: f32, event: &GameEvent);
    pub fn log_input_sample(&mut self, time: f32, player: PlayerId, input: &InputState);
    pub fn end_match(&mut self, result: &MatchResult);
    pub fn end_session(&mut self);
}
```

### 2. Ghost Loader from DB (`src/simulation/ghost.rs` modifications)

Add function to load ghost data from SQLite:

```rust
pub fn load_ghost_from_db(
    db: &SimDatabase,
    match_id: i64,
    player: PlayerId
) -> Result<GhostTrial, String>;

pub fn list_available_ghosts(db: &SimDatabase) -> Vec<GhostInfo>;
```

### 3. Training Mode Changes (`src/bin/training.rs`)

- Replace `TrainingEventBuffer` with `DbEventLogger`
- Add `--db <path>` option (default: `training.db`)
- Log input samples for both players during play
- At match end, events are already in DB (no file write needed)

### 4. Ghost Training Mode (`src/bin/training.rs` or new binary)

Add new mode: `cargo run --bin training -- --ghost <match_id> [--ghost-player L|R]`

- Load ghost input samples from DB for specified player
- User controls the other player
- Ghost input is applied via modified `ghost_input_system`
- Support rewind by resetting game state and re-applying inputs up to target time

## Implementation Steps

### Step 1: Extend Database Schema
- Add `events` table to `SimDatabase::init_schema()`
- Add `input_samples` table
- Add helper methods for inserting/querying

### Step 2: Create DbEventLogger
- New file: `src/events/db_logger.rs`
- Implements same interface as current `EventLogger`
- Writes directly to SQLite
- Batches writes for performance (transaction per match)

### Step 3: Update Training Binary
- Replace `TrainingEventBuffer` with `DbEventLogger`
- Remove `write_evlog()` file writes
- Add `--db` CLI option
- Ensure input samples captured for both players

### Step 4: Add Ghost Loading from DB
- Modify `src/simulation/ghost.rs`
- Add `load_ghost_from_db()` function
- Add `list_available_ghosts()` for UI

### Step 5: Implement Ghost Training Mode
- Add `--ghost <match_id>` and `--ghost-player <L|R>` CLI args
- Load ghost inputs at startup
- Apply ghost inputs instead of AI for the ghost player
- User controls the other player

### Step 6: Add Rewind Support
- Track game state snapshots at intervals (every 1-2 seconds)
- On rewind: restore snapshot, re-apply inputs from that point
- Keybinds: Comma (,) rewind, Period (.) step forward

### Step 7: Cleanup
- Remove remaining evlog references and unused scripts
- Keep compact text serialization for SQLite event rows
- Update analysis tools to read from DB only
- Ensure training outputs produce no `.evlog` files

## Files to Modify

| File | Changes |
|------|---------|
| `src/simulation/db.rs` | Add events, input_samples tables + queries |
| `src/events/mod.rs` | Export new db_logger module, remove evlog_parser/format |
| `src/events/db_logger.rs` | NEW: SQLite-based event logger |
| `src/events/evlog_parser.rs` | DELETE |
| `src/events/format.rs` | DELETE |
| `src/events/logger.rs` | DELETE (replaced by db_logger) |
| `src/simulation/ghost.rs` | Add `load_ghost_from_db()`, remove file loading |
| `src/bin/training.rs` | Use DbEventLogger, add ghost mode |
| `src/training/settings.rs` | Add `--db`, `--ghost`, `--ghost-player` CLI args |
| `src/training/mod.rs` | Export new types |
| `src/training/analysis.rs` | Update to query DB instead of parsing files |

## CLI Usage Examples

```bash
# Normal training (records to SQLite)
cargo run --bin training --db training.db

# List available ghost recordings
cargo run --bin training -- --list-ghosts --db training.db

# Play against ghost of your own gameplay (you = P2/right, ghost = P1/left)
cargo run --bin training -- --ghost 42 --ghost-player L --db training.db

# Play as P1 while ghost plays P2 (replay AI behavior)
cargo run --bin training -- --ghost 42 --ghost-player R --db training.db
```

## Verification

1. **Recording works**: Run training, check SQLite has events + input_samples
2. **Ghost loads**: Run with `--ghost`, verify ghost player moves correctly
3. **Rewind works**: Press comma to rewind, verify state restored
4. **Analysis works**: Existing analysis tools should work with DB queries
