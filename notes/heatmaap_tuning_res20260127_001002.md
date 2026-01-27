# Heatmap Tuning Tournament Results (Extreme)

Baseline DB: tournament_20260126_074832.db

Run date: 2026-01-27 00:12:39

## Variant A: Aggressive Shot Volume (Global)

- Status: FAILED
- Exit code: 101
- Elapsed: 3.8s
- Output (tail):

```warning: unused import: `crate::constants::*`
 --> src/shooting/throw.rs:9:5
  |
9 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/charge_gauge.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/tweak_panel.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/world/mod.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `constants::*`
  --> src/lib.rs:45:9
   |
45 | pub use constants::*;
   |         ^^^^^^^^^^^^

error[E0689]: can't call method `sqrt` on ambiguous numeric type `{float}`
  --> src/ai/capabilities.rs:30:64
   |
30 |         let fall_time = (2.0 * max_jump_height / GRAVITY_FALL).sqrt();
   |                                                                ^^^^

Some errors have detailed explanations: E0425, E0689.
For more information about an error, try `rustc --explain E0425`.
warning: `ballgame` (lib) generated 23 warnings
error: could not compile `ballgame` (lib) due to 473 previous errors; 23 warnings emitted
```

## Variant G: Charge Aggression

- DB: /Users/spncr/dev/ballgame/db/tournament_20260127_001302.db
- Goals/match: 1.431
- Shots/match: 9.484
- Shot%: 0.127
- Avg shot quality: 0.417
- Scoreless rate: 0.204
- Winners: {'left': 301, 'right': 315, 'tie': 284}
- Best profile: v10_Rand_F (42.8%)

## Variant I: Ultra Permissive Everything

- Status: FAILED
- Exit code: 101
- Elapsed: 3.7s
- Output (tail):

```warning: unused import: `crate::constants::*`
 --> src/shooting/throw.rs:9:5
  |
9 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/charge_gauge.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/tweak_panel.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/world/mod.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `constants::*`
  --> src/lib.rs:45:9
   |
45 | pub use constants::*;
   |         ^^^^^^^^^^^^

error[E0689]: can't call method `sqrt` on ambiguous numeric type `{float}`
  --> src/ai/capabilities.rs:30:64
   |
30 |         let fall_time = (2.0 * max_jump_height / GRAVITY_FALL).sqrt();
   |                                                                ^^^^

Some errors have detailed explanations: E0425, E0689.
For more information about an error, try `rustc --explain E0425`.
warning: `ballgame` (lib) generated 23 warnings
error: could not compile `ballgame` (lib) due to 473 previous errors; 23 warnings emitted
```

## Variant J: LOS Almost Off

- Status: FAILED
- Exit code: 101
- Elapsed: 3.7s
- Output (tail):

```warning: unused import: `crate::constants::*`
 --> src/shooting/throw.rs:9:5
  |
9 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/charge_gauge.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/tweak_panel.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/world/mod.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `constants::*`
  --> src/lib.rs:45:9
   |
45 | pub use constants::*;
   |         ^^^^^^^^^^^^

error[E0689]: can't call method `sqrt` on ambiguous numeric type `{float}`
  --> src/ai/capabilities.rs:30:64
   |
30 |         let fall_time = (2.0 * max_jump_height / GRAVITY_FALL).sqrt();
   |                                                                ^^^^

Some errors have detailed explanations: E0425, E0689.
For more information about an error, try `rustc --explain E0425`.
warning: `ballgame` (lib) generated 23 warnings
error: could not compile `ballgame` (lib) due to 473 previous errors; 23 warnings emitted
```

## Variant K: Extreme Range

- Status: FAILED
- Exit code: 101
- Elapsed: 3.5s
- Output (tail):

```warning: unused import: `crate::constants::*`
 --> src/shooting/throw.rs:9:5
  |
9 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/charge_gauge.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/tweak_panel.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/world/mod.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `constants::*`
  --> src/lib.rs:45:9
   |
45 | pub use constants::*;
   |         ^^^^^^^^^^^^

error[E0689]: can't call method `sqrt` on ambiguous numeric type `{float}`
  --> src/ai/capabilities.rs:30:64
   |
30 |         let fall_time = (2.0 * max_jump_height / GRAVITY_FALL).sqrt();
   |                                                                ^^^^

Some errors have detailed explanations: E0425, E0689.
For more information about an error, try `rustc --explain E0425`.
warning: `ballgame` (lib) generated 23 warnings
error: could not compile `ballgame` (lib) due to 473 previous errors; 23 warnings emitted
```

## Variant L: Heatmap Ignore + Low Quality

- Status: FAILED
- Exit code: 101
- Elapsed: 3.6s
- Output (tail):

```warning: unused import: `crate::constants::*`
 --> src/shooting/throw.rs:9:5
  |
9 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/charge_gauge.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/tweak_panel.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/world/mod.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `constants::*`
  --> src/lib.rs:45:9
   |
45 | pub use constants::*;
   |         ^^^^^^^^^^^^

error[E0689]: can't call method `sqrt` on ambiguous numeric type `{float}`
  --> src/ai/capabilities.rs:30:64
   |
30 |         let fall_time = (2.0 * max_jump_height / GRAVITY_FALL).sqrt();
   |                                                                ^^^^

Some errors have detailed explanations: E0425, E0689.
For more information about an error, try `rustc --explain E0425`.
warning: `ballgame` (lib) generated 23 warnings
error: could not compile `ballgame` (lib) due to 473 previous errors; 23 warnings emitted
```

