# CLAUDE.md

Guidance for Claude Code in this repo.

## Project

Modular trick-taking card game platform. Rust backend + Vue 3 frontend. Games: Sheepshead now, Euchre/Hearts/Spades later. Shared engine, pluggable rules per game.

## Repo

```
tricks/
├── server/          # Rust (Axum + WebSockets)
│   └── src/
│       ├── main.rs
│       ├── engine/  # game-agnostic core
│       ├── games/   # per-game rules (sheepshead/, euchre/, …)
│       ├── lobby/   # room/session management
│       └── ws/      # WebSocket handlers
└── client/          # Vue 3 + Vite
    └── src/
        ├── games/   # per-game UI
        └── engine/  # shared state logic (mirrors server)
```

## Commands

```bash
# Rust
cd server
cargo build
cargo run
cargo test
cargo test engine::
cargo test -- --nocapture
cargo clippy -- -D warnings
cargo fmt

# Vue
cd client
npm install
npm run dev          # proxies /ws → localhost:3000
npm run build
npm run test:unit
npx vue-tsc --noEmit
npm run lint
```

> Node: `/usr/local/bin/node` broken (Node 14). Use `export PATH="/opt/homebrew/opt/node@20/bin:$PATH"`.

## Rust MCP Server — USE THIS

Prefer `mcp__rust-code-mcp__*` tools over grep/Bash for all Rust code exploration:

| Task | Tool |
|------|------|
| Find symbol definition | `find_definition` |
| Find all usages | `find_references` |
| Keyword search | `search` |
| Trace call graph | `get_call_graph` |
| Check imports | `get_dependencies` |
| Code metrics | `analyze_complexity` |
| Read file | `read_file_content` |
| Semantic search | `get_similar_code` |

Run `index_codebase` if results seem stale. Run `clear_cache` if you see MetadataCache errors.

## Architecture

### `Game` Trait

All game behavior through one trait. Engine never imports game-specific code — calls through trait only.

Trait encapsulates: deck config, trump rules, card rank order, player count, dealing, bidding/picking phase, scoring.

### State Machine

Phases: Lobby → Dealing → Bidding → Playing → Scoring. Server validates all transitions. Clients send `Action` (`PlayCard`, `PickBlind`); server broadcasts `StateUpdate`.

### Sheepshead

- 5 players; picker takes blind (2 cards), plays vs other 4
- Trump (high→low): ♣J ♠J ♥J ♦J A♦ 10♦ K♦ Q♦ 9♦ 8♦ 7♦
- Plain suit (high→low): A 10 K 9 8 7 (Q and J always trump)
- Points: A=11 10=10 K=4 Q=3 J=2 (total 120). Picker needs >60; 60 exact = loss

### Wire Protocol

JSON via `serde`/TS types. Cards: `{ suit, rank }`. Types duplicated: `server/src/engine/state.rs` ↔ `client/src/engine/types.ts` — edit both when protocol changes.

## Design Rules

- Game rules only in `server/src/games/<name>/` — engine calls through trait
- Server is truth: turn order, legality, trick winners, scores, partner identity
- Client renders server values; never re-derives them
- No `use crate::games::sheepshead` outside `games/sheepshead/`

## Coding Standards

### Universal

- Single responsibility — if purpose needs "and", split it
- No business logic on client — server sends computed values; if client recomputes, that's a server gap
- `Game` trait is open/closed — new game needs no changes to `engine/`, `lobby/`, `ws/`, `bot.rs`, store
- DRY on decisions — duplicate trump rank lookup = bug waiting to happen
- Validate at boundaries only — WS input, user input, external APIs
- Fail closed — reject illegal moves with typed error; never silently coerce

### Rust

- Typed enums/structs over `serde_json::Value` — `Value` only at deser edge
- Errors: `thiserror`-derived enums, not `Result<_, String>`
- One owner per state — no two mutexes for consistent struct
- No spawning at request layer — async work owned by state type
- `assert!`/`panic!` for impossible invariants only — input validation returns `Err`
- Redaction through `GameState::redacted_for(seat, game)` only
- Magic numbers → `server/src/config.rs` or trait methods

### Vue/TS

- View (`*View.vue`) wires layout; presentation in small components. >150 lines or >2-3 sections → decompose
- No re-deriving server state in computeds — read from store
- One Pinia store per responsibility: connection, game, session, lobby, game-specific
- Game-specific UI under `src/games/<name>/` — generic components take no game-specific imports
- Ephemeral timers (trick-hold, toast) → composables, not global store
- CSS tokens (`var(--color-success)`), not literals (`#15803d`)
- No non-null assertions on store state in templates — parent guards

### Testing

- `Game` impl unit tests: deck size, dealing invariants, legal-play, trick-winner, scoring (all branches)
- Room/session tests: mpsc channels, no WebSockets
- Client tests: store dispatcher (protocol surface), not components
- No "add tests later" — behavior change without regression test = future bug

### Doubt rules

- New game behavior → fit in `Game`? No → extend trait, don't special-case
- New "global" client state → probably local to one view
- `serde_json::Value` field → use typed struct
- Client computed doing math → server should send it
