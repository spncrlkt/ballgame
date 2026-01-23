# Ballgame TODO

## Immediate Fixes
- [ ] Make rim bouncier like steps

## Level Design
- [ ] Create system to make levels easier via collage and like/hate system
- [ ] Back wall gutter like pinball - shoot ball on the floor, hit triangle step on way out

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

## Equipment
- [ ] Equipment system (clubs, rackets, mallets)

---

## Done
- [x] Split main.rs into modules (2624 lines → 18 focused files, no module >500 lines)
- [x] Fix viewport and arena wall size (1600×900 window, 1:1 camera, 20px walls, world-space UI)
- [x] Remove possession ball texture swapping, add 10 color palettes that cycle on reset (affects ball, players, baskets)
