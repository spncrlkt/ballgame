# Ballgame TODO - Current Sprint

*See `milestones.md` for full plan: Training Tools → AI Quality → MVP*

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
- [ ] **Polish: full AI for ghost defender** - currently simplified AI, wire up full decision system
- [ ] **Polish: visual ghost mode** - render ghost playback in main game (optional)

---

## P2: AI Navigation

- [ ] **Verify corner step fix** - run training on levels 7-8, check AI climbs
- [ ] **Teach AI jump capability** - skip intermediate steps when direct jump possible
- [ ] **Debug logging** - nav graph already has logging, verify it shows Jump edges

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
- Note: run-ghost uses simplified AI for defender, not full decision system

**Test commands:**
```bash
cargo run --bin test-scenarios           # 35 scenario tests (8 steal tests)
cargo run --bin simulate -- --shot-test  # Shot accuracy
cargo run --bin training                 # Training mode
```

---

## Done (Last 5)

- [x] Ghost system MVP - extract-drives + run-ghost binaries working (2026-01-25)
- [x] HOW_TO_PLAY.md - ASCII controller guide with all controls
- [x] README.md update - All binaries documented, AI profiles section
- [x] Steal out-of-range visual indicator - Orange flash when too far (2026-01-25)
- [x] Steal boundary tests - inside/outside range verification

*See `todone.md` for full archive with commit references*
