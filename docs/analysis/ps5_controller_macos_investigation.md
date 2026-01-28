# PS5 DualSense on macOS Investigation

## Scope
Document how Sony PS5 DualSense (and DualSense Edge) work with macOS, with emphasis on pre-2024 documentation and noting where older guidance appears outdated.

## Summary (from initial research)
- macOS added official DualSense support starting with macOS Big Sur 11.3 (April 26, 2021), covering Bluetooth and USB connections.
- DualSense Edge support on Apple platforms arrived with macOS Ventura 13.3 (March 2023).
- Pairing flow: hold PS + Create to enter pairing mode, then connect via macOS Bluetooth settings; USB-C also works for pairing and play.
- Core inputs are supported through Apple’s Game Controller framework; game support determines which features are exposed.
- Sony documentation states: adaptive triggers work only when the game supports them; haptic feedback is not supported on Mac; mic/speaker do not work on Mac; headset jack requires wired mode.

## Outdated/Conflicting Guidance to Flag
- Older docs commonly say you must re-pair the controller when switching devices. Recent PS5/DualSense updates add multi-device pairing slots and on-controller switching, which can make that advice stale.
- Apple’s controller compatibility list now includes DualSense Edge and newer devices, so pre-2024 Apple docs may be missing recent controllers.

## Open Questions
- Confirm current macOS Game Controller feature exposure for DualSense (which inputs map through GCController on 0.17.3 era Bevy).
- Verify whether any macOS releases after 13.3 added more DualSense features (rumble/advanced haptics) beyond Sony’s documented limitations.

## Next Steps
- Compare Bevy’s input/controller handling to our input + EventBus abstraction.
- Decide best approach for generating searchable local Bevy docs for 0.17.3.

---

# Bevy Input + EventBus Investigation

## Bevy input overview (0.17.3 docs)
- Supported input devices: keyboard, mouse, gamepad, touch. (From `bevy::input` crate docs.)
- Core input resource pattern is `ButtonInput<T>` with `pressed`, `just_pressed`, `just_released` semantics.
- `ButtonInput<KeyCode>` is tied to window focus; `ButtonInput<GamepadButton>` is not.
- Gamepad system has:
  - A `Gamepad` component per connected device, auto-spawned on connection and updated by Bevy’s gamepad processing system.
  - A raw → processed pipeline: raw events are filtered by `GamepadSettings`, then update `Gamepad` state and emit processed axis/button events.
  - Gamepad axes are mapped to [-1.0, 1.0]; gamepad buttons are mapped to [0.0, 1.0].

### Key Bevy APIs in docs (for reference)
- `ButtonInput<T>` methods: `pressed`, `just_pressed`, `just_released`, plus clear/reset helpers for multi-system consumption.
- `Gamepad` component: access axis/button state, sticks, dpad, and vendor/product ids.
- `GamepadSettings`: per-gamepad thresholds/filters; events not meeting thresholds do not register.
- `gamepad_event_processing_system`: consumes raw gamepad events, filters using settings, updates `Gamepad`, and emits processed events.

## Our input + EventBus abstraction (current code)
- Update schedule:
  - `Update`: `input::capture_input` reads keyboard + gamepad, buffers to `PlayerInput`, and emits `GameEvent::ControllerInput` to the `EventBus` for auditability.
  - `Update`: `ai::copy_human_input` copies buffered `PlayerInput` into per-entity `InputState`.
  - `FixedUpdate`: `player::apply_input` consumes `InputState` for physics.
- EventBus:
  - Stores pending events per frame, timestamps them via `update_event_bus_time`, and supports draining/export for logging.
  - Used as the canonical audit trail for input and game events.

## Comparison: Bevy input vs our abstraction
1) **Source of truth**
   - Bevy: input state lives in resources (`ButtonInput<KeyCode>`, `ButtonInput<GamepadButton>`) and `Gamepad` components updated by Bevy input systems.
   - Ours: canonical gameplay input lives in `PlayerInput` (resource) → `InputState` (per-entity), with all inputs mirrored to `EventBus`.

2) **Event vs state**
   - Bevy: offers both event stream (gamepad events) and state queries (`pressed`/`just_pressed`, axis values). Event filtering via `GamepadSettings`.
   - Ours: state is buffered manually; events are a parallel audit trail (EventBus), not the input source of truth.

3) **One-frame consumption**
   - Bevy: `just_pressed`/`just_released` are one-frame; docs suggest clearing if multiple systems should not all react.
   - Ours: one-frame inputs (pickup, throw_released, swap) are latched in `PlayerInput` until consumed; `InputState` is reset/consumed in gameplay systems.

