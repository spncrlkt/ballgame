# Game Systems Tuning Investigation

## Scope
This document focuses on game-system tuning that affects AI behavior and match pacing without changing AI profiles. The goal is to increase scoring and match duration while preserving the current AI decision logic and profile set.

## Current Observations
- Recent tournaments show low average score (~1-2 total per match) and short matches (~52s), far below targets.
- Shot accuracy sits around ~18-22% depending on tuning, indicating the physics/variance model is a primary limiter.
- Turnovers are consistently low, suggesting steals and possession swings are limited by systemic parameters.

## Primary System Knobs (High Leverage)
### Shot Physics and Accuracy
- `SHOT_DISTANCE_VARIANCE` in `src/lib.rs` adds distance-based noise to every shot.
- `speed_randomness` in `src/shooting/throw.rs` adds ±10% random speed variance.
- `SHOT_MAX_VARIANCE` and `SHOT_MIN_VARIANCE` in `src/constants.rs` control angle variance by charge level.
- `SHOT_AIR_VARIANCE_PENALTY` and `SHOT_MOVE_VARIANCE_PENALTY` in `src/constants.rs` penalize airborne and moving shots.
- `SHOT_MAX_SPEED` and `SHOT_HARD_CAP` in `src/constants.rs` cap shot velocity (can suppress long-range makes).
- `BALL_GRAVITY` in `src/constants.rs` affects required shot speed and arc height.
- `SHOT_CHARGE_TIME` in `src/constants.rs` affects rate of shot attempts.

### Shot Cadence and Commit Rules
- Quick-shot power penalty in `src/shooting/throw.rs` (<0.25s => 0.7x power).
- `opponent_too_close` gate in `src/ai/decision.rs` prevents charging when opponent is within `STEAL_RANGE * 1.5`.
- Front-court penalty in `src/ai/decision.rs` reduces shot quality when close to basket.
- `SHOT_GRACE_PERIOD` in `src/constants.rs` controls how long shots ignore player drag.
- `DEFENSE_GRACE_REDUCTION` in `src/constants.rs` amplifies the impact of defensive shot blocking.

### Steal and Turnover Dynamics
- `STEAL_SUCCESS_CHANCE`, cooldowns, and range in `src/constants.rs` set turnover frequency.
- Steal difficulty rubber-banding in `src/steal.rs` can clamp outcomes for streaks but does not influence baseline attempt volume.

### Possession and Loose-Ball Dynamics
- `BALL_PICKUP_RADIUS`, `BALL_FREE_SPEED`, and collision drags in `src/constants.rs` affect recovery and possession tempo.
- `BALL_AIR_FRICTION`/`BALL_ROLL_FRICTION` affect how long loose balls stay in play.

### Rim/Geometry Effects (Scoring Variance)
- `BASKET_SIZE`, `RIM_THICKNESS`, `RIM_BOUNCE_RETENTION`, and `BASKET_PUSH_IN` in `src/constants.rs` affect make rate and rebound behavior.

## Important Implementation Notes
- `SHOT_MAX_POWER` is not used in the throw implementation; increasing it will not change shot behavior.
  If power scaling is desired, it must be wired into `src/shooting/throw.rs` (e.g., as a cap or multiplier).
- `DEFENSE_SHOT_VARIANCE_MAX` is defined but not referenced; if defensive shot variance is desired, it needs wiring.

## Candidate Tuning Experiments (Ordered)
1) **Accuracy + Cadence (Physics-First)** - best first pass to lift scoring with low strategic side effects.
2) **Shot Volume Under Pressure (Decision Gate Tweaks)** - increases attempts without changing physics.
3) **Turnover Rate (Possession Swings)** - raises pace; risk of chaotic play if pushed too far.
4) **Loose-Ball Tempo (Possession Recovery)** - speeds up recovery but can reduce tactical variance.
5) **Rim/Geometry Scoring (Make Rate Bias)** - easy dial, but less principled and affects all shot types.

### A) Accuracy + Cadence (Physics-First)
Goal: raise shot conversion and scoring without changing decision logic.
- Hypothesis: higher accuracy + faster charge time will raise avg score and reduce failed-shot volume without materially increasing turnovers.
- Reduce `SHOT_DISTANCE_VARIANCE` in `src/lib.rs`.
- Tighten `speed_randomness` in `src/shooting/throw.rs` (e.g., 0.95..1.05).
- Reduce `SHOT_CHARGE_TIME` in `src/constants.rs` to increase shot frequency.
- Soften quick-shot penalty in `src/shooting/throw.rs` (<0.25s multiplier).
Results template:
| Tournament DB | Avg Score | Accuracy | Duration | Turnovers | Notes |
| --- | --- | --- | --- | --- | --- |

