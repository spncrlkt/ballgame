# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build           # Debug build
cargo build --release # Release build
cargo run             # Run the game
cargo run -- --replay <file.evlog>  # Run in replay mode
cargo run --bin training            # Run training mode (5 games vs AI)
cargo run --bin training -- --games 3 --profile Aggressive  # Custom training
cargo check           # Check compilation without building
cargo fmt             # Format code
cargo clippy          # Lint code
```

```bash
cargo run --bin test-scenarios           # Run all scenario tests
cargo run --bin test-scenarios -- ball/  # Run category
cargo run --bin test-scenarios -- -v     # Verbose mode (shows failures)
```

Dynamic linking is disabled in `.cargo/config.toml` to avoid macOS dyld issues.

**Binary compatibility:** When updating the main game (`src/main.rs`), also update the other binaries to keep them compatible:
- `src/bin/training.rs` - Training mode (1v1 vs AI with logging)
- `src/bin/test-scenarios.rs` - Scenario test runner

Features like countdown, new resources, or system changes should be propagated to these binaries.

**User Preference:** Do not add or commit code automatically. The user handles git operations.

## Session Checklists

### Get Started

Run at the beginning of each working session:

- [ ] **Read `todo.md`** - Check current sprint tasks and priorities
- [ ] **Read `open_questions.md`** - Review pending questions/decisions
- [ ] **Check git status** - Note any uncommitted work from previous session
- [ ] **Run `cargo check`** - Verify codebase compiles
- [ ] **Run `./scripts/regression.sh`** - Verify visual baseline still matches
- [ ] **Identify scope** - Decide which task(s) to work on this session

### Close Down

Run at the end of each working session (or after ~10 changes):

- [ ] **Run `cargo check`** - Verify compilation
- [ ] **Run `cargo clippy`** - Check for new warnings
- [ ] **Run `./scripts/regression.sh`** - Visual regression test
- [ ] **Update baseline if needed** - `./scripts/regression.sh --update` (if UI changed intentionally)
- [ ] **Review screenshot** - Read `regression/current.png` to verify UI looks correct
- [ ] **Update `todo.md`** - Mark completed items, add new items discovered
- [ ] **Archive done items** - Keep only last 5 in todo.md, move older to `todone.md`
- [ ] **Update `open_questions.md`** - Add any new questions or decisions needed
- [ ] **Update `audit_record.md`** - Document changes and findings
- [ ] **Verify CLAUDE.md accuracy** - Update if architecture changed
- [ ] **Commit changes** - With descriptive message

### Before Starting a Feature

- [ ] **Understand the goal** - What problem are we solving?
- [ ] **Check existing code** - Read related files before modifying
- [ ] **Identify affected systems** - Which modules/components will change?
- [ ] **Consider patterns** - Follow patterns documented in Development Patterns section

### After Completing a Feature

- [ ] **Test manually** - Run the game and verify the feature works
- [ ] **Check for regressions** - Did anything else break?
- [ ] **Update constants** - Move magic numbers to `src/constants.rs`
- [ ] **Update documentation** - Add new components/resources to CLAUDE.md if needed

### Fixing a Bug (Test-Verified)

When fixing a bug, follow this pattern to ensure the fix is properly verified:

1. **Write or strengthen a test** that exposes the bug
   - Test should FAIL when bug is present
   - Test should PASS when bug is fixed
   - Make assertions specific enough that they can't pass accidentally

2. **Verify the test fails** with the bug still in place
   - Run `cargo run --bin test-scenarios -- <test_name> -v`
   - Confirm failure message matches expected behavior

3. **Apply the fix**

4. **Verify the test passes** with the fix applied
   - Run the same test command
   - Confirm the test now passes

5. **Run full test suite** to check for regressions
   - `cargo run --bin test-scenarios`

This pattern ensures:
- The test actually catches the bug (not a false positive)
- The fix actually resolves the issue (not a coincidence)
- Future regressions will be caught

## Todo Management

**Project planning uses three files:**
- `milestones.md` - Master plan with MVP → V0 → V1/Beyond stages and all tasks
- `todo.md` - Current sprint (active work items pulled from milestones)
- `todone.md` - Archive of completed work

**At the start of each session:** Check `todo.md` for current sprint tasks.

**When completing work:**
- Mark items done in `todo.md`, move to Done section
- Keep only last 5 done items in todo.md; archive older ones to `todone.md` with dated header
- Update `milestones.md` if completing a milestone goal

**When adding new tasks:**
- Quick/urgent items → add to `todo.md` Active Work
- Planned features → add to appropriate milestone in `milestones.md`

### Prioritization Process

Use this interactive process when the todo list needs reorganization or a new sprint needs planning:

**Step 1: Clean up**
- Archive all done items to `todone.md` with dated header (e.g., `## Archived 2026-01-23`)
- Keep only last 5 done items in `todo.md`

