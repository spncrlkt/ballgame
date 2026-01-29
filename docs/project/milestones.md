# Ballgame Milestones

## Dependency Chain

```
Training Tools → AI Quality → MVP (playable) → V0 (polished) → V1 (multiplayer)
```

---

## Pre-MVP: Training Pipeline

*The training tools are how we make AI good. They come first.*

**Training Binary UX:**
- [ ] Reset button (Start) wipes logs and restarts session
- [ ] Preserve CLI args on reset, cycle defaults otherwise
- [ ] Clear status display between games

**AI Navigation:**
- [ ] Fix ramp-less level fallback (InterceptDefense assumes ramps exist)
- [ ] Reduce goal oscillation (hysteresis or commitment timers)
- [ ] Teach AI its jump capability (skip intermediate steps)
- [ ] Verify AI can climb corner steps on levels 7-8

**AI Plugin Consolidation:**
- [ ] Create `AiPlugin` - single source of truth for AI systems
- [ ] Fix ghost mode to use full AI (not simplified)

**Done:**
- [x] Ghost System MVP (extract-drives, run-ghost, defense metrics)
- [x] Reachability-aware navigation (exploration data → shooting positions)
- [x] SQLite event logging infrastructure
- [x] Simulation infrastructure (parallel, analytics)

---

## MVP

*Playable solo vs AI - core loop works, games feel competitive*

**AI Behavior:**
- [ ] AI plays competently (no obvious mistakes)
- [ ] Fix shooting - stops taking bad shots, hits easy ones
- [ ] Fix positioning - covers basket, doesn't stand in wrong places

**Movement/Physics:**
- [ ] Tune player movement - speed, acceleration, air control
- [ ] Tune jump feel - height, coyote time, responsiveness

**Done:**
- [x] Stealing mechanics (33% base, 50% if charging, cooldowns)
- [x] AI profiles (10 personas with tunable parameters)
- [x] Scenario test suite (35 tests across 6 categories)

---

## V0

*Polished core + levels - ready to share*

**Polish:**
- [ ] Debug capture cleanup (flag/config audit, schema finalize, sampling validation)
- [ ] UI fix flash on score color
- [ ] D-pad menu UX improvements
- [ ] Viewport testing at all resolutions

**Gameplay Structure:**
- [ ] Win conditions (score limit or time limit)
- [ ] Game state flow (start → play → end → restart)

**Level Design:**
- [ ] Polish existing 10 levels
- [ ] Level editor or easier creation workflow

---

## V1 / Beyond

*Multiplayer, audio, deeper systems*

**Multiplayer:**
- [ ] 1v1 local multiplayer
- [ ] 4-player support
- [ ] Netcode architecture

**Audio:**
- [ ] Sound effects (jumps, shots, scores, steals)
- [ ] Music

**Menus:**
- [ ] Start screen / main menu
- [ ] Pause menu
- [ ] Settings UI

**Physics Overhaul:**
- [ ] Shot trajectory rework (distance-dependent angles)
- [ ] Ball physics tuning

**Persistence:**
- [ ] Save data / player profiles
- [ ] Stats tracking
- [ ] Settings persistence

---

## Decision Docs Needed

- [ ] **Netcode architecture** - Required before multiplayer (V1)
- [ ] **Input-first logging** - Refactor event logging to be replay-deterministic
