# Ballgame Done Archive

## Archived 2026-01-23 (Session 2)
- [x] Steal Phase 1: Pushback on successful steal (STEAL_PUSHBACK_STRENGTH)
- [x] Steal Phase 1: Randomness in contests (STEAL_RANDOM_BONUS)
- [x] Steal Phase 1: Charging penalty (STEAL_CHARGING_PENALTY)
- [x] Steal Phase 1: Cooldown to prevent spam (STEAL_COOLDOWN + StealCooldown component)
- [x] AI enhancement Phase 1: Renamed `AiInput` → `InputState` (unified input buffer)
- [x] AI enhancement Phase 2: Auto-reload config files every 10s (replaced F2 hotkey)
- [x] AI enhancement Phase 3: Created `assets/ai_profiles.txt` with 10 AI personas
- [x] AI enhancement Phase 4: AI profile cycling (D-pad) + random profile on reset (R key)

## Archived 2026-01-23 (Session 1)
- [x] Split main.rs into modules (2624 lines → 18 focused files, no module >500 lines)
- [x] Fix viewport and arena wall size (1600×900 window, 1:1 camera, 20px walls, world-space UI)
- [x] Remove possession ball texture swapping, add 10 color palettes that cycle on reset (affects ball, players, baskets)
- [x] Fix jumping not working (input systems needed .chain() for guaranteed order)
- [x] Fix copy_human_input zeroing jump buffer on first press frame
- [x] Fix ball duplication when switching from debug to non-debug levels
- [x] Fix goal flash resetting to hardcoded color instead of current palette
- [x] Expand palette system to 20 palettes with background/floor/platform colors
- [x] Create assets/palettes.txt file for editable color definitions
- [x] Fix AI not activating when switching from debug to non-debug levels
- [x] Parameterized rim colors (`left_rim`, `right_rim`) in palette file
- [x] Fixed platform/step colors to match walls (query filter bug)
- [x] Fixed viewport cycling to return to spawn size (scale_factor_override)
- [x] Fixed player spawn bug (use Team component instead of X position)
- [x] Added charge gauge to both players
- [x] Renamed palette field `floor` → `platforms`
- [x] Make rim bouncier like steps
- [x] Reorganized 30 color palettes (9 favorites at front, 11 variations, 10 wild)
- [x] Added names to all palettes in assets/palettes.txt
- [x] Made palette count fully dynamic (no hardcoded NUM_PALETTES)
- [x] Created 10 ball styles: wedges, half, spiral, checker, star, swirl, plasma, shatter, wave, atoms
- [x] Made ball styles data-driven via assets/ball_options.txt
- [x] Debug level spawns all ball styles dynamically
- [x] Removed old ball texture files (stripe, dot, ring, solid)
