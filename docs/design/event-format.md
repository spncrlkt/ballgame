# Game Event Log Format

Compact text format for logging all game events. Used for AI simulation analysis, gameplay replay, and analytics pipelines.

## Format

Each line is one event:

```
T:NNNNN|CODE|data...
```

- `T:NNNNN` - Timestamp in milliseconds (5 digits, wraps at 99999)
- `CODE` - 2-character event type code
- `data` - Pipe-separated values specific to event type

## Event Types

### Match Events

| Code | Event | Data Format |
|------|-------|-------------|
| `MS` | Match Start | `level\|level_name\|left_profile\|right_profile\|seed` |
| `ME` | Match End | `score_left\|score_right\|duration` |

### Scoring Events

| Code | Event | Data Format |
|------|-------|-------------|
| `G` | Goal | `player\|score_left\|score_right` |

### Ball Events

| Code | Event | Data Format |
|------|-------|-------------|
| `PU` | Pickup | `player` |
| `DR` | Drop | `player` |
| `SS` | Shot Start | `player\|x,y\|quality` |
| `SR` | Shot Release | `player\|charge\|angle\|power` |

### Steal Events

| Code | Event | Data Format |
|------|-------|-------------|
| `SA` | Steal Attempt | `attacker` |
| `S+` | Steal Success | `attacker` |
| `S-` | Steal Fail | `attacker` |

### Movement Events

| Code | Event | Data Format |
|------|-------|-------------|
| `J` | Jump | `player` |
| `LD` | Land | `player` |

### AI State Events

| Code | Event | Data Format |
|------|-------|-------------|
| `AG` | AI Goal | `player\|goal_name` |
| `NS` | Nav Start | `player\|target_x,target_y` |
| `NC` | Nav Complete | `player` |

### Input Events (optional, sampled)

| Code | Event | Data Format |
|------|-------|-------------|
| `I` | Input | `player\|move_x\|flags` |

Flags: `J`=jump, `T`=throw, `P`=pickup, `-`=none

### Tick Events (optional, sampled)

| Code | Event | Data Format |
|------|-------|-------------|
| `T` | Tick | `frame\|left_x,left_y\|right_x,right_y\|ball_x,ball_y\|ball_state` |

Ball state: `F`=Free, `H`=Held, `I`=InFlight

## Player IDs

- `L` - Left player
- `R` - Right player

## Example Log

```
T:00000|MS|2|Open Floor|Balanced|Aggressive|12345678901234
T:00150|PU|L
T:00320|SS|L|-200.5,-418.2|0.47
T:00850|SR|L|0.65|62.3|720.5
T:01200|G|L|1|0
T:01500|SA|R
T:01520|S-|R
T:03200|AG|R|ChaseBall
T:04500|ME|1|0|45.5
```

## Usage

### In Simulation

```rust
use ballgame::events::{EventBuffer, GameEvent, PlayerId};

let mut buffer = EventBuffer::new();
buffer.set_start_time(time.elapsed_secs());

// Log events
buffer.log(time, GameEvent::Goal {
    player: PlayerId::L,
    score_left: 1,
    score_right: 0,
});

// Get serialized output
let log_text = buffer.serialize();
```

### In Gameplay

```rust
use ballgame::events::{EventLogger, EventLogConfig, GameEvent};

// Create logger resource
let config = EventLogConfig {
    log_dir: "logs".into(),
    enabled: true,
    tick_sample_ms: 100,
    ..default()
};
let logger = EventLogger::new(config);

// Start session
logger.start_session("20260123_143052");

// Log events during gameplay
logger.log(time, GameEvent::Pickup { player: PlayerId::L });

// End session
logger.end_session();
```

## File Extension

Event log files use `.evlog` extension.

## Analytics Pipeline

The format is designed for:

1. **Streaming analysis** - Process line-by-line
2. **Aggregation** - Count events by type, player, time window
3. **Pattern detection** - Sequence matching for play patterns
4. **ML training** - Input sequences for behavior models
