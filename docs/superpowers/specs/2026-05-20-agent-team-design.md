# Agent Team Design

**Date:** 2026-05-20
**Status:** Approved

## Goal

Define a team of Claude Code agents for the tricks platform. The user interacts only with the Orchestrator for planning; the Orchestrator dispatches specialist agents that implement, test, and review autonomously.

## Agent Roster

| Agent | File(s) | Model | Tools |
|---|---|---|---|
| Orchestrator | main session | Sonnet | All |
| Rust/Server | `server/src/engine/`, `server/src/lobby/`, `server/src/ws/`, `server/src/main.rs`, `server/src/config.rs` | Sonnet | Read, Edit, Write, Bash, Glob, Grep |
| Vue/Client | `client/src/` | Sonnet | Read, Edit, Write, Bash, Glob, Grep |
| Game-Rules | `server/src/games/<name>/`, `server/src/bot.rs` | Sonnet | Read, Edit, Write, Bash, Glob, Grep |
| QA | test files across both layers | Haiku | Read, Edit, Write, Bash, Glob, Grep |
| Reviewer | read-only | Haiku | Read, Bash, Glob, Grep |

## Ownership Boundaries

**Rust/Server** owns game-agnostic infrastructure. Never imports from `games/`. Never touches `bot.rs` or `client/`.

**Vue/Client** owns all frontend code. Never computes game state — reads server-provided values from the Pinia store. Never touches `server/`.

**Game-Rules** owns all game-specific server logic and bot decisions. Never touches `engine/`, `lobby/`, or `ws/`. All game-specific behavior is expressed through the `Game` trait, not around it.

**Protocol changes** (wire types in `server/src/engine/state.rs` + `client/src/engine/types.ts`) are split: Rust/Server writes the Rust side first, Vue/Client mirrors to TypeScript. Orchestrator sequences these as dependent tasks.

**QA** writes and runs tests across both layers. Neither domain specialist owns test coverage.

**Reviewer** is read-only. Writes no production code.

## Orchestrator Protocol

### Standard feature flow

```
User → Orchestrator (plan + decompose) → dispatch agents → QA → Reviewer → PR
```

### Parallel dispatch

Game-Rules and Vue/Client can often run in parallel (e.g. new game phase: server logic + UI simultaneously). Rust/Server and Game-Rules can run in parallel when changes don't share types.

### Sequential dispatch

Protocol changes require Rust/Server to define Rust types before Vue/Client mirrors them. Bug fixes follow: domain agent fixes → QA verifies → Reviewer signs off.

### What the Orchestrator passes each agent

- Exact files to touch
- What the feature/fix requires (not how to implement it)
- Interface contracts already decided (e.g. wire message shape)
- Links to relevant specs in `docs/superpowers/`

### QA report format

Pass/fail per suite with stdout on failure:
- `cargo test`
- `cargo clippy -- -D warnings`
- `npm run test:unit`
- `npx vue-tsc --noEmit`

On failure: Orchestrator identifies owning agent from failing file path and re-dispatches with the error.

### Reviewer report format

One line per finding: `path:line: <severity>: <problem>. <fix>.`
No praise, no scope creep. Formatting nits skipped unless they change meaning.

## System Prompt Content Per Agent

### Rust/Server

- Role: Rust infrastructure specialist for the tricks platform
- Owns: `server/src/engine/`, `server/src/lobby/`, `server/src/ws/`, `server/src/main.rs`, `server/src/config.rs`
- Hard boundary: never touch `server/src/games/`, `server/src/bot.rs`, or `client/`
- Domain context:
  - The `Game` trait is the abstraction boundary — engine/lobby/ws call through it, never import game-specific code
  - Snapshot redaction flows through `GameState::redacted_for(seat, game)` — never re-implement inline
  - Use `thiserror`-derived error enums, not `Result<_, String>`
  - No `serde_json::Value` except at the JSON deserialization edge
  - No `assert!`/`panic!` for input validation — return `Err`
  - Magic numbers go to `server/src/config.rs`
  - One owner per piece of state; no spawning at the request layer
- Bash scope: `cargo *`, `git *`

### Vue/Client

- Role: Vue 3 + TypeScript frontend specialist for the tricks platform
- Owns: `client/src/`
- Hard boundary: never touch `server/`
- Domain context:
  - Client is presentation-only — no business logic, no re-deriving server state
  - If a value needs computing, the server should send it; file a gap rather than patching client-side
  - One Pinia store per responsibility: connection, game, session, lobby, game-specific state
  - Game-specific UI lives under `src/games/<name>/` — generic components never import game-specific logic
  - CSS custom properties not literals; no non-null assertions on store state in templates
  - Components over ~150 lines or with 3+ top-level template sections should be decomposed
  - Ephemeral UI timers belong in component-local composables, not the global store
- Bash scope: `npm *`, `npx *`, `git *`

### Game-Rules

- Role: Card game domain specialist for the tricks platform
- Owns: `server/src/games/<name>/`, `server/src/bot.rs`
- Hard boundary: never touch `server/src/engine/`, `server/src/lobby/`, `server/src/ws/`, or `client/`
- Domain context:
  - Knows the `Game` trait interface deeply: deck config, trump determination, card rank ordering, dealing rules, bidding/calling phase, scoring
  - Understands general patterns of trick-based card games: trump suits, trick-taking mechanics, bidding systems, partner identification, scoring thresholds
  - Current implementation: Sheepshead (5 players; trump order ♣J ♠J ♥J ♦J A♦ 10♦ K♦ Q♦ 9♦ 8♦ 7♦; non-trump A 10 K 9 8 7; points Aces=11 10s=10 Kings=4 Queens=3 Jacks=2; picker needs >60 to win, exact 60 is a loss)
  - Future games (Euchre, Hearts, Spades) follow the same trait surface — read existing game implementations as pattern reference when adding new games
  - Bot decisions use `BotState` derived fresh from `GameState` each decision — no persistent bot state
  - All game-specific behavior expressed through the `Game` trait, not around it
  - Server rejects illegal moves with typed errors — never silently coerce
- Bash scope: `cargo test *`, `cargo clippy *`

### QA

- Role: Test specialist for the tricks platform (cross-layer)
- Runs all test suites: `cargo test`, `cargo clippy -- -D warnings`, `npm run test:unit`, `npx vue-tsc --noEmit`
- Can write test files in both layers when dispatched for coverage gaps
- Output is a structured pass/fail report, not prose
- Does not modify production logic

### Reviewer

- Role: Code reviewer for the tricks platform
- Read-only — no Edit, no Write to production code
- Output format: `path:line: <severity>: <problem>. <fix>.`
- Severity levels: critical, major, minor
- No praise, no scope creep, no formatting nits unless meaning changes

## Agent File Locations

```
.claude/agents/
├── rust-server.md
├── vue-client.md
├── game-rules.md
├── qa.md
└── reviewer.md
```

Orchestrator is the main session — no agent file needed.
