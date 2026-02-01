# Ballgame Milestones

```
V0 (today) → V1 (future)
```

---

## V0: Dev Friends Release

*Tight, focused, fun game loop. Endless play, no win conditions. Ready to share.*

**Status:** Closing out today (2026-01-31)

**What's Done:**
- [x] 2v2 gameplay with team mechanics (pass, steal, block, turbo)
- [x] 10 AI profiles with human-realistic timing
- [x] Reachability-aware navigation
- [x] Ghost system (drive recording + replay)
- [x] SQLite event logging
- [x] Training mode with session summaries
- [x] Simulation infrastructure (tournaments, brackets)
- [x] 42 scenario tests + 151 unit tests
- [x] 10 levels with hot-reload config
- [x] 30 color palettes, 10 ball styles
- [x] Preset system (movement, ball, shooting)
- [x] Stealing mechanics with cooldowns
- [x] Countdown system
- [x] Visual regression testing

**Remaining V0 Polish:**
- [ ] Verify DB path consolidation works
- [ ] Manual playtest - confirm game feels fun
- [ ] Fix any critical bugs found during playtest

---

## V1: Full Release

*Everything else. Will prioritize when V0 ships.*

**Win Conditions & Game Flow:**
- [ ] Score limit or time limit
- [ ] Game state flow (start → play → end → rematch)
- [ ] Victory screen

**AI Tuning:**
- [ ] Fix shooting - stops taking bad shots
- [ ] Fix positioning - covers basket correctly
- [ ] Define "bad shot" (distance? angle? defender?)
- [ ] Define "good positioning" (shot line? basket coverage?)

**Movement/Physics Tuning:**
- [ ] Tune player movement (speed, acceleration, air control)
- [ ] Tune jump feel (height, coyote time, responsiveness)
- [ ] Fix step climbing issues
- [ ] Fix AI running jump estimation

**Training Pipeline Polish:**
- [ ] Reset button (Start) wipes logs and restarts
- [ ] Clear status display between games
- [ ] Create `AiPlugin` - single source of truth
- [ ] Fix ghost mode to use full AI

**Level Polish:**
- [ ] Polish existing 10 levels
- [ ] Viewport testing across sizes

**Multiplayer:**
- [ ] Local multiplayer (2 humans)
- [ ] 4-player support (4 humans)
- [ ] Netcode

**Audio:**
- [ ] Sound effects
- [ ] Music

**Menus & UI:**
- [ ] Main menu
- [ ] Pause menu
- [ ] Settings UI
- [ ] Animated gif for readme

**Buff System Tests:**
- [ ] Speed buff test: player with Speed moves faster than baseline
- [ ] Turbo buff test: player with Turbo has larger gauge, faster refill
- [ ] Accuracy buff test: shots have less variance with Accuracy
- [ ] Steal buff test: steal success rate higher with Steal buff
- [ ] Jump buff test: player with Jump reaches higher
- [ ] Defense buff test: steal resistance higher with Defense
- [ ] Recovery buff test: steal/block cooldowns shorter with Recovery

**Technical Debt:**
- [ ] System wiring consolidation across binaries
- [ ] EventBus memory cleanup (unbounded growth)
- [ ] Input capture bug (tweak panel stale PlayerInput)
- [ ] Simulation setup missing Buff component (affects tournament metrics)
- [ ] Training binary should use spawn_characters_for_mode() for buff support

**Ideas (unprioritized):**
- More ball styles
- AI debug level (both players AI)
- Steal balance testing protocol
- Platforming training protocol
- Parallelize heatmap generation

---

*Last updated: 2026-01-31*
