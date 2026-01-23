# Audit Record

Record of changes and audit findings for the ballgame project.

---

## Audit: 2026-01-23 (Session 2) - Steal Simplification & Presets Complete

### Session Summary

Major system overhaul: simplified steal mechanics, completed game presets system, added observer mode.

### Changes Made

**Steal System Simplification:**
- Removed button-mashing contest (14 constants deleted)
- Implemented instant steal attempts with 33% base success chance
- +17% bonus if defender is charging (50% total)
- 0.3s cooldown between attempts, 1s victim no-stealback cooldown
- Simplified `StealContest` resource to just feedback (fail flash)
- Removed mashing logic from AI decision system
- Visual feedback: cooldown indicator + fail flash

**Game Presets System:**
- Created `src/presets/` module (types.rs, database.rs, apply.rs)
- Hierarchical presets: Movement, Ball, Shooting, Global (composite)
- Global presets can set all options including level, palette, ball_style
- 6 Movement presets: Default, Floaty, Responsive, Heavy, Slippery, Precise
- 6 Ball presets: Default, Bouncy, Heavy, Floaty, Pinball, Dead
- 6 Shooting presets: Default, Quick, Power, Wild, Sniper, Spam
- 6 Global presets: Default, Arcade, Realistic, Floaty, Chaos, Tactical
- Hot-reload support via ConfigWatcher

**Cycle System Updates:**
- Global preset is now first/default option
- Reordered: Global → Level → AI Profile → Palette → Ball Style → Viewport → Movement → Ball → Shooting
- D-pad Up cycles backwards through list
- AI Profile: LT selects player (Left/Right), RT cycles profile
- Tab toggles both debug UI and cycle indicator visibility

**Observer Mode:**
- Added to swap_control: Left → Right → Observer → Left
- Observer mode: both players controlled by AI, human spectates

**Other Changes:**
- Removed 3 smallest viewport presets (800x450, 1024x576, 1280x720)
- Palette 26 is now default
- Both players have independent AI profiles

### Audit Findings

| Check | Status | Notes |
|-------|--------|-------|
| CLAUDE.md accuracy | UPDATED | Added presets module, StealCooldown, updated descriptions |
| Input buffering | PASS | All patterns correct |
| Constants | PASS | All steal constants consolidated, no magic numbers |
| System order | PASS | FixedUpdate chain matches main.rs |
| Unused code | PASS | Removed old steal mashing code |
| Pattern violations | PASS | No raw input in FixedUpdate |
| Collision epsilon | N/A | No new collision code |
| Frame-rate physics | PASS | New timers use delta_secs() |
| Compilation | PASS | `cargo check` clean |
| Clippy | WARN | ~10 warnings (type_complexity, collapsible_if - standard) |

### Files Created

- `src/presets/mod.rs` - Presets module root
- `src/presets/types.rs` - Preset type definitions
- `src/presets/database.rs` - PresetDatabase with file parsing
- `src/presets/apply.rs` - Preset application logic
- `src/ui/steal_indicators.rs` - Simplified steal UI

### Files Modified

- `src/steal.rs` - Simplified to cooldown + feedback only
- `src/ball/interaction.rs` - Instant steal logic in pickup_ball
- `src/ai/mod.rs` - Observer mode, removed mashing fields
- `src/ai/decision.rs` - Removed steal mashing logic
- `src/shooting/throw.rs` - Removed contest blocking
- `src/ui/debug.rs` - Cycle system updates, Global first
- `src/constants.rs` - New steal constants, removed old ones
- `src/main.rs` - Added preset resources and systems
- `src/lib.rs` - Added preset exports
- `assets/game_presets.txt` - Full preset definitions
- `CLAUDE.md` - Updated architecture documentation
- `todo.md` - Updated with completed items
- `milestones.md` - Marked stealing mechanics complete
- `code_review_audits.md` - Added session 2 findings
- `audit_record.md` - This entry

---

## Audit: 2026-01-23 (Session 1) - AI Enhancement Plan Complete

### Session Summary

Implemented the complete AI enhancement plan (4 phases):
1. Renamed `AiInput` → `InputState`
2. Added auto-reload config watcher (replaced F2 hotkey)
3. Created AI profiles system with 10 personas
4. Added profile cycling and random profile on reset

### Changes Made

