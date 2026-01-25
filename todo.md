# Ballgame TODO - Current Sprint

*See milestones.md for the full project plan (MVP → V0 → V1)*

## Active Work (Prioritized)

### 0. Immediately
- take init_settings out of vc, make a default copy in VC and have that be the source of truth when cloning and hard-resetting the settings w/ start(double-click)
- [x] ai steals too easy, why is it hard for player to steal?
  - Fixed: Added two tunable AI profile parameters for realistic human timing:
    - `steal_reaction_time` - delay before first steal attempt (0.10-0.30s)
    - `button_presses_per_sec` - max button mash rate (6-15 presses/sec)
  - AI now simulates human reaction time + button mashing limits
  - All pickup_pressed inputs (steals + ball pickups) respect these limits
  - Added 4 unit tests in `src/ai/decision.rs`:
    - `test_button_press_cooldown_limits_rate` - verifies cooldown math
    - `test_steal_reaction_timer_delays_first_attempt` - verifies delay
    - `test_profile_button_timing_in_human_range` - validates profile values
    - `test_all_pickup_pressed_assignments_have_cooldown` - **catches missing cooldowns on new code**
- check shooting heatmap methodology and re-run.
- in training bin start button should wipe all logs and status and start over.if args were used on the command line keep them but cycle through the default options otherwise
- [x] **Fix ball platform display on level 1** - fixed spawn_balls to spawn only one ball on floor
- the steps are all fucked up
- [ ] **Immediate** Run training binary to verify nav graph fix for corner steps
  - Check debug logs show corner ramp nodes with proper Jump edges
  - Verify AI can climb corner steps on levels 7 (Skyway) or 8 (Terraces)
  - Changes made: removed CornerRamp from fill blocks, added nav graph debug logging
- [ ] **Immediate** Teach AI its own jump capability
  - AI should know max jump height and horizontal reach
  - Enable skipping intermediate steps when a single jump can reach a higher platform
  - Avoid inefficient step-by-step climbing when direct jump is possible
- more ball options: more s60 star alts, yin yang, 3D rotated S11 "volleyball", 3D rotated basketball, striped balls, croquet balls, pool balls. full analysis after wards to see patternable "types" to combine
- [x] **Scenario Testing System** - see `notes/testing-plan.md` for full coverage map
  - [x] Create `src/testing/` module (parser, runner, assertions, input injection)
  - [x] Create `assets/test_levels.txt` with minimal test arenas
  - [x] Create `src/bin/test_scenarios.rs` CLI
  - [x] Write initial 9 test scenarios (movement, ball, shooting, collision)
  - [x] Expanded to 33 tests (movement, ball, shooting, scoring, stealing, collision)


### 1. D-Pad Menu UX
- [ ] **P0** Improve D-pad menu display - see `notes/dpad-menu-ux.md`

### 2. AI Behavior
- [ ] **P0** Remove AI handicaps - give AI equal capabilities to player
  - Remove 0.4s jump shot charge cap (decision.rs:588)
  - Remove 0.5s nav jump hold cap (decision.rs:787)
  - Increase jump shot hold time from 0.15s to 0.25s (decision.rs:576-580)
  - Extend jump shot timeout from 0.3s to 1.0s (decision.rs:604-605)
  - See plan file: `~/.claude/plans/smooth-floating-token.md`
- [ ] **P1** Fix AI shooting - takes bad shots, misses easy ones
- [ ] **P2** Fix AI positioning - stands in wrong places, doesn't cover basket well

### 2a. Shot Accuracy Testing (Partial - Needs Overhaul)
- [x] **Reduce upward bias** - Removed angle variance bias entirely (was +0.05)
- [x] **Distance-based speed multiplier** - Simple linear 1.0→1.05 in throw.rs
- [x] **Shot test mode** - `cargo run --bin simulate -- --shot-test [shots] --level [n]`
- [x] **Test level** - Added `test_shot_accuracy` to `assets/test_levels.txt`
- [ ] **Shot System Overhaul** (V1) - Current band-aid multiplier doesn't properly balance:
  - Similar distances (dx=440 vs dx=540) produce wildly different overshoot ratios
  - Piecewise tuning fights symptoms, not root cause
  - **Root cause investigation needed:**
    1. Review minimum-energy trajectory formula - may not suit game feel
    2. Check if angle/direction affects trajectory differently than expected
    3. Consider simpler trajectory system (fixed arc heights, predictable curves)
    4. Analyze interaction between: angle variance, speed randomness, player velocity
  - **Test methodology is solid** - keep shot test mode for validating any fix

### 3. Movement/Physics Tuning
- [ ] **P3** Tune player movement - speed, acceleration, air control
- [ ] **P4** Tune jump feel - height, coyote time, responsiveness

### 4. Polish
- [ ] **P5** UI fix flash on score color
- [ ] **P6** Viewport testing at various resolutions

### 5. Settings
- [ ] **Immediate** Persist per-installation settings (viewport size, etc.) - save on change, load on game start

### 6. Debug
- [ ] Add AI vs AI debug level for Level 2 (both players AI-controlled for testing)

---

## Balance Simulation Suite

Statistical simulation tests for game balance. Run during audits to catch regressions.

**Current simulations:**
- `cargo run --bin simulate -- --shot-test` - Shot accuracy (overshoot/undershoot ratio)
  - Target: 40-60% over/under ratio
  - Tests from 4 positions at varying distances

**Planned simulations:**
- [ ] Steal success rate test - Verify ~33% base steal chance
- [ ] AI win rate balance - Tournament mode across profiles
- [ ] Ball physics consistency - Bounce heights, friction, gravity

**Usage in audits:**
```bash
# Quick balance check
cargo run --bin simulate -- --shot-test 30 --level 3

# Full tournament (slower)
cargo run --bin simulate -- --tournament 5
```

---

## TODO - Debug Level Ball Display
- [ ] Labels need to update color when palette changes (currently static TEXT_SECONDARY)
- [ ] Consider adding a 6th shelf if 55 balls look too cramped
- [ ] Optional: make playable floor ball also a random style on level reset

## Done (recent - see todone.md for full archive)
- [x] **Scenario Testing System** - 33 tests, see `notes/testing-plan.md` for coverage map
- [x] **Shot Accuracy Testing** - Distance-based speed multiplier, 45.8% over/under ratio
- [x] **AI Steal Timing** - Reaction time + button mashing limits for human-like behavior
- [x] **Code review guidelines** - Created `code_review_guidelines.md`, integrated into CLAUDE.md
- [x] **Debug level ball display** - All ball styles on shelves with labels and wave animations
