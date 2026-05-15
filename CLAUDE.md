# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A modular trick-based card game platform with a Rust backend and Vue 3 frontend. The platform is designed to support multiple games (Sheepshead first, then Euchre, Hearts, Spades) through a shared engine with pluggable game-specific rules.

## Repository Structure

```
tricks/
├── server/          # Rust backend (Axum + WebSockets)
│   ├── src/
│   │   ├── main.rs
│   │   ├── engine/  # Core game engine (game-agnostic)
│   │   ├── games/   # Per-game rule modules (sheepshead/, euchre/, etc.)
│   │   ├── lobby/   # Room/session management
│   │   └── ws/      # WebSocket handlers
│   └── Cargo.toml
└── client/          # Vue 3 + Vite frontend
    ├── src/
    │   ├── games/   # Per-game UI components
    │   └── engine/  # Shared game state logic (mirrors server engine)
    ├── package.json
    └── vite.config.ts
```

## Commands

### Backend (Rust)
```bash
cd server
cargo build                          # Build
cargo run                            # Run dev server
cargo test                           # All tests
cargo test engine::                  # Tests in a specific module
cargo test -- --nocapture            # Tests with stdout
cargo clippy -- -D warnings          # Lint
cargo fmt                            # Format
```

### Frontend (Vue)
```bash
cd client
npm install                          # Install deps
npm run dev                          # Dev server (proxies /ws → localhost:3000)
npm run build                        # Production build
npm run test:unit                    # Unit tests (Vitest)
npx vue-tsc --noEmit                 # Type-check without emitting
npm run lint                         # ESLint
```

> **Node path**: the system `node` at `/usr/local/bin/node` is a broken Node 14 install.
> Use Node 20 from Homebrew: `export PATH="/opt/homebrew/opt/node@20/bin:$PATH"` (or add to shell rc).

## Architecture

### Core Abstraction: the `Game` Trait

All game-specific behavior is expressed through a single Rust trait (or equivalent). Each game module implements this trait; the engine is otherwise game-agnostic. Key behaviors the trait encapsulates:

- **Deck configuration** — which cards exist (e.g., Sheepshead uses 32 cards: 7–A)
- **Trump determination** — static (Sheepshead: all Jacks + all Diamonds) or dynamic (led suit)
- **Card rank ordering** — within trump and within plain suits (varies significantly per game)
- **Player count** — valid player counts and seating rules
- **Dealing rules** — number of cards per player, kitty/blind, dealing order
- **Bidding/calling phase** — optional; Sheepshead has picking the blind; Euchre has calling trump
- **Scoring** — how tricks map to points, win conditions

### Real-time Communication

The server uses WebSockets for game state sync. The message protocol should be typed and shared (consider generating TypeScript types from Rust structs via `typeshare` or `ts-rs`). All game state mutations happen server-side; the client is a view layer only.

### Game State Machine

Each active game is a state machine with well-defined phases:
1. **Lobby** — players join, game is selected
2. **Dealing** — cards distributed
3. **Bidding** — game-specific (may be trivial/skipped)
4. **Playing** — trick-by-trick loop
5. **Scoring** — results computed and displayed

State transitions are validated server-side; clients send `Action` events (e.g., `PlayCard`, `PickBlind`) and receive `StateUpdate` broadcasts.

### Sheepshead-Specific Notes

- 5 players; one player picks the blind (2 cards) and plays against the other 4
- Trump order (high to low): ♣J, ♠J, ♥J, ♦J, A♦, 10♦, K♦, Q♦, 9♦, 8♦, 7♦
- Non-trump suit order (high to low): A, 10, K, 9, 8, 7 (Queens and Jacks are always trump)
- Points: Aces=11, 10s=10, Kings=4, Queens=3, Jacks=2 (total 120 points)
- Picker needs >60 points to win; exact 60 is a loss for the picker

## Key Design Decisions

- Game rules live entirely in `server/src/games/<name>/` — the engine never imports game-specific logic directly; it calls through the trait
- Client has no authoritative state; it reconstructs UI from server-pushed snapshots
- Card representation: use a compact integer encoding (suit × 8 + rank) for efficient serialization over WebSocket