**Phase 1: Rename AiInput → InputState**
- Renamed component to better reflect its purpose (unified input buffer for all players)
- Updated all files: `ai/mod.rs`, `ai/decision.rs`, `player/physics.rs`, `shooting/throw.rs`, `shooting/charge.rs`, `steal.rs`, `ball/interaction.rs`, `main.rs`, `lib.rs`
- Updated documentation to reflect new naming

**Phase 2: Auto-Reload Config Files**
- Created `src/config_watcher.rs` with `ConfigWatcher` resource
- Polls config files every 10 seconds, reloads on change:
  - `assets/levels.txt`
  - `assets/palettes.txt`
  - `assets/ai_profiles.txt`
  - `assets/ball_options.txt` (logs only - requires restart)
- Removed F2 hotkey from `src/levels/mod.rs`

**Phase 3: AI Profiles System**
- Created `src/ai/profiles.rs` with `AiProfile` struct and `AiProfileDatabase` resource
- Created `assets/ai_profiles.txt` with 10 AI personalities:
  - Balanced, Aggressive, Defensive, Sniper, Rusher
  - Turtle, Chaotic, Patient, Hunter, Goalie
- Each profile has: position_tolerance, shoot_range, charge_min, charge_max, steal_range, defense_offset
- Added `profile_index` to `AiState` component
- Updated `ai_decision_update` to use per-player profile values
- Removed unused AI constants from `constants.rs`

**Phase 4: Profile Cycling + Random on Reset**
- Added `AiProfile` to `CycleTarget` enum (now 5 targets)
- D-pad Down cycles through: Level → Viewport → Palette → Ball Style → AI Profile
- RT/LT cycles the AI-controlled player's profile
- R key (reset) randomizes AI profile

**Documentation Updates**
- Updated `CLAUDE.md`:
  - Added `AiProfileDatabase` and `ConfigWatcher` to Resources
  - Added `InputState` and `AiState` to Player Components
  - Updated cycle system documentation
  - Changed R key description to mention profile randomization
- Updated `todo.md`:
  - Added Simulation Engine section with automated testing benefits
  - Added Stealing Mechanics section
  - Moved completed AI phases to Done

### Audit Findings

| Check | Status | Notes |
|-------|--------|-------|
| CLAUDE.md accuracy | PASS | Updated with new components and systems |
| Input buffering | PASS | All `just_pressed` in Update systems |
| Constants | PASS | Removed unused AI_ constants, no magic numbers |
| System order | PASS | FixedUpdate chain matches documentation |
| Unused code | PASS | Clean compilation |
| Pattern violations | PASS | No raw input in FixedUpdate |
| Collision epsilon | N/A | No new collision code |
| Frame-rate physics | PASS | No new physics code |
| Compilation | PASS | `cargo check` clean |
| Clippy | WARN | ~30 warnings (type_complexity, standard Bevy patterns) |

### Files Created

- `src/config_watcher.rs` - Config file auto-reload system
- `src/ai/profiles.rs` - AI profile parsing and database
- `assets/ai_profiles.txt` - 10 AI personality definitions

### Files Modified

- `src/ai/mod.rs` - Added profiles module, renamed AiInput→InputState, added profile_index
- `src/ai/decision.rs` - Uses profile values instead of constants
- `src/player/physics.rs` - Random profile on reset
- `src/shooting/throw.rs`, `charge.rs` - InputState rename
- `src/steal.rs` - InputState rename
- `src/ball/interaction.rs` - InputState rename
- `src/levels/mod.rs` - Removed F2 reload (now in config_watcher)
- `src/ui/debug.rs` - Added AiProfile to CycleTarget
- `src/main.rs` - Added ConfigWatcher and AiProfileDatabase resources
- `src/lib.rs` - Added exports for new types
- `src/constants.rs` - Removed unused AI_ constants
- `CLAUDE.md` - Updated architecture documentation
- `todo.md` - Updated with new sections and completed items
- `audit_record.md` - This entry

### Files Deleted

- `~/.claude/plans/eager-floating-scone.md` - Completed plan file

---

## Audit: 2026-01-22 (Session 3)

### Session Summary

Major refactoring: palette system expansion, multiple bug fixes, and module restructuring.

### Changes Made

**Jump System Fix:**
- Added `.chain()` to Update input systems to guarantee execution order
- Order: `capture_input` → `copy_human_input` → `swap_control` → `ai_decision_update`
- Without chaining, `copy_human_input` could run before `capture_input`, losing jump input

