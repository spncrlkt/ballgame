# Open Questions

---

## Training Pipeline

- [ ] **Ghost segmentation** - How to detect drive boundaries? (goals, possession, time?)
- [ ] **Defense metric** - What counts as "AI stopped the play"?

## AI Behavior

- [ ] **Bad shot definition** - Distance? Angle? Defender proximity?
- [ ] **Good positioning** - Near basket? Between ball and basket?

## Architecture

- [ ] **System wiring** - Shared plugins vs accept divergence?
- [ ] **EventBus cleanup** - Clear per frame or limit history?

---

## Resolved

- [x] MVP definition - AI + Movement need to feel good
- [x] Training relation to MVP - Training tools ARE MVP blockers
- [x] ghost-visual.rs - Deleted
- [x] Cooldown timing bug - Fixed (FixedUpdate only)

---

*Last reviewed: 2026-01-30*
