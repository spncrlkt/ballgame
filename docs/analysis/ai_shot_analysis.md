# AI Shot Selection Analysis

*Generated: 2026-01-25*

---

## Shot Quality System

The AI uses a heatmap-derived quality score (0.0-1.0) to evaluate positions:

| Quality | Label | Typical Scenarios |
|---------|-------|-------------------|
| 0.75+ | Excellent | Above basket, 150-400px horizontal |
| 0.55+ | Good | Same level, good angle |
| 0.40+ | Acceptable | Floor shots, moderate distance |
| 0.25+ | Desperate | Behind basket, far away |
| <0.25 | Terrible | Directly under, extreme range |

---

## Profile Thresholds

Different AI profiles have different `min_shot_quality` thresholds:

| Profile | min_shot_quality | Behavior |
|---------|------------------|----------|
| Sniper | 0.60 | Only shoots from excellent positions |
| Turtle | 0.55 | Waits for good positions |
| Defensive, Patient | 0.50 | Balanced approach |
| Goalie | 0.45 | Slightly more willing |
| Balanced | 0.40 | Standard threshold |
| Aggressive | 0.38 | Takes slightly riskier shots |
| Hunter | 0.35 | More aggressive |
| Rusher | 0.20 | Shoots from almost anywhere |
| Chaotic | 0.15 | Shoots from terrible positions |

**Key insight:** Rusher and Chaotic are DESIGNED to take bad shots.

---

## Potential Issues Found

### Issue 1: Charging While Navigating (lines 828-842)

When `nav_controlling` is true and goal is `ChargeShot`:
```rust
if ai_state.current_goal == AiGoal::ChargeShot && nav_controlling {
    // AI starts charging while moving to better position
    input.throw_held = true;
    // ...
}
```

**Problem:** AI might shoot mid-navigation when position isn't optimal yet.

**Impact:** Low-to-medium. Navigation usually completes quickly, but shots could release early.

### Issue 2: Commit Once Charging (lines 494-496)

```rust
} else if already_charging {
    // Commit to the shot once started
    AiGoal::ChargeShot
}
```

**Problem:** Once AI starts charging, it never reconsiders even if position worsens.

**Impact:** Low. AI usually doesn't move much during ground shots.

### Issue 3: Jump Shot Drift (line 676)

```rust
// Drift toward basket while airborne
input.move_x = dx.signum() * 0.3;
```

**Problem:** AI drifts toward basket during jump shots, potentially moving under the basket (low quality zone).

**Impact:** Medium. Could cause AI to release shots directly under basket.

---

## Recommendations

### Quick Fix: Add Quality Check Before Release

In the throw logic, re-evaluate quality before releasing:

```rust
// Before releasing, check if we drifted to a bad position
let current_quality = evaluate_shot_quality(ai_pos, target_basket_pos);
if current_quality < profile.min_shot_quality * 0.7 {
    // Abort shot if position became terrible
    input.throw_held = false;
    input.throw_released = false; // Don't release, abort charge
    ai_state.shot_charge_target = 0.0;
    ai_state.current_goal = AiGoal::AttackWithBall;
}
```

### Better Fix: Stop Movement During ChargeShot

In ChargeShot goal handling, don't allow navigation to move the AI:

```rust
// In ChargeShot, don't move - we committed to this position
if ai_state.current_goal == AiGoal::ChargeShot {
    input.move_x = 0.0;
}
```

### Long-term Fix: Test Scenarios

Create scenario tests to verify:
1. AI doesn't shoot below min_shot_quality
2. AI doesn't drift under basket during jump shots
3. AI aborts shots if pushed to bad position

---

## Event Log Analysis TODO

To find actual bad shots in gameplay:
1. Query SQLite events for ShotRelease events
2. Calculate shot quality at release position
3. Flag shots where quality < profile.min_shot_quality
4. Analyze patterns (which goals, which profiles)

---

## Test Coverage Gap

**No scenario tests exist for AI shot quality.**

Recommended new test: `tests/scenarios/ai/shot_quality.toml`

```toml
name = "AI respects min_shot_quality"
description = "AI with high min_shot_quality should not shoot from floor"

[setup]
level = "test_flat_floor"
# Place AI at floor level, far from basket
# AI profile with high min_shot_quality (0.6+)

[[setup.entities]]
type = "player"
id = "ai"
team = "left"
x = 0.0
y = -398.0
holding_ball = true
ai_profile = "Sniper"  # min_shot_quality = 0.6

[expect]
# After N frames, AI should NOT have shot (quality too low)
# Or AI should have moved to better position first
```

---

*Next action: Implement quick fix and create test scenario*
