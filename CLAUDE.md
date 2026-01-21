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

## Architecture

This is a 2D platformer game built with Bevy 0.17.3 using the Entity Component System (ECS) pattern. The entire game is in `src/main.rs`.

### ECS Structure

**Components:**
- `Player` - Marker for the player entity
- `MoveState` - Tracks jumping state (`is_jumping`) and terminal velocity
- `Velocity` - 2D velocity vector for physics
- `Collider` - Marker for collidable entities
- `Floor` - Platform marker (requires `Collider`)
- `DebugText` - UI text display

**Resources:**
- `DebugInfo` - Debug state (elapsed time, collision info)

**Systems (execution order):**
1. `setup` (Startup) - Spawns camera, player, floor, platforms, debug UI
2. `apply_velocity` (FixedUpdate) - Applies velocity and gravity
3. `move_player` (FixedUpdate) - Handles gamepad input for movement/jumping
4. `check_for_collisions` (FixedUpdate) - AABB collision detection and response
5. `gamepad_log_system` (Update) - Logs gamepad button presses
6. `debug_update_system` (Update) - Updates debug text display

Systems 2-4 are chained in FixedUpdate for deterministic physics.

### Physics Constants

- Gravity: 200.0 units/sec² downward
- Jump velocity: 200.0 units
- Player speed: 500.0 units/sec horizontal
- Player size: 32x64 pixels
- Collision uses Bevy's `Aabb2d` for AABB intersection tests

### Input

Gamepad only (no keyboard support):
- Left stick X: Horizontal movement
- South button (A/X): Jump
