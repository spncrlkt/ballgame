# Testing Strategy

## Quick Reference

```bash
cargo run --bin test-scenarios           # Run all 35 scenario tests
cargo run --bin test-scenarios -- ball/  # Run category
cargo run --bin test-scenarios -- -v     # Verbose mode (shows failures)
./scripts/regression.sh                  # Visual regression test
./scripts/regression.sh --update         # Update baseline screenshots
```

## Automated Testing

### Scenario Tests

Deterministic input-script tests for game mechanics. Each test specifies setup, frame-based inputs, and expected outcomes.

```bash
cargo run --bin test-scenarios                    # Run all tests
cargo run --bin test-scenarios -- movement/       # Run category
cargo run --bin test-scenarios -- ball/pickup     # Run specific test
cargo run --bin test-scenarios -- -v              # Verbose (show failures)
```

**Test categories:**
- `movement/` - Walking, jumping, coyote time, jump buffer
- `ball/` - Pickup, throwing, bouncing, rolling
- `shooting/` - Charge mechanics, shot trajectory
- `steal/` - Steal range, cooldown, success/failure
- `scoring/` - Basket detection, score updates

### Visual Regression

Captures screenshots and compares against baselines:

```bash
./scripts/regression.sh              # Capture and compare
./scripts/regression.sh --update     # Update baselines (after intentional changes)
./scripts/regression.sh <scenario>   # Run single scenario
./scripts/regression.sh --list       # List available scenarios
```

**Files:**
- `showcase/regression/baselines/` - Known-good reference screenshots
- `showcase/regression/current/` - Latest captured screenshots
- `showcase/regression/diffs/` - Visual differences (if ImageMagick installed)

### Balance Testing

For AI and shooting balance verification:

```bash
cargo run --bin simulate -- --shot-test 30 --level 3    # Shot accuracy test
cargo run --bin simulate -- --tournament 5 --parallel 8  # AI tournament
```

See `docs/dev/balance-testing.md` for full workflow.

## Manual Testing Checklist

### Smoke Tests (Run Every Time)

1. **Game Launches** - `cargo run` starts without crash
2. **Player Moves** - Left stick / A+D moves player horizontally
3. **Player Jumps** - Space/W or South button makes player jump
4. **Ball Visible** - Ball spawns at center and is visible

### Movement & Physics
- [ ] Player falls when not on ground
- [ ] Player lands on floor and stops falling
- [ ] Player can walk left/right on floor
- [ ] Variable jump height (tap = short, hold = full)
- [ ] Coyote time: can jump briefly after walking off edge
- [ ] Jump buffer: jump input right before landing works

### Ball Interaction
- [ ] Ball falls with gravity
- [ ] Ball bounces on floor (height decreases each bounce)
- [ ] Ball eventually rolls instead of bouncing
- [ ] Ball stops rolling when slow enough
- [ ] Ball bounces off walls
- [ ] Ball pulses when player is close enough to pick up
- [ ] E key or West button picks up ball when close
- [ ] Picked up ball follows player
- [ ] Ball spin visible during flight

### Shooting
- [ ] Hold F or Right Bumper to charge shot
- [ ] Charge gauge fills inside player sprite
- [ ] Release to throw ball
- [ ] Tap shot = low arc, short distance
- [ ] Full charge = high arc, long distance
- [ ] Air shots have reduced power and more randomness

### Stealing
- [ ] Press pickup near ball holder to attempt steal
- [ ] Steal succeeds if within range and off cooldown
- [ ] Ball transfers to stealer on success
- [ ] Cooldown prevents spam (visual indicator)
- [ ] Out of range shows different feedback

### Scoring
- [ ] Ball entering basket increments score
- [ ] Basket flashes on score
- [ ] Ball resets to center after score

## Level Reference

12 playable levels (levels 3-14):

| # | Name | Description |
|---|------|-------------|
| 1 | Debug | Test level - all ball styles visible |
| 2 | Regression | Test level - frozen countdown |
| 3 | Arena | Corner stairs + central platforms |
| 4 | Open Floor | No platforms, just baskets |
| 5 | Islands | Floating island platforms |
| 6 | Slopes | Corner stairs + side platforms |
| 7 | Tower | Central ascending tower |
| 8 | Skyway | High corner stairs + sky platforms |
| 9 | Terraces | Descending mirror platforms |
| 10 | Catwalk | High center walkway |
| 11 | Bunker | Low baskets + shelter platforms |
| 12 | Pit | Central pit with surrounding platforms |
| 13 | Twin Towers | High baskets with vertical platforms |
| 14+ | Training | Protocol-specific test levels |

**Navigation:**
- `]` key: Next level
- `[` key: Previous level
- `R` / Start: Reset current level

## Debug Tools

| Key | Action |
|-----|--------|
| Tab | Toggle debug HUD (shot info) |
| F1 | Toggle physics tweak panel |
| F2 | Toggle snapshot system on/off |
| F3 | Toggle screenshot capture (JSON only when off) |
| F4 | Manual snapshot (game state + screenshot) |
| V | Cycle viewport size |
| Q / LB | Cycle player control |

**D-pad (gamepad):**
- Up: Viewport options
- Down: Preset options (cycle with D-pad, values with LT/RT)
- Left: AI options (LT: player, RT: profile)
- Right: Level/Palette/BallStyle options

**Snapshot output:** `showcase/snapshots/YYYYMMDD_HHMMSS_trigger.json` and `.png`

## Quick Regression Checks

After changing physics constants:
- [ ] Player can still reach highest platform
- [ ] Ball can still reach baskets
- [ ] Ball doesn't bounce forever or stop too quickly
- [ ] Rolling friction feels natural

After changing input handling:
- [ ] All buffered inputs still work (jump, pickup, throw)
- [ ] Charge doesn't reset unexpectedly

After UI changes:
- [ ] Run `./scripts/regression.sh` and verify screenshots
- [ ] Read `showcase/regression/current/*.png` to visually verify

## Known Edge Cases

1. Picking up ball while throw button held should start charge from 0
2. Rolling ball should fall off platform edges
3. Ball kicked by walking player should leave rolling state
4. Steal cooldown applies after any steal attempt (success or fail)

## Performance Check

If FPS drops below 60:
- Check for entity accumulation (platforms not despawning)
- Check for physics calculations per-frame vs delta-time
- Run `cargo clippy` for potential issues
