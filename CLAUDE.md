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
- The server is the single source of truth for turn order, legality, trick winners, scoring, and partner identity. Clients render server-provided values; they do not re-derive them.
- The wire protocol uses JSON via `serde` / TypeScript types. Cards serialize as `{ suit, rank }` strings. (The earlier plan of a compact integer encoding has not been implemented; revisit only if message size becomes a real cost.)
- Protocol types are duplicated between Rust (`server/src/engine/state.rs`) and TypeScript (`client/src/engine/types.ts`). When changing the protocol, edit both. A `ts-rs`/`typeshare`-based codegen is on the roadmap (see `docs/superpowers/plans/2026-05-17-codebase-refactor.md`, H9).

## Coding Standards

These standards apply to all changes. They exist to keep the platform extensible across multiple games without rewriting the engine, the bots, or the UI for each one.

### Universal principles

- **Single Responsibility.** A file, type, function, component, or store should have one reason to change. If you can describe its purpose with the word "and" — split it.
- **Separation of concerns by layer.** Business rules live in `server/src/games/<name>/` (game-specific) or `server/src/engine/` (game-agnostic). Session/room/network orchestration lives in `server/src/lobby/` and `server/src/ws/`. The client is presentation only.
- **No business logic on the client.** If the client needs a value (current player, trick winner, sorted hand, score breakdown), the server sends it. If you find yourself recomputing game rules in TypeScript, that is a server-side gap — close it instead of patching it client-side.
- **Open/Closed via the `Game` trait.** Adding a new game must not require changes to `engine/`, `lobby/`, `ws/`, `bot.rs`, the Pinia store, or generic components. If it does, the trait surface is incomplete — extend the trait, don't fork the consumer.
- **DRY at the level of decisions, not lines.** Three similar lines are fine; two implementations of "advance turn order" or "trump rank lookup" are not.
- **Trust internal callers; validate at boundaries.** WebSocket inputs, client-side user input, and external data go through validation. Internal trait methods can assume their inputs were already validated by the layer above.
- **Fail closed on game rules.** Server rejects illegal moves with a typed error; never silently coerce or fall back. The client surfaces the error and keeps the previous state.

### Server (Rust)

- **Game-specific code lives only in `server/src/games/<name>/`.** Anything else (engine, lobby, ws, bot helpers) calls through the `Game` trait. If a module needs `use crate::games::sheepshead`, it is in the wrong place.
- **Prefer typed enums and structs over `serde_json::Value`.** `Value` is acceptable only at the JSON-deserialization edge, before being parsed into a typed shape. It is not a substitute for typed state. (See the refactor plan, finding C3.)
- **Errors use `thiserror`-derived enums**, not `Result<_, String>`. Errors that cross the WebSocket boundary are mapped to a stable error code the client can localize.
- **One owner per piece of state.** A struct should not need two mutexes to remain consistent. If two fields must be updated together, they belong inside one lock.
- **No spawning at the request layer.** Long-lived async work (bot drivers, rejoin timers) is owned by the type that owns the state. Request handlers dispatch and reply.
- **`assert!` and `panic!` are for genuinely-impossible invariants only.** Input validation returns `Err`. Per-request panics kill a tokio task — they are bugs, not error handling.
- **Snapshot redaction goes through one helper.** Hidden hands, hidden piles, and per-seat views all flow through `GameState::redacted_for(seat, game)`; never re-implement the redaction inline.
- **Magic numbers move to `server/src/config.rs`** (or to trait methods like `Game::default_victory_goal`). No literal `5`, `24`, `30`, `1200` in handlers.

### Client (Vue 3 / TypeScript)

- **Components do one thing.** A view file (`*View.vue`) wires layout and data; presentation lives in small components under `src/components/` or `src/games/<name>/`. If a component exceeds ~150 lines or has more than 2-3 top-level template sections, decompose it.
- **No re-deriving server state.** Computeds that mirror server data must read it from the store, not recompute it from related fields. (Example: `currentTrickWinner` should be a server-provided value, not a client-side trump-rank evaluation.)
- **One store per responsibility.** Connection state, game state, session state, lobby state, and game-specific state (e.g. Sheepshead picker/partner) live in separate Pinia stores. The protocol dispatcher routes each `StateUpdate` to the relevant store.
- **Game-specific UI lives under `src/games/<name>/`.** Generic components (`CardComponent`, `HandComponent`, `TrickDisplay`) must not import game-specific logic. If `TrickDisplay` needs to know about pickers, that prop is passed in by the Sheepshead-specific parent.
- **Ephemeral UI timers** (completed-trick hold, partner-reveal toast) belong in component-local state via composables (`useTimedReveal`, etc.), not in the global store.
- **Composables are split by domain.** `useGameActions` for generic actions, `useLobbyActions` for room creation/join, `useSheepsheadActions` (under `games/sheepshead/`) for pick/pass/bury/call.
- **Game registry.** New games register a top-level component in `client/src/games/index.ts`; `GameView` selects via `<component :is="...">`. Never hardcode `<sheepshead-table>` in a generic view.
- **CSS uses tokens, not literals.** Colors, spacing, and shadows come from CSS custom properties defined once in `App.vue`. Per-component styles reference `var(--color-success)`, not `#15803d`.
- **No non-null assertions on store state in templates.** The parent guards; child components receive non-null props.

### Testing

- **Game rules are tested at the trait level.** Each `Game` impl gets unit tests for: deck size, dealing invariants, legal-play enforcement, trick-winner correctness, and scoring across all branches (regular win, schneider, leaster, partner cases, going alone).
- **Room/session logic is tested without WebSockets.** Compose `Room` (or its decomposed parts) with mpsc channels and assert on broadcasts.
- **Client tests exercise the store dispatcher**, not the components — the dispatcher is the contract surface for the protocol. Component tests cover rendering only.
- **No "I'll add tests later."** A behavior change without a regression test is a future bug.

### When in doubt

- New game-specific behavior → does it fit in `Game`? If not, extend the trait — don't special-case.
- New cross-cutting client state → is it really cross-cutting? Most "global" UI state is actually local to one view.
- Adding a `serde_json::Value` field → a typed struct is almost certainly the right call.
- Adding a new client computed that does math → check if the server should be sending it directly.