**Step 2: Separate concerns**
- Move decision docs and architectural questions to their own section
- Group related tasks with their decision docs (e.g., netcode doc + multiplayer tasks)

**Step 3: Define milestones (if not already done)**
- Ask: "What's MVP?" → minimum playable game
- Ask: "What's V0?" → first real release
- V1/Beyond = everything else

**Step 4: Categorize tasks into milestones**
- Ask: "What's broken or missing for core loop?" → MVP tasks
- Polish, viewport, extra levels → V0 tasks
- New systems, multiplayer, speculative features → V1/Beyond

**Step 5: Build current sprint**
- Pull active tasks from current milestone (usually MVP) into `todo.md`
- Group by area (e.g., Stealing, AI, Movement)

**Step 6: Prioritize within sprint**
- Ask: "Which area first?" → order the groups
- For each group, ask: "Which task is most impactful?" → order within group
- Assign linear priority numbers (P1, P2, P3...) across all tasks

**Result:** `todo.md` becomes a prioritized, numbered list ready for execution.

## Architecture

This is a 2v2 ball sport game built with Bevy 0.17.3 using the Entity Component System (ECS) pattern.

### Module Structure

```
src/
├── main.rs          # App setup, system registration, setup() function
├── lib.rs           # Re-exports all public types
├── constants.rs     # All tunable values
├── helpers.rs       # Utility functions (move_toward, basket_x_from_offset)
├── input/           # PlayerInput resource, capture_input system
├── player/          # Player components + physics systems
├── ball/            # Ball components, physics, interaction systems
├── shooting/        # Charge, throw, targeting systems
├── scoring/         # Score resource, check_scoring system
├── snapshot.rs      # Game state + screenshot capture on events (F2/F3/F4)
├── steal.rs         # StealContest resource + steal cooldown system
├── levels/          # LevelDatabase, spawning, hot reload
├── presets/         # Game tuning presets (movement, ball, shooting, composite)
├── replay/          # Replay system for playing back recorded .evlog files
├── training/        # Training mode state, session management, summary generation
├── world/           # Platform, Collider, Basket, BasketRim components
└── ui/              # Debug, HUD, animations, charge gauge, tweak panel
```

### ECS Structure

**Resources:**
- `PlayerInput` - Buffered input state (movement, jump, pickup, throw)
- `StealContest` - Steal feedback (fail_flash_timer, out_of_range_timer, entities)
- `Score` - Left/right team scores
- `DebugSettings` - Debug UI visibility
- `CurrentLevel` - Current level number (1-10)
- `CurrentPalette` - Current color palette index (default: 26)
- `PhysicsTweaks` - Runtime-adjustable physics values with panel UI
- `LevelDatabase` - Loaded level definitions from assets/levels.txt
- `LastShotInfo` - Debug info about the most recent shot (angle, power, variance breakdown)
- `BallTextures` - Handles to ball textures (dynamic styles × palettes)
- `ReplayMode` - Controls replay mode (active flag, file path)
- `ReplayData` - Loaded replay data (ticks, events, match info)
- `ReplayState` - Playback state (time, speed, paused, stepping)
- `ViewportScale` - Current viewport preset for testing different screen sizes
- `CycleSelection` - D-pad direction-based cycle state (active_direction, down_option, right_option, ai_player_index)
- `AiProfileDatabase` - Loaded AI personality profiles from assets/ai_profiles.txt
- `ConfigWatcher` - Tracks config file modification times for auto-reload (every 10s)
- `PresetDatabase` - Game tuning presets from assets/game_presets.txt
- `CurrentPresets` - Currently active preset indices (movement, ball, shooting, composite)
- `SnapshotConfig` - Controls automatic game state capture (on_score, on_steal, on_level_change, save_screenshots)
- `SnapshotTriggerState` - Tracks previous frame state for detecting changes

