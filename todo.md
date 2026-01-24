# Ballgame TODO - Current Sprint

*See milestones.md for the full project plan (MVP → V0 → V1)*

## Active Work (Prioritized)

### 0. Immediately
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
- [x] **Scenario Testing System** - see `notes/testing-plan.md` for full plan
  - [x] Create `src/testing/` module (parser, runner, assertions, input injection)
  - [x] Create `assets/test_levels.txt` with minimal test arenas
  - [x] Create `src/bin/test_scenarios.rs` CLI
  - [x] Write initial 9 test scenarios (movement, ball, shooting, collision)
  - [ ] Expand to 20 tests (add scoring, stealing scenarios)


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

### 2a. Shot Accuracy Testing (Overshoot Fix)
- [x] **Reduce upward bias** - Changed angle variance bias from `+0.05` to `+0.01` in throw.rs:149
- [ ] **Extend simulate.rs** - Add `--shot-test` mode for shooting accuracy trials
  - Spawn player at 3-4 fixed positions (varying distances/angles to basket)
  - Fire 30 shots at full charge from each position
  - Track outcome: overshoot (above basket), undershoot (below basket), goal (in basket)
  - Report ratio per position and overall, PASS if over/under ~40-60%
- [ ] **Create test level** - Add `test_shot_accuracy` to `assets/test_levels.txt`
  - Simple flat floor with basket at known height
  - Marked shooter positions at various distances
- [ ] **Tune speed multiplier** - If still overshooting, reduce `1.10` → `1.05` in throw.rs:161

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

## TODO - Debug Level Ball Display
- [ ] Labels need to update color when palette changes (currently static TEXT_SECONDARY)
- [ ] Consider adding a 6th shelf if 55 balls look too cramped
- [ ] Optional: make playable floor ball also a random style on level reset

## Done (recent - see todone.md for full archive)
- [x] **Code review guidelines** - Created comprehensive `code_review_guidelines.md` and integrated quick checklists into CLAUDE.md Maintenance section
- [x] **Debug level ball display** - All ball styles on shelf platforms with labels and wave animations
  - Display balls on 5 shelves (heights 380-780) with style name labels
  - One random playable ball spawned on floor between players
  - Two independent wave animations: scale pulse and spin wave
  - Waves cycle through patterns: left-to-right, right-to-left, center-out, edges-in
- [x] **Replay System** - Play back recorded .evlog files with variable speed
  - Created `src/replay/` module (loader, state, systems, UI)
  - `cargo run -- --replay <file.evlog>` to enter replay mode
  - Enhanced Tick events with velocity data (50ms / 20 Hz)
  - AI goal change logging, steal event detection, fixed ShotStart positions
  - Hermite interpolation for smooth playback
  - Controls: Space=pause, </>=speed, ,/.=step, Home/End=jump
  - Timeline UI with event markers (goals, steals, pickups)
  - AI goal labels above players showing current behavior
- [x] **Analytics System** - Decoupled simulation + analysis workflow
  - Added `--log-events` and `--log-dir` flags to simulate binary
  - Created `src/analytics/` module (parser, metrics, leaderboard, targets, suggestions, defaults)
  - Created `cargo run --bin analyze` for post-simulation analysis
  - Profile leaderboard ranked by win rate
  - Target comparison vs tuning goals (TOML config in `assets/tuning_targets.toml`)
  - Auto-update defaults in `src/constants.rs` with `--update-defaults`
- [x] Created AI simulation system (`cargo run --bin simulate`) for headless AI testing
- [x] Created snapshot system (src/snapshot.rs) - captures game state + screenshots on events
- [x] Created scripts/screenshot.sh and scripts/regression.sh for visual testing
