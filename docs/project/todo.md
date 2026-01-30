# Ballgame TODO - Current Sprint

*See `milestones.md` for full plan | `ideas.md` for notes*

---

## P0: Bug Fixes

- [ ] **Input capture bug** - Tweak panel early-return leaves stale `PlayerInput`

---

## P0: Training Binary UX

- [ ] **Reset button (Start)** - wipes logs, restarts session (preserve CLI args)
- [ ] **Clear status display** between games

---

## P1: AI Plugin Consolidation

- [ ] **Create `AiPlugin`** - Single source of truth for AI systems
- [ ] **Update all binaries** - main, training, simulation use same plugin
- [ ] **Fix ghost mode** - Use full AI decision system

---

## P2: AI Behavior

- [ ] Fix shooting - AI takes bad shots, misses easy ones
- [ ] Fix positioning - AI stands in wrong places, doesn't cover basket

---

## P3: Movement Feel

- [ ] Tune player movement - speed, acceleration, air control
- [ ] Tune jump feel - height, coyote time, responsiveness

---

## Backlog

**Technical Debt:**
- PlayerId → CharacterId migration (~30 deprecation warnings)
- System wiring drift across binaries
- EventBus `processed` grows unbounded

**Features:**
- Visual ghost mode in main game
- More ball styles
- AI debug level (both players AI)

**Documentation:**
- AI_PROFILES.md, LEVELS.md

---

## Done (Last 5)

- [x] **2v2 Readiness & Test Coverage** - Full implementation (2026-01-30)
  - Added 86 new unit tests (145 total, up from 62)
  - CharacterId, format round-trip, analytics parser, assertions, SQLite logger tests
  - Refactored main.rs player spawning to use `spawn_characters_for_mode()`
  - Created reusable `spawn_charge_gauge()` helper in player/spawn.rs
  - PlayerId → CharacterId migration complete
- [x] **Binary Reference Guide** - Created `docs/guides/BINARIES.md` (2026-01-29)
  - All 11 binaries documented with flags, options, examples
  - Updated README.md to link instead of inline docs
- [x] **Unified Run Summary** - Consistent end-of-run output for all binaries (2026-01-29)
  - New `src/run_summary.rs` module with builder pattern
  - 80-char box formatting with Unicode box-drawing characters
  - File category tags: `[DB]`, `[REPORT]`, `[IMG]`, `[DATA]`, `[CFG]`
  - Next step suggestions: primary (→) and secondary (·)
  - Integrated: training, test_scenarios, extract-drives, heatmap, verify_reachability, run-ghost, generate, simulate (tournament/bracket modes)
- [x] **Reachability-Aware Navigation** - NavGraph now uses player exploration data for smarter shooting positions (2026-01-28)
  - `PlatformSource` enum tracks config origin (Floor, CornerRamp, Center, Mirror)
  - `reachability` field (0.0-1.0) from SQLite exploration data
  - `find_shooting_node` prefers high-reachability positions
  - New tests: `reachability_test.rs`, `multihop_test.rs`
- [x] **Accuracy/Cadence Tuning** - Extended preset system with 10 shot params, tested V1-V6 variants (2026-01-28)
  - V3-Forgiving now default: 3.2 goals/match (↑88%), 34.8% accuracy (↑61%)
- [x] **Training Reachability Protocol** - Solo exploration mode for coverage mapping (2026-01-28)
  - `--protocol reachability` flag for level exploration
  - Q/LB advances to next level during exploration

*See `todone.md` for full archive*
