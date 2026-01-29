# Steam Release Guide

This document outlines the process for releasing the ball game on Steam, targeting Steam Deck with a free-to-play + DLC model via Early Access.

## Release Strategy

### Early Access

**What it is:** Release an incomplete but playable game. Players buy in knowing it's unfinished and expect regular updates. You "graduate" to full release (1.0) when ready.

**Why it fits this project:**
- Release with core gameplay (2v2 ball game, local AI opponents)
- Iterate based on player feedback
- Add online multiplayer, more levels, polish over time
- Clear communication: "This is a work in progress"

**Early Access requirements:**
- Playable core loop (you have this)
- Clear roadmap of planned features
- Regular updates (monthly minimum recommended)
- Honest store page about current vs planned state

**Typical Early Access timeline:** 6-18 months before 1.0

### Pricing Model: Free + DLC

**Base game:** Free
- Core gameplay, local AI matches
- Limited levels/cosmetics

**Paid DLC options:**
- Cosmetic packs (ball styles, player skins, palettes)
- Level packs
- "Supporter pack" (all cosmetics + future content)
- Online multiplayer access (if you want to gate it)

**Considerations for free games on Steam:**
- No $100 fee refund (paid games get it back after $1,000 revenue)
- Slightly lower visibility in some algorithms vs paid games
- Higher download numbers, but conversion to DLC matters
- Can change to paid later, but can't go free→paid→free

**Alternative:** $2.99 base game + DLC. Low barrier, gets fee refund, better algorithm treatment.

## Target Platform: Steam Deck (Linux Only)

| Platform | Target Triple | Status |
|----------|---------------|--------|
| Linux (Steam Deck) | `x86_64-unknown-linux-gnu` | Primary |

Steam Deck runs Linux natively. No Windows or macOS builds needed.

**Benefits of Linux-only:**
- Simpler build pipeline
- Native testing on Linux machine
- No cross-compilation headaches
- Steam Deck is the exact target hardware

**Steam Deck Verified:** Aim for "Verified" badge by ensuring:
- All controls work with gamepad (you have this)
- UI readable at 1280x800
- No external launchers or anti-cheat
- Default graphics settings run well on Deck hardware

## Build Setup (Linux)

### Prerequisites

```bash
# On your Linux machine
sudo apt install build-essential pkg-config libasound2-dev libudev-dev
rustup target add x86_64-unknown-linux-gnu
```

### Build Command

```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

### Asset Bundling

```
release/
├── ballgame                  # Linux executable (no extension)
├── assets/
│   ├── fonts/
│   └── textures/
└── config/
    ├── levels.txt
    ├── ai_profiles.txt
    ├── ball_options.txt
    └── game_presets.txt
```

### Build Script

```bash
#!/bin/bash
# scripts/build_release.sh

set -e

TARGET="x86_64-unknown-linux-gnu"
RELEASE_DIR="release"

echo "Building release..."
cargo build --release --target $TARGET

echo "Packaging..."
rm -rf $RELEASE_DIR
mkdir -p $RELEASE_DIR

cp target/$TARGET/release/ballgame $RELEASE_DIR/
cp -r assets $RELEASE_DIR/
cp -r config $RELEASE_DIR/