**Player Components:**
- `Player` - Marker for player entities
- `Velocity` - 2D velocity vector
- `Grounded` - Whether player is on ground
- `CoyoteTimer` - Time remaining for coyote jump
- `JumpState` - Tracks if currently in a jump
- `Facing` - Direction player faces (-1.0 left, 1.0 right) - used for ball/gauge position only
- `HoldingBall` - Reference to held ball entity
- `ChargingShot` - Charge time accumulator
- `TargetBasket` - Which basket (Left/Right) player is aiming at
- `InputState` - Per-player input buffer (human input copied here, AI writes directly)
- `AiState` - AI goal state machine + profile_index for AI personality
- `StealCooldown` - Per-player cooldown timer between steal attempts

**Ball Components:**
- `Ball` - Marker for ball entity
- `BallState` - Free, Held(Entity), or InFlight { shooter, power }
- `BallStyle` - Visual style name (loaded from assets/ball_options.txt: wedges, half, spiral, etc.)
- `BallPlayerContact` - Tracks overlap for collision effects
- `BallPulse` - Animation timer for pickup indicator
- `BallRolling` - Whether ball is rolling on ground (vs bouncing/flying)
- `BallShotGrace` - Post-shot grace timer (100ms of no friction/player drag)
- `BallSpin` - Angular velocity for rotation

**World Components:**
- `Platform` - Collidable platform (requires `Collider`)
- `Collider` - Marker for collidable entities
- `Basket` - Scoring zone (Left or Right)
- `BasketRim` - Marks rim platforms attached to baskets (for collision filtering)
- `LevelPlatform` - Marks platforms that belong to current level (despawned on level change)
- `CornerRamp` - Marks corner step platforms (despawned on level change)

**UI Components:**
- `DebugText` - Debug info display (last shot details)
- `ScoreLevelText` - Score and level display (top of screen)
- `ChargeGaugeBackground` / `ChargeGaugeFill` - Shot charge indicator (inside player)
- `TweakPanel` / `TweakRow` - Physics tweak panel UI
- `ScoreFlash` - Score animation (flashes basket/player on goal)
- `CycleIndicator` - Always-visible 4-line display in top-left showing all D-pad directions and options

### System Execution Order

**Update schedule (chained input group):** `capture_input` → `copy_human_input` → `swap_control` → `mark_nav_dirty_on_level_change` → `rebuild_nav_graph` → `ai_navigation_update` → `ai_decision_update`

**Update schedule (other systems):** `check_settings_reset` → `respawn_player` → `steal_cooldown_update` → `toggle_debug` → `check_config_changes` → `update_debug_text` → `update_score_level_text` → `animate_pickable_ball` → `animate_score_flash` → `update_charge_gauge` → `update_steal_indicators` → `display_ball_wave` → `toggle_tweak_panel` → `update_tweak_panel` → `cycle_viewport` → `unified_cycle_system` → `update_cycle_indicator` → `apply_palette_colors` → `apply_preset_to_tweaks` → `snapshot_trigger_system` → `toggle_snapshot_system` → `toggle_screenshot_capture` → `manual_snapshot` → `save_settings_system`

**FixedUpdate schedule (chained):** `apply_input` → `apply_gravity` → `ball_gravity` → `ball_spin` → `apply_velocity` → `check_collisions` → `ball_collisions` → `ball_state_update` → `ball_player_collision` → `ball_follow_holder` → `pickup_ball` → `steal_cooldown_update` → `update_shot_charge` → `throw_ball` → `check_scoring`

### Input

