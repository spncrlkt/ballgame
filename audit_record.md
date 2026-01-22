# Audit Record

Record of changes and audit findings for the ballgame project.

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