**Jump Buffer Fix:**
- Removed faulty consumption-sync logic in `copy_human_input`
- Old logic incorrectly zeroed jump buffer on the first press frame
- Now simply copies `PlayerInput.jump_buffer_timer` to `AiInput.jump_buffer_timer`

**Ball Spawning Fix:**
- Fixed ball duplication when switching from debug to non-debug levels
- Now despawns ALL balls and respawns correct count for new level
- Debug levels: 6 balls with different styles
- Normal levels: 1 ball with default style

**Goal Flash Fix:**
- Updated `check_scoring` to use `PaletteDatabase` instead of hardcoded `TEAM_LEFT_PRIMARY`/`TEAM_RIGHT_PRIMARY`
- Flash now returns to correct palette color

**Palette System Expansion:**
- Created new `src/palettes/` module (mod.rs, database.rs)
- Extended `Palette` struct with `background`, `floor`, `platform` colors
- Expanded from 10 to 20 palettes
- Palettes loaded from `assets/palettes.txt` (creates default file if missing)
- Background (`ClearColor`) now changes with palette
- Floor/wall/platform colors change with palette
- Updated `spawn_level_platforms` and `spawn_corner_ramps` to accept color parameters

**AI Goal Fix:**
- Added logic to reset AI goals when switching levels
- Debug levels: AI set to `AiGoal::Idle`
- Normal levels: AI set to `AiGoal::ChaseBall` (default)

**Ball Textures:**
- Updated `generate_ball.rs` to 20 palettes
- Generated 120 textures (6 styles × 20 palettes)

### Audit Findings

| Check | Status | Notes |
|-------|--------|-------|
| CLAUDE.md accuracy | NEEDS UPDATE | Add palettes module, update NUM_PALETTES |
| Input buffering | PASS | All `just_pressed` in Update, properly buffered |
| Constants | PASS | No magic numbers in new code |
| System order | PASS | Update systems chained where needed |
| Unused code | PASS | Clean compilation |
| Pattern violations | PASS | No raw input in FixedUpdate |
| Collision epsilon | N/A | No new collision code |
| Frame-rate physics | PASS | No new physics code |
| Compilation | PASS | Builds successfully |
| Clippy | WARN | 44 warnings (type_complexity, standard Bevy patterns) |

### Files Modified

- `src/main.rs` - Palette loading, system chaining, initial colors
- `src/lib.rs` - Added palettes module export
- `src/constants.rs` - Removed old PALETTES array, kept DEFAULT_ colors
- `src/palettes/mod.rs` - New module
- `src/palettes/database.rs` - PaletteDatabase implementation (20 palettes)
- `src/ball/components.rs` - Uses NUM_PALETTES from palettes module
- `src/player/physics.rs` - Ball respawning, AI goal updates, palette colors
- `src/scoring/mod.rs` - Uses PaletteDatabase for flash colors
- `src/levels/mod.rs` - Uses palette colors for reload
- `src/levels/spawning.rs` - Added color parameters to spawn functions
- `src/ai/mod.rs` - Removed faulty jump buffer sync logic
- `src/bin/generate_ball.rs` - Updated to 20 palettes
- `todo.md` - Added completed items
- `audit_record.md` - This entry

### Files Created

- `src/palettes/mod.rs`
- `src/palettes/database.rs`
- `assets/palettes.txt` (on first run)
- 60 new ball texture PNGs (palettes 10-19)

---

## Audit: 2026-01-22 (Session 2)

### Session Summary

Routine audit with no gameplay code changes since last session.

### Changes Made

**CLAUDE.md Fix:**
- Added missing `BasketRim` component to World Components section
- Added missing `CornerRamp` component to World Components section

### Audit Findings

**Compilation:** Clean `cargo check`, no errors

**Clippy:** 22 warnings (all style suggestions, down from 24):
- 1x `derivable_impls` - LevelDatabase::Default can use derive
- 7x `collapsible_if` - nested if statements
- 1x `trim_split_whitespace` - unnecessary trim before split_whitespace
- 6x `type_complexity` - complex Query types (standard for Bevy)
- 1x `too_many_arguments` - respawn_player with 10 args
- 1x `collapsible_else_if`
- 1x `manual_range_patterns` - `5 | 6 | 7` can be `5..=7`

**CLAUDE.md:** Was missing `BasketRim` and `CornerRamp` components - now fixed

