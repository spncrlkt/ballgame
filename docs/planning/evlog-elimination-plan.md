# Complete Evlog Elimination Plan

## Goal

Remove all `.evlog` file infrastructure and migrate entirely to SQLite for event storage and replay. No evlog files will be created, read, or referenced anywhere in the codebase.

---

## Summary of Changes

| Action | Count |
|--------|-------|
| Files to DELETE | 6 |
| Files to CREATE | 1 |
| Files to MODIFY | 14 |
| Docs to UPDATE | 6+ |

---

## Phase 1: Add SQLite Replay Loading

**Goal:** Enable replay system to load from SQLite database.

### 1.1 Add replay loading to SimDatabase

**File:** `src/simulation/db.rs`

Add methods:
```rust
pub fn load_replay_data(&self, match_id: i64) -> Result<ReplayData>
pub fn find_match_by_session(&self, session_id: &str, game_num: u32) -> Result<Option<i64>>
```

Query tick events (type "T") and game events from `events` table, parse serialized data, return `ReplayData` struct.

### 1.2 Create SQLite replay loader

**File:** `src/replay/sqlite_loader.rs` (NEW)

```rust
pub fn load_replay_from_db(db_path: &Path, match_id: i64) -> Result<ReplayData, String>
```

### 1.3 Update replay module

**File:** `src/replay/mod.rs`

- Add `mod sqlite_loader;`
- Export `load_replay_from_db`
- Update `ReplayMode` to support `match_id: Option<i64>`

---

## Phase 2: Update Main Binary

**Goal:** Support SQLite-based replay via command line.

### 2.1 Add --replay-db flag

**File:** `src/main.rs`

- Add `--replay-db <match_id>` CLI argument
- Keep `--replay <file>` for legacy compatibility (temporary)
- Load from SQLite when `--replay-db` is used

---

## Phase 3: Update Training Binary

**Goal:** Training writes only to SQLite, no evlog files.

### 3.1 Remove evlog writing

**File:** `src/bin/training.rs`

- Remove `TrainingEventBuffer` usage
- Remove `evlog_path_for_game()` calls
- Remove file writing to `training_logs/*/game_*.evlog`
- Keep only `SqliteEventLogger` for event storage
- Store `match_id` in `GameResult` instead of `evlog_path`

### 3.2 Update training state

**File:** `src/training/state.rs`

- Add `match_id: Option<i64>` to `GameResult`
- Remove or deprecate `evlog_path` field

### 3.3 Update session management

**File:** `src/training/session.rs`

- Remove `evlog_path_for_game()` function
- Update `GameSummary` to use `match_id`

---

## Phase 4: Update Analysis

**Goal:** All analysis uses SQLite, not evlog parsing.

### 4.1 Remove evlog-based analysis

**File:** `src/training/analysis.rs`

- Remove `analyze_session()` function (the evlog-based one)
- Rename `analyze_session_from_db()` to `analyze_session()`
- Remove all `parse_evlog` imports and calls
- Update `analyze_pursuit_session()` to use SQLite

### 4.2 Update analytics parser

**File:** `src/analytics/parser.rs`

- Remove evlog dependencies
- Use `SimDatabase` queries instead
- Or mark module for deletion if redundant

---

## Phase 5: Update Other Binaries

### 5.1 Update simulate binary

**File:** `src/bin/simulate.rs`

- Ensure it uses `SqliteEventLogger` (already does via simulation runner)
- Remove any evlog file writing

### 5.2 Update/Remove extract-drives

**File:** `src/bin/extract-drives.rs`

- Update to query SQLite for drive extraction
- Or DELETE if functionality is superseded

### 5.3 Check other binaries

Review and update if needed:
- `src/bin/analyze.rs`
- `src/bin/run-ghost.rs`
- `src/bin/heatmap.rs`

---

## Phase 6: Delete Evlog Infrastructure

### 6.1 Delete evlog files

| File | Reason |
|------|--------|
| `src/events/logger.rs` | EventLogger, EventBuffer no longer needed |
| `src/events/evlog_parser.rs` | No more evlog parsing |
| `scripts/parse_training.py` | Python evlog parser |
| `scripts/analyze_training.py` | Python evlog analyzer |

