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

**Ghost System:**
- [ ] Extract player drives from SQLite events (segment by goals)
- [ ] Ghost replay mode - one player follows recorded inputs
- [ ] `--ghost` flag works in simulate binary
- [ ] Defense test: measure AI success at stopping recorded human plays

**AI Navigation:**
- [ ] Fix corner step traversal (nav graph debug logging added)
- [ ] Teach AI its jump capability (skip intermediate steps)
- [ ] Verify AI can climb corner steps on levels 7-8

**WIP Files (need fixing):**
- [ ] `src/bin/ghost-visual.rs` - compilation errors
- [ ] `src/simulation/ghost.rs` - integration pending

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
- [x] Simulation infrastructure (parallel, SQLite, analytics)

---

## V0

*Polished core + levels - ready to share*

**Polish:**
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
