# Ballgame Milestones

## MVP
*Playable solo vs AI - core loop works: move, shoot, score against AI opponents*

**Goals:**
- [ ] Core loop feels complete and fun
- [ ] Single player can play against AI and have a good time

**Stealing Mechanics:**
- [ ] Add randomness to steal contests (not just button mashing)
- [ ] Steal pushback - knockback on successful steal to prevent immediate steal-back
- [ ] Make stealing easier if ball holder is charging a shot
- [ ] Consider other vulnerability states (jumping, recovering from collision)
- [ ] Steal cooldown or fatigue to prevent spam

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

---

## V1 / Beyond
*Future features - multiplayer, new systems, expansion*

**Goals:**
- [ ] Multiplayer support
- [ ] Deeper gameplay systems

**Multiplayer:**
- [ ] Add 1v1 multiplayer
- [ ] Add 4-player multiplayer support
- [ ] Evolution theme for multiplayer/networked games
- [ ] Forks expected - design for branching game modes
- [ ] Consider how ball styles could vary per "species" or game variant

**Equipment:**
- [ ] Equipment system (clubs, rackets, mallets)

**Ball Evolution:**
- [ ] Balls could evolve/mutate based on gameplay
- [ ] Different ball styles could have different physics properties
- [ ] Unlockable ball skins through achievements
- [ ] Ball "lineage" tracking across games

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