4) **Focus behavior**
   - Bevy: keyboard input is focus-sensitive; gamepad input is focus-independent.
   - Ours: no explicit focus handling; capture system ignores input only when tweak panel is open.

5) **Filtering / deadzones**
   - Bevy: axis/button thresholds and filtering are configured via `GamepadSettings` and applied by the processing system.
   - Ours: we apply a manual stick deadzone in `capture_input`, and button thresholds are implicit (pressed/just_pressed).

## Notes / Opportunities
- Bevy’s `GamepadSettings` could replace (or unify with) our manual deadzone logic for consistency across devices.
- If we want the EventBus to mirror Bevy’s raw vs processed input distinction, we could optionally log both raw and filtered values.
- Bevy’s “clear just_pressed” guidance mirrors our latch-and-consume pattern, but we’re doing it manually per action.

## Local Bevy docs (generated)
- `cargo doc -p bevy --no-deps` produced `target/doc/bevy/index.html` (searchable).

## Source notes (bevy_input 0.17.3)
- `InputPlugin` registers input systems in `PreUpdate`, including `gamepad_connection_system` and `gamepad_event_processing_system`, with explicit ordering (processing after connection).
- `gamepad_event_processing_system` clears digital button state each frame before processing raw events.
- `GamepadConnection` keeps entities alive on disconnect (removes `Gamepad` component but preserves `GamepadSettings`).
- Default thresholds:
  - `ButtonSettings`: press 0.75, release 0.65.
  - `AxisSettings`: livezone ±1.0, deadzone ±0.05, threshold 0.01.
  - `ButtonAxisSettings`: high 0.95, low 0.05, threshold 0.01.

## Mapping: Bevy input → our input + EventBus
- `ButtonInput<KeyCode>` / `ButtonInput<GamepadButton>`:
  - `pressed` → `PlayerInput.*_held` fields (jump_held, throw_held).
  - `just_pressed` → one-frame latches (`pickup_pressed`, `swap_pressed`) or `jump_buffer_timer` set.
  - `just_released` → `throw_released` latch.
- `GamepadAxis::LeftStickX`:
  - `Gamepad::get(LeftStickX)` → `PlayerInput.move_x` (after deadzone clamp).
- `GamepadButton::South`:
  - `pressed` / `just_pressed` → `jump_held` / `jump_buffer_timer`.
- `GamepadButton::West`:
  - `just_pressed` → `pickup_pressed`.
- `GamepadButton::RightTrigger`:
  - `pressed` / `just_released` → `throw_held` / `throw_released`.
- `GamepadButton::LeftTrigger`:
  - `just_pressed` → `swap_pressed`.
- `GamepadButtonStateChangedEvent` / `GamepadButtonChangedEvent` / `GamepadAxisChangedEvent`:
  - Conceptually map to `GameEvent::ControllerInput` snapshots (audit trail).

## Open questions for our integration
- Should we rely on Bevy’s `GamepadSettings` (deadzone/threshold) and drop our manual deadzone?
- Do we want to log raw vs filtered axis values in the EventBus for debugging controller drift?

---

# Deeper Input/Update Architecture Investigation

## Current architecture summary (as implemented)
- **Update schedule (main game)**:
  - `update_event_bus_time` runs in Update for timestamping (disabled in replay).
  - Input chain: `input::capture_input` → `ai::copy_human_input` → `ai::swap_control` → nav graph ops → `ai::ai_navigation_update` → `ai::ai_decision_update`.
  - This chain is gated by `not_in_countdown` and `not_replay_active`.
- **FixedUpdate schedule**:
  - Physics chain runs in FixedUpdate (input apply → gravity → ball/player physics → shot/score).
- **Input data flow**:
  - `PlayerInput` (resource) is the human input buffer.
  - `InputState` (component) is the unified per-entity input consumed by physics.
  - `capture_input` latches one-frame actions and emits `GameEvent::ControllerInput` for auditability.
  - `copy_human_input` moves one-frame flags into `InputState` and clears them in `PlayerInput`.
  - `apply_input` consumes jump buffer and applies movement, then clears jump buffer after use.
  - `throw_ball` and `pickup_ball` consume `throw_released` and `pickup_pressed` per entity.
- **Training/simulation/testing**:
  - Training uses the same Update/FixedUpdate pattern (with pause gating and its own restart logic).
  - Simulation runner manually advances `Time<Real>`, `Time<Virtual>`, and `Time<Fixed>`, then runs Update + FixedUpdate schedules.
  - Scenario tests inject input directly in FixedUpdate (bypassing Bevy input resources).

