# Multiplayer Architecture

This document covers netcode models, tradeoffs, and implementation strategy for adding online multiplayer to the ball game.

## Netcode Models

### 1. Lockstep (Deterministic)

**How it works:** All clients run identical simulation. Only inputs are sent over network. Game waits for all inputs before advancing each frame.

```
Frame 10: Wait for Player 1 input... wait for Player 2 input... advance
Frame 11: Wait for Player 1 input... wait for Player 2 input... advance
```

**Pros:**
- Simple to implement
- Minimal bandwidth (just inputs)
- Perfect sync guaranteed

**Cons:**
- Latency = slowest player's ping (feels laggy)
- One slow connection affects everyone
- Dropped packets freeze the game

**Best for:** Turn-based games, slow-paced strategy (Civilization, classic RTS)

**Not great for:** Fast action games like this one

---

### 2. Client-Server Authoritative

**How it works:** Server is the "truth." Clients send inputs to server, server simulates, sends state back to clients. Clients predict locally but server corrects them.

```
Client: "I pressed jump" → Server
Server: Simulates, sends back "You're at position (x,y)"
Client: Snaps to server position (or interpolates)
```

**Pros:**
- Server prevents cheating (authoritative)
- Works well with many players
- Scales to MMO-size

**Cons:**
- Requires dedicated server (cost, infrastructure)
- Input delay = round trip to server
- Correction "snapping" can feel bad

**Best for:** Competitive games where anti-cheat matters, games with many players (FPS, MMOs)

**Consideration:** Requires running servers or paying for hosting

---

### 3. Rollback Netcode (GGPO-style)

**How it works:** Each client predicts the game forward assuming remote inputs stay the same. When real inputs arrive, if prediction was wrong, game "rolls back" and re-simulates to correct state. Player never waits.

```
Frame 10: Predict opponent keeps moving right → render
Frame 11: Predict opponent keeps moving right → render
Frame 12: Actual input arrives for frame 10 (opponent jumped!)
         → Rollback to frame 10, re-simulate 10,11,12 with correct input
         → Render corrected frame 12
```

**Pros:**
- Local actions feel instant (0 input delay for your own moves)
- Handles network variance gracefully
- Peer-to-peer works (no server costs)
- Industry standard for fighting games, action sports

**Cons:**
- Complex to implement correctly
- Requires deterministic simulation
- Visible "teleporting" on bad predictions
- CPU cost of re-simulation

**Best for:** Fast-paced 1v1 or small team games - fighting games, sports games, action games

---

### 4. Snapshot Interpolation

**How it works:** Server sends full game state snapshots at fixed rate. Clients interpolate between snapshots for smooth visuals. Clients are always slightly "in the past."

```
Server: Sends snapshot every 50ms
Client: Receives snapshots, renders interpolated state 100ms behind "now"
```

**Pros:**
- Very smooth visuals
- Handles packet loss well
- Simpler than rollback

**Cons:**
- Everything you see is slightly in the past
- Requires server
- Higher bandwidth (full state vs just inputs)

**Best for:** Games where slight delay is acceptable (some shooters, racing games)

---

## Comparison Table

| Aspect | Lockstep | Client-Server | Rollback | Snapshot |
|--------|----------|---------------|----------|----------|
| Input feel | Laggy | Delayed | Instant | Delayed |
| Implementation | Simple | Medium | Complex | Medium |
| Server needed | No | Yes | No | Yes |
| Bandwidth | Low | Medium | Low | High |
| Anti-cheat | Weak | Strong | Weak | Strong |
| Best player count | 2-4 | Many | 2-4 | Many |

## Recommendation: Rollback

For a 2v2 fast-paced ball game, **rollback netcode** is the standard choice.

**Why rollback fits this game:**
- Ball movement and player actions need instant response
- 4 players is manageable for rollback re-simulation
- Peer-to-peer means no server costs
- Physics is already deterministic (FixedUpdate)

**Games using rollback:**
- Fighting games: Street Fighter, Guilty Gear, Mortal Kombat
- Sports/action: Windjammers, Lethal League, Rivals of Aether
- Platform fighters: MultiVersus, Brawlhalla

---

## Current Codebase Readiness

Rollback requires specific architectural foundations. Current status:

