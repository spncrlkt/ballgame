# Open Questions

---

## V1 Questions (Deferred)

These questions will be addressed when prioritizing V1 work:

**AI Behavior:**
- [ ] Bad shot definition - Distance? Angle? Defender proximity?
- [ ] Good positioning - Near basket? Between ball and basket?

**Training Pipeline:**
- [ ] Ghost segmentation - How to detect drive boundaries?
- [ ] Defense metric - What counts as "AI stopped the play"?

**Architecture:**
- [ ] System wiring - Shared plugins vs accept divergence?
- [ ] EventBus cleanup - Clear per frame or limit history?

---

## Resolved

- [x] MVP definition - AI + Movement need to feel good
- [x] Training relation to MVP - Training tools ARE MVP blockers
- [x] ghost-visual.rs - Deleted
- [x] Cooldown timing bug - Fixed (FixedUpdate only)
- [x] V0/V1 scope - V0 = dev friends release, V1 = full release (2026-01-31)

---

*Last reviewed: 2026-01-31*