Keyboard + Gamepad supported:
- A/D or Left Stick: Horizontal movement
- Space/W or South button: Jump
- E or West button: Pickup ball / Steal
- F or Right Bumper: Charge and throw (hold to charge, release to throw)
- Q or Left Bumper: Cycle player control (Left → Right → Observer → Left)
- R or Start: Reset current level (randomizes AI profile)
- ] key: Next level (keyboard only)
- [ key: Previous level (keyboard only)
- V key: Cycle viewport size (keyboard only)
- Tab: Toggle debug UI (shot info text)
- F1: Toggle physics tweak panel (keyboard only)
- F2: Toggle snapshot system on/off (keyboard only)
- F3: Toggle screenshot capture - JSON only when off (keyboard only)
- F4: Manual snapshot - captures game state + screenshot immediately (keyboard only)

**Controller D-pad Cycle System:**
Each D-pad direction controls different options. Press a direction to select it (and cycle its options if multiple), then use LT/RT to cycle values.

| Direction | Options (D-pad cycles) | Values (LT/RT cycles) |
|-----------|------------------------|----------------------|
| **Up** | Viewport (single) | Viewport sizes |
| **Down** | Composite → Movement → Ball → Shooting | Preset values |
| **Left** | AI (single) | LT: player, RT: profile |
| **Right** | Level → Palette → BallStyle | Values |

Display (top-left, always visible):
```
  Viewport: 1080p
> Composite: Default
  AI: [L* Aggressive] R Passive
  Level: 3/10
```
`>` marks active direction, `*` marks human-controlled player

**Bevy GamepadButton naming (counterintuitive):**
- `LeftTrigger` / `RightTrigger` = Bumpers (LB/RB, digital shoulder buttons)
- `LeftTrigger2` / `RightTrigger2` = Triggers (LT/RT, analog triggers)

**In tweak panel:**
- Up/Down: Select parameter
- Left/Right: Adjust value by ~10%
- R: Reset selected parameter to default
- Shift+R: Reset all parameters to defaults

**Replay mode controls (when running with `--replay <file>`):**
- Space: Toggle pause
- Left/Right arrows: Adjust playback speed (0.25x/0.5x/1x/2x/4x)
- Period (.): Step forward one tick (when paused)
- Comma (,): Step backward one tick (when paused)
- Home: Jump to start
- End: Jump to end

### Training Mode

Training mode (`cargo run --bin training`) lets you play 1v1 against AI across multiple games, logging everything for later analysis.

**Usage:**
```bash
cargo run --bin training                        # 5 games vs Balanced AI
cargo run --bin training -- --games 3           # 3 games
cargo run --bin training -- --profile Aggressive  # Specific AI profile
```

**Controls:**
- A/D or Left Stick: Move
- Space/W or South button: Jump
- E or West button: Pickup/Steal
- F or Right Bumper: Throw (hold to charge)
- Escape: Quit training session

**Output:**
```
training_logs/
└── session_20260123_143022/
    ├── game_1_level4.evlog
    ├── game_2_level7.evlog
    ├── game_3_level2.evlog
    ├── game_4_level9.evlog
    ├── game_5_level3.evlog
    └── summary.json
```

**Post-session analysis:** Ask Claude Code to analyze the training session:
```
"Analyze my training session in training_logs/session_20260123_143022/"
```

**Analysis goal:** When analyzing training sessions, the objective is to identify ways to improve AI behavior. Review the evlog events, player notes, and AI goal transitions to find patterns where the AI makes poor decisions. Then examine the AI code in `src/ai/` and suggest specific changes to improve decision-making, positioning, or timing.

---

## Development Patterns

**IMPORTANT: Follow these patterns for all new code.**

### 1. Input Buffering (MANDATORY)

Any "press" input captured in Update and consumed in FixedUpdate MUST be buffered. This prevents missed inputs due to frame timing differences.

**Pattern:**
```rust
// In PlayerInput resource:
new_action_pressed: bool,

// In capture_input (Update) - ACCUMULATE, don't overwrite:
if keyboard.just_pressed(KeyCode::X) || gamepad.just_pressed(Button) {
    input.new_action_pressed = true;  // Set to true, never set to false here
}

// In consuming system (FixedUpdate) - CONSUME after reading:
if input.new_action_pressed {
    input.new_action_pressed = false;  // Consume immediately
    // ... do action
}
```

**Why:** `just_pressed` only returns true for one Update frame. If FixedUpdate doesn't run that exact frame, the input is lost. Buffering ensures the input persists until consumed.

**Input types that DON'T need buffering:**
- Continuous inputs (movement axis) - overwrite each frame is correct
- Held state (`pressed()` not `just_pressed()`) - checked every frame

### 2. Update vs FixedUpdate

- **Update:** Input capture, UI updates, visual effects (animations, debug text)
- **FixedUpdate:** Physics, movement, collisions, game logic

Systems in FixedUpdate are chained for deterministic order. Never read raw input (`just_pressed`) in FixedUpdate - always use buffered `PlayerInput` resource.

### 3. Child Entities for Player Attachments

Visual elements attached to the player (arrow, charge gauge) should be spawned as child entities:
```rust
let child = commands.spawn((/* components */)).id();
commands.entity(player_entity).add_child(child);
```

Update their transforms relative to player in Update systems.

### 4. Constants in constants.rs

All tunable values go in `src/constants.rs`, grouped by category:
- Visual constants (colors, sizes)
- Physics constants (gravity, speeds)
- Game feel constants (timers, thresholds)
- Arena constants (dimensions, positions)

### 5. Component Queries

Use specific queries with `With<T>` and `Without<T>` filters to avoid ambiguity and improve performance. When querying related entities (player and children), use separate queries.

### 6. Collision Epsilon for Ground Contact

Entities that rest on platforms (player, ball when rolling) must be positioned slightly INTO the platform using `COLLISION_EPSILON`:
```rust
// Correct - ensures overlap detected next frame
transform.translation.y = platform_top + entity_half_height - COLLISION_EPSILON;

// Wrong - zero overlap, ground contact detection fails
transform.translation.y = platform_top + entity_half_height;
```

Without this, entities positioned exactly on top have zero overlap, collision detection fails, and:
- Rolling balls won't detect ground contact and fall through logic breaks
- Grounded state flickers
- Entities may float or behave erratically

### 7. Frame-Rate Independent Physics

All continuous physics must be time-based, not per-frame:
```rust
// Correct - time-based
velocity.y -= GRAVITY * time.delta_secs();           // Subtraction
velocity.x *= FRICTION.powf(time.delta_secs());      // Multiplicative decay

// Wrong - frame-rate dependent
velocity.y -= GRAVITY;        // Faster at higher FPS
velocity.x *= 0.98;           // Faster decay at higher FPS
```

**What needs time-scaling:**
- Gravity, acceleration (use `* delta_secs`)
- Friction, drag, decay (use `.powf(delta_secs)`)
- Timers, cooldowns (use `- delta_secs`)

**What does NOT need time-scaling:**
- One-time events (bounce, jump impulse, collision response)
- State changes (pickup, throw)

---

## Maintenance Checklist

**Remind the user to audit every ~10 changes if they haven't recently.**

For detailed explanations and code examples, see `code_review_guidelines.md`.

### Code Review Quick Checklists

Use these checklists when reviewing any code changes:

**Physics & Timing:**
- [ ] All physics calculations in FixedUpdate (not Update)
- [ ] Time-based values use `* time.delta_secs()` or `.powf(time.delta_secs())`
- [ ] No per-frame multipliers like `velocity *= 0.98`
- [ ] Collision epsilon used for entities resting on surfaces
- [ ] No spiral of death risk (physics steps capped)

**Input Handling:**
- [ ] `just_pressed()` inputs buffered before FixedUpdate consumption
- [ ] Input buffer flags consumed (set to false) after use
- [ ] Continuous inputs (axes) can overwrite each frame
- [ ] No raw input reads in FixedUpdate

**ECS Queries:**
- [ ] Queries fetch only needed components
- [ ] `With<T>`/`Without<T>` filters instead of `Option<&T>` where possible
- [ ] No nested loops over large entity sets (O(n²) collision)
- [ ] Mutable access (`&mut`) only when mutation actually occurs

**Memory & Allocations:**
- [ ] No `Vec::new()`, `String::new()`, `to_string()` in per-frame systems
- [ ] Collections pre-allocated and reused via `.clear()`
- [ ] No string formatting (`format!()`) in hot paths
- [ ] Asset handles cloned, not assets re-added

**System Organization:**
- [ ] Dependent systems ordered with `.after()`/`.before()`/`.chain()`
- [ ] Update = visuals/input, FixedUpdate = physics
- [ ] No GlobalTransform reads before TransformPropagate
- [ ] New system dependencies documented in CLAUDE.md

**Component Design:**
- [ ] Per-entity data in Components, not Resource hashmaps
- [ ] Global singletons as Resources, not singleton entities
- [ ] Components small and focused
- [ ] Flag fields preferred over frequent add/remove component

### Audit Checklist

When asked to "audit", "review", or "check the repo", perform these checks:

**Quick Checks (every audit):**
1. **Compilation** - Run `cargo check` and `cargo clippy`
2. **Visual regression** - Run `./scripts/regression.sh` to capture and compare against baseline
3. **CLAUDE.md accuracy** - Verify architecture section matches actual code
4. **Pattern violations** - Check for raw input reads in FixedUpdate, unbuffered press inputs, missing collision epsilon
5. **Constants** - No magic numbers in code; all tunable values in `src/constants.rs`

**Code Review (every audit):**
Run the full code review process from `code_review_prompt.md`. This includes:
- Deep investigation of codebase for anti-patterns
- Research game dev best practices from authoritative sources
- Grade each area: Physics, Input, ECS, AI, Performance, Game Design
- Create dated review file: `code_review_YYYY-MM-DD.md`
- Update `code_review_guidelines.md` with new patterns/resources discovered
- Log findings to `code_review_audits.md`

**Balance Testing (when relevant):**
- `cargo run --bin simulate -- --shot-test 30 --level 3` (target: 40-60% over/under ratio)
- `cargo run --bin simulate -- --tournament 5 --parallel 8` (AI match testing)
- See `notes/balance-testing-workflow.md` for full iterative workflow

**After auditing:**
- Write findings to `audit_record.md` with commit reference
- Update `todo.md` - add improvement tasks from code review, move completed items to Done
- Archive old done records to `todone.md` with dated header

### Scaling Concerns to Monitor

These areas may need attention as the game grows:

| Area | Current State | Watch For |
|------|---------------|-----------|
| Collision loops | O(balls × platforms) ~40 | Adding more physics objects |
| String allocations | ~164 `to_string()` calls | Per-frame style/debug updates |
| RNG instantiation | 23 `thread_rng()` calls | Adding particle systems |
| HashMap lookups | String keys for ball styles | Per-frame texture changes |

### UI/UX Changes

**Always verify UI changes visually using screenshots.**

When making changes to UI elements (text positioning, HUD layout, debug displays, indicators):

1. Run the game and trigger a snapshot with F4 (or let events trigger automatically)
2. Read the screenshot from `snapshots/` directory to verify the change looks correct
3. Check for: text clipping, overlapping elements, correct positioning, readability

The snapshot system captures both JSON (game state) and PNG (screenshot) to `snapshots/` directory. Use this to verify visual changes without manually inspecting the running game.

**Snapshot triggers:**
- Automatic: score changes, steal attempts, level changes
- Manual: F4 key

**Output location:** `snapshots/YYYYMMDD_HHMMSS_trigger.json` and `.png`

### Visual Regression Testing

**Scripts:**
- `./scripts/screenshot.sh` - Capture a single screenshot (game auto-quits after startup)
- `./scripts/regression.sh` - Capture and compare against baseline
- `./scripts/regression.sh --update` - Update baseline with current screenshot

**Workflow:**
1. Run `./scripts/regression.sh` to capture current state and compare to baseline
2. Review output: PASS (match), REVIEW (small diff), or FAIL (significant diff)
3. If changes are intentional, update baseline with `./scripts/regression.sh --update`
4. Read screenshots directly with Read tool to verify visual changes

**Files:**
- `regression/baseline.png` - Known-good reference screenshot
- `regression/current.png` - Most recent captured screenshot
- `regression/diff.png` - Visual diff (if ImageMagick is installed)

**Notes:**
- Small differences are normal due to timing/compression variations
- Debug level (1/11) shows all ball styles for visual testing
- The game uses `--screenshot-and-quit` flag to auto-exit after startup screenshot

