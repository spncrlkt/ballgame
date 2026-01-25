# Ballgame TODO - Current Sprint

*See `milestones.md` for full plan: Training Tools → AI Quality → MVP*

---

## P0: Training Binary UX

*Top priority - enables faster AI iteration*

- [ ] **Reset button (Start)** - wipes all logs and status, restarts session
  - If CLI args were used, keep them
  - If no args, cycle through default options
- [ ] **Clear status display** between games
- [ ] **Fix ghost-visual.rs compilation** - Bevy Bundle errors, unused imports

---

## P1: Ghost System

*Scripted replay for AI defense testing*

- [ ] **Drive extractor** - parse evlog, segment by goals, output input sequences
- [ ] **Ghost replay mode** - one player follows recorded inputs
- [ ] **`--ghost` integration** - already has CLI flag, needs wiring up
- [ ] **Defense metric** - % of drives where AI prevents the score

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

**WIP files with issues:**
- `src/bin/ghost-visual.rs` - Bundle errors, needs Bevy 0.17 fixes
- `src/simulation/ghost.rs` - exists but not integrated

**Test commands:**
```bash
cargo run --bin test-scenarios           # 35 scenario tests (8 steal tests)
cargo run --bin simulate -- --shot-test  # Shot accuracy
cargo run --bin training                 # Training mode
```

---

## Done (Last 5)

- [x] Steal out-of-range visual indicator - Orange flash when too far (2026-01-25)
- [x] Steal boundary tests - inside/outside range verification
- [x] StealContest.out_of_range_timer - Tracks near-miss feedback state
- [x] Full test coverage audit - 60% coverage, gaps documented
- [x] Tournament analysis - Found profile shooting thresholds issue

*See `todone.md` for full archive with commit references*
