# Ballgame TODO - Current Sprint

*See `milestones.md` for full plan: Training Tools → AI Quality → MVP*

---

## Code Review Available (2026-01-25)

**Deep analysis completed** - see `code_review_2026-01-25.md` for:
- Best practices library with sources
- Anti-patterns found in codebase
- Prioritized improvement plan (P0-P3)
- Game design fundamentals

**Top findings:**
- `ai/decision.rs` needs splitting (1195 lines)
- RNG should consolidate to seeded resource (21 calls)
- Overall grades: Physics A+, Input A+, AI B+

---

## P0: SQLite Ghost Replay System

*See `notes/sqlite-ghost-replay-plan.md` for full implementation plan*

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

*Unify AI decision-making across all contexts - see `notes/ai-plugin-plan.md`*

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

**Tournament Simulation Bug (see `notes/tournament_analysis.md`):**
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

- [x] **AI architecture refactor** - AiCapabilities + world_model.rs for single source of physics truth (2026-01-25)
- [x] **Fix JumpAt horizontal movement** - AI now moves toward landing during entire jump arc (was stopping mid-air)
- [x] **Fix calculate_edge overlap case** - Jump from edge of overlap, not center (avoids ceiling collision)
- [x] Ghost system MVP - extract-drives + run-ghost binaries working (2026-01-25)
- [x] HOW_TO_PLAY.md - ASCII controller guide with all controls

*See `todone.md` for full archive with commit references*
