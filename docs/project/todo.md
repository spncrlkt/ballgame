# Ballgame TODO - Current Sprint

*See `milestones.md` for full plan | `ideas.md` for notes*

---

## P0: Bug Fixes

- [ ] **Input capture bug** - Tweak panel early-return leaves stale `PlayerInput`

---

## P0: Training Binary UX

- [ ] **Reset button (Start)** - wipes logs, restarts session (preserve CLI args)
- [ ] **Clear status display** between games

---

## P1: AI Plugin Consolidation

- [ ] **Create `AiPlugin`** - Single source of truth for AI systems
- [ ] **Update all binaries** - main, training, simulation use same plugin
- [ ] **Fix ghost mode** - Use full AI decision system

---

## P2: AI Behavior

- [ ] Fix shooting - AI takes bad shots, misses easy ones
- [ ] Fix positioning - AI stands in wrong places, doesn't cover basket

---

## P3: Movement Feel

- [ ] Tune player movement - speed, acceleration, air control
- [ ] Tune jump feel - height, coyote time, responsiveness

---

## Backlog

**Technical Debt:**
- PlayerId → CharacterId migration (~30 deprecation warnings)
- System wiring drift across binaries
- EventBus `processed` grows unbounded

**Features:**
- Visual ghost mode in main game
- More ball styles
- AI debug level (both players AI)

**Documentation:**
- AI_PROFILES.md, LEVELS.md

---

## Done (Last 5)

- [x] Cooldown timing bug fix (2026-01-30)
- [x] Nav/Pathfinding debug logging (2026-01-30)
- [x] Binary Reference Guide (2026-01-29)
- [x] Unified Run Summary (2026-01-29)
- [x] Reachability-Aware Navigation (2026-01-28)

*See `todone.md` for full archive*