## Assumptions embedded in our input flow
1) **Update→FixedUpdate sequencing**  
   We assume the input chain completes before each FixedUpdate tick so that `InputState` reflects the latest buffered input.
2) **One-frame latch behavior**  
   We assume `pickup_pressed`, `throw_released`, and `swap_pressed` are latched until consumed, even if multiple FixedUpdate steps happen per Update (or vice versa).
3) **Jump buffering is global (per-entity)**  
   We assume a single `jump_buffer_timer` per `InputState` is enough to model press timing across different schedules and headless mode.
4) **Deadzone is manual and consistent**  
   We assume our `STICK_DEADZONE` (0.25) and Bevy’s gamepad filtering are not both active in ways that conflict or double-filter.
5) **Time delta reliability**  
   We assume `Time::delta_secs()` is non-zero in all runtime modes; many systems clamp to 1/60 to avoid zero dt.

## Cross-check vs Bevy input behavior (0.17.3)
- Bevy input systems run in **PreUpdate**, and gamepad digital state is **cleared each frame** before raw events are applied. Our `capture_input` runs in Update and samples `Gamepad` state that Bevy has already processed.
- Bevy has **gamepad filtering** (deadzone, thresholds) via `GamepadSettings` before updating `Gamepad` state and emitting processed events.
- `ButtonInput` semantics: `just_pressed` and `just_released` are truly one-frame unless explicitly cleared.

## Risks / Mismatches to flag
- **Double filtering**: We use `STICK_DEADZONE = 0.25` in `capture_input`, but Bevy already applies deadzone/thresholds (default axis deadzone ±0.05). This can over-dampen or flatten inputs, especially near center.
- **Multi-step timing**: In headless/simulation modes, Update and FixedUpdate are manually driven. If Update runs less frequently than FixedUpdate (or vice versa), single-frame latches may last longer than intended or be consumed too soon.
- **Input state clearing**: Bevy clears `ButtonInput` just_* each frame, but our latches are explicit and may persist across frames by design. That’s good for consumption, but it can differ from Bevy’s “edge-triggered” intent if not carefully reset.
- **Focus handling**: Bevy keyboard input is tied to window focus, gamepad is not. Our `capture_input` does not explicitly model focus transitions, which could explain odd input transitions on focus changes.

## Concrete observations in code
- `capture_input` collects keyboard and gamepad state by polling `ButtonInput<KeyCode>` and `Gamepad` components and computes `move_x` by summing inputs, then clamps.
- `copy_human_input` moves single-frame flags to `InputState`, resets them in `PlayerInput`.
- `apply_input` consumes jump buffer and turns `jump_held` into variable jump height behavior.
- `throw_ball` and `pickup_ball` consume per-entity flags and do not re-clear them elsewhere.
- AI input uses `InputState` directly, with its own timers in Update, while physics consumes it in FixedUpdate.

## Potential adjustments (if we want to align more with Bevy semantics)
- **Unify deadzone filtering**: Consider using Bevy `GamepadSettings` thresholds as the only filter and lower or remove `STICK_DEADZONE` in `capture_input` to avoid double filtering.
- **Explicit input “frame”**: If we ever run multiple FixedUpdates per Update, consider a small input-queue or per-tick snapshot to prevent under/over consumption.
- **Focus-aware input gating**: Optionally gate keyboard input on window focus or mimic Bevy’s focus release behavior for consistency.

## Clarifying questions
1) Do you want Update to always run exactly once per FixedUpdate tick in all modes (game, training, simulation), or is minor skew acceptable?
2) Should the human input latches be “one Update frame” or “one FixedUpdate tick” semantics?
3) Are we intentionally overriding Bevy’s default gamepad filtering with our own (STICK_DEADZONE 0.25), or should we standardize on Bevy settings?
4) Should the EventBus record raw vs filtered values for debugging controller drift/latency?

## TODO (suggestions to verify)
- Update/FixedUpdate cadence: keep Update once per frame and FixedUpdate at 60 Hz; in headless/sim, ensure input snapshots refresh at least once per FixedUpdate tick to avoid stale input.
- Latch semantics: treat action latches as “one FixedUpdate tick” and clear after physics consumption.
- Deadzone filtering: prefer Bevy `GamepadSettings` as the single source of filtering; reduce/remove manual `STICK_DEADZONE` if using Bevy thresholds.
- EventBus logging: add optional debug logging for raw vs filtered values to diagnose drift/latency, keep filtered-only by default.
