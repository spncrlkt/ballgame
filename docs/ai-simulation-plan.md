# AI Simulation System - Design Plan

## Overview

A headless simulation tool to test AI behavior with real game physics, decisions, and interactions over time. Unlike the heatmap tool (which simulates just ball trajectories), this runs the full Bevy ECS with all systems.

## Goals

1. **Test AI decision-making** - Verify AI makes reasonable choices
2. **Measure AI performance** - Score rate, time to score, navigation success
3. **Compare AI profiles** - Which profiles perform better on which levels
4. **Regression testing** - Detect AI behavior changes after code changes
5. **Balance tuning** - Find optimal profile parameters

## Architecture

### Approach: Headless Bevy App

Use Bevy's `MinimalPlugins` instead of `DefaultPlugins` to run without rendering:

```rust
App::new()
    .add_plugins(MinimalPlugins)  // No window, no rendering
    .add_plugins(TimePlugin)       // Need time for physics
    // ... game systems ...
```

### Key Differences from Main Game

| Aspect | Main Game | Simulation |
|--------|-----------|------------|
| Plugins | DefaultPlugins | MinimalPlugins + TimePlugin |
| Window | Yes | No |
| Rendering | Yes | No |
| Input | Human + AI | AI only (both players) |
| Speed | 60 FPS | As fast as possible |
| Duration | Unlimited | Fixed time/score limit |
| Output | Visual | JSON statistics |

## Simulation Modes

### 1. Single Match
Run one game with specific configuration:
```bash
cargo run --bin simulate -- --level 3 --left Balanced --right Aggressive --duration 60
```

Output:
```json
{
  "level": "Islands",
  "duration_secs": 60.0,
  "left_profile": "Balanced",
  "right_profile": "Aggressive",
  "score": { "left": 3, "right": 5 },
  "shots": { "left": 12, "right": 18 },
  "shot_accuracy": { "left": 0.25, "right": 0.28 },
  "avg_time_to_score": { "left": 15.2, "right": 10.8 },
  "steals": { "left": 2, "right": 4 },
  "jumps": { "left": 45, "right": 62 }
}
```

### 2. Profile Tournament
Run all profile combinations:
```bash
cargo run --bin simulate -- --tournament --level 3 --matches 10
```

Output: Win rate matrix for each profile matchup.

### 3. Level Sweep
Test one profile across all levels:
```bash
cargo run --bin simulate -- --level-sweep --profile Balanced --duration 30
```

Output: Performance metrics per level.

### 4. Regression Test
Compare current AI to baseline metrics:
```bash
cargo run --bin simulate -- --regression
```

Output: PASS/FAIL with diff if metrics deviate significantly.

## Metrics to Collect

### Per-Player Metrics
- Goals scored
- Shots attempted / made (accuracy)
- Time holding ball
- Distance traveled
- Jumps executed
- Steals attempted / successful
- Navigation paths taken
- Time in each AI goal state

### Per-Match Metrics
- Total duration
- Score differential
- Ball possession time (per team)
- Lead changes

### Aggregate Metrics (over multiple matches)
- Win rate
- Average score
- Consistency (std dev)

## Implementation Steps

### Phase 1: Basic Headless Runner
1. Create `src/bin/simulate.rs`
2. Set up MinimalPlugins + game systems
3. Add simulation control resource (duration, stop conditions)
4. Add metrics collection resource
5. Run single match, output JSON

### Phase 2: Metrics Collection
1. Create `SimMetrics` resource to track all stats
2. Add observer systems that don't affect gameplay
3. Hook into scoring, shooting, stealing events
4. Track AI state transitions

### Phase 3: Multi-Match & Comparison
1. Add match iteration logic
2. Implement tournament mode
3. Implement level sweep
4. Add baseline comparison

### Phase 4: Analysis Tools
1. Generate summary reports
2. Create comparison tables
3. Optional: generate charts (PNG output like heatmap)

## File Structure

```
src/
├── bin/
│   ├── heatmap.rs       # Existing
│   ├── generate_ball.rs # Existing
│   └── simulate.rs      # NEW: AI simulation runner
├── simulation/          # NEW module
│   ├── mod.rs
│   ├── config.rs        # SimConfig resource
│   ├── metrics.rs       # SimMetrics resource
│   ├── runner.rs        # Headless app setup
│   └── analysis.rs      # Stats aggregation
```

## Key Technical Considerations

### 1. Determinism
- Seed RNG for reproducible results
- Fixed timestep (already using FixedUpdate)

### 2. Speed
- Remove all rendering/UI systems
- Consider multiple simultaneous simulations (parallel)

### 3. State Reset
- Need clean reset between matches
- Respawn players/ball, reset scores

### 4. No Human Input
- Both players must be AI-controlled
- Remove HumanControlled component from all players

### 5. Exit Conditions
- Time limit reached
- Score limit reached (first to N)
- Stalemate detection (no score for X seconds)

## CLI Interface

```
USAGE:
    simulate [OPTIONS]

OPTIONS:
    --level <N>         Level number (1-12, default: 2)
    --left <PROFILE>    Left player AI profile (default: Balanced)
    --right <PROFILE>   Right player AI profile (default: Balanced)
    --duration <SECS>   Match duration in seconds (default: 60)
    --score-limit <N>   End when a player reaches N points
    --matches <N>       Number of matches to run (default: 1)
    --tournament        Run all profile combinations
    --level-sweep       Test across all levels
    --seed <N>          RNG seed for reproducibility
    --output <FILE>     Output JSON file (default: stdout)
    --quiet             Suppress progress output
    --regression        Compare to baseline metrics
```

## Example Output

### Single Match
```json
{
  "config": {
    "level": 3,
    "level_name": "Islands",
    "left_profile": "Balanced",
    "right_profile": "Aggressive",
    "duration_limit": 60.0,
    "seed": 12345
  },
  "result": {
    "duration": 60.0,
    "winner": "right",
    "score": { "left": 2, "right": 4 }
  },
  "left_stats": {
    "shots_attempted": 8,
    "shots_made": 2,
    "accuracy": 0.25,
    "possession_time": 28.5,
    "steals_attempted": 3,
    "steals_successful": 1,
    "jumps": 34,
    "distance_traveled": 4520.0
  },
  "right_stats": { ... }
}
```

### Tournament Summary
```
Profile Matchup Win Rates (10 matches each):

              | Balanced | Aggressive | Defensive | Sniper
--------------+----------+------------+-----------+--------
Balanced      |    -     |    40%     |    70%    |   55%
Aggressive    |   60%    |     -      |    80%    |   65%
Defensive     |   30%    |    20%     |     -     |   40%
Sniper        |   45%    |    35%     |    60%    |    -

Best overall: Aggressive (68% win rate)
```

## Next Steps

1. Review this plan
2. Implement Phase 1 (basic runner)
3. Test with simple match
4. Iterate on metrics collection
