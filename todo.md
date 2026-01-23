# Ballgame TODO

## Immediate Fixes
- [ ] UI fix flash on score color
- [ ] Viewport: Test FixedVertical scaling at all resolutions (especially 4K/Ultrawide)
- [ ] Viewport: Verify arena is fully visible and not cropped at edges
- [ ] Viewport: Check text/UI positioning at different resolutions

## Level Design
- [ ] Create system to make levels easier via collage and like/hate system
- [ ] Back wall gutter like pinball - shoot ball on the floor, hit triangle step on way out

## Simulation Engine & Automated Testing
A robust simulation engine enables both design iteration and automated testing.

**Core Framework:**
- [ ] Headless simulation mode (no rendering, fast-forward time)
- [ ] Data collection: score distributions, possession time, shot attempts
- [ ] Deterministic replay from recorded inputs

**Design Analysis:**
- [ ] Level design metrics - which levels play well, scoring patterns
- [ ] AI persona comparisons - pit different profiles against each other
- [ ] Team composition analysis - which AI pairings work well together
- [ ] Speed mechanics tuning - acceleration curves, air control feel
- [ ] Shooting mechanics analysis - variance, arc, charge timing sweet spots

**Automated Testing Benefits:**
- [ ] Regression tests: run N games, verify win rates stay stable
- [ ] Balance validation: detect OP profiles or broken mechanics
- [ ] Performance benchmarks: track physics tick time across changes
- [ ] Fuzz testing: random AI matchups to find edge case bugs

## Multiplayer
- [ ] Add netcode decision doc
- [ ] Add 1v1 multiplayer
- [ ] Add 4-player multiplayer support

## Long-term: Network Game Design
- [ ] Evolution theme for multiplayer/networked games
- [ ] Forks expected - design for branching game modes
- [ ] Consider how ball styles could vary per "species" or game variant

## Ball Evolution Thoughts
- [ ] Balls could evolve/mutate based on gameplay
- [ ] Different ball styles could have different physics properties
- [ ] Unlockable ball skins through achievements
- [ ] Ball "lineage" tracking across games

## AI
- [ ] Add NPC AI scripting via Lua decision doc

## Stealing Mechanics
- [ ] Add randomness to steal contests (not just button mashing)
- [ ] Steal pushback - knockback on successful steal to prevent immediate steal-back
- [ ] Make stealing easier if ball holder is charging a shot
- [ ] Consider other vulnerability states (jumping, recovering from collision)
- [ ] Steal cooldown or fatigue to prevent spam

## Equipment
- [ ] Equipment system (clubs, rackets, mallets)

---

## Done
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
- [x] AI enhancement Phase 1: Renamed `AiInput` → `InputState` (unified input buffer)
- [x] AI enhancement Phase 2: Auto-reload config files every 10s (replaced F2 hotkey)
- [x] AI enhancement Phase 3: Created `assets/ai_profiles.txt` with 10 AI personas
- [x] AI enhancement Phase 4: AI profile cycling (D-pad) + random profile on reset (R key)
