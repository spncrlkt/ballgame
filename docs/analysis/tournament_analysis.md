# Tournament Simulation Analysis

*Run: 2026-01-25*
*Config: 270 matches, 10 profiles, 3 matches per pair, Level 2, 45s duration*

---

## CRITICAL BUG FOUND

### 4 Profiles NEVER Shoot

| Profile | min_shot_quality | Shots Taken | Wins |
|---------|------------------|-------------|------|
| Defensive | 0.50 | **0** | 0 |
| Patient | 0.50 | **0** | 0 |
| Sniper | 0.60 | **0** | 0 |
| Turtle | 0.55 | **0** | 0 |

### Root Cause

On Level 2 "Open Floor", floor shots achieve max ~0.51 quality.
Profiles with min_shot_quality >= 0.50 can barely shoot or not at all.

### Impact

- **54.8% of matches ended 0-0**
- **80% of matches had fewer than 5 total shots**
- These 4 profiles are effectively broken

---

## Profile Rankings

| Profile | W | L | D | Win% | Goals | Shots | Avg Quality |
|---------|---|---|---|------|-------|-------|-------------|
| Balanced | 29 | 7 | 18 | 53.7% | 45 | 146 | 0.48 |
| Goalie | 26 | 9 | 19 | 48.1% | 34 | 121 | 0.49 |
| Chaotic | 16 | 11 | 27 | 29.6% | 20 | 100 | 0.47 |
| Aggressive | 14 | 7 | 33 | 25.9% | 20 | 120 | 0.49 |
| Rusher | 14 | 9 | 31 | 25.9% | 17 | 157 | 0.46 |
| Hunter | 13 | 13 | 28 | 24.1% | 21 | 87 | 0.49 |
| Defensive | 0 | 13 | 41 | 0.0% | 0 | 0 | - |
| Sniper | 0 | 16 | 38 | 0.0% | 0 | 0 | - |
| Turtle | 0 | 13 | 41 | 0.0% | 0 | 0 | - |
| Patient | 0 | 14 | 40 | 0.0% | 0 | 0 | - |

---

## Shot Quality Distribution

| Profile | Min | Avg | Max | Shot Count |
|---------|-----|-----|-----|------------|
| Chaotic | 0.17 | 0.47 | 0.51 | 45 |
| Rusher | 0.28 | 0.46 | 0.51 | 50 |
| Balanced | 0.32 | 0.48 | 0.51 | 50 |
| Aggressive | 0.37 | 0.49 | 0.51 | 47 |
| Hunter | 0.38 | 0.49 | 0.51 | 38 |
| Goalie | 0.45 | 0.49 | 0.51 | 48 |

**Key insight:** Max quality achieved is 0.51. No profile ever found a position with quality > 0.51 on Level 2.

---

## Recommended Fixes

### Option A: Lower Profile Thresholds (Quick Fix)

```
Sniper:    0.60 → 0.48  (shoots from good positions)
Turtle:    0.55 → 0.42  (patient but will shoot)
Defensive: 0.50 → 0.42  (defensive but functional)
Patient:   0.50 → 0.42  (patient but functional)
```

### Option B: Add Desperation Timer

After holding ball for N seconds without shooting, progressively lower threshold:
- 0-5s: Use profile's min_shot_quality
- 5-10s: min_shot_quality * 0.8
- 10-15s: min_shot_quality * 0.6
- 15s+: Shoot from any position

### Option C: Boost Floor Shot Quality

In `evaluate_shot_quality()`, increase base quality from 0.45 to 0.52:
```rust
// Start with base quality that allows floor shots
let mut quality = 0.52;  // Was 0.45
```

---

## Action Items

1. **IMMEDIATE**: Fix profile thresholds so all profiles can shoot
2. **TEST**: Re-run tournament after fix, verify 0-0 rate drops below 10%
3. **CONSIDER**: Add desperation timer for better gameplay feel

---

*Analysis completed by tournament simulation*
