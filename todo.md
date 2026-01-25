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

## Notes

**WIP files with issues:**
- `src/bin/ghost-visual.rs` - Bundle errors, needs Bevy 0.17 fixes
- `src/simulation/ghost.rs` - exists but not integrated

**Test commands:**
```bash
cargo run --bin test-scenarios           # 33 scenario tests
cargo run --bin simulate -- --shot-test  # Shot accuracy
cargo run --bin training                 # Training mode
```
