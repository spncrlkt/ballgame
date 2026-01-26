# Testing Strategy

Quick manual testing checklist for rapid iteration. Run these after significant changes.

## Smoke Tests (Run Every Time)

1. **Game Launches** - `cargo run` starts without crash
2. **Player Moves** - Left stick moves player horizontally
3. **Player Jumps** - South button (A/X) makes player jump
4. **Ball Visible** - Ball spawns at center and is visible

## Core Mechanics Checklist

### Movement & Physics
- [ ] Player falls when not on ground
- [ ] Player lands on floor and stops falling
- [ ] Player can walk left/right on floor
- [ ] Player can jump (tap = short, hold = full height)
- [ ] Coyote time: can jump briefly after walking off edge
- [ ] Jump buffer: jump input right before landing still works

### Ball Interaction
- [ ] Ball falls with gravity
- [ ] Ball bounces on floor (height decreases each bounce)
- [ ] Ball eventually rolls instead of bouncing
- [ ] Ball stops rolling when slow enough
- [ ] Ball bounces off walls
- [ ] Ball pulses when player is close enough to pick up
- [ ] West button (X/Square) picks up ball when close
- [ ] Picked up ball follows player
- [ ] Facing arrow matches player direction

### Shooting
- [ ] Hold R trigger to charge shot
- [ ] Charge gauge fills (green->red)
- [ ] Release to throw ball
- [ ] Tap shot = low arc, short distance
- [ ] Full charge = high arc, long distance
- [ ] Air shots have reduced power and more randomness

### Steal Contest
- [ ] Press pickup near ball holder to start contest
- [ ] Mash to win steal
- [ ] Ball transfers on successful steal

### Scoring
- [ ] Ball entering basket increments score
- [ ] Ball resets to center after score

### Level System
- [ ] Press R/Start to respawn
- [ ] Respawn cycles through 5 levels
- [ ] Level number shows in debug HUD
- [ ] Platforms change between levels
- [ ] Player/ball reset to spawn positions

## Platform-Specific Tests

### Levels
| Level | Description | Verify |
|-------|-------------|--------|
| 1 | Two mid platforms L/R | Both platforms present |
| 2 | 4 ascending stairs L->R | All 4 visible, heights increase |
| 3 | Central tower (3 stacked) | Centered, can climb |
| 4 | V-shape (4 platforms) | Symmetrical V pattern |
| 5 | Scattered (5 platforms) | Various heights/positions |

## Quick Regression Tests

After changing physics constants, verify:
- [ ] Player can still reach highest platform
- [ ] Ball can still reach baskets
- [ ] Ball doesn't bounce forever or stop too quickly
- [ ] Rolling friction feels natural

After changing input handling:
- [ ] All buffered inputs still work (jump, pickup, throw)
- [ ] Deadzone prevents unwanted direction changes
- [ ] Charge doesn't reset unexpectedly

## Debug Tools

- **Tab**: Toggle debug HUD
- **R/Start**: Respawn (cycles level)
- **Debug HUD shows**: Level, Score, FPS, Position, Charge%

## Known Edge Cases

1. Picking up ball while throw button held should start charge from 0
2. Rolling ball should fall off platform edges
3. Ball kicked by walking player should leave rolling state
4. Steal contest timeout resolves to defender on tie

## Performance Check

If FPS drops below 60:
- Check for entity accumulation (platforms not despawning)
- Check for physics calculations running every frame vs delta-time
