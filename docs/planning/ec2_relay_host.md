# EC2 Relay Host Plan (Rollback)

Network trial architecture for LA↔NY testing with instant local input feel.

## Architecture

```
┌─────────────────┐         ┌─────────────────┐         ┌─────────────────┐
│   LA Laptop     │   UDP   │   EC2 Relay     │   UDP   │   NY Laptop     │
│ (controllers    │ ──────► │ (us-east-1)     │ ◄────── │ (controllers    │
│  0,1 → L0,L1)   │ ◄────── │                 │ ──────► │  0,1 → R0,R1)   │
│                 │         │ Forwards only   │         │                 │
│ Full physics    │         │ No game logic   │         │ Full physics    │
│ Rollback sync   │         │                 │         │ Rollback sync   │
└─────────────────┘         └─────────────────┘         └─────────────────┘
```

**Model:** Peer-to-peer rollback with relay (GGPO-style)
- Each client runs full physics simulation
- Your inputs apply instantly (0 frame delay)
- Remote inputs predicted, corrected via rollback when actual arrives
- EC2 just forwards UDP packets (no game logic)

## Why Rollback

| Model | Your Input Delay | Their Input Delay |
|-------|------------------|-------------------|
| Client-server | ~120ms (7-8 frames) | Instant on server |
| **Rollback** | **Instant** | **Visually corrected** |

With rollback, both LA and NY players feel instant response. Remote player may "teleport" slightly on misprediction, but your own actions are always responsive.

## Current State

### Already Built

| Component | Location | Status |
|-----------|----------|--------|
| Protocol types | `ballgame-protocol/src/` | Complete |
| `GameStateSnapshot` | `game_state.rs` | All fields serializable |
| `AgentInput` | `input.rs` | All input types covered |
| Deterministic physics | `FixedUpdate` schedule | Required for rollback |
| Snapshot creation | `server/snapshot.rs` | `create_game_snapshot()` |

### Missing for Rollback

| Component | Work Required |
|-----------|---------------|
| bevy_ggrs integration | Add dependency, adapt systems |
| State save/restore | `GameSnapshot::capture()` and `restore()` |
| UDP relay server | Standalone binary, ~100 lines |
| GGRS socket adapter | Connect GGRS to relay via UDP |
| Determinism verification | Test same inputs → same result |

## Implementation

### Phase 1: UDP Relay Server (~2 hours)

**New crate: `relay-server/` (standalone, not Bevy)**

```rust
// relay-server/src/main.rs
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

fn main() {
    let socket = UdpSocket::bind("0.0.0.0:7777").unwrap();
    let mut clients: HashMap<u8, SocketAddr> = HashMap::new();
    let mut buf = [0u8; 2048];

    println!("Relay listening on :7777");

    loop {
        let (len, src) = socket.recv_from(&mut buf).unwrap();

        // First byte = client ID (0 = LA, 1 = NY)
        let client_id = buf[0];
        clients.insert(client_id, src);

        // Forward to all other clients
        for (&id, &addr) in &clients {
            if id != client_id {
                let _ = socket.send_to(&buf[..len], addr);
            }
        }
    }
}
```

Add heartbeats and room codes for production, but this works for testing.

### Phase 2: bevy_ggrs Integration (~8 hours)

**Check Bevy 0.17 compatibility:**
```bash
cargo search bevy_ggrs  # Verify latest version supports Bevy 0.17
```

If not compatible, options:
1. Use GGRS directly without bevy_ggrs wrapper
2. Implement minimal rollback ourselves (more work)

**Cargo.toml additions:**
```toml
[dependencies]
bevy_ggrs = "0.17"  # or compatible version
ggrs = "0.10"
bincode = "1.3"     # For fast input serialization
```

**System migration:**

Move physics systems from `FixedUpdate` to GGRS schedule:

```rust
// Before
app.add_systems(FixedUpdate, (
    apply_input,
    apply_gravity,
    apply_velocity,
    check_collisions,
    // ...
).chain());

// After
app.add_systems(GgrsSchedule, (
    apply_input,
    apply_gravity,
    apply_velocity,
    check_collisions,
    // ...
).chain());
```

**Component registration for rollback:**

```rust
// Mark components that need save/restore
app.rollback_component_with_clone::<Transform>();
app.rollback_component_with_clone::<Velocity>();
app.rollback_component_with_clone::<BallState>();
app.rollback_component_with_clone::<Grounded>();
app.rollback_component_with_clone::<HoldingBall>();
app.rollback_component_with_clone::<ChargingShot>();
// ... all gameplay-affecting components
```

### Phase 3: State Save/Restore (~4 hours)

**If not using bevy_ggrs** (implementing manually):

```rust
// ballgame/src/netcode/snapshot.rs

#[derive(Clone, Serialize, Deserialize)]
pub struct RollbackSnapshot {
    pub frame: u32,
    pub players: [PlayerState; 4],
    pub ball: BallState,
    pub score: (u32, u32),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PlayerState {
    pub position: Vec2,
    pub velocity: Vec2,
    pub grounded: bool,
    pub holding_ball: bool,
    pub charge_time: f32,
    pub facing: f32,
}

impl RollbackSnapshot {
    pub fn capture(world: &World) -> Self { /* query all entities */ }
    pub fn restore(&self, world: &mut World) { /* set all components */ }
}
```

### Phase 4: GGRS Socket Adapter (~3 hours)

**Connect GGRS to UDP relay:**