### 6.2 Update format.rs

**File:** `src/events/format.rs`

- KEEP `serialize_event()` - still used by SQLite logger
- DELETE `parse_event()` - no longer needed

### 6.3 Update events module

**File:** `src/events/mod.rs`

Remove exports:
```rust
// DELETE these lines:
pub mod evlog_parser;
pub use evlog_parser::{...};
pub use logger::{EventBuffer, EventLogConfig, EventLogger};
```

Keep:
```rust
pub use format::serialize_event;
pub use sqlite_logger::{SqliteEventLogger, flush_events_to_sqlite};
pub use types::{ControllerSource, GameConfig, GameEvent, PlayerId};
```

### 6.4 Update lib.rs

**File:** `src/lib.rs`

Remove re-exports of deleted items.

---

## Phase 7: Documentation Cleanup

### 7.1 Delete evlog documentation

**File:** `docs/design/event-format.md` - DELETE

### 7.2 Update documentation files

Update these to remove evlog references:
- `CLAUDE.md` - remove --replay file references, update training output
- `docs/guides/training-workflow.md` - already SQLite-focused, remove evlog mentions
- `docs/guides/TRAINING.md` - update if exists
- `docs/design/functional_spec.md` - update replay/training sections
- `docs/project/milestones.md` - mark evlog elimination complete
- `docs/planning/sqlite-ghost-replay-plan.md` - mark complete or delete

---

## Implementation Order

Execute in this order to avoid breaking the build:

1. **Phase 1** - Add SQLite replay loading (additive)
2. **Phase 2** - Add --replay-db to main (additive)
3. **Phase 3** - Update training binary
4. **Phase 4** - Update analysis
5. **Phase 5** - Update other binaries
6. **Phase 6** - Delete evlog code (only after nothing depends on it)
7. **Phase 7** - Documentation cleanup

---

## Files Summary

### DELETE (6 files)
```
src/events/logger.rs
src/events/evlog_parser.rs
scripts/parse_training.py
scripts/analyze_training.py
docs/design/event-format.md
src/replay/loader.rs (after migration complete)
```

### CREATE (1 file)
```
src/replay/sqlite_loader.rs
```

### MODIFY (14 files)
```
src/simulation/db.rs          - Add replay loading queries
src/replay/mod.rs             - Export sqlite_loader, update ReplayMode
src/main.rs                   - Add --replay-db flag
src/bin/training.rs           - Remove evlog writing
src/training/state.rs         - Add match_id field
src/training/session.rs       - Remove evlog_path_for_game
src/training/analysis.rs      - Remove evlog-based analysis
src/analytics/parser.rs       - Use SQLite or delete
src/bin/simulate.rs           - Verify SQLite-only
src/bin/extract-drives.rs     - Update or delete
src/events/mod.rs             - Remove evlog exports
src/events/format.rs          - Keep serialize_event only
src/lib.rs                    - Update exports
CLAUDE.md                     - Update documentation
```

---

## Verification

After each phase:
```bash
cargo check
cargo clippy
cargo test
```

After Phase 3 (training update):
```bash
# Run training
cargo run --bin training -- --games 1

# Verify NO evlog files created
ls training_logs/session_*/*.evlog 2>&1 | grep -q "No such file"

# Verify SQLite has events
sqlite3 training.db "SELECT COUNT(*) FROM events;"
```

After Phase 6 (deletion):
```bash
# Verify no evlog references remain
grep -r "evlog" src/ --include="*.rs" | grep -v "// deprecated"
grep -r "\.evlog" src/ --include="*.rs"

# Full build
cargo build --all-targets

# Regression test
./scripts/regression.sh
```

Final verification:
```bash
# Test replay from SQLite
cargo run -- --replay-db 1

# Test training produces valid data
cargo run --bin training -- --games 2
sqlite3 training.db "SELECT id, level_name, score_left, score_right FROM matches ORDER BY id DESC LIMIT 2;"
```