### B) Shot Volume Under Pressure (Decision Gate Tweaks)
Goal: increase shot attempts by reducing defensive blocking rules.
- Hypothesis: more attempts will raise total score but may lower accuracy slightly and reduce possession time per team.
- Reduce `opponent_too_close` threshold multiplier in `src/ai/decision.rs`.
- Lower front-court penalty in `src/ai/decision.rs`.
Results template:
| Tournament DB | Avg Score | Accuracy | Duration | Turnovers | Notes |
| --- | --- | --- | --- | --- | --- |

### C) Turnover Rate (Possession Swings)
Goal: increase steals without overhauling AI logic.
- Hypothesis: higher turnovers will increase shot count and pacing but may shorten matches if possessions chain too fast.
- Raise `STEAL_SUCCESS_CHANCE` slightly, or lower `STEAL_COOLDOWN` / `STEAL_FAIL_COOLDOWN`.
- Consider modest `STEAL_RANGE` increase if attempts remain low.
Results template:
| Tournament DB | Avg Score | Accuracy | Duration | Turnovers | Notes |
| --- | --- | --- | --- | --- | --- |

### D) Loose-Ball Tempo (Possession Recovery)
Goal: increase pace and reduce long dead-ball sequences.
- Hypothesis: faster recoveries will raise possessions per match, mildly increasing score and turnovers.
- Increase `BALL_PICKUP_RADIUS` in `src/constants.rs`.
- Raise `BALL_FREE_SPEED` in `src/constants.rs` so balls become free sooner.
- Reduce `BALL_AIR_FRICTION` / `BALL_ROLL_FRICTION` in `src/constants.rs` to keep balls moving.
Results template:
| Tournament DB | Avg Score | Accuracy | Duration | Turnovers | Notes |
| --- | --- | --- | --- | --- | --- |

### E) Rim/Geometry Scoring (Make Rate Bias)
Goal: slightly increase makes without changing shot behavior.
- Hypothesis: higher make rate raises score with minimal AI behavior change, but may mask underlying shot-selection issues.
- Increase `BASKET_SIZE` or reduce `RIM_THICKNESS` in `src/constants.rs`.
- Increase `RIM_BOUNCE_RETENTION` in `src/constants.rs` to reduce harsh rim-outs.
Results template:
| Tournament DB | Avg Score | Accuracy | Duration | Turnovers | Notes |
| --- | --- | --- | --- | --- | --- |

## Suggested Evaluation Plan
- Run a single tournament with the current profile set after each tuning package.
- Track: avg score, duration, accuracy, turnovers, missed shots, and leaderboard stability.
- Favor small, isolated parameter changes to identify sensitivity.

## Parameter Grouping Strategy (for Tournament Comparisons)
To get clearer readouts from tournaments, group related parameters into fixed sets and iterate sets rather than one-off tweaks. This reduces noise from interdependent knobs and makes comparisons more direct.

Suggested grouping:
- **Shot Physics Set**: `SHOT_DISTANCE_VARIANCE`, `SHOT_MAX_VARIANCE`, `SHOT_MIN_VARIANCE`, `speed_randomness`.
- **Shot Cadence Set**: `SHOT_CHARGE_TIME`, quick-shot power penalty threshold/multiplier.
- **Shot Decision Gate Set**: front-court penalty, `opponent_too_close` threshold.
- **Steal/Turnover Set**: `STEAL_SUCCESS_CHANCE`, `STEAL_COOLDOWN`, `STEAL_FAIL_COOLDOWN`, `STEAL_RANGE`.
- **Ball/Possession Set**: `BALL_PICKUP_RADIUS`, `BALL_FREE_SPEED`, `BALL_AIR_FRICTION`, `BALL_ROLL_FRICTION`.
- **Rim/Geometry Set**: `BASKET_SIZE`, `RIM_THICKNESS`, `RIM_BOUNCE_RETENTION`, `BASKET_PUSH_IN`.

Recommendation:
- Only change one set per tournament to keep results attributable.
- Use consistent levels and match counts for each set.
- When a set looks promising, run a follow-up tournament with the same set to confirm stability.

## Open Questions
- Should system tuning prioritize longer matches or higher scoring first?
- Are we willing to accept lower accuracy if it materially increases scoring volume?
- Should steal frequency be raised enough to change pace, or kept lower to avoid chaos?

## Wrap-Up
This document captures the current tuning landscape, ordered experiment sets, and hypothesis templates. It is intended to be updated as tournament results are collected and compared across sets.
