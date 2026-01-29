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
│     - Generates per-level scoring probability heatmaps      │
│     - Identifies "dead zones" and optimal positions         │
│     - Output: showcase/heatmaps/heatmap_score_<level>_<uuid>_<side>.png │
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
├─────────────────────────────────────────────────────────────┤
│  4. AI MATCH ANALYSIS (optional)                            │
│     cargo run --bin simulate -- --tournament 5              │
│     cargo run --bin analyze -- logs/                        │
│     - Run AI vs AI matches and analyze outcomes             │
│     - Identify profile balance issues                       │
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

**Tournament testing** for AI balance coverage:
```bash
cargo run --bin simulate -- --tournament 5 --parallel 8
cargo run --bin analyze -- logs/
```

---

## Step 2: Verifying shot_quality.rs

The AI uses `src/ai/shot_quality.rs` to decide when to shoot. These quality values should align with actual heatmap success rates.

### Key Test Positions (Right Basket at 624, 150)

| Position | World Coords | Quality Value | Label | Heatmap |
|----------|--------------|---------------|-------|---------|
| Above basket | (324, 360) | 0.936 | EXCELLENT | Green |
| Floor shot | (-200, -418) | 0.394 | DESPERATE | Orange-red |
| Directly under | (624, -200) | 0.184 | TERRIBLE | Dark red |
| Behind basket | (750, 150) | 0.240 | TERRIBLE | Red-orange |
| Mid-range | (0, -418) | 0.394 | DESPERATE | Orange |

### Last Verified: 2026-01-24

**Summary:** The shot_quality.rs values align well with heatmap visual zones. The function is slightly conservative (lower values than raw success rates), which is appropriate for AI decision-making since it encourages seeking better positions.

### Verification Process

1. Run `cargo run --bin heatmap -- score`
2. Open `showcase/heatmaps/heatmap_score_<level>_<uuid>_<side>.png` and visually check the color zones
3. Run quality analysis on test positions (see test positions above)
4. If discrepancy > 10% in zone classification, update the quality formulas

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

## SQLite Integration (Match Results Only)

Match results from tournament/multi-match modes can be stored in SQLite:

```bash
# Run tournament and store results in database
cargo run --bin simulate -- --tournament 5 --db sim_results.db --parallel 8

# Query match results
sqlite3 sim_results.db "SELECT * FROM matches ORDER BY created_at DESC LIMIT 10"
```

**Note:** Shot test results are printed to stdout only and not stored in the database. The `--db` flag only works with match-based modes (tournament, multi-match, level-sweep).

**Benefits:**
- Track AI match outcomes over time
- Compare profile performance across sessions
- Build historical baseline for AI balance

---

## Parallel Testing

Parallel execution is available for match-based modes (not shot tests):

```bash
# Fast tournament: 8 parallel workers
cargo run --bin simulate -- --tournament 5 --parallel 8

# Multi-match with parallelism
cargo run --bin simulate -- --matches 20 --parallel 8

# Level sweep with parallelism
cargo run --bin simulate -- --level-sweep 5 --left Sniper --parallel 8
```

**Note:** Shot tests (`--shot-test`) run sequentially. The `--parallel` flag is ignored for shot tests.

For comprehensive shot accuracy testing across levels, run separately:
```bash
for level in 2 3 4 5 6; do
  cargo run --bin simulate -- --shot-test 30 --level $level
done
```

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
