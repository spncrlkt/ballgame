# Evlog Elimination Plan (SQLite-Only)

Goal: remove all `.evlog` infrastructure and migrate fully to SQLite for event storage and replay.

## Phases (Checklist)

### Phase 1: SQLite replay loading
- [x] Add replay queries in `src/simulation/db.rs` (ordered tick + game events, match lookup).
- [x] Create `src/replay/sqlite_loader.rs`.
- [x] Update `src/replay/mod.rs` to support `match_id` replay.
- [x] Verify replay loads by `match_id`.

### Phase 2: CLI support
- [x] Add `--replay-db` in `src/main.rs`.
- [x] Deprecate `--replay <file>` in help text (temporary compatibility if needed).
- [x] Verify `cargo run -- --replay-db <id>`.

### Phase 3: Training writes only SQLite
- [x] Remove evlog writes in `src/bin/training.rs`.
- [x] Replace `GameResult.evlog_path` with `match_id` in `src/training/state.rs`.
- [x] Update `src/training/session.rs` summary + remove `evlog_path_for_game`.
- [ ] Verify no `.evlog` files under `training_logs/`.

### Phase 4: Analysis uses SQLite only
- [x] Remove evlog analysis path in `src/training/analysis.rs`.
- [x] Update/remove `src/analytics/parser.rs` evlog usage.
- [x] Verify analysis runs from DB only.

### Phase 5: Other binaries + config
- [x] Update `src/bin/simulate.rs` to ensure no evlog output.
- [x] Update/remove `src/bin/run-ghost.rs` evlog usage.
- [x] Update/remove `src/bin/extract-drives.rs` evlog usage.
- [x] Update `src/simulation/ghost.rs` evlog load path.
- [x] Remove evlog flags/docs from `src/simulation/config.rs`.
- [x] Persist simulation events to SQLite per match.
- [x] Add smoke test for simulation event persistence.

### Phase 6: Delete evlog infrastructure
- [x] Delete `src/events/evlog_parser.rs` and `src/events/logger.rs`.
- [x] Remove evlog exports in `src/events/mod.rs` and `src/lib.rs`.
- [x] Remove evlog replay loader + `--replay` file-path support.
- [x] Keep `parse_event` for DB event parsing/replay.
- [x] Delete `scripts/parse_training.py` and `scripts/analyze_training.py`.
- [x] Verify no `evlog` references in `src/` and `scripts/`.

### Phase 7: Docs cleanup
- [x] Remove evlog references from active docs (guides, specs, milestones).
- [x] Mark migration progress in planning docs.

## Files Summary

**Delete**
- `src/events/logger.rs`
- `src/events/evlog_parser.rs`
- `scripts/parse_training.py`
- `scripts/analyze_training.py`
- `docs/design/event-format.md`
- `src/replay/loader.rs` (after SQLite replay is verified)

**Create**
- `src/replay/sqlite_loader.rs`

**Modify**
- `src/simulation/db.rs`
- `src/replay/mod.rs`
- `src/main.rs`
- `src/bin/training.rs`
- `src/training/state.rs`
- `src/training/session.rs`
- `src/training/analysis.rs`
- `src/analytics/parser.rs`
- `src/bin/simulate.rs`
- `src/bin/extract-drives.rs`
- `src/bin/run-ghost.rs`
- `src/simulation/ghost.rs`
- `src/simulation/config.rs`
- `src/events/mod.rs`
- `src/events/format.rs`
- `src/lib.rs`
- `CLAUDE.md`, `docs/guides/training-workflow.md`, `docs/guides/TRAINING.md`
- `docs/design/functional_spec.md`, `docs/project/milestones.md`, `docs/project/todo.md`

## Verification

After each phase:
```bash
cargo check
cargo clippy
cargo test
```

After Phase 3:
```bash
cargo run --bin training -- --games 1
ls training_logs/session_*/*.evlog 2>&1 | grep -q "No such file"
sqlite3 training.db "SELECT COUNT(*) FROM events;"
```

After Phase 6:
```bash
rg -n "evlog|\\.evlog" src scripts
cargo build --all-targets
./scripts/regression.sh
```

Final:
```bash
cargo run -- --replay-db 1
cargo run --bin training -- --games 2
sqlite3 training.db "SELECT id, level_name, score_left, score_right FROM matches ORDER BY id DESC LIMIT 2;"
```
