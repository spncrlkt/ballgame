# Test Coverage Audit

*Generated: 2026-01-25*

## Current Test Count: 35 scenarios

---

## Coverage by Mechanic

### Movement (8 tests) - GOOD COVERAGE
| Mechanic | Constant | Test | Status |
|----------|----------|------|--------|
| Walking | MOVE_SPEED | walk_left, walk_right | ✅ |
| Jumping | JUMP_VELOCITY | jump_basic, jump_max_height | ✅ |
| Jump while moving | - | jump_while_moving | ✅ |
| Air control | AIR_ACCEL | air_control | ✅ |
| Coyote time | COYOTE_TIME | coyote_time | ✅ |
| Jump buffer | JUMP_BUFFER_TIME | jump_buffer | ✅ |
| Jump cut (short hop) | JUMP_CUT_MULTIPLIER | ❌ MISSING |
| Ground acceleration | GROUND_ACCEL | ❌ MISSING |
| Ground deceleration | GROUND_DECEL | ❌ MISSING |

### Collision (3 tests) - ADEQUATE
| Mechanic | Constant | Test | Status |
|----------|----------|------|--------|
| Platform landing | COLLISION_EPSILON | platform_landing | ✅ |
| Platform head bonk | - | platform_head_bonk | ✅ |
| Wall collision | WALL_THICKNESS | wall_stops_player | ✅ |
| Corner step bounce | STEP_BOUNCE_RETENTION | ❌ MISSING |
| Rim bounce | RIM_BOUNCE_RETENTION | ❌ MISSING |

### Ball Physics (7 tests) - GOOD COVERAGE
| Mechanic | Constant | Test | Status |
|----------|----------|------|--------|
| Ball gravity | BALL_GRAVITY | bounce_floor | ✅ |
| Ball bounce | BALL_BOUNCE | bounce_floor, bounce_rim | ✅ |
| Ball rolling | BALL_ROLL_FRICTION | rolling_stops | ✅ |
| Ball pickup (stationary) | BALL_PICKUP_RADIUS | pickup_stationary | ✅ |
| Ball pickup (moving) | - | pickup_while_moving | ✅ |
| Ball drop on jump | - | drop_on_jump | ✅ |
| Pass through own basket | - | pass_through_own_basket | ✅ |
| Ball air friction | BALL_AIR_FRICTION | ❌ MISSING |
| Ball ground friction | BALL_GROUND_FRICTION | ❌ MISSING |
| Ball spin | BALL_SPIN_FACTOR | ❌ MISSING |
| Ball-player collision | BALL_PLAYER_DRAG_X/Y | ❌ MISSING |
| Ball kick | BALL_KICK_STRENGTH | ❌ MISSING |

### Shooting (5 tests) - ADEQUATE
| Mechanic | Constant | Test | Status |
|----------|----------|------|--------|
| Basic shot | SHOT_DEFAULT_ANGLE | shoot_basic | ✅ |
| Aim left | - | shoot_aim_left | ✅ |
| Shot from elevation | - | shoot_from_elevation | ✅ |
| Max charge | SHOT_CHARGE_TIME | shoot_max_charge | ✅ |
| Shot while jumping | - | shoot_while_jumping | ✅ |
| Shot variance (tap) | SHOT_MAX_VARIANCE | ❌ MISSING |
| Shot variance (full) | SHOT_MIN_VARIANCE | ❌ MISSING |
| Air variance penalty | SHOT_AIR_VARIANCE_PENALTY | ❌ MISSING |
| Move variance penalty | SHOT_MOVE_VARIANCE_PENALTY | ❌ MISSING |
| Quick shot threshold | SHOT_QUICK_THRESHOLD | ❌ MISSING |
| Shot grace period | SHOT_GRACE_PERIOD | ❌ MISSING |

