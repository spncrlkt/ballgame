# Tomorrow's Plan: 20 Improvements

*Focus: Test-driven AI bug finding and game feel improvements*

---

## Category 1: Steal System Verification (5 items)

### 1. Visual Indicator for Out-of-Range
**Problem:** The `out_of_range_timer` was added but no visual feedback shows to players.
**Test Plan:**
- Add visual indicator (different color flash than fail flash)
- Create test scenario: press steal at 70px, verify visual appears
- Manual play test to confirm feedback is noticeable

### 2. Steal Range Visual During Gameplay
**Problem:** Players can't see the actual steal range (60px).
**Test Plan:**
- Add optional debug overlay showing steal range circle around player
- Toggle with debug key (Tab)
- Verify circle matches STEAL_RANGE constant

### 3. Steal Success Rate Balance Test
**Problem:** Base 25% success rate may be too low.
**Test Plan:**
- Create simulation that runs 1000 steal attempts at various ranges
- Track success/fail ratio per distance
- Output statistics to verify RNG is fair

### 4. AI Steal Timing Analysis
**Problem:** AI may steal at better moments than humans.
**Test Plan:**
- Parse SQLite events from training sessions
- Extract steal attempt timing relative to player positions
- Compare AI steal distance vs human steal distance at time of attempt

### 5. Steal Cooldown Verification
**Problem:** Different cooldowns (0.2/0.3/0.5/1.0) may cause confusion.
**Test Plan:**
- Create test scenarios for each cooldown type
- Verify correct cooldown applied in each situation
- Add cooldown remaining to debug display

---

## Category 2: AI Decision Making (5 items)

### 6. AI Shot Selection Quality
**Problem:** AI takes shots from bad positions.
**Test Plan:**
- Log shot quality score at time of AI shot decision
- Identify shots with quality < 0.3 (should be rare)
- Analyze what goal state led to bad shot decision

### 7. AI Defense Positioning
**Problem:** AI doesn't cover basket well when defending.
**Test Plan:**
- Create "defense audit" test: spawn AI defender, human attacker with ball
- Measure AI position relative to shot line
- Verify AI moves to intercept position (not toward attacker)

### 8. AI Reaction to Player Charging
**Problem:** AI should attempt steal more when defender is charging (40% vs 25%).
**Test Plan:**
- Create test: holder charges shot, AI in steal range
- Verify AI attempts steal during charge window
- Check if AI notices charging state change

### 9. AI Jump Usage on Platforms
**Problem:** AI may not use jumps efficiently to reach elevated positions.
**Test Plan:**
- Run training on levels 5-8 (have platforms)
- Log AI jump events vs successful platform landings
- Identify cases where AI should have jumped but didn't

### 10. AI Profile Parameter Sensitivity
**Problem:** Some profile parameters may have no effect.
**Test Plan:**
- Create two profiles with extreme parameter differences
- Run tournament between them
- Verify measurable behavior differences (shot timing, steal frequency)

---

## Category 3: Movement and Physics (5 items)

### 11. Coyote Time Feel Test
**Problem:** Current coyote time (0.1s) may feel too short.
**Test Plan:**
- Create test scenarios with platform edge jumps
- Vary coyote time values (0.05, 0.1, 0.15, 0.2)
- Document which feels most responsive

### 12. Jump Buffer Effectiveness
**Problem:** Jump buffer may not catch all early jump presses.
**Test Plan:**
- Create test: press jump 2 frames before landing
- Verify jump triggers immediately on landing
- Test edge cases (landing while holding jump)

### 13. Air Control Balance
**Problem:** Air control may be too weak for precise landings.
**Test Plan:**
- Create platforming test level with narrow ledges
- Measure success rate of precise landings
- Compare air accel vs ground accel values

### 14. Ball Bounce Consistency
**Problem:** Ball bounces may feel unpredictable.
**Test Plan:**
- Create ball drop tests at fixed heights
- Measure bounce heights over multiple runs
- Verify deterministic behavior (same drop = same bounce)

### 15. Shot Power/Angle Relationship
**Problem:** Shot trajectory may not match charge intuition.
**Test Plan:**
- Create shot test: same position, varying charge times
- Graph resulting trajectories
- Verify smooth power progression (no jumps or dead zones)

---

## Category 4: Visual/UX Improvements (3 items)

### 16. Charge Gauge Visibility
**Problem:** Charge gauge inside player may be hard to see.
**Test Plan:**
- Compare current gauge to alternative positions (above player, screen corner)
- Get user feedback on visibility
- Consider adding numeric charge % to debug display

### 17. Score Flash Duration
**Problem:** Score flash may be too brief to notice.
**Test Plan:**
- Measure current flash duration
- Compare to goal celebration in other games
- Consider extending or adding sound effect

### 18. Countdown Visibility
**Problem:** Countdown may be hard to see on certain palettes.
**Test Plan:**
- Test countdown on all 35 palettes
- Identify low-contrast combinations
- Add outline/shadow if needed

---

## Category 5: Infrastructure (2 items)

### 19. Test Coverage Report
**Problem:** Unknown which mechanics lack test coverage.
**Test Plan:**
- List all game mechanics
- Cross-reference with existing scenario tests
- Identify gaps (e.g., ball rolling friction, wall bounces)

### 20. Event Log Analysis Tools
**Problem:** Manual event-log reading is slow.
**Test Plan:**
- Create analysis script that summarizes SQLite events
- Output: steal stats, shot stats, goal times, possession changes
- Identify "interesting" moments (steal streaks, comeback scores)

---

## Testing Strategy

### For Each Improvement:
1. **Write test first** - Create scenario test that exposes issue
2. **Verify test fails** - Confirm test catches the bug
3. **Implement fix** - Make minimal change
4. **Verify test passes** - Confirm fix works
5. **Verify rollback fails** - Revert fix, confirm test catches it
6. **Re-apply fix** - Permanent change

### AI Bug Identification Process:
1. Run 10 training games with logging
2. Parse SQLite events for anomalies:
   - Steal attempts outside range
   - Shots with quality < 0.3
   - Long possession times without shot
   - AI goal state stuck in same mode
3. Create test scenario reproducing anomaly
4. Fix root cause
5. Verify fix with same test

---

## Priority Order

**Tomorrow (High Impact):**
1. Visual indicator for out-of-range (#1)
2. AI shot selection quality (#6)
3. Test coverage report (#19)
4. Event log analysis tools (#20)

**This Week:**
5-10. AI decision making improvements

**Later:**
11-18. Movement/visual polish

---

*Created: 2026-01-25*
