# Codex TODO

## All Tasks (Top)

- [x] Evlog elimination (full SQLite migration, zero `.evlog` usage).
- [x] Simulation: confirm event rows persisted for parallel runs.
- [ ] Tooling: resolve `/bin/ps` permission warning in login shells (affects sqlite3 commands).
- [ ] Cooldown timing fix (steal + UI timers).
- [ ] AI-vs-AI opponent selection.
- [ ] AI defense stalling: fix goal targeting + verify goal events/analysis alignment.
- [ ] Scenario tests for cooldown timing + AI-vs-AI defense.
- [ ] EventBus `processed` retention policy (clear/limit after logging).
- [ ] Input capture: prevent stale `PlayerInput` while tweak panel open.
- [ ] System wiring drift: consolidate schedules across binaries.
- [ ] Deterministic sim mode (seed + fixed timestep).
- [ ] Logging boundaries to reduce training/simulation I/O.
- [ ] Reduce re-export surface in `src/lib.rs`.
- [ ] Define zone geometry constants + formulas.
- [ ] Defensive test matrix (levels, seeds, profiles, expected goal mix).
- [ ] Zone occupancy + threat ranking derivation from tick data.
- [ ] Heatmap tooling: per-level basket height/geometry support.
- [ ] Heatmap extension: zone map + threat overlay (PNG + JSON).
- [ ] Per-level heatmap generation/storage.
- [ ] NavGraph enrichment from heatmap outputs (zone IDs + shot spots).

## Primary Focus This Session

- [x] Complete evlog elimination (see checklist below).

## Evlog Elimination Checklist

### Phase 1: SQLite replay loading
- [x] Add replay queries in `src/simulation/db.rs` (match lookup, ordered events).
- [x] Create `src/replay/sqlite_loader.rs`.
- [x] Update `src/replay/mod.rs` for `match_id` replay mode.
- [x] Verify replay loads from `match_id`.

### Phase 2: CLI support
- [x] Add `--replay-db` in `src/main.rs`.
- [x] Deprecate/flag `--replay` file usage in CLI help.
- [x] Verify `cargo run -- --replay-db <id>`.

### Phase 3: Training writes only SQLite
- [x] Remove evlog writing in `src/bin/training.rs`.
- [x] Add `match_id` to `GameResult` in `src/training/state.rs`.
- [x] Update `src/training/session.rs` (`GameSummary`, remove `evlog_path_for_game`).
- [ ] Verify `training_logs/` produces no `.evlog` and DB has events.

### Phase 4: Analysis uses SQLite only
- [x] Remove evlog analysis path in `src/training/analysis.rs`.
- [x] Update/remove `src/analytics/parser.rs` evlog usage.
- [x] Rename DB analysis entrypoints where needed.
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
- [x] Confirm `parse_event` stays (used for DB event parsing/replay).
- [x] Delete `scripts/parse_training.py` and `scripts/analyze_training.py`.
- [x] Verify no `evlog` references in `src/` and `scripts/`.

### Phase 7: Docs cleanup
- [x] Remove evlog references from active docs (guides, specs, milestones).
- [x] Mark evlog elimination progress in planning docs.