**Input Buffering:** Correct
- All `just_pressed` inputs captured in `capture_input` (Update)
- Buffered via `PlayerInput` resource fields
- Consumed in FixedUpdate systems:
  - `apply_input` consumes `jump_buffer_timer` (line 1152)
  - `pickup_ball` consumes `pickup_pressed` (lines 1527, 1537)
  - `throw_ball` consumes `throw_released` (line 1743)
  - `cycle_target` consumes `cycle_target_pressed` (line 1644)

**Frame-Rate Independence:** Correct
- Gravity: `* time.delta_secs()` (lines 1178, 1283)
- Friction: `.powf(time.delta_secs())` (lines 1279, 1285)
- Acceleration: `move_toward(..., rate * time.delta_secs())` (line 1129)
- Timers: `- time.delta_secs()` (lines 1143, 1270, 1591, 1629, 1959, 2033)

**Collision Epsilon:** Correct
- Player floor landing: line 1234-1235 uses `- COLLISION_EPSILON`
- Ball floor landing: line 1360-1361 uses `- COLLISION_EPSILON`
- All resting entities properly embedded into platforms

**System Order:** Matches CLAUDE.md documentation exactly
- Update: 11 systems
- FixedUpdate: 15 systems chained

**No Dead Code:** No unused code warnings from clippy

**No Pattern Violations:** No raw input reads in FixedUpdate systems

### Files Modified

- `CLAUDE.md` - Added BasketRim and CornerRamp to World Components
- `audit_record.md` - This entry

### Code Stats

- `src/main.rs`: ~2367 lines (over the 2000 line threshold - consider modularization)
- `src/lib.rs`: 146 lines (shared trajectory calculation)

---

## Audit: 2026-01-22

### Session Summary

Routine audit with no code changes since last session.

### Changes Made

**CLAUDE.md Fix:**
- Added missing `LevelDatabase` resource to the Resources section

### Audit Findings

**Compilation:** Clean `cargo check`, no errors

**Clippy:** 24 warnings (all style suggestions, same as previous audit):
- 1x `derivable_impls` - LevelDatabase::Default can use derive
- 8x `collapsible_if` - nested if statements
- 2x `trim_split_whitespace` - unnecessary trim before split_whitespace
- 7x `type_complexity` - complex Query types (standard for Bevy)
- 2x `too_many_arguments` - functions with 9 args
- 1x `collapsible_else_if`
- 1x `manual_range_patterns` - `5 | 6 | 7` can be `5..=7`

**CLAUDE.md:** Was missing `LevelDatabase` resource - now fixed

**Input Buffering:** Correct
- All `just_pressed` inputs captured in `capture_input` (Update)
- Buffered via `PlayerInput` resource
- Consumed in FixedUpdate systems (`apply_input`, `pickup_ball`, `throw_ball`)

**Frame-Rate Independence:** Correct
- Gravity: `* time.delta_secs()` (lines 971, 1071)
- Friction: `.powf(time.delta_secs())` (lines 1067, 1073)
- Acceleration: `* time.delta_secs()` (line 922)
- Timers: `- time.delta_secs()` (lines 812, 936, 1058, 1338, 1678)

**Collision Epsilon:** Correct
- Player floor landing: line 1023 uses `- COLLISION_EPSILON`
- Ball floor landing: line 1126 uses `- COLLISION_EPSILON`
- All resting entities properly embedded into platforms

**System Order:** Matches CLAUDE.md documentation exactly

**No Dead Code:** Previous `save_to_file` removal confirmed

**No Pattern Violations:** No raw input reads in FixedUpdate systems

### Files Modified

- `CLAUDE.md` - Added LevelDatabase to Resources
- `audit_record.md` - This entry

### Code Stats

- `src/main.rs`: 2034 lines (approaching 2000 line threshold mentioned in Future Plans)

---

## Audit: 2026-01-21 (Session 3)

### Session Summary

Complete trajectory system overhaul with optimal angle calculation and ceiling awareness.

### Changes Made

**Dynamic Trajectory System:**
- Replaced fixed arc with `calculate_shot_trajectory()` function
- Calculates optimal angle using `θ = 45° + arctan(dy/dx)/2`
- Respects ceiling constraints with binary search for max arc
- Returns power, arc, and variance penalties
- New `ShotTrajectory` struct holds calculation results

