# Ballgame TODO - Current Sprint

*See `milestones.md` for full plan: Training Tools → AI Quality → MVP*

---

## CRITICAL: Fix Steal System - FIXED

**Root cause found:** Silent out-of-range failures. When pressing steal at 61-100px (outside 60px range),
the input was consumed but no feedback was given. Players thought they "failed" but nothing happened.

- [x] **Root cause analysis** - Found: no feedback for out-of-range attempts
- [x] **Investigate AI steal behavior** - AI waits reaction time, moves closer, always in range when pressing
- [x] **Review steal RNG** - RNG is fair, issue was players wasting attempts outside range
- [x] **Fix the underlying problem** - Added `StealOutOfRange` event and feedback

**FIX DETAILS:**
- Added `STEAL_NEAR_MISS_RANGE` (100px) - get feedback if close but not close enough
- Added `StealOutOfRange` event + visual indicator (out_of_range_timer)
- Added `STEAL_OUT_OF_RANGE_COOLDOWN` (0.2s) - short cooldown, less punishing
- Tests: `steal_boundary_outside.toml`, `steal_boundary_inside.toml`

**Steal differential enforcement remains active** (safety net, max 2 steal difference)

---

## AUDIT RESULTS (2026-01-25)

| Check | Status |
|-------|--------|
| Compilation | PASS (3 warnings in generate_ball.rs) |
| Clippy | PASS (~20 warnings, non-critical) |
| Tests | **35/35 PASS** |
| Test Coverage | **60%** (23 mechanics missing tests) |

**Test Coverage Gaps (see `notes/test_coverage_audit.md`):**
- AI: 0% coverage (no tests for navigation, shot decision, profiles)
- Shooting variance: not tested
- Ball physics edge cases: not tested

**AI Shot Analysis (see `notes/ai_shot_analysis.md`):**
- Found 3 potential issues with shot selection
- Issue 1: AI charges while navigating (may shoot mid-move)
- Issue 2: AI commits once charging (never reconsiders)
- Issue 3: Jump shot drift (AI may drift under basket)
- Rusher/Chaotic profiles DESIGNED to take bad shots (min_shot_quality 0.2/0.15)

**CRITICAL: Tournament Simulation Bug (see `notes/tournament_analysis.md`):**
- **4 profiles NEVER shoot**: Defensive, Patient, Sniper, Turtle
- Root cause: min_shot_quality too high for floor shots (max quality ~0.51)
- **54.8% of matches ended 0-0**
- **80% of matches had <5 total shots**
- FIX NEEDED: Lower thresholds or add desperation timer

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
cargo run --bin test-scenarios           # 35 scenario tests (8 steal tests)
cargo run --bin simulate -- --shot-test  # Shot accuracy
cargo run --bin training                 # Training mode
```

---

## Done (Last 5)

- [x] Steal out-of-range visual indicator - Orange flash when too far (2026-01-25)
- [x] Steal out-of-range feedback - Added StealOutOfRange event + feedback (2026-01-25)
- [x] steal_boundary_outside.toml test - Verifies out-of-range feedback
- [x] steal_boundary_inside.toml test - Verifies in-range attempt fires
- [x] StealContest.out_of_range_timer - Tracks near-miss feedback state

---

## Next Session Notes

**STEAL FIX COMPLETE - Visual feedback now works:**
- RED flash = steal attempted, RNG failed (25% success rate)
- ORANGE flash = too far away (need to get closer)
- No flash = nothing happened (way too far, >100px)

**Visual indicators by color:**
| Color | Meaning | Range |
|-------|---------|-------|
| Orange | Out of range | 60-100px |
| Red | Steal failed | <60px, RNG fail |
| Yellow | Vulnerable | Defender charging |

**Files changed this session:**
- `src/ui/steal_indicators.rs` - Added StealOutOfRangeFlash component + visual
- `src/ball/interaction.rs` - Added near-miss detection
- `src/steal.rs` - Added out_of_range_timer/entity to StealContest
- `src/constants.rs` - Added STEAL_NEAR_MISS_RANGE (100px), cooldown (0.2s)
- `src/events/types.rs` - Added StealOutOfRange event
- `tests/scenarios/stealing/` - Added 2 boundary tests

**Tomorrow priorities (from notes/tomorrow_plan.md):**
1. Play-test the new visual feedback - is orange flash noticeable?
2. AI shot selection quality analysis - why does AI take bad shots?
3. Test coverage report - what mechanics lack tests?
4. Event log analysis tools - make evlog parsing easier
