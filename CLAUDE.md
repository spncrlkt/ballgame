# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build           # Debug build
cargo build --release # Release build
cargo run             # Run the game
cargo check           # Check compilation without building
cargo fmt             # Format code
cargo clippy          # Lint code
```

No tests exist yet. Dynamic linking is disabled in `.cargo/config.toml` to avoid macOS dyld issues.

## Todo Management

**Check `todo.md` at the start of each session.** When completing work:
- Move completed items to the "Done" section at the bottom
- Add new tasks discovered during development
- Keep categories organized (Immediate Fixes, Level Design, Multiplayer, AI, Equipment)

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
├── steal.rs         # StealContest resource + system
├── levels/          # LevelDatabase, spawning, hot reload
├── world/           # Platform, Collider, Basket, BasketRim components
└── ui/              # Debug, HUD, animations, charge gauge, tweak panel
```

### ECS Structure

**Resources:**
- `PlayerInput` - Buffered input state (movement, jump, pickup, throw)
- `StealContest` - Active steal contest state
- `Score` - Left/right team scores
- `DebugSettings` - Debug UI visibility
- `CurrentLevel` - Current level number (1-10)
- `PhysicsTweaks` - Runtime-adjustable physics values with panel UI
- `LevelDatabase` - Loaded level definitions from assets/levels.txt
- `LastShotInfo` - Debug info about the most recent shot (angle, power, variance breakdown)

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

**Ball Components:**
- `Ball` - Marker for ball entity
- `BallState` - Free, Held(Entity), or InFlight { shooter, power }
- `BallPlayerContact` - Tracks overlap for collision effects
- `BallPulse` - Animation timer for pickup indicator
- `BallRolling` - Whether ball is rolling on ground (vs bouncing/flying)
- `BallShotGrace` - Post-shot grace timer (100ms of no friction/player drag)

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
- `TargetMarker` - White marker shown in targeted basket

### System Execution Order

**Update schedule:** `capture_input` → `respawn_player` → `toggle_debug` → `update_debug_text` → `update_score_level_text` → `animate_pickable_ball` → `animate_score_flash` → `update_charge_gauge` → `update_target_marker` → `toggle_tweak_panel` → `update_tweak_panel`

**FixedUpdate schedule (chained):** `apply_input` → `cycle_target` → `apply_gravity` → `ball_gravity` → `apply_velocity` → `check_collisions` → `ball_collisions` → `ball_state_update` → `ball_player_collision` → `ball_follow_holder` → `pickup_ball` → `steal_contest_update` → `update_shot_charge` → `throw_ball` → `check_scoring`

### Input

Keyboard + Gamepad supported:
- A/D or Left Stick: Horizontal movement
- Space/W or South button: Jump
- E or West button: Pickup ball / Steal
- F or Right Bumper: Charge and throw (hold to charge, release to throw)
- Q or Left Bumper: Cycle target basket
- R or Start: Reset current level
- ] or Right Trigger: Next level
- [ or Left Trigger: Previous level
- Tab: Toggle debug UI
- F1: Toggle physics tweak panel

**Bevy GamepadButton naming (counterintuitive):**
- `LeftTrigger` / `RightTrigger` = Bumpers (LB/RB, digital shoulder buttons)
- `LeftTrigger2` / `RightTrigger2` = Triggers (LT/RT, analog triggers)

**In tweak panel:**
- Up/Down: Select parameter
- Left/Right: Adjust value by ~10%
- R: Reset selected parameter to default
- Shift+R: Reset all parameters to defaults

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

When asked to "audit", "review", or "check the repo", perform these checks:

1. **CLAUDE.md accuracy** - Verify architecture section matches actual code (components, resources, systems)
2. **Input buffering** - All `just_pressed` inputs consumed in FixedUpdate must be buffered
3. **Constants** - No magic numbers in code; all tunable values in `src/constants.rs`
4. **System order** - Verify FixedUpdate chain matches documented order
5. **Unused code** - Look for dead code, unused imports, commented-out blocks
6. **Pattern violations** - Check for raw input reads in FixedUpdate, unbuffered press inputs
7. **Collision epsilon** - All entities resting on platforms must use `- COLLISION_EPSILON` positioning to ensure overlap is detected next frame (prevents floating/falling through)
8. **Frame-rate independent physics** - All continuous physics (gravity, friction, drag) must use `* time.delta_secs()` or `.powf(time.delta_secs())`. Per-frame multipliers like `velocity *= 0.98` are bugs.
9. **Compilation** - Run `cargo check` and `cargo clippy`

**After auditing:**
- Compact the conversation context and get a fresh read of the codebase
- Write the audit findings and changes since last audit to `audit_record.md`
- Update `todo.md` - move completed items to Done section, add any new tasks discovered