**New Shot Constants:**
- `SHOT_MAX_SPEED = 800` - Caps total velocity magnitude (prevents rocket shots)
- `SHOT_MIN_ARC = 0.5` - Minimum arc ratio (~27° flat shot)
- `SHOT_MAX_ARC = 3.0` - Maximum arc ratio (~72° lob shot)
- `SHOT_CEILING_MARGIN = 60` - Stay this far below ceiling
- `SHOT_DISTANCE_VARIANCE_FACTOR = 0.0003` - +30% variance at 1000 units
- `SHOT_ARC_VARIANCE_FACTOR = 0.15` - Variance per unit arc deviation

**Removed Constants:**
- `SHOT_MIN_POWER` - No longer used (trajectory calculates exact power needed)
- `SHOT_BASE_ARC` - Replaced by dynamic optimal arc calculation
- `SHOT_AIR_POWER_PENALTY` - Removed, using variance-only difficulty

**Auto-Aim for All Shots:**
- Both grounded and airborne shots use auto-aim trajectory
- Difficulty comes from variance penalties, not power reduction

**Variance System:**
- Base: 50% at zero charge → 2% at full charge
- Air penalty: +10%
- Movement penalty: +10% at full speed
- Distance penalty: proportional to shot distance
- Arc penalty: when forced away from optimal angle

**Speed Cap:**
- Total ball speed capped at `SHOT_MAX_SPEED`
- Prevents extreme velocities for near-vertical shots
- Scales vx and vy proportionally to preserve direction

### Audit Findings

**Compilation:** Clean `cargo check`, no errors

**Clippy:** 24 warnings (all style suggestions):
- 1x `derivable_impls` - LevelDatabase::Default can use derive
- 8x `collapsible_if` - nested if statements
- 2x `trim_split_whitespace` - unnecessary trim
- 7x `type_complexity` - complex Query types (standard for Bevy)
- 2x `too_many_arguments` - functions with 9 args
- 1x `collapsible_else_if`
- 1x `manual_range_patterns`

**No Dead Code:** Previous `save_to_file` was removed

**Input Buffering:** Correct - all `just_pressed` in Update systems

**Frame-Rate Independence:** Correct
- Gravity: `* time.delta_secs()`
- Friction: `.powf(time.delta_secs())`
- Acceleration: `* time.delta_secs()`
- Timers: `- time.delta_secs()`

**Collision Epsilon:** Correct - used in all ground contact positioning

**System Order:** Matches CLAUDE.md documentation

**CLAUDE.md:** Accurate - architecture section matches code

### Files Modified

- `src/main.rs` - Trajectory system overhaul
- `audit_record.md` - This entry

---

## Audit: 2026-01-21 (Session 2)

### Session Summary

Major shooting mechanics overhaul and movement system improvement.

### Changes Made

**Auto-Aim Shooting System:**
- Added `calculate_perfect_shot_power()` function using projectile motion physics
- Formula: `vx = sqrt(g * dx² / (2 * (arc * dx - dy)))`
- Grounded shots auto-aim to target basket based on facing direction
- Friction compensation for long-distance shots (symmetric around 75%, range 50%-100%)

**Progressive Variance System:**
- Replaced `SHOT_MAX_RANDOMNESS` with progressive variance
- `SHOT_MAX_VARIANCE = 0.50` (50% at zero charge)
- `SHOT_MIN_VARIANCE = 0.02` (2% at full charge)
- Variance applied to both angle and power
- Every shot has some variance, even at full charge

**Shooting Penalties:**
- `SHOT_AIR_VARIANCE_PENALTY = 0.10` (10% for airborne shots)
- `SHOT_MOVE_VARIANCE_PENALTY = 0.10` (10% at full horizontal speed, proportional)
- Penalties stack: grounded+stationary = 2%, airborne+moving = 22%

**Charge Time:**
- `SHOT_CHARGE_TIME` reduced from 2.0s to 1.6s

**Acceleration-Based Movement:**
- Replaced instant velocity assignment with acceleration/deceleration system
- Added `move_toward()` helper function
- New constants:
  - `GROUND_ACCEL = 2400` (snappy start)
  - `GROUND_DECEL = 1800` (slight slide when stopping)
  - `AIR_ACCEL = 1500` (committed but adjustable jumps)
  - `AIR_DECEL = 900` (momentum preserved in air)
- All four values added to `PhysicsTweaks` for runtime tuning
- Movement feels smoother and more natural

### Audit Findings

**Compilation:** Clean `cargo check`, no errors

