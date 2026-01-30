# Ballgame Milestones

```
Training Tools → AI Quality → MVP → V0 → V1
```

---

## Pre-MVP: Training Pipeline

**Training Binary UX:**
- [ ] Reset button (Start) wipes logs and restarts
- [ ] Clear status display between games

**AI Plugin Consolidation:**
- [ ] Create `AiPlugin` - single source of truth
- [ ] Fix ghost mode to use full AI

**Done:**
- [x] Ghost System MVP
- [x] Reachability-aware navigation
- [x] SQLite event logging
- [x] Simulation infrastructure

---

## MVP

*Playable solo vs AI - core loop works*

**AI Behavior:**
- [ ] Fix shooting - stops taking bad shots
- [ ] Fix positioning - covers basket correctly

**Movement/Physics:**
- [ ] Tune player movement
- [ ] Tune jump feel

**Done:**
- [x] Stealing mechanics
- [x] AI profiles (10 personas)
- [x] Scenario tests (42 tests)

---

## V0

*Polished - ready to share*

- [ ] Win conditions (score/time limit)
- [ ] Game state flow (start → play → end)
- [ ] Polish existing 10 levels
- [ ] Viewport testing

---

## V1 / Beyond

**Multiplayer:**
- [ ] Local multiplayer
- [ ] 4-player support
- [ ] Netcode

**Audio:**
- [ ] Sound effects
- [ ] Music

**Menus:**
- [ ] Main menu
- [ ] Pause menu
- [ ] Settings UI

---

## Technical Debt

- [ ] PlayerId → CharacterId migration
- [ ] System wiring consolidation
- [ ] EventBus memory cleanup