echo "Release built to $RELEASE_DIR/"
ls -la $RELEASE_DIR/
```

## Steamworks Partner Account

### Step-by-Step Setup

1. **Go to** https://partner.steamgames.com

2. **Create account** with your Steam login
   - Use your personal Steam account or create a new one for business

3. **Accept agreements**
   - Steam Distribution Agreement
   - Tax interview (W-9 for US, W-8BEN for international)

4. **Add payment info**
   - Bank account for revenue deposits
   - Minimum payout threshold: $100

5. **Pay $100 app credit fee**
   - Per-game fee, covers one App ID
   - Refunded after $1,000 gross revenue (not applicable for free games)

6. **Wait for approval** (2-5 business days)

7. **Create your app**
   - Steamworks dashboard → "Create new app"
   - Choose "Game"
   - Get your App ID (you'll need this everywhere)

### Account vs App

- **Partner account:** Your developer identity, one-time setup
- **App:** Each game you release, $100 each
- You can have multiple apps under one partner account

## Steamworks Features (Post-Launch)

Ship without these initially. Add later as needed.

### Steam Overlay

Minimal integration - just lets Steam overlay work in-game. Worth having.

```rust
// Basic init - allows overlay to function
if let Ok(client) = steamworks::Client::init() {
    // Steam is running, overlay will work
}
// If init fails, game still works (non-Steam launch)
```

### Achievements

Good for engagement. Define in Steamworks dashboard, trigger from code.

```rust
client.user_stats().set_achievement("FIRST_GOAL");
```

**When to add:** After core gameplay is stable, achievements give players goals.

### Cloud Saves

Automatically sync save files across devices. Config in Steamworks dashboard.

**When to add:** When you have meaningful progression to save.

### Leaderboards

Global/friends scoreboards.

```rust
client.user_stats().find_or_create_leaderboard(
    "HighScores",
    LeaderboardSortMethod::Descending,
    LeaderboardDisplayType::Numeric,
);
```

**When to add:** When you have a scoring/ranking system worth comparing.

### Online Multiplayer (Steam Networking)

**This is the big one for your roadmap.**

Steam provides:
- **Steam Networking Sockets:** Low-level UDP with NAT traversal
- **Steam Lobbies:** Matchmaking, friend invites
- **Steam Relay:** Traffic routing through Valve servers (reduces cheating, improves connectivity)

```rust
// Simplified - actual implementation is more involved
let networking = client.networking_sockets();
let lobby = client.matchmaking().create_lobby(LobbyType::Public, 4);
```

**When to add:** This is a significant feature. Plan for it as a major milestone.

**Alternatives to Steam Networking:**
- Self-hosted servers (more control, more work)
- Third-party services (Nakama, PlayFab)
- Peer-to-peer with a relay service

## Multiplayer

Online multiplayer is planned as a first-class feature. See **[docs/multiplayer_architecture.md](docs/multiplayer_architecture.md)** for:

- Netcode models comparison (lockstep, client-server, rollback, snapshot)
- Why rollback is recommended for this game
- Current codebase readiness assessment
- Bevy networking library options (bevy_ggrs, lightyear)
- Steam lobby/networking integration
- Implementation roadmap

**Summary:** Use `bevy_ggrs` (rollback netcode) with Steam lobbies. The codebase has good foundations (deterministic FixedUpdate physics, input buffering) but needs state serialization added.

## Store Page Assets

| Asset | Dimensions | Purpose |
|-------|------------|---------|
| Header capsule | 460x215 | Store page header, wishlist |
| Small capsule | 231x87 | Search results, lists |
| Large capsule | 616x353 | Featured sections |
| Hero graphic | 3840x1240 | Top of store page (optional) |
| Logo | 1280x720 | Overlaid on hero |
| Screenshots | 1920x1080 | Gameplay showcase (5+ required) |
| Trailer | 1080p+ | Autoplay on store page |

**Tips:**
- Capsules need readable text at small sizes
- Screenshots should show actual gameplay
- Trailer: 30-90 seconds, show gameplay fast

## SteamPipe Upload

### Directory Structure (Simplified for Linux-only)

```
steam_build/
├── scripts/
│   ├── app_build.vdf
│   └── depot_linux.vdf
└── content/
    └── (your release files)
```

### app_build.vdf

```
"appbuild"
{
    "appid" "YOUR_APP_ID"
    "desc" "Build description"
    "buildoutput" "../output/"
    "contentroot" "../content/"
    "setlive" "beta"  // or "default" for live
    "depots"
    {
        "YOUR_DEPOT_ID" "depot_linux.vdf"
    }
}
```

### depot_linux.vdf

```
"DepotBuildConfig"
{
    "DepotID" "YOUR_DEPOT_ID"
    "contentroot" "../content/"
    "FileMapping"
    {
        "LocalPath" "*"
        "DepotPath" "."
        "recursive" "1"
    }
}
```

### Upload Command

```bash
steamcmd +login YOUR_USERNAME +run_app_build scripts/app_build.vdf +quit
```

## Release Checklist

### Account Setup
- [ ] Steamworks partner account created
- [ ] Tax/banking info submitted
- [ ] $100 fee paid
- [ ] App ID obtained

### Pre-Release (Early Access)
- [ ] Linux build runs on Steam Deck
- [ ] Gamepad controls work fully
- [ ] UI readable at 1280x800
- [ ] Store page assets created
- [ ] Store page text written (current state + roadmap)
- [ ] Screenshots captured
- [ ] Trailer created (optional but recommended)
- [ ] Build uploaded via SteamPipe
- [ ] Build review passed
- [ ] Store page review passed

### Launch Day
- [ ] Set release date
- [ ] Prepare announcement post
- [ ] Tell friends to wishlist/download
- [ ] Monitor Steam discussions

### Post-Launch Priorities
1. Respond to player feedback
2. Regular updates (monthly minimum)
3. Add Steamworks integration (achievements, cloud saves)
4. Online multiplayer milestone
5. DLC content releases
6. Graduate from Early Access

## Open Questions

- [ ] Game title for Steam?
- [ ] Free or low-cost ($2.99)?
- [ ] What's included in free tier vs DLC?
- [ ] Timeline estimate for online multiplayer?
- [ ] Networking library preference (bevy_ggrs, lightyear, custom)?

---

*Last updated: 2026-01-29*