| Requirement | Status | Notes |
|-------------|--------|-------|
| Deterministic physics | ✅ Ready | FixedUpdate with fixed timestep |
| Input buffering | ✅ Ready | PlayerInput resource exists |
| Serializable game state | ⚠️ Needed | Add derive macros to components |
| State snapshot/restore | ⚠️ Needed | Save/load full game state |
| Separate render from sim | ✅ Partial | Update vs FixedUpdate split exists |
| Fixed-point math | ❓ Optional | Floats work if all clients use same hardware |

### What Needs to Be Added

**1. State Serialization**

All gameplay-relevant components need `Serialize`/`Deserialize`:

```rust
use serde::{Serialize, Deserialize};

#[derive(Component, Serialize, Deserialize)]
pub struct Velocity(pub Vec2);

#[derive(Component, Serialize, Deserialize)]
pub struct BallState {
    pub state: BallStateEnum,
}

// ... etc for all physics/gameplay components
```

**2. Snapshot System**

Capture and restore full game state:

```rust
#[derive(Serialize, Deserialize)]
pub struct GameSnapshot {
    pub frame: u32,
    pub players: Vec<PlayerSnapshot>,
    pub ball: BallSnapshot,
    pub score: (u32, u32),
}

#[derive(Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub position: Vec2,
    pub velocity: Vec2,
    pub grounded: bool,
    pub holding_ball: bool,
    pub charging: Option<f32>,
}

impl GameSnapshot {
    pub fn capture(world: &World) -> Self { /* ... */ }
    pub fn restore(&self, world: &mut World) { /* ... */ }
}
```

**3. Re-simulation Loop**

Ability to run N simulation frames without rendering:

```rust
fn resimulate(world: &mut World, from_frame: u32, inputs: &[FrameInputs]) {
    for frame_inputs in inputs {
        apply_inputs(world, frame_inputs);
        run_physics_step(world);  // Your FixedUpdate systems
    }
}
```

---

## Bevy Networking Libraries

### bevy_ggrs (Recommended)

GGRS is a Rust implementation of GGPO, the industry-standard rollback library.

```toml
[dependencies]
bevy_ggrs = "0.15"  # verify Bevy 0.17 compatibility
ggrs = "0.10"
```

**What it provides:**
- Rollback algorithm (save/load/resimulate)
- Input synchronization
- Peer-to-peer networking
- Spectator support

**What you provide:**
- Game state serialization
- Input encoding
- Physics systems adapted to GGRS schedule

**Integration pattern:**

```rust
use bevy_ggrs::prelude::*;

// Mark components for rollback
#[derive(Component, Clone, Copy, Reflect, Default)]
#[reflect(Component)]
pub struct Velocity(pub Vec2);

// Register with GGRS
app.rollback_component_with_clone::<Velocity>();

// Add systems to GGRS schedule (not regular FixedUpdate)
app.add_systems(GgrsSchedule, (
    apply_input,
    apply_gravity,
    apply_velocity,
    check_collisions,
    // ... your physics systems
).chain());
```

**Pros:**
- Battle-tested GGPO algorithm
- Active Bevy integration
- Good documentation
- Handles the hard parts

**Cons:**
- Learning curve for GGRS concepts
- May lag behind latest Bevy version
- Need to restructure systems for GGRS schedule

### lightyear

Full-featured networking framework supporting multiple netcode models.

```toml
[dependencies]
lightyear = "0.15"  # verify version
```

**What it provides:**
- Client-server AND peer-to-peer modes
- Rollback support
- Interest management (hide distant entities)
- Lobby system
- Prediction and interpolation

**Pros:**
- Very full-featured
- Flexible architecture
- Good for complex games

**Cons:**
- Steeper learning curve
- More configuration needed
- Might be overkill for 2v2

### Steam Networking + Custom

Use Steam's transport layer, implement rollback yourself.

```rust
use steamworks::Client;

// Steam provides:
// - NAT traversal (P2P connections work)
// - Relay servers (fallback if P2P fails)
// - Lobbies and matchmaking

// You implement:
// - Rollback logic
// - State serialization
// - Input synchronization
```

**Pros:**
- Full control
- No external dependencies beyond Steam
- Learn the concepts deeply

**Cons:**
- Most implementation work
- Easy to introduce subtle bugs
- Reinventing solved problems

---

## Steam Integration

