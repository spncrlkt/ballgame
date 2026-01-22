# Audit Record

Record of changes and audit findings for the ballgame project.

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
