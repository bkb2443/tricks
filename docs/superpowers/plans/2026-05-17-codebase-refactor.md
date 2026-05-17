# Codebase Refactor Plan

> Adversarial review of the trick-based card game platform, focused on architecture, SOLID adherence, complexity, code quality, and proper separation of concerns between client (view) and server (authoritative business logic).

**Scope:** Both `server/` (Rust + Axum) and `client/` (Vue 3 + Pinia). Per CLAUDE.md, the engine must remain game-agnostic and the client must remain a pure view layer over server snapshots.

**Format:** Each finding has a **Severity** (Critical / High / Medium / Low), a **Location** with file:line references, a **Why it matters** explanation, and a **Refactor** prescription. The end of the doc has a sequenced rollout.

---

## Executive Summary

The bones are sound: the `Game` trait, room-based session isolation, broadcast/private channel split, and the snapshot-with-redacted-hands pattern are all good calls. But the code has drifted in three measurable ways:

1. **Business logic has leaked client-side.** The Pinia store re-implements `current_player` advancement on `card_played`, recomputes the trick winner for highlighting, and resorts hands using a copy of the Sheepshead trump rules. CLAUDE.md says "client has no authoritative state; it reconstructs UI from server-pushed snapshots" — today it does not.
2. **SOLID violations cluster around two god objects.** `lobby/room.rs` (676 lines) and `client/src/stores/game.ts` (230 lines, but holding 14 refs + 11 derived + the full update reducer) each own seat state, chat, scoring, timers, transitions, and broadcast plumbing. They will fight every new game and every new feature.
3. **The `Game` trait abstraction has holes the size of a category mistake.** `GameState.meta: serde_json::Value` and `extra_piles: Vec<(String, Vec<Card>)>` are stringly-typed escape hatches; `bot.rs` is a free module that imports `Sheepshead` directly and admits as much in a `FIXME`; the default `apply_play` is duplicated as a free function (`apply_play_generic`) — i.e., the abstraction is being routed around rather than extended.

Aside from the structural items, there is duplication (the default `apply_play` body appears twice, scoring helpers are inlined twice in `score_game`), magic numbers in both halves, and protocol/type duplication between Rust and TypeScript that the original plan flagged but never resolved.

The plan below sequences fixes by blast radius. **Phase 1 (quick wins / pure cleanups)** removes duplication without changing behavior. **Phase 2 (architectural)** locks SRP back down and moves the remaining client logic to the server. **Phase 3 (abstraction)** restores the `Game` trait so the second game (Euchre) can plug in without touching `bot.rs`, the room, or the store.

---

## Critical Findings

### C1. Duplicated `apply_play` body — engine/game.rs

- **Severity:** Critical (correctness time bomb)
- **Location:** `server/src/engine/game.rs:54-127` (`apply_play_generic` free fn) and `server/src/engine/game.rs:190-263` (default trait method `Game::apply_play`)
- **Why it matters:** These are line-for-line the same 70-line block. The free function exists so `Sheepshead::apply_play` (rules.rs:156-179) can run partner-revelation logic before delegating to it; the default trait method does the same work. Any fix to one (e.g., a turn-order bug, scoring edge case) will silently skip the other. This is the most dangerous duplication in the codebase because the failure mode is "the second game's rules silently differ from Sheepshead."
- **Refactor:** Delete the inline body in `Game::apply_play`'s default; have the default delegate to `apply_play_generic(self, state, seat, card)`. Keep the free function as the single source of truth. Sheepshead override stays unchanged.

### C2. Client computes server state — stores/game.ts

- **Severity:** Critical (architectural rule violation)
- **Location:**
  - `client/src/stores/game.ts:97-125` — `card_played` handler advances `current_player` locally: `s.current_player = (trick.led_by + trick.plays.length) % s.player_count`
  - `client/src/stores/game.ts:60-65` — `currentTrickWinner` recomputes trick winner via `trickWinnerIndex` (client-side trump rules)
  - `client/src/engine/sort.ts` — full Sheepshead trump/plain-rank table re-implemented in TS for sorting hands and computing winners
