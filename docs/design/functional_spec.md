# Functional Specification

This document describes all intended behaviors in the game, organized for interaction testing. Each feature includes happy path behaviors and edge cases.

---

## Table of Contents

1. [Game Modes](#1-game-modes)
2. [Core Gameplay Loop](#2-core-gameplay-loop)
3. [Player Movement](#3-player-movement)
4. [Ball Mechanics](#4-ball-mechanics)
5. [Pickup & Steal](#5-pickup--steal)
6. [Shooting](#6-shooting)
7. [Scoring](#7-scoring)
8. [AI System](#8-ai-system)
9. [Level System](#9-level-system)
10. [UI/HUD](#10-uihud)
11. [Configuration & Persistence](#11-configuration--persistence)
12. [Debug/Dev Features](#12-debugdev-features)

---

## 1. Game Modes

### 1.1 Normal Play (Default)

**Entry:** `cargo run`

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| M1.1 | Launch game | Window opens, level 1 loads, 4 players spawn, countdown starts |
| M1.2 | Game runs | 2v2 gameplay with human control and AI opponents |
| M1.3 | Settings persist | Level, palette, ball style, viewport preserved between sessions |

### 1.2 Training Mode

**Entry:** `cargo run --bin training`

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| M2.1 | Launch training | 1v1 match starts, human vs AI |
| M2.2 | `--games N` flag | Runs N consecutive games |
| M2.3 | `--profile X` flag | AI uses specified profile |
| M2.4 | Game ends | Winner determined, moves to next game |
| M2.5 | Session ends | Writes summary + analysis outputs to `training_logs/session_*/` |
| M2.6 | Escape key | Quits training session early, still writes summary |

### 1.3 Replay Mode

**Entry:** `cargo run -- --replay-db <match_id>`

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| M3.1 | Launch replay | Game loads, displays timeline, starts paused or at 1x |
| M3.2 | Space key | Toggles pause/play |
| M3.3 | Left/Right arrows | Cycle playback speed (0.25x/0.5x/1x/2x/4x) |
| M3.4 | Period (.) when paused | Step forward one tick |
| M3.5 | Comma (,) when paused | Step backward one tick |
| M3.6 | Home key | Jump to start |
| M3.7 | End key | Jump to end |
| M3.8 | Playback finishes | Holds at final frame |

### 1.4 Debug Level Mode

**Trigger:** Level with `debug: true` flag in levels.txt

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| M4.1 | Enter debug level | All ball styles spawn on shelves, labeled |
| M4.2 | AI behavior | AI stands idle (Idle goal) |
| M4.3 | Ball wave animation | One row highlights per second |
| M4.4 | Playable ball | Random style ball on floor |

---

## 2. Core Gameplay Loop

### 2.1 Match Structure

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| G1.1 | Match start | Ball spawns center floor, all players spawn at positions |
| G1.2 | Countdown | 3-2-1-GO! displays, input disabled during countdown |
| G1.3 | Ball acquisition | Players race to pickup ball first |
| G1.4 | Offense | Ball holder navigates toward opponent basket |
| G1.5 | Defense | Non-holders attempt steals or position to block |
| G1.6 | Score event | Points awarded, ball resets, AI goals reset |

### 2.2 Teams

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| G2.1 | Left team target | Left team players target RIGHT basket |
| G2.2 | Right team target | Right team players target LEFT basket |
| G2.3 | Score attribution | Scoring in left basket awards points to RIGHT team |
| G2.4 | Score attribution | Scoring in right basket awards points to LEFT team |

---

## 3. Player Movement

### 3.1 Horizontal Movement

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| P1.1 | Press A/D or Left Stick | Player accelerates in that direction |
| P1.2 | Release input | Player decelerates (slight slide) |
| P1.3 | Max speed | Player cannot exceed 300 px/s |
| P1.4 | Ground acceleration | 2400 px/s² when starting from stop |
| P1.5 | Ground deceleration | 1800 px/s² when stopping |
| P1.6 | Air acceleration | 1500 px/s² (reduced control in air) |
| P1.7 | Air deceleration | 900 px/s² (momentum preserved in air) |
| P1.8 | Facing direction | Player faces direction of last input |

**Edge Cases:**

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| P1.E1 | Analog stick in deadzone (<0.25) | No movement registered |
| P1.E2 | Wall collision | Player stops at wall, no clipping |
| P1.E3 | Movement while holding ball | Same speed, ball follows |

### 3.2 Jumping

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| P2.1 | Press Space/W or South button | Player jumps (650 px/s initial velocity) |
| P2.2 | Hold jump button | Full jump height |
| P2.3 | Release jump early | Jump cut (velocity * 0.4), shorter jump |
| P2.4 | Rising gravity | 980 px/s² while going up |
| P2.5 | Falling gravity | 1400 px/s² while falling (fast fall) |
| P2.6 | Coyote time | Can jump 0.1s after leaving platform |
| P2.7 | Jump buffer | Jump input remembered 0.1s before landing |
| P2.8 | In-air control | Can adjust horizontal movement while airborne |

**Edge Cases:**

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| P2.E1 | Jump at platform edge | Coyote time allows jump after leaving |
| P2.E2 | Jump input before landing | Buffered, executes on land |
| P2.E3 | Double-tap jump in air | Second input ignored (not double jump) |
| P2.E4 | Jump while falling | Not allowed unless coyote time active |

### 3.3 Platform Collision

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| P3.1 | Land on platform | Player rests on surface, grounded = true |
| P3.2 | Hit platform from below | Player bounces down, doesn't pass through |
| P3.3 | Hit platform from side | Player stops, slides along if moving diagonally |
| P3.4 | Fall off platform | Grounded = false, coyote timer starts |

---

## 4. Ball Mechanics

### 4.1 Ball States

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| B1.1 | Free state | Ball moves independently, subject to physics |
| B1.2 | Held state | Ball follows player position (facing side, center height) |
| B1.3 | InFlight state | Ball was thrown, has shooter reference |
| B1.4 | State: Free → Held | On pickup by player |
| B1.5 | State: Held → InFlight | On throw |
| B1.6 | State: InFlight → Free | After landing, bouncing stops, or speed drops |

### 4.2 Ball Physics (Free/InFlight)

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| B2.1 | Gravity | Ball falls at 800 px/s² |
| B2.2 | Air friction | 95% velocity retained per second |
| B2.3 | Platform bounce | 70% velocity retained, reflects off surface |
| B2.4 | Ground friction | 60% velocity retained per bounce |
| B2.5 | Rolling mode | Engages when speed < 200 px/s on ground |
| B2.6 | Roll friction | 60% velocity retained per second while rolling |
| B2.7 | Spin | Ball rotates based on velocity (0.01 rad/px) |
| B2.8 | Spin decay | 50% spin retained per second (airborne) |

### 4.3 Special Surfaces

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| B3.1 | Corner steps (erratic) | 92% velocity retained, up to 35° deflection |
| B3.2 | Basket rims (snappy) | 85% velocity retained, up to 20° deflection |
| B3.3 | Normal platforms | 70% velocity retained, up to 20° deflection |

### 4.4 Ball-Player Collision

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| B4.1 | Free ball hits player | Ball velocity reduced (0.7x horizontal, 0.4x vertical) |
| B4.2 | Slow ball + moving player | Ball kicked in player's direction (100 px/s) |
| B4.3 | Ball in shot grace period | No drag applied for 100ms after throw |
| B4.4 | Defender blocks shot | Grace period reduced by 70% |

**Edge Cases:**

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| B4.E1 | Ball hits shooter during grace | No drag (shooter excluded from grace reduction) |
| B4.E2 | Multiple players hit ball | Each applies drag independently |
| B4.E3 | Held ball touches other player | No collision effect (ball is held) |

---

## 5. Pickup & Steal

### 5.1 Ball Pickup

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| S1.1 | Press E near free ball (<50px) | Ball becomes Held, attaches to player |
| S1.2 | Press E too far from ball | Nothing happens |
| S1.3 | Ball already held | Cannot pickup (try steal instead) |
| S1.4 | Multiple players same distance | First to press wins |

### 5.2 Steal Mechanics

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| S2.1 | Press E near opponent holding ball (<60px) | Steal attempt initiated |
| S2.2 | Base success chance | 33% chance to succeed |
| S2.3 | Opponent charging shot | +17% bonus (total 50% chance) |
| S2.4 | Successful steal | Ball transfers to attacker |
| S2.5 | Successful steal pushback | Victim pushed 400 px/s away + slight upward nudge |
| S2.6 | Failed steal | Visual fail flash (0.15s) |
| S2.7 | Attacker cooldown | 0.3s cooldown after any attempt |
| S2.8 | Victim cooldown | 1.0s cooldown after losing ball (can't steal back) |

**Edge Cases:**

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| S2.E1 | Press E while on cooldown | Nothing happens |
| S2.E2 | Free ball and opponent both in range | Pickup takes priority |
| S2.E3 | Multiple attackers same frame | Each rolls independently |

---

## 6. Shooting

### 6.1 Charge Mechanics

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| T1.1 | Hold F/RB while holding ball | Charge accumulates |
| T1.2 | Full charge time | 1.6 seconds to max |
| T1.3 | Release F/RB | Ball thrown |
| T1.4 | Quick shot (<0.4s charge) | 50% power multiplier |
| T1.5 | Charge gauge display | Green-to-red fill inside player |
| T1.6 | Not holding ball | Charge does nothing |

### 6.2 Throw Trajectory

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| T2.1 | Target basket | Ball aims at team's target basket |
| T2.2 | Elevation angle | Calculated for physics arc (30° to 85°) |
| T2.3 | Speed calculation | Based on distance to target |
| T2.4 | Speed boost | 10% overshoot compensation |
| T2.5 | Speed randomness | ±10% variation |
| T2.6 | Hard cap | Maximum 2000 px/s regardless of calculation |

### 6.3 Accuracy/Variance

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| T3.1 | Base variance (no charge) | 50% variance |
| T3.2 | Base variance (full charge) | 2% variance |
| T3.3 | Air penalty | +10% if airborne when throwing |
| T3.4 | Move penalty | +10% if moving at full speed (scales with speed) |
| T3.5 | Distance penalty | +0.025% per pixel distance |
| T3.6 | Variance application | Angle varies by ±(variance × 30°) |
| T3.7 | Upward bias | +5% upward bias on variance |

**Edge Cases:**

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| T3.E1 | Throw while jumping | Air penalty applies |
| T3.E2 | Throw while moving backward | Move penalty still applies |
| T3.E3 | Throw at point-blank range | Minimal distance variance |
| T3.E4 | Throw across full court | Maximum distance variance |

---

## 7. Scoring

### 7.1 Score Detection

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| SC1.1 | Ball center enters basket bounds | Score triggered |
| SC1.2 | Ball touches basket but center outside | No score |
| SC1.3 | Ball passes through basket while held | Score (2 points) |
| SC1.4 | Ball passes through basket in flight | Score (1 point) |

### 7.2 Point Values

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| SC2.1 | Carry-in (ball held) | 2 points |
| SC2.2 | Throw-in (ball in flight) | 1 point |
| SC2.3 | Team attribution | Scoring in LEFT basket → RIGHT team gets points |
| SC2.4 | Team attribution | Scoring in RIGHT basket → LEFT team gets points |

### 7.3 Post-Score Reset

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| SC3.1 | Ball reset | Ball teleports to center floor, velocity zeroed |
| SC3.2 | Ball state | Ball becomes Free |
| SC3.3 | Holder released | Scoring player loses HoldingBall component |
| SC3.4 | AI reset | All AI players reset to ChaseBall goal |
| SC3.5 | AI navigation clear | All AI paths and targets cleared |
| SC3.6 | Basket flash | Basket flashes (gold for carry, white for throw) |
| SC3.7 | Player flash | Scorer flashes same color as basket |
| SC3.8 | Flash duration | 0.6 seconds |

---

## 8. AI System

### 8.1 AI Goals (State Machine)

| ID | Goal | Behavior |
|----|------|----------|
| AI1.1 | Idle | Stand still, do nothing (debug levels) |
| AI1.2 | ChaseBall | Move toward free ball, pick it up |
| AI1.3 | AttackWithBall | Navigate toward basket, find shooting position |
| AI1.4 | ChargeShot | At basket, charge and throw |
| AI1.5 | AttemptSteal | Move toward ball holder, attempt steal |
| AI1.6 | InterceptDefense | Position on shot line between opponent and basket |
| AI1.7 | PressureDefense | Close-range defense, stay near opponent |

### 8.2 Goal Transitions

| ID | From | To | Condition |
|----|------|----|-----------|
| AI2.1 | ChaseBall | AttackWithBall | Picked up ball |
| AI2.2 | AttackWithBall | ChargeShot | In shooting range, good position |
| AI2.3 | ChargeShot | ChaseBall | Ball released, no longer holding |
| AI2.4 | ChaseBall | AttemptSteal | Opponent has ball, close range |
| AI2.5 | AttemptSteal | ChaseBall | Ball became free |
| AI2.6 | Any | InterceptDefense | Opponent has ball, far away |
| AI2.7 | InterceptDefense | PressureDefense | Close to opponent |
| AI2.8 | Any | ChaseBall | After score (all AI reset) |

### 8.3 Navigation

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| AI3.1 | Pathfinding | A* algorithm finds route across platforms |
| AI3.2 | Jump detection | AI calculates jump trajectories to reach platforms |
| AI3.3 | Drop detection | AI can drop off platforms intentionally |
| AI3.4 | Stuck detection | If not moving for 2s while trying, recalculate path |
| AI3.5 | Path recalc | Recalculates if target moved >100px |

### 8.4 AI Profiles

Profiles define AI personality via parameters loaded from `config/ai_profiles.txt`.

| Parameter | Effect |
|-----------|--------|
| position_tolerance | Distance before "at target" (pixels) |
| shoot_range | Distance from basket to attempt shots |
| charge_min/max | Min/max charge time |
| steal_range | Distance to initiate steal |
| defense_offset | How far from basket to defend |
| min_shot_quality | Minimum position quality to shoot |
| pressure_distance | How close to stay to ball carrier |
| aggression | How relentlessly pursue (0-1) |
| defensive_iq | Shot line positioning accuracy (0-1) |

---

## 9. Level System

### 9.1 Level Loading

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| L1.1 | Game start | Level 1 loads from database |
| L1.2 | Level file | Read from `config/levels.txt` |
| L1.3 | ] key | Load next level |
| L1.4 | [ key | Load previous level |
| L1.5 | R key | Reset current level, randomize AI profiles |
| L1.6 | Level wrap | After last level, wraps to first |

### 9.2 Level Geometry

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| L2.1 | Basket height | Configurable per level |
| L2.2 | Basket push-in | Distance from wall (configurable) |
| L2.3 | Mirror platforms | Symmetric platforms on both sides |
| L2.4 | Center platforms | Platforms at center (x=0) |
| L2.5 | Corner steps | Staircase in bottom corners |

### 9.3 Level Transition

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| L3.1 | Platform cleanup | All LevelPlatform entities despawned |
| L3.2 | New geometry | New platforms spawned from level data |
| L3.3 | Basket reposition | Baskets move to new height/position |
| L3.4 | Ball reset | Ball teleports to center |
| L3.5 | Countdown trigger | 3-2-1 countdown starts |
| L3.6 | AI reset | All AI reset to ChaseBall |
| L3.7 | Nav graph rebuild | Navigation graph regenerated |

---

## 10. UI/HUD

### 10.1 Score Display

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| U1.1 | Position | Top center of screen |
| U1.2 | Format | "Left - Right" |
| U1.3 | Update | Real-time as scores change |
| U1.4 | Colors | Team colors from current palette |

### 10.2 Cycle Indicator

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| U2.1 | Position | Top-left corner, 4 lines |
| U2.2 | Active marker | `>` before active D-pad direction |
| U2.3 | Human marker | `*` next to human-controlled player |
| U2.4 | Content | Viewport, Presets, AI, Level/Palette/Style |

### 10.3 Charge Gauge

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| U3.1 | Position | Inside player, opposite side from ball |
| U3.2 | Background | Always visible black bar |
| U3.3 | Fill | Green-to-red gradient, scales with charge |
| U3.4 | Height | Same as player height |
| U3.5 | Visibility | Only visible when charging |

### 10.4 Countdown Display

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| U4.1 | Position | Center screen |
| U4.2 | Numbers 3-2-1 | Gold/amber color, pulsing |
| U4.3 | GO! | Green color |
| U4.4 | Duration | ~3.3 seconds total |
| U4.5 | Input blocking | Player/AI input disabled during countdown |

### 10.5 Steal Indicators

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| U5.1 | Cooldown display | Shows remaining seconds above player |
| U5.2 | Fail flash | Brief red flash on failed steal |
| U5.3 | Duration | Fail flash lasts 0.15s |

### 10.6 Score Flash

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| U6.1 | Basket flash | Basket pulses on score |
| U6.2 | Player flash | Scorer pulses on carry-in |
| U6.3 | Carry-in color | Gold |
| U6.4 | Throw-in color | White |
| U6.5 | Duration | 0.6 seconds |

---

## 11. Configuration & Persistence

### 11.1 Settings File

**File:** `config/init_settings.json`

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| C1.1 | Save trigger | On window close |
| C1.2 | Level persists | Current level saved/restored |
| C1.3 | Palette persists | Current palette index saved/restored |
| C1.4 | Ball style persists | Selected style saved/restored |
| C1.5 | Viewport persists | Viewport preset saved/restored |
| C1.6 | AI profiles persist | Selected profiles saved/restored |
| C1.7 | Double-click Start | Randomizes AI profiles only |

### 11.2 Hot Reload

**Check interval:** Every 10 seconds

| ID | File | Effect |
|----|------|--------|
| C2.1 | ai_profiles.txt | AI behavior updates immediately |
| C2.2 | game_presets.txt | Presets reload |
| C2.3 | palettes.txt | Colors reload |
| C2.4 | levels.txt | Level data reloads |
| C2.5 | ball_options.txt | Ball styles reload |

### 11.3 Presets

| ID | Type | Contains |
|----|------|----------|
| C3.1 | Movement | Jump, gravity, speed, acceleration values |
| C3.2 | Ball | Ball physics values |
| C3.3 | Shooting | Charge time, variance values |
| C3.4 | Composite | Bundles Movement + Ball + Shooting |

---

## 12. Debug/Dev Features

### 12.1 Debug Text (Tab key)

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| D1.1 | Toggle | Tab key shows/hides |
| D1.2 | Position | Center-bottom |
| D1.3 | Content | Last shot: angle, speed, variance breakdown |

### 12.2 Tweak Panel (F1 key)

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| D2.1 | Toggle | F1 shows/hides panel |
| D2.2 | Position | Right side of screen |
| D2.3 | Navigation | Up/Down selects parameter |
| D2.4 | Adjustment | Left/Right changes by ±10% |
| D2.5 | Reset single | R key resets selected parameter |
| D2.6 | Reset all | Shift+R resets all parameters |
| D2.7 | Persistence | Changes do NOT persist to file |

### 12.3 Snapshot System

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| D3.1 | F2 key | Toggle snapshot system on/off |
| D3.2 | F3 key | Toggle screenshot capture (JSON only when off) |
| D3.3 | F4 key | Manual snapshot immediately |
| D3.4 | Auto triggers | Score, steal, level change |
| D3.5 | Output location | `snapshots/YYYYMMDD_HHMMSS_trigger.json` and `.png` |
| D3.6 | JSON content | Timestamp, score, level, ball state, player states, shot info |

### 12.4 Viewport Cycling

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| D4.1 | V key or D-pad Up | Cycle viewport size |
| D4.2 | Presets | 1600x900, 1080p, 1440p, Ultrawide, 4K |
| D4.3 | Camera | Always shows full arena height |
| D4.4 | Width | Adapts per viewport setting |

### 12.5 Visual Regression

| ID | Behavior | Expected Outcome |
|----|----------|------------------|
| D5.1 | `./scripts/regression.sh` | Capture and compare to baseline |
| D5.2 | `--update` flag | Accept current as new baseline |
| D5.3 | Output | PASS, REVIEW, or FAIL |
| D5.4 | Files | baseline.png, current.png, diff.png |

---

## Appendix A: Input Reference

### Keyboard

| Key | Action |
|-----|--------|
| A/D | Move left/right |
| Space/W | Jump |
| E | Pickup/Steal |
| F | Throw (hold to charge) |
| Q | Cycle player control |
| R | Reset level |
| ] | Next level |
| [ | Previous level |
| V | Cycle viewport |
| Tab | Toggle debug text |
| F1 | Toggle tweak panel |
| F2 | Toggle snapshot system |
| F3 | Toggle screenshot capture |
| F4 | Manual snapshot |

### Controller

| Button | Action |
|--------|--------|
| Left Stick | Move |
| South (A) | Jump |
| West (X) | Pickup/Steal |
| Right Bumper | Throw |
| Left Bumper | Cycle control |
| Start | Reset level |
| D-pad | Cycle options (see D-pad system) |
| LT/RT | Adjust selected option |

### D-pad System

| Direction | Options |
|-----------|---------|
| Up | Viewport |
| Down | Composite → Movement → Ball → Shooting presets |
| Left | AI (LT: player, RT: profile) |
| Right | Level → Palette → Ball Style |

---

## Appendix B: Constants Reference

| Constant | Value | Description |
|----------|-------|-------------|
| PLAYER_SIZE | 32×64 px | Player hitbox |
| BALL_SIZE | 26×26 px | Ball hitbox |
| MOVE_SPEED | 300 px/s | Max horizontal speed |
| JUMP_VELOCITY | 650 px/s | Initial jump speed |
| GRAVITY_RISE | 980 px/s² | Gravity while rising |
| GRAVITY_FALL | 1400 px/s² | Gravity while falling |
| BALL_GRAVITY | 800 px/s² | Ball gravity |
| BALL_BOUNCE | 0.7 | Bounce coefficient |
| BALL_PICKUP_RADIUS | 50 px | Pickup range |
| STEAL_RANGE | 60 px | Steal attempt range |
| STEAL_SUCCESS_CHANCE | 0.33 | Base steal chance |
| STEAL_CHARGING_BONUS | 0.17 | Bonus if victim charging |
| SHOT_CHARGE_TIME | 1.6s | Time to full charge |
| SHOT_QUICK_THRESHOLD | 0.4s | Quick shot cutoff |
| SHOT_MAX_VARIANCE | 0.50 | Variance at no charge |
| SHOT_MIN_VARIANCE | 0.02 | Variance at full charge |
| COYOTE_TIME | 0.1s | Post-edge jump window |
| JUMP_BUFFER_TIME | 0.1s | Pre-land jump buffer |

---

## Appendix C: Test Scenario Categories

For automated testing, scenarios can be organized into these categories:

### Unit-Level Tests
- Individual physics calculations
- Variance computation
- Trajectory math
- Collision detection

### Integration Tests
- Pickup mechanics (approach → press → hold)
- Steal flow (approach → press → roll → outcome)
- Shooting flow (hold → charge → release → flight → score)
- AI goal transitions

### System Tests
- Full match playthrough
- Level transitions
- Settings persistence
- Hot reload behavior

### Regression Tests
- Visual baseline comparison
- Known gameplay sequences via replay

---

## Appendix D: Known Gaps / Issues

| ID | Description | Status |
|----|-------------|--------|
| GAP1 | No automated test suite | Manual testing only |
| GAP2 | Tweak values don't persist | Session-only changes (intentional) |