## Variant M: Overly Conservative (High Thresholds)

- Status: FAILED
- Exit code: 101
- Elapsed: 3.6s
- Output (tail):

```warning: unused import: `crate::constants::*`
 --> src/shooting/throw.rs:9:5
  |
9 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/charge_gauge.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/tweak_panel.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/world/mod.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `constants::*`
  --> src/lib.rs:45:9
   |
45 | pub use constants::*;
   |         ^^^^^^^^^^^^

error[E0689]: can't call method `sqrt` on ambiguous numeric type `{float}`
  --> src/ai/capabilities.rs:30:64
   |
30 |         let fall_time = (2.0 * max_jump_height / GRAVITY_FALL).sqrt();
   |                                                                ^^^^

Some errors have detailed explanations: E0425, E0689.
For more information about an error, try `rustc --explain E0425`.
warning: `ballgame` (lib) generated 23 warnings
error: could not compile `ballgame` (lib) due to 473 previous errors; 23 warnings emitted
```

## Variant N: Min Quality Floor Collapse

- Status: FAILED
- Exit code: 101
- Elapsed: 3.5s
- Output (tail):

```warning: unused import: `crate::constants::*`
 --> src/shooting/throw.rs:9:5
  |
9 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/charge_gauge.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/tweak_panel.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/world/mod.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `constants::*`
  --> src/lib.rs:45:9
   |
45 | pub use constants::*;
   |         ^^^^^^^^^^^^

error[E0689]: can't call method `sqrt` on ambiguous numeric type `{float}`
  --> src/ai/capabilities.rs:30:64
   |
30 |         let fall_time = (2.0 * max_jump_height / GRAVITY_FALL).sqrt();
   |                                                                ^^^^

Some errors have detailed explanations: E0425, E0689.
For more information about an error, try `rustc --explain E0425`.
warning: `ballgame` (lib) generated 23 warnings
error: could not compile `ballgame` (lib) due to 472 previous errors; 23 warnings emitted
```

## Variant O: Extreme Charge + Range

- Status: FAILED
- Exit code: 101
- Elapsed: 3.5s
- Output (tail):

```warning: unused import: `crate::constants::*`
 --> src/shooting/throw.rs:9:5
  |
9 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/charge_gauge.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/tweak_panel.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/world/mod.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `constants::*`
  --> src/lib.rs:45:9
   |
45 | pub use constants::*;
   |         ^^^^^^^^^^^^

error[E0689]: can't call method `sqrt` on ambiguous numeric type `{float}`
  --> src/ai/capabilities.rs:30:64
   |
30 |         let fall_time = (2.0 * max_jump_height / GRAVITY_FALL).sqrt();
   |                                                                ^^^^

Some errors have detailed explanations: E0425, E0689.
For more information about an error, try `rustc --explain E0425`.
warning: `ballgame` (lib) generated 23 warnings
error: could not compile `ballgame` (lib) due to 472 previous errors; 23 warnings emitted
```

## Variant P: Heatmap Weight Extreme

- Status: FAILED
- Exit code: 101
- Elapsed: 3.4s
- Output (tail):

```warning: unused import: `crate::constants::*`
 --> src/shooting/throw.rs:9:5
  |
9 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/charge_gauge.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/ui/tweak_panel.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `crate::constants::*`
 --> src/world/mod.rs:5:5
  |
5 | use crate::constants::*;
  |     ^^^^^^^^^^^^^^^^^^^

warning: unused import: `constants::*`
  --> src/lib.rs:45:9
   |
45 | pub use constants::*;
   |         ^^^^^^^^^^^^

error[E0689]: can't call method `sqrt` on ambiguous numeric type `{float}`
  --> src/ai/capabilities.rs:30:64
   |
30 |         let fall_time = (2.0 * max_jump_height / GRAVITY_FALL).sqrt();
   |                                                                ^^^^

Some errors have detailed explanations: E0425, E0689.
For more information about an error, try `rustc --explain E0425`.
warning: `ballgame` (lib) generated 23 warnings
error: could not compile `ballgame` (lib) due to 473 previous errors; 23 warnings emitted
```

# Analysis

## Baseline
- Goals/match: 1.981
- Shots/match: 8.959
- Shot%: 0.196
- Avg shot quality: 0.427
- Scoreless rate: 0.121

## Winners by Metric
- Highest goals/match: Variant G (1.431)
- Highest shots/match: Variant G (9.484)
- Highest avg shot quality: Variant G (0.417)
- Lowest scoreless rate: Variant G (0.204)

## Recommendations
- If shots/match spikes but goals/match drops (like extreme charge), pull back charge_min/charge_max reductions and raise min_shot_quality slightly.
- If goals/match holds while shots rise, keep LOS threshold low and margin high; that combo drives pace without destroying accuracy.
- If avg shot quality drops too far, increase HEATMAP_SCORE_WEIGHT_DEFAULT or raise per-level heatmap_score_weight on low-shot levels.
- Overly conservative settings (high LOS + high min_shot_quality) predictably reduce pace; avoid those if the goal is action.
- The best target is a variant that beats baseline on shots/match and goals/match while keeping scoreless rate low.
