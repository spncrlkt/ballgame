# Code Review Audit Log

Results from code reviews performed during audits.

---

## 2026-01-24

### 1. Duplication (4 issues)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `main.rs:482-586` + `player/physics.rs:403-473` | Ball spawning logic duplicated between setup and level change | Extract common patterns into shared helpers |
| `ui/tweak_panel.rs:69-127` | Three large match statements with identical 14-case patterns | Consolidate into single data structure |
| `ui/debug.rs:330-341` | Four separate identical gamepad D-pad input checks | Extract helper for collecting D-pad directions |
| `ui/debug.rs:428-496` | Identical cycle wrapping logic (`(i+1)%len`) repeated | Extract `cycle_index(current, forward, max)` helper |

### 2. Complexity (4 issues)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `ui/debug.rs:298-550+` | `unified_cycle_system` handles too many concerns (857 line file) | Split into direction selection and per-direction cycling systems |
| `player/physics.rs:242-401` | `respawn_player` handles reset AND level change | Split into `handle_reset` and `handle_level_change` |
| `shooting/throw.rs:108-150` | 13 variance variables with multiple accumulations | Extract `calculate_shot_variance()` returning a struct |
| `ball/physics.rs:120-226` | 5 separate bounce calls with similar patterns | Extract `bounce_with_type()` helper |

### 3. Naming (3 issues)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `player/components.rs` | `HoldingBall` name unclear (marker with entity reference) | Consider `BallHolder` or `CarryingBall(Entity)` |
| `ball/components.rs` + `scoring/mod.rs` | `CurrentPalette`/`CurrentLevel` ambiguous | Rename to `PaletteIndex`/`LevelNumber` |
| `ui/debug.rs:64-70` | `DownOption::next()` confusing (Down is D-pad direction) | Rename to `cycle()` |

### 4. Structure (3 issues)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `ui/debug.rs:23-125` | Cycle enum types mixed with system functions | Move to new `src/ui/cycle.rs` module |
| `input/mod.rs`, `ai/mod.rs`, `ball/interaction.rs` | Input buffering consumption scattered | Add `src/input/buffering.rs` submodule |
| `main.rs:81` + `palettes/database.rs:65-87` | Palette loading responsibility unclear | Ensure all loading goes through database module |

### 5. Pattern Violations (1 issue)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `ui/debug.rs:294` | `STICK_ACTIVE_DEADZONE` magic number not in constants.rs | Move to constants.rs |

### Summary

| Category | Count |
|----------|-------|
| Duplication | 4 |
| Complexity | 4 |
| Naming | 3 |
| Structure | 3 |
| Pattern Violations | 1 |
| **Total** | **15** |

**Note:** No input buffering or frame-rate independence violations. Two clippy errors fixed during audit (never_loop in simulation/runner.rs and training.rs). Regression baseline updated (debug level with all ball styles).

---

## 2026-01-23 (Session 2)

### 1. Duplication (2 issues)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `main.rs:318-373` + `player/physics.rs:359-427` | Ball spawning logic still duplicated | Extract to `ball/spawning.rs` |
| `helpers.rs` | `apply_bounce_deflection` created but not used everywhere | Replace all inline bounce calculations |

### 2. Complexity (1 issue)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `player/physics.rs:198-356` | `respawn_player` still 150+ lines | Split into `reset_game()` and `change_level()` |

### 3. Naming (1 issue)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `presets/types.rs` | `CompositePreset` used internally but displayed as "Global" | Consider renaming to `GlobalPreset` |

### 4. Structure (2 issues)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `ui/debug.rs` | 540+ lines with cycling, debug, viewport, presets all mixed | Split into `ui/cycle.rs` and keep debug.rs focused |
| `ball/components.rs:86` | `CurrentPalette` still in ball module | Move to `palettes/mod.rs` |

### 5. Pattern Violations (0 issues)

No new pattern violations found. All input buffering, frame-rate independence, and collision epsilon patterns are correctly followed.

### Summary

| Category | Count |
|----------|-------|
| Duplication | 2 |
| Complexity | 1 |
| Naming | 1 |
| Structure | 2 |
| Pattern Violations | 0 |
| **Total** | **6** |

**Note:** This session focused on steal simplification and preset system completion. The issues identified are pre-existing technical debt, not new problems.

---

## 2026-01-23