Regardless of netcode library, Steam provides:

### Lobbies

Matchmaking and friend invites:

```rust
let matchmaking = client.matchmaking();

// Create lobby
matchmaking.create_lobby(LobbyType::FriendsOnly, 4, |result| {
    match result {
        Ok(lobby_id) => println!("Lobby created: {}", lobby_id),
        Err(e) => println!("Failed: {}", e),
    }
});

// Join lobby
matchmaking.join_lobby(lobby_id, |result| { /* ... */ });

// Invite friend
client.friends().activate_game_overlay_invite_dialog(lobby_id);
```

### Steam Networking Sockets

Low-level reliable/unreliable messaging:

```rust
let networking = client.networking_sockets();

// Connect to peer via Steam ID
let connection = networking.connect_p2p(peer_steam_id, 0, &[]);

// Send data
networking.send_message(connection, data, SendFlags::RELIABLE);

// Receive data
let messages = networking.receive_messages(connection, 10);
```

### Steam Relay

If direct P2P fails, Steam routes traffic through their servers automatically. No code change needed - it's transparent.

---

## Implementation Roadmap

### Phase 1: Foundation (Pre-multiplayer)

**Goal:** Prepare codebase for networking without breaking single-player.

- [ ] Add `Serialize`/`Deserialize` to all gameplay components
- [ ] Implement `GameSnapshot` capture/restore
- [ ] Add snapshot unit tests (capture → modify → restore → verify)
- [ ] Benchmark re-simulation speed (need ~8 frames in <16ms)

### Phase 2: Local Rollback Testing

**Goal:** Test rollback locally before adding networking.

- [ ] Integrate `bevy_ggrs` with local sessions
- [ ] Move physics systems to GGRS schedule
- [ ] Test with simulated latency/packet loss
- [ ] Verify determinism (same inputs = same result)

### Phase 3: Networking

**Goal:** Connect two clients over network.

- [ ] Add Steam lobby creation/joining
- [ ] Connect GGRS to Steam networking transport
- [ ] Test on LAN first
- [ ] Test over internet

### Phase 4: Polish

**Goal:** Production-ready multiplayer.

- [ ] Handle disconnections gracefully
- [ ] Add reconnection support
- [ ] Implement spectator mode
- [ ] Add matchmaking UI
- [ ] Test with real players on Steam Deck

---

## Testing Strategy

### Determinism Testing

Same inputs must produce identical results:

```rust
#[test]
fn determinism_test() {
    let inputs = vec![/* recorded input sequence */];

    let result1 = run_simulation(&inputs);
    let result2 = run_simulation(&inputs);

    assert_eq!(result1.final_state, result2.final_state);
}
```

### Network Condition Simulation

Test with artificial latency and packet loss:

```rust
// GGRS supports this for testing
let mut sess = SessionBuilder::new()
    .with_input_delay(2)  // Frames of input delay
    // ...
```

### Cross-platform Determinism

If targeting multiple platforms, verify same results:
- Run same replay on Linux (Steam Deck) and dev machine
- Compare final states

For Linux-only release, this is less of a concern.

---

## Resources

**Rollback Netcode:**
- [GGPO: Good Game, Peace Out](https://www.ggpo.net/) - Original GGPO
- [Infil's Rollback Explainer](https://ki.infil.net/w02-netcode.html) - Excellent visual guide
- [Core-A Gaming: Analysis of Rollback](https://www.youtube.com/watch?v=0NLe4IpdS1w) - Video explainer

**Bevy Networking:**
- [bevy_ggrs docs](https://github.com/gschup/bevy_ggrs)
- [lightyear docs](https://github.com/cBournhonesque/lightyear)
- [Bevy Cheatbook: Networking](https://bevy-cheatbook.github.io/patterns/network.html)

**Steam:**
- [Steamworks Networking](https://partner.steamgames.com/doc/features/multiplayer/networking)
- [steamworks-rs](https://crates.io/crates/steamworks)

---

## Open Questions

- [ ] Target latency tolerance? (Rollback handles ~100-150ms well)
- [ ] Ranked matchmaking or casual only?
- [ ] Cross-play considerations? (Linux-only simplifies this)
- [ ] Spectator mode priority?
- [ ] Replay system integration with netcode?

---

*Last updated: 2026-01-29*
