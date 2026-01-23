# Ballgame TODO - Current Sprint

*See milestones.md for the full project plan (MVP → V0 → V1)*

## Active Work (Prioritized)

### 1. Steal Improvements (Phase 2)
- [ ] **P1** Visual feedback - cooldown indicator, progress bars, vulnerable highlight
- [ ] **P2** AI mashing - randomized burst pattern during steal contests
- [ ] **P3** Balance tuning - diminishing returns on button mashing (cap at 8 effective)
- [ ] **P4** Block throws - defender can't throw ball during active steal contest

### 2. Game Presets System
- [ ] **P5** Data structures - MovementPreset, BallPreset, ShootingPreset, CompositePreset
- [ ] **P6** File loading - parse assets/game_presets.txt with hot-reload support
- [ ] **P7** PhysicsTweaks integration - presets populate the existing tweaks resource
- [ ] **P8** Cycling UI - add Movement/Ball/Shooting/Composite to D-pad cycle targets

### 3. AI Behavior
- [ ] **P9** Fix AI shooting - takes bad shots, misses easy ones
- [ ] **P10** Fix AI positioning - stands in wrong places, doesn't cover basket well

### 4. Movement/Physics
- [ ] **P11** Tune player movement - speed, acceleration, air control
- [ ] **P12** Tune jump feel - height, coyote time, responsiveness

---

## Done (recent - see todone.md for full archive)
- [x] Steal Phase 1: Pushback on successful steal (STEAL_PUSHBACK_STRENGTH)
- [x] Steal Phase 1: Randomness in contests (STEAL_RANDOM_BONUS)
- [x] Steal Phase 1: Charging penalty (STEAL_CHARGING_PENALTY)
- [x] Steal Phase 1: Cooldown to prevent spam (STEAL_COOLDOWN + StealCooldown component)
- [x] Made ball styles data-driven via assets/ball_options.txt
- [x] Debug level spawns all ball styles dynamically
- [x] AI enhancement Phase 1: Renamed `AiInput` → `InputState` (unified input buffer)
- [x] AI enhancement Phase 2: Auto-reload config files every 10s (replaced F2 hotkey)
- [x] AI enhancement Phase 3: Created `assets/ai_profiles.txt` with 10 AI personas
- [x] AI enhancement Phase 4: AI profile cycling (D-pad) + random profile on reset (R key)
