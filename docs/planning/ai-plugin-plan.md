# Plan: Consolidate AI Decision-Making Module

## Summary

Create a Bevy Plugin (`AiPlugin`) that encapsulates all AI decision-making systems. Each context (game, training, simulation, ghost) will use this single plugin instead of duplicating system registration. Ghost mode will use the full AI decision system with entity markers to control which players are AI-controlled.

## Current Problems

1. **Duplicate code** - Same AI system registration in 4 files
2. **Simplified ghost AI** - 120-line custom implementation that doesn't match real AI behavior
3. **No standard pattern** - Each binary has its own way of setting up AI

## Approach

### 1. Create `src/ai/plugin.rs`

```rust
use bevy::prelude::*;
use super::{
    AiProfileDatabase, NavGraph,
    ai_decision_update, ai_navigation_update,
    mark_nav_dirty_on_level_change, rebuild_nav_graph,
};

/// AI decision-making plugin
///
/// Registers all AI systems for navigation and decision-making.
/// AI runs for all players WITHOUT the `HumanControlled` marker.
///
/// # Usage
/// ```rust
/// app.add_plugins(AiPlugin);
/// ```
pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        // Resources
        app.init_resource::<NavGraph>();
        app.init_resource::<AiProfileDatabase>();

        // Systems (chained for correct execution order)
        app.add_systems(Update, (
            mark_nav_dirty_on_level_change,
            rebuild_nav_graph,
            ai_navigation_update,
            ai_decision_update,
        ).chain());
    }
}
```

### 2. Update `src/ai/mod.rs`

Add plugin module and export:
```rust
mod plugin;
pub use plugin::AiPlugin;
```

### 3. Simplify each context

**Main game (`src/main.rs`):**
```rust
// Before: 8 lines of system registration
// After:
app.add_plugins(AiPlugin);
```

**Training (`src/bin/training.rs`):**
```rust
// Same simplification
app.add_plugins(AiPlugin);
```

**Simulation (`src/simulation/runner.rs`):**
```rust
// Same simplification
app.add_plugins(AiPlugin);
```

**Ghost (`src/bin/run-ghost.rs`):**
```rust
// Delete ai_decision_for_right_only function (~120 lines)
// Use plugin:
app.add_plugins(AiPlugin);
// Ghost inputs write to left player's InputState directly
// Full AI handles right player defense
```

### 4. Ghost mode: Use entity markers

The existing `ai_decision_update` already filters with `Without<HumanControlled>`:
```rust
Query<..., (With<Player>, Without<HumanControlled>)>
```

For ghost mode:
- Left player: Add marker component (e.g., `GhostControlled`)
- Use existing filter OR add ghost to the Without clause

**Option A (simplest):** Mark ghost player as `HumanControlled` - AI won't process it, ghost system writes inputs directly.

**Option B (cleaner):** Add `GhostControlled` marker, update AI query to `Without<GhostControlled>`.

Recommend **Option A** - no AI code changes needed, ghost player is "human" from AI's perspective.

---

## Files to Modify

| File | Changes |
|------|---------|
| `src/ai/plugin.rs` | **CREATE** - New plugin (~30 lines) |
| `src/ai/mod.rs` | Add `mod plugin; pub use plugin::AiPlugin;` |
| `src/main.rs` | Remove inline AI systems, add `app.add_plugins(AiPlugin)` |
| `src/bin/training.rs` | Remove inline AI systems, add plugin |
| `src/simulation/runner.rs` | Remove inline AI systems, add plugin |
| `src/bin/run-ghost.rs` | Delete `ai_decision_for_right_only` (~120 lines), add plugin, mark left player as HumanControlled |
| `src/lib.rs` | Export `AiPlugin` |

---

## Implementation Steps

1. Create `src/ai/plugin.rs` with AiPlugin
2. Export from `src/ai/mod.rs` and `src/lib.rs`
3. Update `src/main.rs` to use plugin (verify game still works)
4. Update `src/bin/training.rs` (verify training works)
5. Update `src/simulation/runner.rs` (verify simulation works)
6. Update `src/bin/run-ghost.rs`:
   - Delete simplified AI function
   - Add plugin
   - Mark ghost (left) player with `HumanControlled`
   - Verify ghost trials run with full AI defense

---

## Verification

1. **Compilation:** `cargo check`
2. **Tests:** `cargo run --bin test-scenarios` - 35/35 pass
3. **Simulation:** `cargo run --bin simulate -- --shot-test 30 --level 3`
4. **Ghost:** `cargo run --bin run-ghost <trial_file>` - Full AI defense
5. **Manual:** Run main game, play against AI

---

## Future Enhancements (not in scope)

- `AiPlugin::with_config(AiConfig)` for customization
- Configurable navigation (disable for simple arenas)
- Debug visualization mode (draw nav graph, goals)
