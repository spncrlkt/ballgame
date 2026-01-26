# How to Play

A 2v2 ball sport game. Score by throwing the ball into the opponent's basket.

## Controls

```
              CONTROLLER                              KEYBOARD
         ___________________________
        /   [LB]         [RB]       \            LB = Q (Cycle Player)
       /   Cycle       THROW/       \            RB = F (Throw - hold to charge)
      /    Player      CHARGE        \
     /  _________________________     \
    | /                           \    |
    ||    [D-PAD]        [YBXA]    |   |         D-PAD = Arrow Keys (menus)
    ||       ^          [Y]        |   |
    ||     < + >     [X]   [B]     |   |         A/South = Space/W (Jump)
    ||       v          [A]        |   |         X/West  = E (Pickup/Steal)
    ||                             |   |
    ||   [L-STICK]    [R-STICK]    |   |         L-STICK = A/D (Move)
    ||      ( )          ( )       |   |
    | \___________________________|   |
    |   [BACK]   [XBOX]   [START]     |         START = R (Reset Level)
     \_______________________________/
```

## Basic Actions

| Action | Controller | Keyboard |
|--------|------------|----------|
| **Move** | Left Stick | A / D |
| **Jump** | A (South) | Space or W |
| **Pickup Ball** | X (West) | E |
| **Steal** | X (West) | E (near opponent) |
| **Throw** | RB (hold to charge) | F (hold to charge) |
| **Cycle Player** | LB | Q |
| **Reset Level** | Start | R |

## How to Score

1. **Pick up the ball** - Walk near it and press X/E
2. **Get to a good position** - Higher platforms = better shots
3. **Charge your throw** - Hold RB/F, release when ready
4. **Score!** - Ball in basket = 1 point, carry-in = 2 points

## Tips

- **Charge matters** - Longer charge = more accurate shot
- **Elevation helps** - Shoot from platforms for better angles
- **Steal attempts** - Press X/E near an opponent holding the ball (33% chance, +17% if they're charging)
- **Jump shots** - You can throw while airborne
- **Watch the gauge** - The charge bar shows your current power

## Game Modes

- **1v1 vs AI** - Default mode, you control one player
- **Observer** - Press LB/Q twice to watch AI vs AI
- **Cycle through** - LB/Q cycles: Left Player → Right Player → Observer

## D-Pad Options (Controller)

Press a D-pad direction to select, then use LT/RT to cycle values:

| Direction | What it changes |
|-----------|-----------------|
| Up | Viewport size |
| Down | Game presets (Composite/Movement/Ball/Shooting) |
| Left | AI profile (LT: player, RT: profile) |
| Right | Level / Palette / Ball Style |

## Keyboard Extras

| Key | Action |
|-----|--------|
| ] | Next level |
| [ | Previous level |
| V | Cycle viewport |
| Tab | Toggle debug info |
| F1 | Physics tweak panel |

---

*Run with `cargo run` or use training mode: `cargo run --bin training`*