**Clippy:** 25 warnings (all style, not bugs):
- 1x `dead_code` - `save_to_file` method never used
- 1x `derivable_impls` - `LevelDatabase::Default` can use derive
- 8x `collapsible_if` - nested if statements
- 2x `trim_split_whitespace` - unnecessary trim before split
- 7x `type_complexity` - complex query types (standard for Bevy)
- 2x `too_many_arguments` - functions with 9 args
- 1x `collapsible_else_if` - else { if } can collapse
- 1x `manual_range_patterns` - `5 | 6 | 7` can be `5..=7`

**Dead Code:** `LevelDatabase::save_to_file()` is never used (line 467)

**Minor Magic Number:** Ball shot grace period `0.1` (line 1487) could be a constant

**Input Buffering:** Correct - all press inputs in Update, consumed in FixedUpdate

**Frame-Rate Independence:** Correct
- Friction uses `.powf(time.delta_secs())`
- Gravity/velocity uses `* time.delta_secs()`
- New acceleration uses `* time.delta_secs()`

**Collision Epsilon:** Correct - used in all ground contact positioning

**System Order:** Correct chain in FixedUpdate

**CLAUDE.md:** Accurate - no updates needed, architecture matches code

### Files Modified

- `src/main.rs` - Shooting and movement systems
- `audit_record.md` - This entry

---

## Audit: 2026-01-21

### Session Summary

Major feature additions and refinements to the ball sport game.

### Changes Made

**Level System:**
- Expanded from 5 to 10 symmetric levels with named configurations
- Added `LEVEL_NAMES` constant array and `NUM_LEVELS = 10`
- All levels use horizontal symmetry via `spawn_mirrored_platform()` helper
- Added `LevelPlatform` component to mark despawnable level platforms

**Scoring:**
- Carrying ball into goal now scores 2 points (throw-in scores 1)
- Added `ScoreFlash` component with timer, flash_color, original_color
- Gold flash for 2-point carry-in, white flash for 1-point throw
- Both basket and player flash on carry-in score

**Ball Pickup Pulse:**
- Changed to 5 cycles/second (was 3)
- Implemented dark→regular→light→regular color pattern using `-cos(t)`
- Color interpolation: dark orange (0.5, 0.25, 0.05) ↔ regular (0.9, 0.5, 0.1) ↔ light (0.95, 0.75, 0.55)
- Size pulse reduced to ±3%

**Ball Position & Visuals:**
- Ball now positioned inside player rectangle at facing side, middle height
- Removed direction arrow (`FacingArrow` component and `update_facing_arrow` system deleted)
- Charge gauge moved inside player, opposite side of ball

**Post-Shot Grace Period:**
- Added `BallShotGrace` component with 100ms timer
- During grace: no friction applied, no player collision drag
- Prevents immediate slowdown after shooting

**Physics Tweak Panel:**
- Adjustment increments changed to ~10% of default value
- Added `R` to reset selected parameter to default
- Added `Shift+R` to reset all parameters to defaults
- Modified parameter names highlighted in red when value differs from default
- Added helper methods: `get_default_value()`, `is_modified()`, `reset_value()`, `reset_all()`, `get_step()`

**Debug Display:**
- Level name now shown alongside level number (e.g., "Lv:3/10 Tower")

### Audit Findings

**Compilation:** Clean `cargo check`, no errors

**Clippy:** 10 warnings (all style, not bugs):
- 2x `collapsible_if` - nested if statements can be collapsed
- 8x `type_complexity` - complex query types (standard for Bevy)

**Input Buffering:** Correct - all press inputs buffered in `PlayerInput` resource, consumed in FixedUpdate

**Frame-Rate Independence:** Correct
- Friction uses `.powf(time.delta_secs())`
- Gravity/velocity uses `* time.delta_secs()`

**Collision Epsilon:** Correct - `COLLISION_EPSILON` used for skin width in all ground contact positioning

**System Order:** Correct chain in FixedUpdate

**CLAUDE.md:** Updated with:
- Added resources: `CurrentLevel`, `PhysicsTweaks`
- Added ball components: `BallRolling`, `BallShotGrace`
- Added world component: `LevelPlatform`
- Fixed UI components: removed `FacingArrow`, added `TweakPanel`, `TweakRow`, `ScoreFlash`
- Updated system schedules
- Added tweak panel input documentation
- Added post-audit note about compacting and audit_record.md

### Files Modified

- `src/main.rs` - All feature changes
- `CLAUDE.md` - Architecture documentation updated
- `audit_record.md` - Created (this file)

---
