# Functional Spec Coverage Matrix

Maps functional specification IDs from `docs/design/functional_spec.md` to test coverage.

**Legend:**
- ✅ Covered - Automated test exists
- ⚠️ Partial - Some coverage, gaps remain
- ❌ Gap - No automated test
- 📋 Manual - Manual testing only

---

## 1. Game Modes (M1-M4)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| M1.1-M1.3 | Normal play mode | manual only | 📋 Manual |
| M2.1-M2.7 | Training mode lifecycle | manual only | ❌ Gap |
| M3.1-M3.8 | Replay mode controls | manual only | ❌ Gap |
| M4.1-M4.4 | Debug level mode | manual only | 📋 Manual |

**Priority:** M2.x (Training mode) - 7 behaviors untested

---

## 2. Core Gameplay Loop (G1-G2)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| G1.1 | Match start - ball/players spawn | scoring/ball_respawn.toml | ⚠️ Partial |
| G1.2 | Countdown | manual only | ❌ Gap |
| G1.3-G1.5 | Ball acquisition/offense/defense | manual only | 📋 Manual |
| G1.6 | Score event resets | scoring/*.toml | ✅ Covered |
| G2.1-G2.4 | Team targeting/attribution | scoring/score_basic.toml, own_goal.toml | ✅ Covered |

---

## 3. Player Movement (P1-P3)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| P1.1-P1.2 | Horizontal movement | movement/walk_left.toml, walk_right.toml | ✅ Covered |
| P1.3-P1.7 | Speed/acceleration values | movement/*.toml (implicit) | ⚠️ Partial |
| P1.8 | Facing direction | ball/pickup_*.toml (implicit) | ⚠️ Partial |
| P1.E1 | Deadzone handling | manual only | ❌ Gap |
| P1.E2 | Wall collision | collision/wall_stops_player.toml | ✅ Covered |
| P1.E3 | Movement while holding | manual only | 📋 Manual |
| P2.1-P2.3 | Jump mechanics | movement/jump_basic.toml | ✅ Covered |
| P2.4-P2.5 | Gravity values | movement/jump_max_height.toml | ⚠️ Partial |
| P2.6 | Coyote time | movement/coyote_time.toml | ✅ Covered |
| P2.7 | Jump buffer | movement/jump_buffer.toml | ✅ Covered |
| P2.8 | In-air control | movement/air_control.toml | ✅ Covered |
| P2.E1-E4 | Jump edge cases | movement/*.toml | ✅ Covered |
| P3.1-P3.4 | Platform collision | collision/platform_*.toml | ✅ Covered |

---

## 4. Ball Mechanics (B1-B4)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| B1.1-B1.3 | Ball states (Free/Held/InFlight) | ball/*.toml | ✅ Covered |
| B1.4-B1.6 | State transitions | ball/pickup*.toml, shooting/*.toml | ✅ Covered |
| B2.1-B2.2 | Ball gravity/air friction | ball/bounce_floor.toml | ⚠️ Partial |
| B2.3-B2.4 | Bounce physics | ball/bounce_floor.toml | ✅ Covered |
| B2.5-B2.6 | Rolling mode/friction | ball/rolling_stops.toml | ✅ Covered |
| B2.7-B2.8 | Spin mechanics | visual only | 📋 Manual |
| B3.1-B3.3 | Special surfaces | ball/bounce_rim.toml | ⚠️ Partial |
| B4.1-B4.4 | Ball-player collision | manual only | ❌ Gap |
| B4.E1-E3 | Collision edge cases | manual only | ❌ Gap |

**Priority:** B4.x (Ball-player collision) - 7 behaviors untested

---

## 5. Pickup & Steal (S1-S2)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| S1.1 | Pickup near ball | ball/pickup_stationary.toml | ✅ Covered |
| S1.2 | Pickup too far | stealing/steal_boundary_outside.toml (related) | ⚠️ Partial |
| S1.3-S1.4 | Pickup priority | manual only | 📋 Manual |
| S2.1-S2.2 | Steal attempt/base chance | stealing/steal_in_range.toml | ✅ Covered |
| S2.3 | Charging bonus | stealing/steal_while_charging.toml | ✅ Covered |
| S2.4-S2.5 | Successful steal + pushback | stealing/steal_knockback.toml | ✅ Covered |
| S2.6 | Failed steal flash | visual only | 📋 Manual |
| S2.7 | Attacker cooldown | stealing/steal_cooldown.toml | ✅ Covered |
| S2.8 | Victim cooldown | stealing/no_stealback_cooldown.toml | ✅ Covered |
| S2.E1-E3 | Steal edge cases | stealing/*.toml | ✅ Covered |

---

## 6. Shooting (T1-T3)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| T1.1-T1.2 | Charge mechanics | shooting/shoot_max_charge.toml | ✅ Covered |
| T1.3-T1.4 | Release/quick shot | shooting/shoot_basic.toml | ✅ Covered |
| T1.5 | Charge gauge display | visual only | 📋 Manual |
| T1.6 | Not holding ball | manual only | 📋 Manual |
| T2.1-T2.6 | Throw trajectory | shooting/*.toml | ⚠️ Partial |
| T3.1-T3.2 | Base variance | simulation shot-test | ⚠️ Partial |
| T3.3 | Air penalty | shooting/shoot_while_jumping.toml | ✅ Covered |
| T3.4 | Move penalty | manual only | ❌ Gap |
| T3.5-T3.7 | Distance penalty/bias | simulation shot-test | ⚠️ Partial |
| T3.E1-E4 | Shooting edge cases | shooting/*.toml | ⚠️ Partial |

---

## 7. Scoring (SC1-SC3)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| SC1.1-SC1.2 | Score detection bounds | scoring/score_basic.toml | ✅ Covered |
| SC1.3-SC1.4 | Carry-in vs throw-in | scoring/score_increments.toml | ✅ Covered |
| SC2.1-SC2.2 | Point values | scoring/score_increments.toml | ✅ Covered |
| SC2.3-SC2.4 | Team attribution | scoring/own_goal.toml | ✅ Covered |
| SC3.1-SC3.5 | Post-score reset | scoring/ball_respawn.toml | ✅ Covered |
| SC3.6-SC3.8 | Score flash | visual only | 📋 Manual |

---

## 8. AI System (AI1-AI3)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| AI1.1 | Idle goal | manual (debug level) | 📋 Manual |
| AI1.2 | ChaseBall goal | none | ❌ Gap |
| AI1.3 | AttackWithBall goal | none | ❌ Gap |
| AI1.4 | ChargeShot goal | none | ❌ Gap |
| AI1.5 | AttemptSteal goal | none | ❌ Gap |
| AI1.6 | InterceptDefense goal | none | ❌ Gap |
| AI1.7 | PressureDefense goal | none | ❌ Gap |
| AI2.1-AI2.8 | Goal transitions | none | ❌ Gap |
| AI3.1-AI3.5 | Navigation | reachability_test.rs, multihop_test.rs | ⚠️ Partial |

**Priority:** AI1.x, AI2.x - 15 behaviors untested (highest priority gap)

---

## 9. Level System (L1-L3)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| L1.1-L1.6 | Level loading/navigation | manual only | ❌ Gap |
| L2.1-L2.5 | Level geometry | manual only | 📋 Manual |
| L3.1-L3.7 | Level transition | manual only | ❌ Gap |

**Priority:** L3.x (Level transitions) - 7 behaviors untested

---

## 10. UI/HUD (U1-U6)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| U1.1-U1.4 | Score display | visual regression | ⚠️ Partial |
| U2.1-U2.4 | Cycle indicator | visual regression | ⚠️ Partial |
| U3.1-U3.5 | Charge gauge | visual only | 📋 Manual |
| U4.1-U4.5 | Countdown display | visual only | 📋 Manual |
| U5.1-U5.3 | Steal indicators | visual only | 📋 Manual |
| U6.1-U6.5 | Score flash | visual only | 📋 Manual |

---

## 11. Configuration & Persistence (C1-C3)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| C1.1-C1.7 | Settings persistence | manual only | ❌ Gap |
| C2.1-C2.5 | Hot reload | manual only | ❌ Gap |
| C3.1-C3.4 | Presets | manual only | 📋 Manual |

---

## 12. Debug/Dev Features (D1-D5)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| D1.1-D1.3 | Debug text | manual only | 📋 Manual |
| D2.1-D2.7 | Tweak panel | manual only | 📋 Manual |
| D3.1-D3.6 | Snapshot system | manual only | 📋 Manual |
| D4.1-D4.4 | Viewport cycling | manual only | 📋 Manual |
| D5.1-D5.4 | Visual regression | regression.sh | ✅ Covered |

---

## 13. Ghost System (GH1-GH3)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| GH1.1-GH1.3 | Drive recording | manual only | ❌ Gap |
| GH2.1-GH2.6 | Ghost replay | manual only | ❌ Gap |
| GH3.1-GH3.3 | Analysis outputs | manual only | ❌ Gap |

**Priority:** GH1-GH3 - 12 behaviors untested

---

## 14. Event Logging (EL1-EL3)

| Spec ID | Description | Test File | Status |
|---------|-------------|-----------|--------|
| Database schema | SQLite tables | sqlite_logger::tests | ✅ Covered |
| Event types | T, G, P, SR, etc. | sqlite_logger::tests | ✅ Covered |

---

## Summary Statistics

| Category | Total IDs | Covered | Partial | Gap | Manual |
|----------|-----------|---------|---------|-----|--------|
| Game Modes | 22 | 0 | 0 | 14 | 8 |
| Core Loop | 10 | 2 | 1 | 1 | 6 |
| Movement | 22 | 14 | 4 | 1 | 3 |
| Ball | 20 | 8 | 5 | 7 | 0 |
| Pickup/Steal | 14 | 10 | 1 | 0 | 3 |
| Shooting | 17 | 5 | 5 | 1 | 6 |
| Scoring | 14 | 10 | 0 | 0 | 4 |
| AI System | 20 | 0 | 2 | 15 | 3 |
| Level System | 18 | 0 | 0 | 14 | 4 |
| UI/HUD | 21 | 0 | 2 | 0 | 19 |
| Config | 16 | 0 | 0 | 12 | 4 |
| Debug | 22 | 1 | 0 | 0 | 21 |
| Ghost | 12 | 0 | 0 | 12 | 0 |
| Event Log | 6 | 6 | 0 | 0 | 0 |
| **TOTAL** | **234** | **56** | **20** | **77** | **81** |

**Coverage Rate:** 24% automated, 8.5% partial, 33% gaps, 34.5% manual-only

---

## Priority Gaps (Must Test)

1. **AI System (AI1-AI2)** - 15 behaviors, core gameplay
2. **Ghost System (GH1-GH3)** - 12 behaviors, new feature
3. **Training Mode (M2)** - 7 behaviors, workflow
4. **Level System (L3)** - 7 behaviors, transitions
5. **Ball-Player Collision (B4)** - 7 behaviors, physics

---

## Test File Reference

### Scenario Tests (35 total)
- `ball/` - 7 tests
- `collision/` - 3 tests
- `movement/` - 8 tests
- `scoring/` - 4 tests
- `shooting/` - 5 tests
- `stealing/` - 8 tests

### Unit Tests (62 total)
- `simulation/bracket/` - bracket seeding, match format
- `simulation/db/` - database operations
- `simulation/reachability_test/` - position sampling
- `simulation/multihop_test/` - pathfinding
- `events/sqlite_logger/` - event logging
- `analytics/db_analytics/` - profile analysis
- `training/` - protocol parsing, analysis

---

*Last updated: 2026-01-30*