### 1. Duplication (5 issues)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `main.rs:318-373` + `player/physics.rs:383-449` | Ball spawning logic duplicated in `setup()` and `spawn_balls()` | Move `spawn_balls` to `ball/mod.rs` or `levels/spawning.rs` and share |
| `ball/physics.rs:133-254` | Bounce deflection calculation repeated 6 times | Extract `apply_deflect_bounce()` helper |
| `player/physics.rs:236-244` + `ui/debug.rs:284-297` | Level cycling bounds logic duplicated | Add `CurrentLevel::cycle_next()`/`cycle_prev()` methods |
| `levels/spawning.rs:63-138` | Left/right corner ramp spawning loops nearly identical | Extract helper with `side: f32` parameter |
| `main.rs:21` + `config_watcher.rs:20` | `BALL_OPTIONS_FILE` constant defined twice | Move to `constants.rs` |

### 2. Complexity (4 issues)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `ui/debug.rs:416-466` | `apply_palette_colors` has 12 query parameters | Split into smaller systems per entity type |
| `ui/tweak_panel.rs:69-127` | `PhysicsTweaks` uses error-prone index-based get/set | Use enum or param struct array |
| `player/physics.rs:198-380` | `respawn_player` is 180+ lines doing too much | Split into `handle_reset()` and `handle_level_change()` |
| `ai/decision.rs:97-118` | Nested if-else chains in goal selection | Use match with guards or early returns |

### 3. Naming (5 issues)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `ball/components.rs:104` | `BallPlayerContact.overlapping` sounds like verb | Rename to `is_overlapping` |
| `scoring/mod.rs:21` | `CurrentLevel` is 1-indexed, causing conversions everywhere | Store 0-indexed or add `index()`/`display_number()` helpers |
| `player/components.rs:47` + `world/mod.rs:28` | `Team::Left` and `Basket::Left` confusing (Left team scores in Right basket) | Rename baskets to `LeftSide`/`RightSide` or document |
| `ball/components.rs:119` | `BallShotGrace` sounds like state but holds timer | Rename to `BallShotGraceTimer` |
| `ui/tweak_panel.rs:22` | `shot_max_power` defined but unused | Remove or wire up to throw system |

### 4. Structure (5 issues)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `ball/components.rs:86` | `CurrentPalette` is global color resource in ball module | Move to `palettes/mod.rs` |
| `player/physics.rs:383-449` | `spawn_balls` helper spawns balls but lives in player module | Move to `ball/mod.rs` or `levels/spawning.rs` |
| `lib.rs:53-139` | `ShotTrajectory` and `calculate_shot_trajectory` in lib.rs | Move to `shooting/mod.rs` |
| `player/physics.rs:335-378` + `config_watcher.rs:120-153` | Level geometry update logic duplicated | Extract shared `apply_level_geometry()` in `levels/spawning.rs` |
| `ui/debug.rs` | 540+ lines with cycling, debug, viewport all mixed | Split into `ui/cycle.rs`, `ui/debug.rs`, `ui/viewport.rs` |

### 5. Pattern Violations (5 issues)

| Location | Problem | Suggested Fix |
|----------|---------|---------------|
| `main.rs:24-45` | `load_ball_style_names()` reads file directly instead of using database pattern | Create `BallStyleDatabase` resource |
| `ui/tweak_panel.rs:232-236` | Tweak panel formatting indices wrong (5,6,7,10 don't match fields) | Fix indices or use per-param format |
| `levels/database.rs:206-214` | `LevelDatabase` has `len()` but no `is_empty()` | Add `is_empty()` method |
| `ball/components.rs:57-59` | `BallTextures` has `len()` but no `is_empty()` | Add `is_empty()` method |
| `main.rs:28` | Unnecessary `return` in closure | Remove `return` keyword |

### Summary

| Category | Count |
|----------|-------|
| Duplication | 5 |
| Complexity | 4 |
| Naming | 5 |
| Structure | 5 |
| Pattern Violations | 5 |
| **Total** | **24** |

**Priority fixes:**
1. Extract bounce deflection helper (~60 lines duplication)
2. Extract level geometry update helper (duplication in respawn + config reload)
3. Move `CurrentPalette` to palettes module
4. Fix tweak panel formatting indices (potential bug)
5. Add missing `is_empty()` methods (clippy compliance)

