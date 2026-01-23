# Ballgame TODO - Current Sprint

*See milestones.md for the full project plan (MVP → V0 → V1)*

## Active Work (Prioritized)

### 1. Stealing Mechanics
- [ ] **P1** Steal pushback - knockback on successful steal to prevent immediate steal-back
- [ ] **P2** Add randomness to steal contests (not just button mashing)
- [ ] **P3** Make stealing easier if ball holder is charging a shot
- [ ] **P4** Steal cooldown or fatigue to prevent spam

### 2. AI Behavior
- [ ] **P5** Fix AI shooting - takes bad shots, misses easy ones
- [ ] **P6** Fix AI positioning - stands in wrong places, doesn't cover basket well

### 3. Movement/Physics
- [ ] **P7** Tune player movement - speed, acceleration, air control
- [ ] **P8** Tune jump feel - height, coyote time, responsiveness

---

## Done (recent - see todone.md for full archive)
- [x] Made ball styles data-driven via assets/ball_options.txt
- [x] Debug level spawns all ball styles dynamically
- [x] AI enhancement Phase 1: Renamed `AiInput` → `InputState` (unified input buffer)
- [x] AI enhancement Phase 2: Auto-reload config files every 10s (replaced F2 hotkey)
- [x] AI enhancement Phase 3: Created `assets/ai_profiles.txt` with 10 AI personas
- [x] AI enhancement Phase 4: AI profile cycling (D-pad) + random profile on reset (R key)
