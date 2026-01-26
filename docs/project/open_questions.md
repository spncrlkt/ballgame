# Open Questions & Decisions

*Questions that need answers before proceeding with certain work.*

---

## Training Pipeline (P0-P2)

- [ ] **Ghost segmentation** - How to detect "drive" boundaries in the event stream?
  - By goal events? By possession changes? By time windows?
- [ ] **Defense success metric** - What counts as "AI stopped the play"?
  - Steal? Block? Shot miss? Any non-score outcome?
- [ ] **Input format** - Store raw inputs or preprocessed actions?

## AI Behavior (P3)

- [ ] **Bad shot definition** - What makes a shot "bad"?
  - Distance? Angle? Defender proximity? Shot quality score?
- [ ] **Good positioning** - What defines correct positioning?
  - Near basket? Between ball and basket? Based on ball holder?
- [ ] **Profile tuning** - Should profiles affect positioning or just timing?

## AI Navigation (P2)

- [ ] **Ramp-less level handling** - How should AI reach elevated opponents in levels without corner steps?
  - Option A: Always use NavGraph pathfinding (requires valid edges to platforms)
  - Option B: Direct jump toward platform if reachable
  - Option C: Give up and patrol floor (wait for opponent to come down)
- [ ] **Goal oscillation** - AI rapidly switches goals (7 instances in 23s test). Causes:
  - Conditions flip-flop near thresholds?
  - Need longer commitment timers?
  - Hysteresis (different thresholds for entering vs exiting goal)?

## Code Quality

- [ ] **Clippy warnings** - ~7 warnings remain (type_complexity, collapsible_if)
  - Worth fixing or leave as standard Bevy patterns?
- [ ] **ghost-visual.rs** - Fix or delete? (Currently broken)

---

## Resolved

- [x] **MVP definition** - Both AI + Movement need to feel good
- [x] **Training relation to MVP** - Training tools ARE MVP blockers (how we make AI good)
- [x] **Done item verification** - All 63 tests pass, items verified

---

*Last reviewed: 2026-01-25 (AI architecture session)*
