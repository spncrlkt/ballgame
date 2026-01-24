# Ballgame TODO - Current Sprint

*See milestones.md for the full project plan (MVP → V0 → V1)*

## Active Work (Prioritized)

### 0. Immediately
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
- the ball spawning from our previous work isnt working
- create another set of debug levels for programmatic access for testing. to test basic interactions like shooting, platforming, stealing etc. create a series of tests that use the headless simulation and event logging to confirm that all our basic features don't regress. create a set of input scripts that perform the desired actions and check event log for testing.
- 

### 1. D-Pad Menu UX
- [ ] **P0** Improve D-pad menu display - see `notes/dpad-menu-ux.md`

### 2. AI Behavior
- [ ] **P1** Fix AI shooting - takes bad shots, misses easy ones
- [ ] **P2** Fix AI positioning - stands in wrong places, doesn't cover basket well

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
