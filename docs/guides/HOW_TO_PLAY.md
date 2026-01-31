# How to Play

A 2v2 ball sport game. Score by throwing the ball into the opponent's basket.

## Controls

```
              CONTROLLER                              KEYBOARD
         ___________________________
        /   [LB]         [RB]       \            LB = Q (Pass/Steal - context)
       /   Pass/       Throw/       \            RB = F (Throw - hold to charge)
      /    Steal       Block         \
     /  _________________________     \
    | /                           \    |
    ||    [D-PAD]        [YBXA]    |   |         D-PAD = Arrow Keys (menus)
    ||       ^          [Y]        |   |
    ||     < + >     [X]   [B]     |   |         A/South = Space/W (Jump)
    ||       v          [A]        |   |         X/West  = E (Turbo)
    ||                             |   |
    ||   [L-STICK]    [R-STICK]    |   |         L-STICK = A/D (Move)
    ||      ( )          ( )       |   |
    | \___________________________|   |
    |   [BACK]   [XBOX]   [START]     |         START = R (Reset Level)
     \_______________________________/
```

## Modal Input System

Controls change based on ball possession context:

| Context | LB / Q | RB / F | X / E | A / Space |
|---------|--------|--------|-------|-----------|
| **Holding ball** | Pass | Shoot | Turbo | Jump |
| **Teammate has ball** | Steal | Block | Turbo | Jump |
| **Opponent has ball** | Steal | Block | Turbo | Jump |
| **Free ball nearby** | Pickup | Pickup | Turbo | Jump |

## Basic Actions

| Action | Controller | Keyboard | Notes |
|--------|------------|----------|-------|
| **Move** | Left Stick | A / D | |
| **Jump** | A (South) | Space or W | |
| **Pickup Ball** | LB or RB | Q or F | When near free ball |
| **Steal** | LB | Q | When near opponent with ball |
| **Throw** | RB (hold) | F (hold) | Charge to power up |
| **Pass** | LB | Q | When holding ball |
| **Block** | RB | F | When not holding ball |
| **Turbo** | X (West) | E | Speed boost, drains gauge |
| **Cycle Player** | D-pad Right | ] | Cycle character control |
| **Reset Level** | Start | R | |

## New Mechanics

### Pass (LB / Q when holding ball)
- Auto-aims at your teammate
- Faster than a shot, direct line
- Can be intercepted by opponent's block
- Visual indicator shows pass target

### Block (RB / F when not holding ball)
- Creates intercept zone around you
- Catches incoming passes and shots
- Slows your horizontal movement
- Has cooldown after use

### Turbo (X / E - hold)
- Speed boost while held
- Drains turbo gauge
- Refills when released
- Good for chasing or escaping

## How to Score

1. **Pick up the ball** - Walk near it and press LB/Q or RB/F
2. **Get to a good position** - Higher platforms = better shots
3. **Charge your throw** - Hold RB/F, release when ready
4. **Score!** - Ball in basket = 1 point, carry-in = 2 points

## Tips

- **Charge matters** - Longer charge = more accurate shot
- **Elevation helps** - Shoot from platforms for better angles
- **Steal attempts** - Press LB/Q near an opponent holding the ball (33% chance, +17% if they're charging)
- **Jump shots** - You can throw while airborne
- **Watch the gauge** - The charge bar shows your current power
- **Use turbo wisely** - Great for chasing or creating separation
- **Block opponents** - Intercept their shots and passes
- **Pass to teammates** - Sometimes passing creates better opportunities

## Game Modes

- **2v2 vs AI** - Default mode, you control one player
- **Observer** - Cycle past all players to watch AI vs AI
- **Cycle through** - D-pad Right cycles through all characters

## D-Pad Options (Controller)

Press a D-pad direction to select, then use LT/RT to cycle values:

| Direction | What it changes |
|-----------|-----------------|
| Up | Viewport size |
| Down | Game presets (Composite/Movement/Ball/Shooting) |
| Left | AI profile (LT: player, RT: profile) |
| Right | Character / Level / Palette / Ball Style |

## Keyboard Extras

| Key | Action |
|-----|--------|
| ] | Cycle character / Next level |
| [ | Previous level |
| V | Cycle viewport |
| Tab | Toggle debug info |
| F1 | Physics tweak panel |

---

*Run with `cargo run` or use training mode: `cargo run --bin training`*
