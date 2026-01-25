# Balance Testing Workflow

Based on game balance research and industry best practices.

## Core Principle: Iterative Testing

The workflow follows an iterative cycle: **Modify → Test → Analyze → Refine**

- **Automated balance testing** can identify statistical imbalances in hours vs weeks of human playtesting
- **AI testing cannot capture subjective feel** - statistical analysis identifies imbalances but human testing validates "game feel"
- **Real basketball physics** inform our targets: optimal launch angle varies by distance (72° close, 51° mid, 45° far)

---

## Workflow Steps

```
┌─────────────────────────────────────────────────────────────┐
│  1. VISUALIZE                                               │
│     cargo run --bin heatmap -- score                        │
│     - Generates scoring probability heatmap                 │
│     - Identifies "dead zones" and optimal positions         │
│     - Output: heatmap_score.png                             │
├─────────────────────────────────────────────────────────────┤
│  2. VERIFY AI QUALITY ESTIMATES                             │
│     Compare shot_quality.rs values to heatmap:              │
│     - Sample 5+ key positions                               │
│     - Flag if >10% discrepancy                              │
│     - Update formulas to match actual success rates         │
├─────────────────────────────────────────────────────────────┤
│  3. STATISTICAL TESTING                                     │
│     cargo run --bin simulate -- --shot-test 50 --level 3    │
│     Target: 40-60% overshoot/undershoot ratio               │
│     - Results stored in SQLite for trend analysis           │
│     - Use --parallel N for faster batch testing             │
├─────────────────────────────────────────────────────────────┤
│  4. TREND ANALYSIS                                          │
│     cargo run --bin analyze -- --db sim_results.db --trend  │
│     - Compare current results to historical baseline        │
│     - Detect regressions early                              │
│     - Track improvement over tuning iterations              │
├─────────────────────────────────────────────────────────────┤
│  5. HUMAN FEEL TESTING                                      │
│     Play the game manually and note:                        │
│     - Do shots "feel" satisfying?                           │
│     - Is variance frustrating or appropriate?               │
│     - Does skill feel rewarded?                             │
│     (AI testing cannot capture subjective feel)             │
├─────────────────────────────────────────────────────────────┤
│  6. TUNE & ITERATE                                          │
│     Adjust physics parameters, then return to step 1        │
│     - Small changes, test frequently                        │
│     - Document what changed and why                         │
└─────────────────────────────────────────────────────────────┘
```

---

## When to Run This Workflow

Run the full workflow after any change to:
- `src/shooting/throw.rs` - Shot physics
- `src/shooting/trajectory.rs` - Trajectory calculations
- `src/ball/` - Ball physics (gravity, bounce, friction)
- Basket positions or arena layout
- Before major releases (full regression)

**Quick checks** (just step 3) during normal development:
```bash
cargo run --bin simulate -- --shot-test 30 --level 3
```

**Fast parallel testing** for comprehensive coverage:
```bash
cargo run --bin simulate -- --shot-test 100 --level 3 --parallel 8 --db sim_results.db
```

---

## Step 2: Verifying shot_quality.rs

The AI uses `src/ai/shot_quality.rs` to decide when to shoot. These quality values should align with actual heatmap success rates.

### Key Test Positions (Right Basket)

| Position | Description | Expected Quality | Heatmap Target |
|----------|-------------|------------------|----------------|
| Above basket (y+150, x-300) | Elevated, good angle | 0.75+ (EXCELLENT) | 60-80% |
| Floor shot (y=-418, x=-200) | Common shot | 0.35-0.50 | 30-50% |
| Directly under basket | Very difficult | < 0.40 (ACCEPTABLE) | < 30% |
| Behind basket (wall side) | Bad angle | < 0.55 (GOOD) | < 40% |
| Mid-range (y=0, x=-350) | Medium distance | 0.45-0.60 | 40-55% |

### Verification Process

1. Run `cargo run --bin heatmap -- score`
2. Open `heatmap_score.png` and note success rates at test positions
3. Compare to `shot_quality.rs` values at same positions
4. If discrepancy > 10%, update the quality formulas

---

## Heatmap vs Game Physics Alignment

The heatmap now matches actual game physics from `throw.rs`:

| Factor | Heatmap | Game |
|--------|---------|------|
| Angle variance | SHOT_MIN_VARIANCE + distance | Same (ideal shot) |
| Speed randomness | ±10% (0.9..1.1) | ±10% (0.9..1.1) |
| Distance multiplier | 1.0→1.05 linear | 1.0→1.05 linear |

**Note:** The heatmap represents "ideal" fully-charged stationary shots. Real game shots also include:
- Charge-based variance (MAX→MIN based on charge time)
- Air penalty (+10% variance when airborne)
- Movement penalty (+10% variance at full horizontal speed)
- Power multiplier (0.5x for quick shots < 400ms charge)

---

## SQLite Integration

Balance test results are stored in SQLite for historical analysis:

```bash
# Run tests and store results
cargo run --bin simulate -- --shot-test 50 --level 3 --db sim_results.db

# Query historical data
sqlite3 sim_results.db "SELECT * FROM shot_tests ORDER BY created_at DESC LIMIT 10"

# Analyze trends
cargo run --bin analyze -- --db sim_results.db --trend shot_accuracy --days 7
```

**Benefits:**
- Track balance metrics over time
- Detect regressions automatically
- Compare before/after physics changes
- Build historical baseline for "known good" state

---

## Parallel Testing

For comprehensive balance testing, use parallel execution:

```bash
# Fast: 8 parallel workers
cargo run --bin simulate -- --shot-test 100 --parallel 8

# Full regression: all levels
for level in 1 2 3 4 5; do
  cargo run --bin simulate -- --shot-test 50 --level $level --parallel 8 --db sim_results.db
done
```

The simulation infrastructure uses `HeadlessAppBuilder` with minimal thread pools, allowing many parallel simulations without hitting OS thread limits.

---

## Known Issues (V1 Trajectory Overhaul)

The minimum-energy trajectory formula has known limitations:

1. **Fixed angle calculation** - Uses single optimal angle regardless of distance
   - Real physics: 72° close, 51° mid, 45° far
   - Current: Same formula for all distances

2. **Aim point** - Currently aims at basket center
   - Real physics: Aim ~5cm from back of rim for ~3% better accuracy

3. **Variance magnitude** - ±10% may be too high
   - Real player intra-individual velocity SD is 0.05-0.13 m/s

These are tracked in `milestones.md` under "Physics/Shooting Overhaul".

---

## Sources

- [Automated Game Balance with Autonomous Agents (Politowski et al., 2023)](https://arxiv.org/abs/2304.08699)
- [Game Balance Concepts - Metrics & Statistics](https://gamebalanceconcepts.wordpress.com/2010/08/25/level-8-metrics-and-statistics/)
- [AI-Powered Playtesting (Wayline)](https://www.wayline.io/blog/ai-powered-playtesting-revolutionizing-game-balance)
- [Physics of Basketball Free-Throw Shooting (NC State)](https://engr.ncsu.edu/news/2009/11/06/nothing-but-net-the-physics-of-free-throw-shooting/)
