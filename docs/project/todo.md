# Ballgame TODO - Current Sprint

*See `milestones.md` for full plan: Training Tools → AI Quality → MVP*

---

## gameplay notes 
- we need to have an animated gif of a good starting 3-2-1 -> 20 seconds of action ending in a point. we need a system to recreate a full replay (like our ghost/replay system) and record it as a gif for the front page demo on our readme. lets organize and fix the leaky abstractions that we have laid out across our events/replay system
- archive level system. move to archived file, remove all references, ensure no path to read from the archived levels file
- documentantion roundup: assets, notes, settings, readme, todo systems, etc let's try to categorize and organize these files into a structure in our project that works
- the ai steals too easily and it is too hard for me to steal. lets set up a training protocol for stealing only with a flat level and normal baskets no stairs. we'll play for 60 seconds and record steal attempts and successful steals. make sure we are logging enough information to analyze the fairness of player stealing vs ai stealing.
- the platforming and understanding of steps and platforms are still inefficient. lets set up a training protocol for steps and platforming. make the level look like Skyway. 

## Code Review Available (2026-01-25)

**Deep analysis completed** - see `docs/reviews/code_review_2026-01-25.md` for:
- Best practices library with sources
- Anti-patterns found in codebase
- Prioritized improvement plan (P0-P3)
- Game design fundamentals

**Top findings:**
- `ai/decision.rs` needs splitting (1195 lines)
- RNG should consolidate to seeded resource (21 calls)
- Overall grades: Physics A+, Input A+, AI B+

---

## P0: Evlog Elimination (Next Session Priority)

*See `docs/planning/evlog-elimination-plan.md` for full implementation plan*

- [ ] **Complete evlog elimination** - Migrate fully to SQLite, remove all .evlog infrastructure
  - Phase 1: Add SQLite replay loading to SimDatabase
  - Phase 2: Add --replay-db flag to main binary
  - Phase 3: Update training binary to remove evlog writing
  - Phase 4: Update analysis to use SQLite only
  - Phase 5: Update other binaries
  - Phase 6: Delete evlog code (logger.rs, evlog_parser.rs, Python scripts)
  - Phase 7: Documentation cleanup

---

## P0: SQLite Ghost Replay System

*See `docs/planning/sqlite-ghost-replay-plan.md` for full implementation plan*

- [ ] **Implement SQLite-based training + ghost replay** - Record inputs to DB, replay as ghost

---

## P0: Training Binary UX

*Top priority - enables faster AI iteration*

- [ ] **Reset button (Start)** - wipes all logs and status, restarts session
  - If CLI args were used, keep them
  - If no args, cycle through default options
- [ ] **Clear status display** between games

---

## P1: Ghost System (MVP DONE)

*Scripted replay for AI defense testing*

- [x] **Drive extractor** - `cargo run --bin extract-drives <session_dir>`
- [x] **Ghost replay mode** - `cargo run --bin run-ghost <trial.ghost>`
- [x] **Defense metric** - run-ghost shows defense rate and outcome breakdown
- [ ] **Polish: visual ghost mode** - render ghost playback in main game (optional)

---

## P1.5: AI Plugin Consolidation

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

- Settings file: move init_settings out of VC, use template as default
- Settings persistence: save viewport/prefs on change, load on start
- Ball options: more styles (yin yang, volleyball, pool balls, etc.)
- Debug level: labels update color on palette change
- AI debug level: both players AI-controlled for testing

---

## Known Issues

**Tournament Simulation Bug (see `docs/analysis/tournament_analysis.md`):**
- 4 profiles NEVER shoot: Defensive, Patient, Sniper, Turtle
- Root cause: min_shot_quality too high for floor shots (max quality ~0.51)
- 54.8% of matches ended 0-0
- FIX NEEDED: Lower thresholds or add desperation timer

**Ghost system status:**
- `src/bin/run-ghost.rs` - Working ghost trial runner
- `src/bin/extract-drives.rs` - Working drive extractor
- `src/simulation/ghost.rs` - Core ghost types and systems
- Note: run-ghost uses simplified AI - will be fixed by AI Plugin consolidation (P1.5)

**Test commands:**
```bash
cargo run --bin test-scenarios           # 35 scenario tests (8 steal tests)
cargo run --bin simulate -- --shot-test  # Shot accuracy
cargo run --bin training                 # Training mode
```

---

## Done (Last 5)

- [x] **AI pressure/steal fixes** - Meta-analysis of 19 sessions → 4 fixes implemented (2026-01-25)
  - Profile: steal_range 128→100, pressure_distance 82→120
  - Intercept position closer to opponent (0.3-0.7x instead of 1.0x)
  - PressureDefense window widened 34px→97px
  - Result: AI steal attempts 0.26/game → 13/game
- [x] **AI architecture refactor** - AiCapabilities + world_model.rs for single source of physics truth (2026-01-25)
- [x] **Fix JumpAt horizontal movement** - AI now moves toward landing during entire jump arc (was stopping mid-air)
- [x] **Fix calculate_edge overlap case** - Jump from edge of overlap, not center (avoids ceiling collision)
- [x] Ghost system MVP - extract-drives + run-ghost binaries working (2026-01-25)

*See `todone.md` for full archive with commit references*