### Stealing (8 tests) - EXCELLENT COVERAGE
| Mechanic | Constant | Test | Status |
|----------|----------|------|--------|
| Steal in range | STEAL_RANGE | steal_in_range | ✅ |
| Steal out of range | - | steal_out_of_range | ✅ |
| Steal boundary (inside) | STEAL_RANGE | steal_boundary_inside | ✅ |
| Steal boundary (outside) | STEAL_NEAR_MISS_RANGE | steal_boundary_outside | ✅ |
| Steal cooldown | STEAL_COOLDOWN | steal_cooldown | ✅ |
| No stealback cooldown | STEAL_VICTIM_COOLDOWN | no_stealback_cooldown | ✅ |
| Steal knockback | STEAL_PUSHBACK_STRENGTH | steal_knockback | ✅ |
| Steal while charging | STEAL_CHARGING_BONUS | steal_while_charging | ✅ |
| Steal success rate | STEAL_SUCCESS_CHANCE | ❌ MISSING (statistical) |
| Steal differential | MAX_STEAL_DIFFERENTIAL | ❌ MISSING |

### Scoring (4 tests) - ADEQUATE
| Mechanic | Constant | Test | Status |
|----------|----------|------|--------|
| Basic scoring | BASKET_SIZE | score_basic | ✅ |
| Score increments | - | score_increments | ✅ |
| Own goal | - | own_goal | ✅ |
| Ball respawn | BALL_SPAWN | ball_respawn | ✅ |

### AI (0 tests) - NO COVERAGE
| Mechanic | Constant | Test | Status |
|----------|----------|------|--------|
| AI navigation | NAV_* | ❌ MISSING |
| AI shot decision | min_shot_quality | ❌ MISSING |
| AI steal decision | steal_range | ❌ MISSING |
| AI goal transitions | - | ❌ MISSING |
| AI profile effects | - | ❌ MISSING |

---

## Coverage Summary

| Category | Tested | Missing | Coverage |
|----------|--------|---------|----------|
| Movement | 8 | 3 | 73% |
| Collision | 3 | 2 | 60% |
| Ball Physics | 7 | 5 | 58% |
| Shooting | 5 | 6 | 45% |
| Stealing | 8 | 2 | 80% |
| Scoring | 4 | 0 | 100% |
| AI | 0 | 5 | 0% |
| **Total** | **35** | **23** | **60%** |

---

## Priority Gaps (Most Important Missing Tests)

### HIGH PRIORITY - Affects Gameplay Feel
1. **AI navigation test** - Verify AI can reach platforms
2. **AI shot decision test** - Verify AI respects min_shot_quality
3. **Shot variance test** - Verify tap vs full charge variance
4. **Ball-player collision test** - Verify drag/kick behavior

### MEDIUM PRIORITY - Edge Cases
5. **Jump cut test** - Verify short hop works
6. **Corner step bounce test** - Verify ball physics on steps
7. **Steal differential test** - Verify max 2 steal difference

### LOWER PRIORITY - Polish
8. **Ground accel/decel tests** - Movement feel tuning
9. **Ball spin test** - Visual effect verification
10. **Shot grace period test** - Post-shot behavior

---

## Recommended New Tests

### 1. AI Shot Quality (ai/shot_quality.toml)
```
Setup: AI with ball, far from basket (low quality position)
Input: Wait for AI to act
Expect: AI moves closer OR doesn't shoot (respects min_shot_quality)
```

### 2. AI Navigation (ai/platform_climb.toml)
```
Setup: AI on ground, goal on elevated platform
Input: Wait for AI navigation
Expect: AI reaches platform within N frames
```

### 3. Shot Variance (shooting/variance_tap_vs_full.toml)
```
Setup: Player at fixed position with ball
Input: Multiple tap shots, multiple full charge shots
Expect: Tap shots have more spread than full charge
```

### 4. Jump Cut (movement/jump_cut.toml)
```
Setup: Player on ground
Input: Jump press, immediate release
Expect: Lower max height than held jump
```

---

*Next action: Create the HIGH PRIORITY tests*
