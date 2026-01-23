# Open Questions & Stray Thoughts

A collection of questions, ideas, and considerations to evaluate periodically.

---

## UI/UX

- [ ] D-pad menu: Should there be visual arrows (↑↓←→) to indicate which direction maps to which row?
- [ ] D-pad menu: Is the `>` marker for active direction visible enough?
- [ ] AI display: Is `[L* Balanced] R Balanced` easy to parse quickly?
- [ ] Should there be key hints showing LT/RT controls?

## Regression Testing

- [ ] Should we use a non-debug level for regression baseline (more deterministic)?
- [ ] Should we add multiple baseline screenshots (different levels, viewports)?
- [ ] Worth installing ImageMagick for proper pixel diff comparison?

## AI Behavior (from todo.md P1-P2)

- [ ] What makes the AI take "bad shots"? Distance? Angle? Timing?
- [ ] What defines "good positioning" for the AI? Near basket? Between ball and basket?
- [ ] Should AI profiles affect positioning strategy or just shooting parameters?

## Code Quality

- [ ] 53 clippy warnings - worth fixing collapsible_if patterns or leave as-is?
- [ ] type_complexity warnings in Bevy queries - worth creating type aliases?

---

*Last reviewed: 2026-01-23*
