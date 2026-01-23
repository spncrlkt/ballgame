# Ballgame TODO - Current Sprint

*See milestones.md for the full project plan (MVP → V0 → V1)*

## Active Work (Prioritized)

### 1. AI Behavior
- [ ] **P1** Fix AI shooting - takes bad shots, misses easy ones
- [ ] **P2** Fix AI positioning - stands in wrong places, doesn't cover basket well

### 2. Movement/Physics Tuning
- [ ] **P3** Tune player movement - speed, acceleration, air control
- [ ] **P4** Tune jump feel - height, coyote time, responsiveness

### 3. Polish
- [ ] **P5** UI fix flash on score color
- [ ] **P6** Viewport testing at various resolutions

---

## Done (recent - see todone.md for full archive)
- [x] Simplified steal system - removed button mashing, instant steal attempts (33% base, 50% if charging)
- [x] Added 1-second no-stealback cooldown for steal victims
- [x] Both players have independent AI profiles (LT selects player, RT cycles profile)
- [x] Added Observer mode to player control cycling (Left → Right → Observer → Left)
- [x] Game Presets system - Movement/Ball/Shooting/Global presets with hot-reload
- [x] Added extreme presets (Slippery, Precise, Pinball, Dead, Sniper, Spam, Chaos, Tactical)
- [x] Global preset is now first/default option in D-pad menu
- [x] D-pad menu stays visible (toggle with Tab), Up cycles backwards
- [x] Removed smallest 3 viewport options (too small to be useful)
- [x] Palette 26 is now the default
