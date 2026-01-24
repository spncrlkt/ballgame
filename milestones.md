# Ballgame Milestones

## MVP
*Playable solo vs AI - core loop works: move, shoot, score against AI opponents*

**Goals:**
- [ ] Core loop feels complete and fun
- [ ] Single player can play against AI and have a good time

**Stealing Mechanics:**
- [x] Simplified steal system - instant attempts, no button mashing (33% base chance)
- [x] Steal pushback - knockback on successful steal
- [x] 1-second no-stealback cooldown for victims
- [x] Make stealing easier if ball holder is charging a shot (+17% = 50% total)
- [x] Steal cooldown (0.3s) to prevent spam
- [ ] Consider other vulnerability states (jumping, recovering from collision)

**AI Behavior:**
- [ ] AI plays competently and is fun to play against
- [ ] Fix AI positioning - stands in wrong places, doesn't cover basket well
- [ ] Fix AI shooting - takes bad shots, misses easy ones

**Movement/Physics:**
- [ ] Tune player movement - speed, acceleration, air control
- [ ] Tune jump feel - height, coyote time, responsiveness

---

## V0
*Polished core + levels - multiple levels, tuned AI, good game feel*

**Goals:**
- [ ] Multiple polished levels
- [ ] Viewport works at all resolutions
- [ ] Visual polish complete
- [ ] Ready to share with others for feedback

**Polish & Fixes:**
- [ ] UI fix flash on score color
- [ ] Viewport: Test FixedVertical scaling at all resolutions (especially 4K/Ultrawide)
- [ ] Viewport: Verify arena is fully visible and not cropped at edges
- [ ] Viewport: Check text/UI positioning at different resolutions

**Level Design:**
- [ ] Create system to make levels easier via collage and like/hate system
- [ ] Back wall gutter like pinball - shoot ball on the floor, hit triangle step on way out

**Gameplay Structure:**
- [ ] Win conditions - score limit or time limit to end games
- [ ] Game state flow - start → play → end → restart cycle
- [ ] Passing mechanics - teammate-to-teammate ball passing
- [ ] Ball physics tuning - bounce, weight, speed feel

**Scoring Feedback:**
- [ ] Make scoring feel impactful beyond flash animation

---

## V1 / Beyond
*Future features - multiplayer, new systems, expansion*

**Goals:**
- [ ] Multiplayer support
- [ ] Deeper gameplay systems

**Audio:**
- [ ] Sound effects - jumps, shots, scores, steals, bounces
- [ ] Music - background tracks
- [ ] Audio settings - volume controls

**Menus & UI:**
- [ ] Start screen / main menu
- [ ] Pause menu
- [ ] Settings UI
- [ ] Tutorial / onboarding for new players

**Game Modes:**
- [ ] Timed matches
- [ ] First-to-X scoring
- [ ] Round-based play

**Controller Feel:**
- [ ] Haptics/rumble for shots, scores, steals

**Multiplayer:**
- [ ] Add 1v1 multiplayer
- [ ] Add 4-player multiplayer support
- [ ] Evolution theme for multiplayer/networked games
- [ ] Forks expected - design for branching game modes
- [ ] Consider how ball styles could vary per "species" or game variant
- [ ] Matchmaking - how players find games

**Equipment:**
- [ ] Equipment system (clubs, rackets, mallets)

**Persistence:**
- [ ] Save data / player profiles
- [ ] Stats tracking
- [ ] Unlocks system

**Ball Evolution:**
- [ ] Balls could evolve/mutate based on gameplay
- [ ] Different ball styles could have different physics properties
- [ ] Unlockable ball skins through achievements
- [ ] Ball "lineage" tracking across games

**Interaction Logging & Analytics:**
- [ ] Event logging system - structured JSON/CSV output for all game events
- [ ] Player actions: movement inputs, jumps, shots (charge time, angle, result)
- [ ] Ball events: pickups, drops, bounces, basket entries, steals (success/fail)
- [ ] Scoring events: timestamps, positions, shot trajectories, AI states
- [ ] AI decisions: goal changes, target positions, decision reasoning
- [ ] Physics snapshots: periodic state dumps (positions, velocities)
- [ ] Session metadata: level, palette, AI profiles, preset configurations
- [ ] Log rotation/compression for long sessions
- [ ] Export formats compatible with pandas, R, SQL databases
- [ ] Real-time streaming option for live dashboards
- [ ] Replay reconstruction from logged events
- [ ] In-game replay browser - list/filter/search replays with metadata (date, profiles, score, level)

**Simulation Engine & Automated Testing:**
- [ ] Headless simulation mode (no rendering, fast-forward time)
- [ ] Data collection: score distributions, possession time, shot attempts
- [ ] Deterministic replay from recorded inputs
- [ ] Level design metrics - which levels play well, scoring patterns
- [ ] AI persona comparisons - pit different profiles against each other
- [ ] Team composition analysis - which AI pairings work well together
- [ ] Speed mechanics tuning - acceleration curves, air control feel
- [ ] Shooting mechanics analysis - variance, arc, charge timing sweet spots
- [ ] Regression tests: run N games, verify win rates stay stable
- [ ] Balance validation: detect OP profiles or broken mechanics
- [ ] Performance benchmarks: track physics tick time across changes
- [ ] Fuzz testing: random AI matchups to find edge case bugs

---

## Decision Docs Needed
*These decisions affect multiple milestones and should be resolved early.*

- [ ] **Netcode architecture** - Required before multiplayer work (V1)
- [ ] **NPC AI scripting via Lua** - Could improve AI for MVP or defer to V1
