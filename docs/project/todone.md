# Ballgame Done Archive

## Archived 2026-01-28 (Accuracy/Cadence Tuning)

- [x] **Fix calculate_edge overlap case** - Jump from edge of overlap, not center (avoids ceiling collision)

---

## Archived 2026-01-25 (AI Architecture Refactor)

**AI Physics Consolidation:**
- [x] Created `src/ai/capabilities.rs` - Single source of truth for physics values (max_jump_height, time_to_peak, etc.)
- [x] Created `src/ai/world_model.rs` - Platform data extraction for ceiling checks
- [x] Updated `navigation.rs` - Use AiCapabilities, fixed overlap case in calculate_edge()
- [x] Updated `decision.rs` - Removed 4 duplicated physics formulas, use shared capabilities

**Critical Bug Fixes:**
- [x] **JumpAt horizontal movement** - AI was stopping horizontal movement during jump hold phase, causing it to fall back down instead of reaching platforms. Fixed by setting move_x toward landing point during entire jump arc.
- [x] **calculate_edge overlap** - When platforms overlap horizontally, edge calculation now jumps from OUTSIDE the overlap to arc over, not from center (which would hit ceiling).

**Verification:**
- pursuit2 test: Still time dropped from 47% → 6%
- AI successfully reaches elevated platforms and catches opponent

**Known remaining issues (see todo.md P2):**
- InterceptDefense ramp fallback assumes ramps exist in all levels
- Goal oscillation (7 instances in 23s test)

---

## Archived 2026-01-25 (Steal System Fix + AI/Simulation Work)

**Commits: 8f26e36 → 045d8c2**

**Steal System Complete:**
- [x] Root cause analysis - Found: no feedback for out-of-range attempts
- [x] Investigate AI steal behavior - AI waits reaction time, moves closer, always in range when pressing
- [x] Review steal RNG - RNG is fair, issue was players wasting attempts outside range
- [x] Fix underlying problem - Added `StealOutOfRange` event and feedback
- [x] Steal out-of-range visual indicator - Orange flash when too far (2026-01-25)
- [x] `steal_boundary_outside.toml` test - Verifies out-of-range feedback
- [x] `steal_boundary_inside.toml` test - Verifies in-range attempt fires
- [x] `StealContest.out_of_range_timer` - Tracks near-miss feedback state

**Test Suite Status (35/35 PASS):**
- Movement: 8 tests
- Ball: 7 tests
- Shooting: 5 tests
- Scoring: 4 tests
- Stealing: 8 tests (including 2 new boundary tests)
- Collision: 3 tests

---

## Archived 2026-01-25 (Full Audit & Verification)

**Commits: 6a3c6ab → 9f6ee23**

**Simulation Infrastructure Complete (63 tests verify)**
- [x] Sprint 1: HeadlessAppBuilder - shared app setup for all simulation modes
- [x] Sprint 1: Event emission consolidation - `emit_game_events()` shared function
- [x] Sprint 2: Runner modularization - control.rs, setup.rs, shot_test.rs (52% reduction)
- [x] Sprint 3: Parallel simulation - Rayon + `--parallel N` CLI flag
- [x] Sprint 3: SQLite database - `SimDatabase` with WAL mode, `--db` option
- [x] Sprint 4: Unified parser (legacy) - `ParsedEvlog` struct, 6 unit tests
- [x] Sprint 4: Analytics + SQLite - `analyze_profile()`, `compare_profiles()`, leaderboard

**AI Steal Timing (4 unit tests + 6 scenario tests)**
- [x] `steal_reaction_time` parameter (0.10-0.30s delay before first attempt)
- [x] `button_presses_per_sec` parameter (6-15 presses/sec limit)
- [x] All 10 AI profiles have human-realistic timing values

**Shot Accuracy Testing**
- [x] Distance-based speed multiplier (1.0→1.05)
- [x] `--shot-test` mode in simulate binary
- [x] Synced heatmap with game physics

**Documentation**
- [x] `code_review_guidelines.md` - 18KB comprehensive reference
- [x] `notes/balance-testing-workflow.md` - Iterative tuning workflow
- [x] `notes/testing-plan.md` - 33 scenario test coverage map

---

