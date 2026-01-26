# AI Profile Evolution Notes

Reference document for AI tournament simulation and profile optimization.

## Quick Reference Commands

```bash
# Run tournament (N rounds, 8 parallel workers, 60s matches)
rm -f logs/*.db && cargo run --release --bin simulate -- --tournament N --parallel 8 --duration 60 --db logs/tournament.db

# Analyze results and save to timestamped file
cargo run --release --bin analyze -- logs/tournament.db 2>&1 | tee rankings_vX_$(date +%Y%m%d_%H%M%S).txt
```

## Evolution History

### Generation Summary

| Gen | Date | Profiles | Champion | Win% | Notes |
|-----|------|----------|----------|------|-------|
| v1 | Jan 25 | 5 | Rusher | 26.3% | Original baseline profiles |
| v2 | Jan 25 | 10 | Rusher | 24.7% | Scaled extremes of top 5 |
| v3 | Jan 25 | 57 | Rush_Patient | 26.9% | 50 variants + historical |
| v4 | Jan 25 | 59 | v1_Rusher | 26.3% | v1_Rusher reclaimed top spot |

### Key Finding
**v1_Rusher appears to be at or near the global optimum.** Four generations of evolutionary optimization have not found a profile that consistently outperforms the original Rusher baseline.

## Optimal Parameter Ranges (from v1-v4 analysis)

| Parameter | Optimal Range | Notes |
|-----------|---------------|-------|
| aggression | 0.85-0.95 | Sweet spot; >0.97 or <0.82 underperforms |
| position_patience | 0.45-0.55 | v4_Pat_50 was best new profile |
| min_shot_quality | 0.22-0.28 | Lower is better; sniper builds fail |
| charge_min | 0.25-0.35 | Quick shots need to be viable |
| charge_max | 0.55-0.75 | Don't overcharge |
| shoot_range | 600-800 | Must be large enough for arena |
| steal_range | 80-120 | Standard range works |

## What Works

1. **Rush lineage dominates** - High aggression + moderate patience
2. **Lower min_shot_quality** - Profiles that wait for perfect shots lose
3. **Quick charging** - charge_min 0.25-0.35s optimal
4. **Moderate patience** - 0.45-0.55 outperforms extremes

## What Fails

1. **Sniper builds** - High min_shot_quality (>0.35) consistently underperform
2. **Extreme patience** - >0.60 patience loses to aggressive profiles
3. **Low aggression** - <0.80 aggression can't compete
4. **Defensive focus** - High defensive_iq without offense loses

## Code Changes Made During Optimization

### src/constants.rs
- `SHOT_MAX_VARIANCE`: 0.50 -> 0.35 (reduce angle deviation on quick shots)

### src/shooting/throw.rs
- Quick shot penalty: 50% at <400ms -> 70% at <250ms (make AI quick shots viable)

### src/ai/decision.rs
- Added forced shot after 6 seconds of ball possession (prevents stalling)
- Seek utility: height bonus up to +0.15, path cost 0.3->0.15

### src/ai/navigation.rs
- Path cost: `dx + dy * 2.0` -> `dx + dy_up * 1.2 + dy_down * 0.3`

## Rankings Files

Rankings are saved with timestamps for historical comparison:
- `rankings_v4_20260125_161950.txt` - v4 tournament (59 profiles)

## Next Steps to Try

1. **Micro-optimize around v1_Rusher** - Create v5 with ±5% variations of Rusher params
2. **Level-specific profiles** - Some profiles may excel on certain level types
3. **Increase match duration** - Current 60s may not differentiate skill well
4. **Fix underlying issues** - Shot accuracy at 14.6% suggests physics/aiming problems

## Simulation Health Metrics

Target vs Actual (from v4):
- Avg score: 0.4 (target: 14.0) - FAIL
- Shot accuracy: 14.6% (target: ~50%) - Very low
- Turnovers/match: 4.2 (target: 20.0) - Low engagement

The low scores suggest fundamental issues beyond profile tuning - may need physics/shooting system changes.