```rust
use ggrs::{Message, NonBlockingSocket};

pub struct RelaySocket {
    udp: UdpSocket,
    relay_addr: SocketAddr,
    client_id: u8,
}

impl NonBlockingSocket<u8> for RelaySocket {
    fn send_to(&mut self, msg: &Message, addr: &u8) {
        let mut data = vec![self.client_id];
        data.extend(bincode::serialize(msg).unwrap());
        let _ = self.udp.send_to(&data, self.relay_addr);
    }

    fn receive_all_messages(&mut self) -> Vec<(u8, Message)> {
        let mut messages = Vec::new();
        let mut buf = [0u8; 2048];

        while let Ok((len, _)) = self.udp.recv_from(&mut buf) {
            let sender_id = buf[0];
            if let Ok(msg) = bincode::deserialize(&buf[1..len]) {
                messages.push((sender_id, msg));
            }
        }
        messages
    }
}
```

### Phase 5: Input Encoding (~2 hours)

**Pack local inputs for GGRS:**

```rust
// GGRS uses a fixed-size input type
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Pod, Zeroable)]
pub struct GgrsInput {
    pub buttons: u16,   // Bitfield: jump, shoot, action, turbo, etc.
    pub move_x: i8,     // -127 to 127, scaled from -1.0 to 1.0
}

impl GgrsInput {
    const JUMP: u16 = 1 << 0;
    const SHOOT: u16 = 1 << 1;
    const ACTION: u16 = 1 << 2;
    const TURBO: u16 = 1 << 3;
    // ...
}
```

### Phase 6: AWS Setup (~1 hour)

**EC2 Instance:**
- Type: t3.micro (free tier or ~$8/month)
- Region: us-east-1 (between LA and NY)
- Security group: Allow **UDP 7777** inbound from anywhere

**Deploy relay:**
```bash
# On EC2
cargo build --release -p relay-server
./target/release/relay-server
# Logs: "Relay listening on :7777"
```

**Client usage:**
```bash
# LA laptop
cargo run -p ballgame -- --netplay 0 --relay 54.xxx.xxx.xxx:7777 --slots 0,1

# NY laptop
cargo run -p ballgame -- --netplay 1 --relay 54.xxx.xxx.xxx:7777 --slots 2,3
```

## Files to Create/Modify

| File | Action |
|------|--------|
| `relay-server/` | Create - New crate for UDP relay |
| `relay-server/Cargo.toml` | Create |
| `relay-server/src/main.rs` | Create |
| `Cargo.toml` (workspace) | Modify - Add relay-server to members |
| `ballgame/Cargo.toml` | Modify - Add bevy_ggrs, ggrs, bincode |
| `ballgame/src/netcode/mod.rs` | Create - Rollback module |
| `ballgame/src/netcode/socket.rs` | Create - GGRS socket adapter |
| `ballgame/src/netcode/input.rs` | Create - Input encoding |
| `ballgame/src/netcode/snapshot.rs` | Create - If not using bevy_ggrs auto |
| `ballgame/src/main.rs` | Modify - Add --netplay flag, GGRS setup |
| `ballgame/src/lib.rs` | Modify - Export netcode module |

## Verification

### 1. Determinism Test (Critical)
```bash
# Run same inputs twice, verify identical result
cargo run --bin test-scenarios -- determinism/
```

Create test that:
1. Records input sequence
2. Runs simulation twice with same inputs
3. Asserts final state matches exactly

### 2. Local Rollback Test
```bash
# Terminal 1
cargo run -p ballgame -- --netplay 0 --relay 127.0.0.1:7777 --slots 0,1

# Terminal 2
cargo run -p ballgame -- --netplay 1 --relay 127.0.0.1:7777 --slots 2,3

# Terminal 3 (relay)
cargo run -p relay-server
```

### 3. Simulated Latency Test
```bash
# Add artificial delay to test rollback behavior
cargo run -p ballgame -- --netplay 0 --relay 127.0.0.1:7777 --fake-ping 100
```

### 4. EC2 Test
1. Deploy relay to EC2
2. Both laptops connect to EC2 relay
3. Play full match, note any teleporting/corrections

## Expected Feel

| Player | Your Actions | Their Actions |
|--------|--------------|---------------|
| LA | Instant | May see ~4 frame correction on misprediction |
| NY | Instant | May see ~1 frame correction on misprediction |

Rollback handles up to ~150ms well. LA↔NY is ~80ms, well within range.

## Cost

| Resource | Cost |
|----------|------|
| t3.micro on-demand | ~$8/month |
| t3.micro spot | ~$2.50/month |
| Free tier (first year) | $0 |
| Data transfer | <$1 (inputs are tiny) |

## Risk: bevy_ggrs Compatibility

If bevy_ggrs doesn't support Bevy 0.17 yet:

**Option A: Use GGRS directly**
- More code but full control
- ~4 extra hours of work

**Option B: Minimal rollback implementation**
- Build save/restore ourselves
- Use existing snapshot code as base
- More work but no external dependency

**Option C: Wait for bevy_ggrs update**
- Check their GitHub for 0.17 support timeline

I recommend checking compatibility first:
```bash
cargo add bevy_ggrs --dry-run
```

## Timeline

| Phase | Work |
|-------|------|
| 1. Relay server | 2 hours |
| 2. bevy_ggrs integration | 8 hours |
| 3. State save/restore | 4 hours |
| 4. Socket adapter | 3 hours |
| 5. Input encoding | 2 hours |
| 6. AWS setup | 1 hour |
| 7. Testing & debugging | 4 hours |
| **Total** | **~24 hours** |

---

This doc: `docs/planning/ec2_relay_host.md`