- **Why it matters:** CLAUDE.md is explicit: "All game state mutations happen server-side; the client is a view layer only." Today the client is duplicating turn-order math (which the server already knows but doesn't send) and trump rules (which belong to the `Game` impl). When Euchre lands, every one of these client paths will have to fork on `game_name` or — more likely — silently break. The test suite at `stores/game.test.ts:98-110, 200-227` even documents these as "BUG FIX" cases — they're patching symptoms of the wrong layer doing the work.
- **Refactor:**
  - **Server:** Make `CardPlayed` carry `current_player: usize` (mirror what `BidPlaced` already does at state.rs:164). Make `TrickComplete` carry the winning play index so the client doesn't recompute it. Optionally include a `sorted_hand: Vec<Card>` on `Snapshot` / `HandUpdated` so clients render in display order without trump knowledge.
  - **Client:** Delete `trickWinnerIndex` from `engine/sort.ts`; drive `currentTrickWinner` from a server-provided index. Delete the trump-rank/plain-rank tables — sorting becomes a deterministic order driven by either a server-sent `display_index` or by a thin Sheepshead-specific sorter that imports a tiny `SHEEPSHEAD_SORT_RANK` table (one place, not two).
  - The `card_played` handler should not advance `current_player` at all once the server includes it on the event.

### C3. Stringly-typed game metadata — engine/state.rs

- **Severity:** Critical (type safety + abstraction leak)
- **Location:** `server/src/engine/state.rs:43` (`meta: serde_json::Value`) and every access in `games/sheepshead/rules.rs` (e.g., `state.meta["picker"].as_u64().unwrap_or(0) as usize`, `state.meta["sub_phase"].as_str().unwrap_or("picking")`, `state.meta["going_alone"].as_bool().unwrap_or(false)` — repeated dozens of times in rules.rs, room.rs, and bot.rs)
- **Why it matters:** Every read is a runtime cast with a silent fallback. A typo in `"sub_phsae"` is a compile-time pass and a runtime "always picking" bug. The bot module's `state.meta["called_suit"].as_str()` + `serde_json::from_str(&format!("\"{}\"", s))` round-trip is a smell on a smell. This also forces the client to treat `meta` as `Record<string, unknown>` (types.ts:37), pushing the same fragility forward.
- **Refactor:** Give `Game` an associated type:
  ```rust
  pub trait Game: Send + Sync {
      type Meta: Serialize + DeserializeOwned + Default + Clone + Send + Sync;
      // ...
  }
  ```
  Replace `meta: serde_json::Value` with a typed enum-or-trait-object that each game implements. Concretely: introduce `pub enum GameMeta { Sheepshead(SheepsheadMeta), Euchre(EuchreMeta), Empty }` and migrate `room.rs` + `rules.rs` to read typed fields. The wire format keeps `serde(tag = "game")`-style JSON tagging, so clients see a discriminated union (`{ game: 'sheepshead', picker, sub_phase, ... }`) instead of `Record<string, unknown>`. If the cost of a trait associated type is too high, a typed wrapper struct that owns its JSON and exposes `picker() -> Option<usize>`, `sub_phase() -> SubPhase`, etc., is still a strict improvement over raw `serde_json::Value` indexing.

### C4. Bot module hardcoded to Sheepshead — bot.rs

- **Severity:** Critical (Open/Closed violation; trait abstraction bypass)
- **Location:**
  - `server/src/bot.rs:126` — `// FIXME: bid logic is hardcoded to Sheepshead; needs game: &dyn Game param when second game is added`
  - `server/src/bot.rs:5` — `use crate::games::sheepshead::Sheepshead;` directly imported in what is supposed to be a generic module
  - `server/src/bot.rs:127-145` — `bid_action` instantiates `&Sheepshead` to pass to `should_pick` etc.
  - `server/src/bot.rs:228-274` — `choose_call` uses `Sheepshead.trump_rank` directly
- **Why it matters:** Adding Euchre means an `if game_name == "sheepshead" { ... } else if "euchre" { ... }` ladder. That's exactly what the `Game` trait was supposed to prevent. The trait abstraction is degraded into "we have a Game trait but the AI doesn't use it."
- **Refactor:** Introduce a `BotStrategy` trait owned by each game module:
  ```rust
  pub trait BotStrategy: Send + Sync {
      fn bid_action(&self, state: &GameState, seat: usize) -> serde_json::Value;
      fn play_card(&self, state: &GameState, seat: usize) -> Option<Card>;
  }
  ```
  Have `Game` return its `&dyn BotStrategy`, or store the bot strategy alongside the rules impl. Move the contents of `bot.rs` to `games/sheepshead/bot.rs`. The shared bookkeeping (`BotState`, `build_bot_state`, `current_winner`, `min_winning_trump`) becomes a small `engine/bot_helpers.rs` that operates strictly through the `Game` trait — it already nearly does, except for `point_value()` which duplicates `game.card_points()`.

### C5. `Room` is a god object — lobby/room.rs

- **Severity:** Critical (SRP — 676 lines, 8 responsibilities, 6 mutexes)
- **Location:** `server/src/lobby/room.rs` (entire file)
- **Why it matters:** `Room` owns: seat lifecycle, name uniqueness, host election, broadcast plumbing, private channel routing, lobby chat with history, system chat, disconnect tracking with 30s timers, force-bot, extend-rejoin, game start, bot-driving async loop, hand transitions, session scoring, victory detection, snapshot redaction. Each is a reason to change. The two independent mutexes (`seats` and `state`) are an ordering hazard already — e.g., `start_game` locks `seats`, then `start_game_inner` locks `state`, while `bots_running` (an AtomicBool) tries to paper over the resulting race.
- **Refactor:** Split into focused types that each own one concern, composed into `Room`:
  ```
  Room (orchestrator, ~150 lines)
   ├── SeatManager       — Vec<SeatState>, join/leave/rejoin, name uniqueness
   ├── LobbyChat         — chat history, rate-limited broadcast, system messages
   ├── RejoinTracker     — 30s deadlines, force_bot, extend_rejoin, expiry timers
   ├── GameSession       — current GameState, deal-next-hand, redacted snapshots
   ├── SessionScorer     — running totals, victory_goal, max_hands, session_winner
   ├── BotDriver         — drive_bots async task (one per room, not respawned per action)
   └── Broadcaster       — broadcast_tx + send_private (collapses the broadcast/mpsc duality)
  ```
  Each gets its own module under `lobby/`. The handler ends up calling `room.seats().join_lobby(...)`, `room.session().apply_bid(...)`, etc. Mutex hierarchy becomes obvious (and lockable in one order); tests get tractable per-component.

---

## High-Priority Findings

### H1. WebSocket handler has business logic — ws/handler.rs

- **Severity:** High (separation of concerns)
- **Location:**
  - `server/src/ws/handler.rs:117-122` — JoinRoom auto-creates a room if `room_id` not found; defaults `victory_goal=24`
  - `server/src/ws/handler.rs:144` — `lobby.create_room(game, 5, 24)` — hardcodes player count and victory goal
  - `server/src/ws/handler.rs:127-131` — fill_bots branch handles `fill_bots() + start_game()`
  - `server/src/ws/handler.rs:188-192, 200-204` — handler spawns `drive_bots` after every Bid/PlayCard
- **Why it matters:** The handler is the protocol boundary. It should deserialize, dispatch, and reply. Knowing the default victory goal, the player count for Sheepshead, the "if fill_bots then start" recipe, and re-driving bots after each action belong in the lobby/room layer.
- **Refactor:**
  - Move `victory_goal=24` and `player_count=5` into the `Game` trait (`fn default_victory_goal()`, `fn default_player_count()` via `valid_player_counts()[0]`).
  - Replace the `spawn(drive_bots)` calls with the room owning a single persistent bot task notified by a `tokio::sync::Notify` whenever state changes. The handler never spawns; it just calls `room.notify_action_applied()`.
  - Move the "JoinRoom auto-creates" branch into `lobby.find_or_create(...)`.

### H2. `useGameStore` is doing too much — stores/game.ts

- **Severity:** High (SRP on the client)
- **Location:** `client/src/stores/game.ts` (entire file)
- **Why it matters:** 14 top-level refs, 11 derived computeds, a 100-line `handleUpdate` switch, a setTimeout for trick display, another for partner reveal, plus reset(). Holds connection-adjacent state (`roomCode`, `seat`), game state (`gameState`, `myHand`), session state (`sessionScores`, `sessionWinner`), lobby state (`seats`, `lobbyChat`), queue state (`queueStatus`), and ephemeral UI state (`completedTrick`, `partnerRevealedSeat`). A new feature touches it; a test must mount it; a Sheepshead-specific change leaks across all consumers.
- **Refactor:** Split into focused stores:
  - `useConnectionStore` — `roomId`, `seat`, `roomCode`, `error`
  - `useGameStore` — `gameState`, `myHand`, `phase`, `isMyTurn`, `currentTrickWinner`
  - `useSessionStore` — `sessionScores`, `sessionWinner`
  - `useLobbyStore` — `seats`, `lobbyChat`, `queueStatus`, `isLobby`
  - `useSheepsheadStore` (game-specific) — `picker`, `isPicker`, `isCallingPhase`, `callableSuits`, `calledSuit`, `partnerRevealedSeat`
  - Move ephemeral UI state (`completedTrick` 1500ms hold, `partnerRevealedSeat` 2000ms hold) into local component state with composables (`useCompletedTrick()`, `usePartnerReveal()`), since they're presentation timing, not data.
  - The `StateUpdate` dispatch becomes a thin router that calls into each store's handler.

### H3. `GameTable.vue` is 391 lines, 7 responsibilities

- **Severity:** High (SRP on the client)
- **Location:** `client/src/games/sheepshead/GameTable.vue`
- **Why it matters:** One file owns header (phase/dealer/trick counter), seat rail, trick display orchestration, bidding panel mounting, my-hand section with role/turn badges, session scoreboard, hand-complete view, session-over view, phase toast, partner toast, and trick-history disclosure. Iterating on the scoreboard requires re-reasoning about the toast layer.
- **Refactor:** Decompose into:
  ```
  GameTable.vue (layout-only, ~80 lines)
   ├── <GameHeader>         (phase badge, dealer, trick counter)
   ├── <SeatRail>           (other-player seats with current-player highlight)
   ├── <TrickDisplay>       (already a component — good)
   ├── <BiddingPanel>       (already a component — but see H4)
   ├── <MyHandPanel>        (hand + role badges + turn glow)
   ├── <SessionScoreboard>  (progress bars to victory goal)
   ├── <HandResult>         (between-hand scoring summary)
   ├── <SessionResult>      (final winner screen)
   ├── <CompletedTrickHistory> (expandable list)
   └── <PhaseToast> / <PartnerToast>  (or fold into <ToastStack>)
  ```
  Each consumes its own store(s) — no prop-drilling through `GameTable`.

### H4. `BiddingPanel.vue` mixes three sub-phases

- **Severity:** High (SRP)
- **Location:** `client/src/games/sheepshead/BiddingPanel.vue` (153 lines)
- **Why it matters:** Picking, burying, and calling are different game mechanics with different UIs, different validation, and different "whose turn is it" rules. They share only a wrapper div. Three `<template v-else-if>` branches each gate a different control set, and `burySelection` ref leaks across sub-phases.
- **Refactor:**
  ```
  BiddingPanel.vue (sub-phase switch only)
   ├── <PickingPanel>   (pick / pass buttons + waiting state)
   ├── <BuryingPanel>   (own state: burySelection, validation, submit)
   └── <CallingPanel>   (callable suits + go-alone)
  ```
  Each consumes the Sheepshead store directly.

### H5. Lock granularity and ordering — Room

- **Severity:** High (correctness)
- **Location:** `server/src/lobby/room.rs:81-89` — `seats: Mutex<Vec<SeatState>>`, `state: Mutex<Option<GameState>>`, `session_scores: Mutex<Vec<i32>>`, `chat_history: Mutex<VecDeque<...>>`, `max_hands: Mutex<Option<u32>>`, `hands_played: Mutex<u32>` — six independent mutexes
- **Why it matters:** `start_game` (`room.rs:265-276`) locks `seats`, drops, then `start_game_inner` locks `state`. `play_card` (`room.rs:454-514`) locks `state`, drops, then locks `session_scores`, then `hands_played`, then `max_hands`. Any code path that needs two together at once invites a deadlock the moment someone reorders. `drive_bots` (`room.rs:529-586`) re-locks `state` 5+ times in one iteration, each acquire/release racing with player input.
- **Refactor:**
  - Consolidate room state into one `Mutex<RoomInner>` (or `RwLock<RoomInner>`), with `RoomInner` holding all fields except the broadcast channel. One lock to take, one to drop.
  - For the bot driver loop, hold the lock for one logical "decide + apply" transaction rather than reacquiring per field read.
  - Alternative: actor pattern — the room runs an async task that owns the state and processes a `RoomCommand` mpsc; no mutexes at all. This is heavier to introduce but eliminates the entire class of bugs.

### H6. Default victory goal & player count are magic numbers

- **Severity:** Medium-High
- **Location:** `server/src/ws/handler.rs:119, 121, 144`, `server/src/lobby/matchmaker.rs:80` — all hardcode `lobby.create_room(<game>, 5, 24)`. Client `client/src/games/sheepshead/GameTable.vue:34` hardcodes `VICTORY_GOAL = 24`.
- **Why it matters:** Adding Hearts/Euchre/Spades means finding every `24` and `5` literal. The client constant will drift from the server's the first time someone changes one.
- **Refactor:** Add `fn default_victory_goal(&self) -> i32` and `fn default_player_count(&self) -> usize` to `Game`. Server reads from the trait. Server includes `victory_goal` in the `Snapshot` payload (or in a new `RoomConfig` message sent on join). Client reads from the snapshot, never from a constant.

### H7. Bot driver spawn-and-forget — handler.rs and room.rs

- **Severity:** High (correctness, resource leaks)
- **Location:** `server/src/ws/handler.rs:189-191, 201-203` — `tokio::spawn(async move { room_arc.drive_bots().await })` after every successful Bid/PlayCard. `server/src/lobby/room.rs:529-532` — `bots_running` AtomicBool guards reentry.
- **Why it matters:** Every player action spawns a task that immediately checks an atomic and returns. The AtomicBool guard is correct only because the loop is one-shot per call; the design is "spawn-then-noop" which is a code smell. Worse, the lobby `start_game` does `tokio::spawn(drive_bots)` at room.rs:273 too — but if a player action lands before the spawn happens, the spawn at handler.rs:190 races with it.
- **Refactor:** Room owns exactly one long-lived bot task started in `start_game`. The handler calls `room.notify_action_applied()` which `notify_one`s a `tokio::sync::Notify`. The bot loop `await`s the notify, then drains pending decisions. No spawning at the request layer.

### H8. Snapshot-redaction logic duplicated

- **Severity:** Medium-High (duplication)
- **Location:** `server/src/lobby/room.rs:381-388` (rejoin), `server/src/lobby/room.rs:596-602` (start_next_hand) — both clone state, clear other hands, clear extra_piles.
- **Why it matters:** Redaction rules are scattered. When game-specific rules decide some piles ARE visible (e.g., Sheepshead post-pick reveal of the blind to all, if that became a feature), it'll be missed in one of the two places.
- **Refactor:** `GameState::redacted_for(&self, seat: usize, game: &dyn Game) -> GameState`. The `Game` trait gets a hook `fn visible_extra_piles(&self, state: &GameState, seat: usize) -> Vec<&str>` so games can opt extra piles back in. Both call sites become one line.

### H9. Protocol types duplicated between Rust and TypeScript

- **Severity:** High (drift surface area)
- **Location:** `server/src/engine/state.rs` (ClientMessage, StateUpdate, GameState, SeatInfo, GamePhase) vs `client/src/engine/types.ts` (same shapes restated)
- **Why it matters:** Every protocol change requires editing two files. CLAUDE.md flags this as TODO ("consider generating TypeScript types from Rust structs via `typeshare` or `ts-rs`") — it should not remain a TODO. Today the duplication is small; the day Sheepshead picks up calling-from-the-blind variants it'll be the source of every "client and server disagree about message shape" bug.
- **Refactor:** Add `ts-rs` (build-time) or `typeshare` (CLI) to the Cargo deps. Annotate the Rust structs/enums with `#[derive(TS)]` (ts-rs) or `#[typeshare]`. Generate `client/src/engine/protocol.generated.ts` from `cargo run --bin generate-types` or a `build.rs` task. Replace hand-written types.ts with a re-export plus client-only helpers (Card constructors etc.).

### H10. `extra_piles: Vec<(String, Vec<Card>)>` is stringly-typed

- **Severity:** Medium-High
- **Location:** `server/src/engine/state.rs:37`, used in `engine/dealer.rs:13`, `games/sheepshead/rules.rs:97-98, 398-403`
- **Why it matters:** "blind" is a magic string. The Sheepshead pick code does `iter().position(|(name, _)| name == "blind")` — fragile and untyped.
- **Refactor:** Enum-keyed map (`HashMap<PileKind, Vec<Card>>` with `PileKind` per-game), or extra piles become typed associations stored in `Game::Meta`. Since extra piles are inherently game-specific, the cleanest fix is to fold them into the typed meta from C3.

---

## Medium-Priority Findings

### M1. Duplicated scoring logic — sheepshead/rules.rs

- **Location:** `server/src/games/sheepshead/rules.rs:288-356` — the "going alone" and "called partner" branches each compute identical schneider gates with copy-pasted match arms.
- **Refactor:** Extract `fn schneider_score(picker_share: i32, defender_share: i32, mode: ScoreMode) -> ScoreDistribution`. Both branches become one call with `ScoreMode::Alone` or `ScoreMode::Partner`.

### M2. `apply_bid` broadcasts via opaque JSON — bid_result

- **Location:** `server/src/engine/game.rs:30-31` (`BidResult.broadcast_payload: Option<serde_json::Value>`)
- **Why it matters:** Re-introduces the stringly-typed escape hatch (C3) into the protocol layer. The room blindly forwards the JSON, so consumers (incl. the bot) parse a payload whose shape only the game module knows.
- **Refactor:** Define an enum `BidBroadcast { Raw, SubPhaseUpdate { sub_phase: SubPhase, callable_suits: Vec<SuitName> }, ... }` per game (or globally). The room can pattern-match instead of forwarding opaque blobs.

### M3. `Sheepshead::deal` panics on invalid input

- **Location:** `server/src/games/sheepshead/rules.rs:76-77` — `assert_eq!(player_count, 5, ...)`, `assert_eq!(shuffled_deck.len(), 32, ...)`
- **Why it matters:** A misconfigured room (or a future bug that lets a 4-player Sheepshead room start) crashes the tokio task with a panic instead of erroring out cleanly. `Game::deal` currently returns `DealResult`; making it `Result<DealResult, GameError>` lets the room surface the error to the client.
- **Refactor:** Change `Game::deal -> Result<DealResult, GameError>`. Validation moves from runtime asserts to the trait contract.

### M4. `Result<_, String>` everywhere — no typed errors

- **Location:** All `apply_bid`, `apply_play`, `handle_lobby_chat`, `force_bot`, `extend_rejoin`, etc. (See `engine/game.rs:177-181`, `lobby/room.rs:226-247, 398-427`)
- **Why it matters:** `thiserror = "2"` is already a dependency (Cargo.toml:17) but unused. String errors mean callers can't pattern-match (e.g., distinguish "not your turn" from "card not in hand" from "wrong sub-phase") and the client can't localize messages.
- **Refactor:** Introduce `engine::GameError`, `engine::ProtocolError`, `lobby::RoomError` enums with `thiserror`. Map at the protocol boundary to a typed client-facing error (e.g., `{ code: 'not_your_turn', message: '...' }`) so the client can show different UI for different errors.

### M5. `#![allow(dead_code)]` with stale TODOs

- **Location:** `server/src/lobby/room.rs:1-3` and `server/src/lobby/matchmaker.rs:1-2` reference "Task 6" of a plan whose tasks are long done.
- **Refactor:** Remove the file-level allow. Audit the `#[allow(dead_code)]` markers on individual items — many (e.g., `SeatState::is_human`, `SeatState::ws_id` on disconnected variants) are actually used; the rest should be deleted.

### M6. Bot's `point_value` duplicates `Game::card_points`

- **Location:** `server/src/bot.rs:53-62`
- **Refactor:** Delete `point_value`; call `game.card_points(card)` (already in scope as `&dyn Game`).

### M7. Trump rules duplicated client-side — engine/sort.ts

- **Location:** `client/src/engine/sort.ts:7-32` reproduces the Sheepshead trump rank table
- **Why it matters:** Already covered under C2 — listed here for the second-game audit. Even if the server starts sending pre-sorted hands, this file shouldn't grow Euchre's dynamic-trump rules.
- **Refactor:** Either delete (server pre-sorts and tags winning play) or move the table into `client/src/games/sheepshead/sort.ts` so it's clearly the Sheepshead-only fallback.

### M8. `useGame.ts` mixes generic and Sheepshead-specific actions

- **Location:** `client/src/composables/useGame.ts:33-51` — `pick`, `pass`, `bury`, `callAce`, `goAlone` are Sheepshead-specific in a file named "useGame"
- **Refactor:** Split into:
  - `useGameActions()` — `playCard`, `startGame`, `sendLobbyChat`, `forceBot`, `extendRejoin`, queue actions
  - `useLobbyActions()` — `createRoom`, `joinRoom`, `joinWithCode`, `createPrivateRoom`, `createSoloRoom`
  - `useSheepsheadActions()` (in `games/sheepshead/`) — `pick`, `pass`, `bury`, `callAce`, `goAlone`

### M9. Magic timeouts scattered across the codebase

- **Location:** `room.rs:321` (30s rejoin), `room.rs:527` (1200ms bot delay), `matchmaker.rs:13` (60s queue timeout), `matchmaker.rs:14` (8 max hands), `stores/game.ts:140` (1500ms completed-trick hold), `stores/game.ts:177` (2000ms partner reveal), `GameTable.vue:64` (1500ms phase toast)
- **Refactor:** Centralize: server in `server/src/config.rs` (loadable from env for tests/prod), client in `client/src/config.ts`.

### M10. Hardcoded `<sheepshead-table>` in GameView

- **Location:** `client/src/views/GameView.vue:16`
- **Why it matters:** Plug-and-play games require switching on `game_name`, but there's no registry pattern yet — adding Euchre means editing GameView.
- **Refactor:** A `games/index.ts` registry: `{ sheepshead: SheepsheadTable, euchre: EuchreTable }` keyed by name. GameView uses `<component :is="registry[gameName]" />`.

---

## Low-Priority Findings

### L1. WebSocket: no reconnect logic — socket.ts

- **Location:** `client/src/engine/socket.ts:21-48` — single `new WebSocket(...)`, no retry on close
- **Refactor:** Exponential backoff reconnect (already useful for rejoin flow, since the server has 30s rejoin support but the client gives up immediately).

### L2. Per-component CSS with hardcoded colors

- **Location:** Across all `.vue` files — `#15803d`, `#7c3aed`, `#22c55e`, `#9ca3af`, etc.
- **Refactor:** CSS custom properties on `:root` in App.vue (`--color-success`, `--color-picker`, `--color-bg-panel`...). Use semantic names, not raw hex.

### L3. Accessibility gaps

- **Location:**
  - `CardComponent.vue:42-43` — `role="button"` only when `selectable`, but no keyboard handler
  - `GameTable.vue:84-90` — toasts lack `aria-live` regions
  - No focus management between phases
- **Refactor:** Add `@keydown.enter`/`@keydown.space` to card click handlers; `role="status" aria-live="polite"` to toasts; make the bidding panel announce its own state to screen readers.

### L4. `localStorage.getItem('guestName')` read twice — HomeView

- **Location:** `client/src/views/HomeView.vue:10, 48`
- **Refactor:** Read once in `<script setup>`; delete the `onMounted` re-read.

### L5. `gameStarted` computed name is misleading

- **Location:** `client/src/stores/game.ts:42` — `gameStarted = gameState.value !== null`. True any time we're in a room, including lobby phase.
- **Refactor:** Rename to `hasGameState` or change semantics to `gameState.value !== null && gameState.value.phase !== 'lobby'`.

### L6. `<sheepshead-table>` non-null-assertion

- **Location:** `client/src/games/sheepshead/GameTable.vue:17` — `store.gameState!`
- **Refactor:** The router/view layer should not mount this component until the snapshot has arrived. Make the guard the parent's job, drop the assertion.

### L7. Tests for `room.rs` are thin

- **Location:** `server/src/lobby/room.rs:631-676` — three tests covering only join and chat validation
- **Refactor:** Add tests for: disconnect/rejoin happy + expiry paths, force_bot permission, extend_rejoin twice (should fail second time), hand_complete updates session_scores, victory triggers SessionOver, snapshot redaction.

### L8. CLAUDE.md says compact integer card encoding; code uses JSON strings

- **Location:** `CLAUDE.md:98` — "compact integer encoding (suit × 8 + rank)" vs `engine/card.rs:32-36` JSON struct
- **Refactor:** Either implement the encoding (worth it if we hit per-message size pressure) or update CLAUDE.md to match reality. Recommend: update the doc — JSON is fine at this scale and `typeshare`-friendly.

### L9. `Cargo.toml` edition = "2024"

- **Location:** `server/Cargo.toml:4` — fine on current toolchains but worth pinning a minimum Rust version in CI.
- **Refactor:** Add `rust-version = "1.85"` (or whatever stabilized the let-chains used at `rules.rs:166-167`).

### L10. `console.info/warn/error` direct calls in production code

- **Location:** `client/src/engine/socket.ts:28, 33, 36, 45, 54`
- **Refactor:** Tiny logger wrapper with level gating so production builds can suppress.

---

## Sequenced Rollout

This is the order I'd execute. Each phase is independently shippable.

### Phase 1 — Quick wins (no behavioral change)

Pure cleanups. Each is small, each removes a maintenance hazard, none changes the wire protocol.

- [ ] **P1.1** Collapse `Game::apply_play` default into `apply_play_generic` (C1)
- [ ] **P1.2** Delete `bot.rs::point_value` in favor of `game.card_points()` (M6)
- [ ] **P1.3** Remove file-level `#![allow(dead_code)]` from `room.rs`/`matchmaker.rs`; clean per-item allows (M5)
- [ ] **P1.4** Factor Sheepshead scoring helpers (M1)
- [ ] **P1.5** Fix `localStorage` double-read (L4); rename `gameStarted` (L5)
- [ ] **P1.6** Tighten `Sheepshead::deal` from `assert!` to `Result` (M3)
- [ ] **P1.7** Introduce `thiserror` error enums in engine + room (M4) — wire format keeps a string message field for now

### Phase 2 — Centralize business logic server-side (C2)

Goal: make the client a true view. After this, switching to Euchre needs no client logic.

- [ ] **P2.1** Add `current_player` to `CardPlayed` server message; stop the client from advancing it (C2)
- [ ] **P2.2** Add `winning_play_index` to `TrickComplete`; remove `currentTrickWinner` recompute (C2)
- [ ] **P2.3** Either pre-sort hands on the server (preferred) or move trump-rank table out of generic `engine/sort.ts` into `games/sheepshead/sort.ts` (C2, M7)
- [ ] **P2.4** Strengthen `apply_bid` `broadcast_payload` to typed variants (M2)
- [ ] **P2.5** Add `victory_goal` + `player_count` defaults to the `Game` trait, ship them in the snapshot, delete client `VICTORY_GOAL = 24` (H6)
- [ ] **P2.6** Generate TS protocol types from Rust (`ts-rs` or `typeshare`) (H9)

### Phase 3 — Restore the abstraction

Goal: second game (Euchre) plugs in via the trait without touching room/bot/store.

- [ ] **P3.1** Introduce `Game::Meta` associated type (or typed wrapper) replacing `serde_json::Value` (C3)
- [ ] **P3.2** Fold `extra_piles` into typed meta or enum-keyed map (H10)
- [ ] **P3.3** Introduce `BotStrategy` trait; move bot logic to `games/sheepshead/bot.rs` (C4)
- [ ] **P3.4** Implement `Game::redact_for(seat) -> GameState` and `Game::visible_extra_piles` (H8)

### Phase 4 — Break up god objects

Goal: SRP. Each component has one reason to change.

- [ ] **P4.1** Split `Room` into `SeatManager` / `LobbyChat` / `RejoinTracker` / `GameSession` / `SessionScorer` / `BotDriver` / `Broadcaster` (C5)
- [ ] **P4.2** Consolidate room mutexes into one `RwLock<RoomInner>` (or migrate to actor pattern) (H5)
- [ ] **P4.3** Make the room own a single long-lived bot task driven by `tokio::sync::Notify` (H7)
- [ ] **P4.4** Move business logic out of `ws::handler.rs::route` (H1)

### Phase 5 — Client decomposition

Goal: small components, each with one responsibility.

- [ ] **P5.1** Split `useGameStore` into connection / game / session / lobby / sheepshead stores (H2)
- [ ] **P5.2** Decompose `GameTable.vue` into `GameHeader`, `SeatRail`, `MyHandPanel`, `SessionScoreboard`, `HandResult`, `SessionResult`, `CompletedTrickHistory`, `ToastStack` (H3)
- [ ] **P5.3** Split `BiddingPanel.vue` into `PickingPanel` / `BuryingPanel` / `CallingPanel` (H4)
- [ ] **P5.4** Split `useGame` composable into `useGameActions` / `useLobbyActions` / `useSheepsheadActions` (M8)
- [ ] **P5.5** Game registry in `client/src/games/index.ts` (M10)

### Phase 6 — Polish

- [ ] **P6.1** Centralize magic timeouts (M9)
- [ ] **P6.2** WebSocket reconnect with backoff (L1)
- [ ] **P6.3** CSS custom properties / design tokens (L2)
- [ ] **P6.4** Accessibility pass (L3)
- [ ] **P6.5** Test coverage for `Room` paths (L7)
- [ ] **P6.6** Reconcile CLAUDE.md card-encoding note (L8)

---

## What I'm Deliberately Not Recommending

A few things I considered and rejected as scope creep or premature:

- **Replacing the Pinia store with an event-sourced log of `StateUpdate`s.** Tempting for time-travel debugging, but adds complexity the current product doesn't need.
- **Switching `Room` to an actor pattern as a hard requirement.** It's a clean fix for H5 but heavier than a consolidated lock — leave it as an option if locking gets unwieldy after Phase 4.
- **Persisting game state to a database.** The product is ephemeral by design; persistence is a feature decision, not a refactor.
- **Replacing JSON with binary protocol or compact card encoding.** No measured perf issue yet; defer until there's pressure.
- **Adding integration tests over the WebSocket protocol.** Worth doing eventually, but the per-unit tests cover most paths once Phase 2 collapses the duplicated state.
