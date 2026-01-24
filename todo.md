# Ballgame TODO - Current Sprint

*See milestones.md for the full project plan (MVP → V0 → V1)*

## Active Work (Prioritized)

### 0. D-Pad Menu UX
- [ ] **P0** Improve D-pad menu display - see `notes/dpad-menu-ux.md`

### 1. AI Behavior
- [ ] **P1** Fix AI shooting - takes bad shots, misses easy ones
- [ ] **P2** Fix AI positioning - stands in wrong places, doesn't cover basket well

### 2. Movement/Physics Tuning
- [ ] **P3** Tune player movement - speed, acceleration, air control
- [ ] **P4** Tune jump feel - height, coyote time, responsiveness

### 3. Polish
- [ ] **P5** UI fix flash on score color
- [ ] **P6** Viewport testing at various resolutions

### 4. Settings
- [ ] **Immediate** Persist per-installation settings (viewport size, etc.) - save on change, load on game start

### 5. Debug
- [ ] Add AI vs AI debug level for Level 2 (both players AI-controlled for testing)

---

## Done (recent - see todone.md for full archive)
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