## Archived 2026-01-24 (Scenario Testing Session)

**Commits: f610bba → 03a5651**

- [x] **Scenario Testing System** - 33 tests passing across 6 categories
  - Movement: 8 tests (walk, jump, coyote time, jump buffer, air control)
  - Ball: 7 tests (pickup, bounce, rolling, near basket)
  - Shooting: 5 tests (basic, max charge, aim, elevation, jumping)
  - Scoring: 4 tests (basic, increments, respawn, own goal)
  - Stealing: 6 tests (range, cooldown, charging, knockback, immunity)
  - Collision: 3 tests (wall, platform landing, head bonk)
- [x] Shot Accuracy Testing - Implemented distance-based speed multiplier
  - 45.8% over/under ratio (PASS: within 40-60%)
  - `cargo run --bin simulate -- --shot-test` for testing
- [x] AI Steal Timing - Added reaction time + button mashing limits
  - `steal_reaction_time` (0.10-0.30s) and `button_presses_per_sec` (6-15/sec)

---

## Archived 2026-01-23 (Session 3)

- [x] Simplified steal system - removed button mashing, instant steal attempts (33% base, 50% if charging)
- [x] Added 1-second no-stealback cooldown for steal victims
- [x] Both players have independent AI profiles (LT selects player, RT cycles profile)
- [x] Added Observer mode to player control cycling (Left → Right → Observer → Left)
- [x] Game Presets system - Movement/Ball/Shooting/Global presets with hot-reload

---

## Archived 2026-01-23 (Session 2)

- [x] Steal Phase 1: Pushback on successful steal (STEAL_PUSHBACK_STRENGTH)
- [x] Steal Phase 1: Randomness in contests (STEAL_RANDOM_BONUS)
- [x] Steal Phase 1: Charging penalty (STEAL_CHARGING_PENALTY)
- [x] Steal Phase 1: Cooldown to prevent spam (STEAL_COOLDOWN + StealCooldown component)
- [x] AI enhancement Phase 1: Renamed `AiInput` → `InputState` (unified input buffer)
- [x] AI enhancement Phase 2: Auto-reload config files every 10s (replaced F2 hotkey)
- [x] AI enhancement Phase 3: Created `assets/ai_profiles.txt` with 10 AI personas
- [x] AI enhancement Phase 4: AI profile cycling (D-pad) + random profile on reset (R key)

---

## Archived 2026-01-23 (Session 1)

- [x] Split main.rs into modules (2624 lines → 18 focused files, no module >500 lines)
- [x] Fix viewport and arena wall size (1600×900 window, 1:1 camera, 20px walls, world-space UI)
- [x] Remove possession ball texture swapping, add 10 color palettes that cycle on reset (affects ball, players, baskets)
- [x] Fix jumping not working (input systems needed .chain() for guaranteed order)
- [x] Fix copy_human_input zeroing jump buffer on first press frame
- [x] Fix ball duplication when switching from debug to non-debug levels
- [x] Fix goal flash resetting to hardcoded color instead of current palette
- [x] Expand palette system to 20 palettes with background/floor/platform colors
- [x] Create assets/palettes.txt file for editable color definitions
- [x] Fix AI not activating when switching from debug to non-debug levels
- [x] Parameterized rim colors (`left_rim`, `right_rim`) in palette file
- [x] Fixed platform/step colors to match walls (query filter bug)
- [x] Fixed viewport cycling to return to spawn size (scale_factor_override)
- [x] Fixed player spawn bug (use Team component instead of X position)
- [x] Added charge gauge to both players
- [x] Renamed palette field `floor` → `platforms`
- [x] Make rim bouncier like steps
- [x] Reorganized 30 color palettes (9 favorites at front, 11 variations, 10 wild)
- [x] Added names to all palettes in assets/palettes.txt
- [x] Made palette count fully dynamic (no hardcoded NUM_PALETTES)
- [x] Created 10 ball styles: wedges, half, spiral, checker, star, swirl, plasma, shatter, wave, atoms
- [x] Made ball styles data-driven via assets/ball_options.txt
- [x] Debug level spawns all ball styles dynamically
- [x] Removed old ball texture files (stripe, dot, ring, solid)
