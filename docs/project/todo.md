# Ballgame TODO - Current Sprint

*See `milestones.md` for full plan: Training Tools → AI Quality → MVP*
*See `ideas.md` for non-prioritized ideas and notes*
*See `docs/archive/code_review_2026-01-25.md` for deep code analysis*

---

## P0: Bug Fixes

**Cooldown timing bug** - `steal_cooldown_update` runs in BOTH Update and FixedUpdate across multiple binaries (main.rs, training.rs, run-ghost.rs, runner.rs), causing timers to tick twice as fast.
- [ ] Consolidate to FixedUpdate only (physics timing)
- [ ] Verify UI in `steal_indicators.rs` still works (reads timers in Update)

**Input capture bug** - When tweak panel is open, `capture_input` early-returns but `copy_human_input` continues applying stale `PlayerInput`.
- [ ] Clear input state when panel is open, or skip copy_human_input

---

## P0: Training Binary UX

*Top priority - enables faster AI iteration*

- [ ] **Reset button (Start)** - wipes all logs and status, restarts session
  - If CLI args were used, keep them
  - If no args, cycle through default options
- [ ] **Clear status display** between games

---

## P1: AI Plugin Consolidation

*Unify AI decision-making across all contexts - see `docs/planning/ai-plugin-plan.md`*

- [ ] **Create `AiPlugin`** - Single source of truth for AI system registration
- [ ] **Update main game** - Use AiPlugin instead of inline systems
- [ ] **Update training/simulation** - Same plugin usage
- [ ] **Fix ghost mode** - Delete simplified AI, use full decision system with HumanControlled marker

Benefits: ~120 lines deleted, full AI defense in ghost mode, cleaner architecture

---

## P2: AI Navigation

- [ ] **Verify corner step fix** - run training on levels 7-8, check AI climbs
- [ ] **Teach AI jump capability** - skip intermediate steps when direct jump possible
- [ ] **Debug logging** - nav graph already has logging, verify it shows Jump edges
- [ ] **Fix ramp-less level fallback** - InterceptDefense assumes ramps exist; in `steps: 0` levels, AI targets nonexistent corner ramps instead of using platforms or direct pursuit (see decision.rs:953-974)
- [ ] **Reduce goal oscillation** - 7 oscillation instances observed in pursuit2 test; may need hysteresis or commitment timers

---

## P3: AI Behavior (after training tools work)

- [ ] Fix shooting - AI takes bad shots, misses easy ones
- [ ] Fix positioning - AI stands in wrong places, doesn't cover basket

---

## P4: Movement Feel (after AI works)

- [ ] Tune player movement - speed, acceleration, air control
- [ ] Tune jump feel - height, coyote time, responsiveness

---

## Backlog (not prioritized)

**Technical Debt:**
- System wiring drift: consolidate schedules across binaries (main/training/simulation/run-ghost diverged)
- EventBus `processed` retention: clear/limit after logging (grows unbounded)
- Reduce re-export surface in `src/lib.rs`
- Deterministic sim mode (seed + fixed timestep for reproducible runs)

**AI Improvements:**
- AI-vs-AI opponent selection: `ai_navigation_update` targets `HumanControlled` only; `ai_decision_update` uses "any other player" - inconsistent
- Define zone geometry constants + formulas (for defense positioning)
- Defensive test matrix (levels, seeds, profiles, expected goal mix)

**Features:**
- Visual ghost mode: render ghost playback in main game (optional polish)
- Settings file: move init_settings out of VC, use template as default
- Settings persistence: save viewport/prefs on change, load on start
- Ball options: more styles (yin yang, volleyball, pool balls, etc.)
- Debug level: labels update color on palette change
- AI debug level: both players AI-controlled for testing

**Simulation/Heatmaps:**
- Generate heatmaps for all levels (only Arena + Open Floor currently have full set)
- Add `--preset` flag to simulate binary for easier variant testing

---

## Known Issues

**Tournament Simulation Bug (see `docs/archive/tournament_analysis.md`):**
- 4 profiles NEVER shoot: Defensive, Patient, Sniper, Turtle
- Root cause: min_shot_quality too high for floor shots (max quality ~0.51)
- 54.8% of matches ended 0-0
- FIX NEEDED: Lower thresholds or add desperation timer

**Ghost system (MVP complete):**
- `src/bin/run-ghost.rs` - Working ghost trial runner
- `src/bin/extract-drives.rs` - Working drive extractor
- Note: run-ghost uses simplified AI - will be fixed by AI Plugin consolidation (P1)

**Test commands:**
```bash
cargo run --bin test-scenarios           # 35 scenario tests (8 steal tests)
cargo run --bin simulate -- --shot-test  # Shot accuracy
cargo run --bin training                 # Training mode
```

---

## Done (Last 5)

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
- [x] **Evlog elimination complete** - Full SQLite migration, all .evlog infrastructure removed (2026-01-26)

*See `todone.md` for full archive with commit references*
