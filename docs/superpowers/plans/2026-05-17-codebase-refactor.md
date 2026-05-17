# Codebase Refactor Plan

> Adversarial review of the trick-based card game platform, focused on architecture, SOLID adherence, complexity, code quality, and proper separation of concerns between client (view) and server (authoritative business logic).

**Scope:** Both `server/` (Rust + Axum) and `client/` (Vue 3 + Pinia). Per CLAUDE.md, the engine must remain game-agnostic and the client must remain a pure view layer over server snapshots.

**Format:** Each finding has a **Severity** (Critical / High / Medium / Low), a **Location** with file:line references, a **Why it matters** explanation, and a **Refactor** prescription. The end of the doc has a sequenced rollout.

> **Post-merge update (2026-05-17, after `6be2853` + `ee28200` + Euchre work):** The Euchre game and a partial refactor of bots and the trick-winner protocol landed. The audit below has been revised in-place: each finding is tagged ✅ resolved / ⚠️ partial / ❌ open / 🆕 new (introduced by the merge). Six findings were resolved or partially resolved (C1, C2, C4, M5 partially, M10, M7-as-relocation). Five new findings were introduced, mostly because Euchre exposed asymmetries that didn't exist when only Sheepshead existed. Items already open before the merge are now **more** urgent because the second game makes every abstraction hole visible. See the "Post-Merge Status Update" section at the end for the re-prioritized rollout.

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

- **Status:** ✅ **Resolved** in `6be2853`. The default trait method now has no body — every `Game` impl must supply its own `apply_play`, which delegates to `apply_play_generic`. The free function gained an `active_seats: Option<&[usize]>` parameter to support Euchre's going-alone (one defender sits out). Both Sheepshead (`rules.rs:178`) and Euchre (`rules.rs:177`) now call it.
- **Severity:** Critical (correctness time bomb)
- **Location:** `server/src/engine/game.rs:54-127` (`apply_play_generic` free fn) and `server/src/engine/game.rs:190-263` (default trait method `Game::apply_play`)
- **Why it matters:** These are line-for-line the same 70-line block. The free function exists so `Sheepshead::apply_play` (rules.rs:156-179) can run partner-revelation logic before delegating to it; the default trait method does the same work. Any fix to one (e.g., a turn-order bug, scoring edge case) will silently skip the other. This is the most dangerous duplication in the codebase because the failure mode is "the second game's rules silently differ from Sheepshead."
- **Refactor:** Delete the inline body in `Game::apply_play`'s default; have the default delegate to `apply_play_generic(self, state, seat, card)`. Keep the free function as the single source of truth. Sheepshead override stays unchanged.

### C2. Client computes server state — stores/game.ts

- **Status:** ⚠️ **Mostly resolved** in `ee28200`. `CardPlayed` now carries `current_trick_winner: Option<usize>` and `next_player: usize` (state.rs:159). The store reads them directly (game.ts:93, 102) instead of recomputing. `TrickDisplay.vue:27-31` reads `currentWinnerSeat` from props. **Still open:** `client/src/engine/sort.ts:75-104` retains `trickWinnerIndex` and the Sheepshead trump table — now dead code from the store, but `engine/sort.ts:54` `sortHand` still uses the local trump table to display Sheepshead hands in display order. See finding **N4** below: the Sheepshead trump table is still in a "generic" engine file rather than under `games/sheepshead/`.
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

- **Status:** ❌ **Still open and now twice as bad.** `meta: serde_json::Value` is unchanged. Euchre's rules.rs now adds 20+ new untyped accesses (`state.meta["turned_up_card"]`, `state.meta["sub_phase"]`, `state.meta["caller_seat"].as_u64()`, `state.meta["going_alone"].as_bool()`, `state.meta["sits_out"]`, `state.meta["passed_round1"]`, `state.meta["passed_round2"]`). The client `useEuchreState()` composable has type-guards `if (store.gameState?.game_name !== 'euchre') return null` on every getter — that's the cost of untyped meta surfacing.
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

- **Status:** ⚠️ **Half resolved** in `8db9ddc`. Per-game bot modules now exist: `server/src/games/sheepshead/bot.rs` and `server/src/games/euchre/bot.rs`. Generic helpers (`build_bot_state`, `current_winner`, `min_winning_trump`, `point_value`, `lowest_card`, `highest_point_card`) stayed in `server/src/bot.rs`. **What's still wrong:**
  - Dispatch is via string match on `game_name`, not the trait: `server/src/games/mod.rs:16-30` has `match state.game_name.as_str() { "sheepshead" => ..., "euchre" => ... }`. Adding Hearts requires editing this file — exactly the Open/Closed violation the refactor was supposed to prevent. See finding **N5**.
  - `server/src/bot.rs:151-158` adds a second dispatch layer (`bot::bid_action` → `games::bot_bid` → `sheepshead::bot::bid_action`). The room calls through `crate::bot::bid_action` (`room.rs:578`); should call through the trait.
  - The generic helpers should move to `engine/bot_helpers.rs`; `bot.rs` becomes just the dispatcher and ideally that dispatcher disappears once `Game` owns `BotStrategy`.
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

- **Status:** ❌ **Still open.** `server/src/lobby/room.rs` is 683 lines now (was 676 — basically unchanged). All six mutexes still independent. The Euchre work didn't touch this file beyond minor adjustments.
- **Severity:** Critical (SRP — 683 lines, 8 responsibilities, 6 mutexes)
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

- **Status:** ❌ **Still open and now contains a latent bug.** `handler.rs:119, 121, 144` still hardcode `lobby.create_room(game, 5, 24)`. The literal `5` is wrong for Euchre (4 players); the literal `24` is wrong for Euchre's victory goal (typically 10). When a user calls "create Euchre room" today, the room is built with 5 seats and victory_goal=24. Combined with **N3** below (client hardcodes `VICTORY_GOAL = 10` in Euchre's GameTable), there is now a real seat-count/victory-goal disagreement between server and client.
- **Severity:** High (separation of concerns + active bug for Euchre)
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

- **Status:** ❌ **Still open, and now asymmetrically wrong across games.** Euchre's game-specific state was correctly extracted into `client/src/games/euchre/state.ts` (`useEuchreState()` exposing `callerSeat`, `sitsOut`, `calledSuit`, `turnedUpCard`, `subPhase`). But Sheepshead's game-specific state (`picker`, `isPicker`, `partnerRevealedSeat`) is still mixed into `useGameStore` (game.ts:32-39, 17, 152-158). New asymmetry violates the CLAUDE.md standard added in `b3c946a` ("game-specific state ... live in separate Pinia stores"). See finding **N6**.
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

- **Status:** ❌ **Significantly worse after the merge — promoted to "must fix before Hearts/Spades."** Instead of being decomposed, the file was **cloned**. `client/src/games/sheepshead/GameTable.vue` (395 lines) and `client/src/games/euchre/GameTable.vue` (425 lines) are 80%+ identical: phase toast, header, seat rail, trick display invocation, my-hand section, session scoreboard, hand-complete view, session-over view, completed-trick history, and ~200 lines each of essentially identical CSS. Compare `GameTable.vue` for both — the script blocks differ only in role-badge fields (picker vs caller/sits-out) and a few words; the templates differ only in which game-specific store provides badges; the styles differ in caller vs picker color. Hearts/Spades will create a third and fourth copy. See finding **N1**.
- **Severity:** Critical (was High) — duplication is now actively blocking new games
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

- **Status:** ⚠️ **Half resolved.** Euchre's `BiddingPanel.vue` (40 lines) correctly switches into `OrderingPanel`, `DiscardingPanel`, `CallingPanel` sub-components. Sheepshead's `BiddingPanel.vue` (159 lines) was untouched — still has picking/burying/calling all inline, with `burySelection` ref leaking across sub-phases. Asymmetry. See finding **N7**.
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

- **Status:** ❌ **Still open.** Unchanged.
- **Severity:** High (correctness)
- **Location:** `server/src/lobby/room.rs:81-89` — `seats: Mutex<Vec<SeatState>>`, `state: Mutex<Option<GameState>>`, `session_scores: Mutex<Vec<i32>>`, `chat_history: Mutex<VecDeque<...>>`, `max_hands: Mutex<Option<u32>>`, `hands_played: Mutex<u32>` — six independent mutexes
- **Why it matters:** `start_game` (`room.rs:265-276`) locks `seats`, drops, then `start_game_inner` locks `state`. `play_card` (`room.rs:454-514`) locks `state`, drops, then locks `session_scores`, then `hands_played`, then `max_hands`. Any code path that needs two together at once invites a deadlock the moment someone reorders. `drive_bots` (`room.rs:529-586`) re-locks `state` 5+ times in one iteration, each acquire/release racing with player input.
- **Refactor:**
  - Consolidate room state into one `Mutex<RoomInner>` (or `RwLock<RoomInner>`), with `RoomInner` holding all fields except the broadcast channel. One lock to take, one to drop.
  - For the bot driver loop, hold the lock for one logical "decide + apply" transaction rather than reacquiring per field read.
  - Alternative: actor pattern — the room runs an async task that owns the state and processes a `RoomCommand` mpsc; no mutexes at all. This is heavier to introduce but eliminates the entire class of bugs.

### H6. Default victory goal & player count are magic numbers

- **Status:** ❌ **Still open and now causes an active server/client disagreement for Euchre.** Server hardcodes `(game, 5, 24)` in `handler.rs:119, 121, 144` and `matchmaker.rs:80` regardless of game. Client hardcodes `VICTORY_GOAL = 24` for Sheepshead and `VICTORY_GOAL = 10` for Euchre in the respective GameTables. An Euchre room therefore opens with 5 seats and a server victory goal of 24, while the client displays "first to 10" — they are computing different things. See finding **N3**.
- **Severity:** High (was Medium-High) — now a real bug, not just a smell
- **Location:** `server/src/ws/handler.rs:119, 121, 144`, `server/src/lobby/matchmaker.rs:80` — all hardcode `lobby.create_room(<game>, 5, 24)`. Client `client/src/games/sheepshead/GameTable.vue:34` hardcodes `VICTORY_GOAL = 24`.
- **Why it matters:** Adding Hearts/Euchre/Spades means finding every `24` and `5` literal. The client constant will drift from the server's the first time someone changes one.
- **Refactor:** Add `fn default_victory_goal(&self) -> i32` and `fn default_player_count(&self) -> usize` to `Game`. Server reads from the trait. Server includes `victory_goal` in the `Snapshot` payload (or in a new `RoomConfig` message sent on join). Client reads from the snapshot, never from a constant.

### H7. Bot driver spawn-and-forget — handler.rs and room.rs

- **Status:** ❌ **Still open.** Unchanged — `handler.rs:189-191, 201-203` still `tokio::spawn(drive_bots)` after every Bid/PlayCard, and `bots_running: AtomicBool` is the only thing keeping them from piling up.
- **Severity:** High (correctness, resource leaks)
- **Location:** `server/src/ws/handler.rs:189-191, 201-203` — `tokio::spawn(async move { room_arc.drive_bots().await })` after every successful Bid/PlayCard. `server/src/lobby/room.rs:529-532` — `bots_running` AtomicBool guards reentry.
- **Why it matters:** Every player action spawns a task that immediately checks an atomic and returns. The AtomicBool guard is correct only because the loop is one-shot per call; the design is "spawn-then-noop" which is a code smell. Worse, the lobby `start_game` does `tokio::spawn(drive_bots)` at room.rs:273 too — but if a player action lands before the spawn happens, the spawn at handler.rs:190 races with it.
- **Refactor:** Room owns exactly one long-lived bot task started in `start_game`. The handler calls `room.notify_action_applied()` which `notify_one`s a `tokio::sync::Notify`. The bot loop `await`s the notify, then drains pending decisions. No spawning at the request layer.

### H8. Snapshot-redaction logic duplicated

- **Status:** ✅ **Resolved.** `GameState::redacted_for(seat, game)` is now the single helper used by all three Snapshot call sites (`room.rs::join_lobby`, `room.rs::on_rejoin`, `room.rs::start_next_hand`). The `Game` trait gained `visible_extra_piles(&self, state, seat) -> Vec<&'static str>` with a default returning empty (i.e. all piles hidden), matching today's behavior for both Sheepshead and Euchre. Games can now opt piles back in per seat — e.g., a Sheepshead variant that reveals the blind post-pick would override `visible_extra_piles` to return `vec!["blind"]` when `state.meta["picker"]` matches `seat`. Six unit tests in `engine/state.rs` cover the helper.
- **Severity:** Medium-High (duplication, now also a latent abstraction hole)
- **Location:** `server/src/lobby/room.rs:381-388` (rejoin), `server/src/lobby/room.rs:596-602` (start_next_hand) — both clone state, clear other hands, clear extra_piles.
- **Why it matters:** Redaction rules are scattered. When game-specific rules decide some piles ARE visible (e.g., Sheepshead post-pick reveal of the blind to all, if that became a feature), it'll be missed in one of the two places.
- **Refactor:** `GameState::redacted_for(&self, seat: usize, game: &dyn Game) -> GameState`. The `Game` trait gets a hook `fn visible_extra_piles(&self, state: &GameState, seat: usize) -> Vec<&str>` so games can opt extra piles back in. Both call sites become one line.

### H9. Protocol types duplicated between Rust and TypeScript

- **Status:** ❌ **Still open.** No `ts-rs`/`typeshare`. `CardPlayed` got two new fields server-side and client-side as a matched pair — that's the kind of change that will rot the first time someone updates only one side.
- **Severity:** High (drift surface area)
- **Location:** `server/src/engine/state.rs` (ClientMessage, StateUpdate, GameState, SeatInfo, GamePhase) vs `client/src/engine/types.ts` (same shapes restated)
- **Why it matters:** Every protocol change requires editing two files. CLAUDE.md flags this as TODO ("consider generating TypeScript types from Rust structs via `typeshare` or `ts-rs`") — it should not remain a TODO. Today the duplication is small; the day Sheepshead picks up calling-from-the-blind variants it'll be the source of every "client and server disagree about message shape" bug.
- **Refactor:** Add `ts-rs` (build-time) or `typeshare` (CLI) to the Cargo deps. Annotate the Rust structs/enums with `#[derive(TS)]` (ts-rs) or `#[typeshare]`. Generate `client/src/engine/protocol.generated.ts` from `cargo run --bin generate-types` or a `build.rs` task. Replace hand-written types.ts with a re-export plus client-only helpers (Card constructors etc.).

### H10. `extra_piles: Vec<(String, Vec<Card>)>` is stringly-typed

- **Status:** ❌ **Still open and now plural.** Sheepshead writes `"blind"` and Euchre writes `"kitty"` (`server/src/games/euchre/rules.rs:129`). Two magic strings, no shared key type.
- **Severity:** Medium-High
- **Location:** `server/src/engine/state.rs:37`, used in `engine/dealer.rs:13`, `games/sheepshead/rules.rs:97-98, 398-403`
- **Why it matters:** "blind" is a magic string. The Sheepshead pick code does `iter().position(|(name, _)| name == "blind")` — fragile and untyped.
- **Refactor:** Enum-keyed map (`HashMap<PileKind, Vec<Card>>` with `PileKind` per-game), or extra piles become typed associations stored in `Game::Meta`. Since extra piles are inherently game-specific, the cleanest fix is to fold them into the typed meta from C3.

---

## Medium-Priority Findings

### M1. Duplicated scoring logic — sheepshead/rules.rs

- **Status:** ❌ **Still open.** `rules.rs` unchanged.
- **Location:** `server/src/games/sheepshead/rules.rs:288-356` — the "going alone" and "called partner" branches each compute identical schneider gates with copy-pasted match arms.
- **Refactor:** Extract `fn schneider_score(picker_share: i32, defender_share: i32, mode: ScoreMode) -> ScoreDistribution`. Both branches become one call with `ScoreMode::Alone` or `ScoreMode::Partner`.

### M2. `apply_bid` broadcasts via opaque JSON — bid_result

- **Status:** ❌ **Still open.** Euchre uses the same opaque-JSON pattern: `euchre/rules.rs:351` constructs `broadcast_payload: Some(serde_json::json!({...}))` for the round-1-to-round-2 transition.
- **Location:** `server/src/engine/game.rs:30-31` (`BidResult.broadcast_payload: Option<serde_json::Value>`)
- **Why it matters:** Re-introduces the stringly-typed escape hatch (C3) into the protocol layer. The room blindly forwards the JSON, so consumers (incl. the bot) parse a payload whose shape only the game module knows.
- **Refactor:** Define an enum `BidBroadcast { Raw, SubPhaseUpdate { sub_phase: SubPhase, callable_suits: Vec<SuitName> }, ... }` per game (or globally). The room can pattern-match instead of forwarding opaque blobs.

### M3. `Sheepshead::deal` panics on invalid input

- **Status:** ❌ **Still open and now plural.** Euchre adds new asserts: `euchre/rules.rs:102-103, 122` (player_count=4, deck=24, kitty=4). Per **H1** above, if a Euchre room ever opens with the wrong seat count due to the handler hardcoding `5`, this will crash a tokio task instead of returning an error.
- **Location:** `server/src/games/sheepshead/rules.rs:76-77` — `assert_eq!(player_count, 5, ...)`, `assert_eq!(shuffled_deck.len(), 32, ...)` — and now Euchre as well.
- **Why it matters:** A misconfigured room (or a future bug that lets a 4-player Sheepshead room start) crashes the tokio task with a panic instead of erroring out cleanly. `Game::deal` currently returns `DealResult`; making it `Result<DealResult, GameError>` lets the room surface the error to the client.
- **Refactor:** Change `Game::deal -> Result<DealResult, GameError>`. Validation moves from runtime asserts to the trait contract.

### M4. `Result<_, String>` everywhere — no typed errors

- **Status:** ❌ **Still open.**
- **Location:** All `apply_bid`, `apply_play`, `handle_lobby_chat`, `force_bot`, `extend_rejoin`, etc. (See `engine/game.rs:177-181`, `lobby/room.rs:226-247, 398-427`)
- **Why it matters:** `thiserror = "2"` is already a dependency (Cargo.toml:17) but unused. String errors mean callers can't pattern-match (e.g., distinguish "not your turn" from "card not in hand" from "wrong sub-phase") and the client can't localize messages.
- **Refactor:** Introduce `engine::GameError`, `engine::ProtocolError`, `lobby::RoomError` enums with `thiserror`. Map at the protocol boundary to a typed client-facing error (e.g., `{ code: 'not_your_turn', message: '...' }`) so the client can show different UI for different errors.

### M5. `#![allow(dead_code)]` with stale TODOs

- **Status:** ❌ **Still open.** `room.rs:1-3` and `matchmaker.rs:1-2` still reference "Task 6". Nothing changed.
- **Location:** `server/src/lobby/room.rs:1-3` and `server/src/lobby/matchmaker.rs:1-2` reference "Task 6" of a plan whose tasks are long done.
- **Refactor:** Remove the file-level allow. Audit the `#[allow(dead_code)]` markers on individual items — many (e.g., `SeatState::is_human`, `SeatState::ws_id` on disconnected variants) are actually used; the rest should be deleted.

### M6. Bot's `point_value` duplicates `Game::card_points`

- **Status:** ❌ **Still open.** `bot.rs:53-62` still defines `point_value(card)` with the Sheepshead-specific table baked in (Ace=11, Ten=10, etc.). `lowest_card` and `highest_point_card` (bot.rs:116-144) both call it. Since Euchre has different card_points (0 for everything — Euchre scores tricks, not points), passing Euchre cards through `lowest_card`/`highest_point_card` will sort them by *Sheepshead* point values. Check whether `euchre/bot.rs` actually relies on these helpers — if so, this is a latent ranking bug for Euchre bots.
- **Location:** `server/src/bot.rs:53-62`
- **Refactor:** Delete `point_value`; call `game.card_points(card)` (already in scope as `&dyn Game`).

### M7. Trump rules duplicated client-side — engine/sort.ts

- **Status:** ⚠️ **Partially resolved**. Euchre's sort logic correctly lives in `client/src/games/euchre/sort.ts` and uses an `Euchre`-specific `sortHandEuchre(cards, calledSuit)`. **But** the Sheepshead trump table was never moved out of `engine/sort.ts` — it's still imported as a "generic" sort via `HandComponent.vue:4` `import { sortHand } from '@/engine/sort'`. `HandComponent.vue:11, 16` now accepts an optional `sortFn` prop, which Euchre uses (`euchre/GameTable.vue:84, 157`); Sheepshead still relies on the default `engine/sort.ts` import, which is hardcoded to Sheepshead's rules. The dead `trickWinnerIndex` (`engine/sort.ts:75-104`) is also still exported. See finding **N4**.
- **Location:** `client/src/engine/sort.ts:7-32` reproduces the Sheepshead trump rank table; `engine/sort.ts:75-104` is dead.
- **Refactor:** Move the Sheepshead trump table into `client/src/games/sheepshead/sort.ts`; create a `useSheepsheadSort()` analogue to Euchre's; make Sheepshead's GameTable pass `:sort-fn` to `HandComponent` like Euchre does; delete `trickWinnerIndex` entirely (server is authoritative now).

### M8. `useGame.ts` mixes generic and Sheepshead-specific actions

- **Status:** ⚠️ **Half resolved.** Euchre's bidding actions live in `client/src/games/euchre/useEuchreBidding.ts` (`orderUp`, `euchrePass`, `discard`, `callSuit`). But Sheepshead's `pick`/`pass`/`bury`/`callAce`/`goAlone` are still in the generic `useGame()` (`useGame.ts:33-51`). Asymmetry again.
- **Location:** `client/src/composables/useGame.ts:33-51` — `pick`, `pass`, `bury`, `callAce`, `goAlone` are Sheepshead-specific in a file named "useGame"
- **Refactor:** Split into:
  - `useGameActions()` — `playCard`, `startGame`, `sendLobbyChat`, `forceBot`, `extendRejoin`, queue actions
  - `useLobbyActions()` — `createRoom`, `joinRoom`, `joinWithCode`, `createPrivateRoom`, `createSoloRoom`
  - `useSheepsheadActions()` (in `games/sheepshead/`) — `pick`, `pass`, `bury`, `callAce`, `goAlone`

### M9. Magic timeouts scattered across the codebase

- **Status:** ❌ **Still open and now plural** — Euchre's GameTable adds its own 1500ms phase-toast timer (`euchre/GameTable.vue:75`).
- **Location:** `room.rs:321` (30s rejoin), `room.rs:534` (1200ms bot delay), `matchmaker.rs:13` (60s queue timeout), `matchmaker.rs:14` (8 max hands), `stores/game.ts:120` (1500ms completed-trick hold), `stores/game.ts:157` (2000ms partner reveal), `sheepshead/GameTable.vue:63` and `euchre/GameTable.vue:75` (1500ms phase toast).
- **Refactor:** Centralize: server in `server/src/config.rs` (loadable from env for tests/prod), client in `client/src/config.ts`.

### M10. Hardcoded `<sheepshead-table>` in GameView

- **Status:** ✅ **Resolved** in `ee28200`. `GameView.vue` now uses `<component :is="gameTable" />` with a registry `GAME_TABLES` (`GameView.vue:8-11`) using `defineAsyncComponent` for lazy-loading. There's also a separate `client/src/engine/games.ts` `GAMES` registry exposing game metadata (label, playerCount). Two registries that could be unified eventually, but the hardcoding is gone.

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

---

## Post-Merge New Findings (introduced by the Euchre work)

These were not in the original review because Sheepshead-only code wasn't exhibiting them. Adding Euchre made each one visible.

### N1. `GameTable.vue` was cloned, not decomposed — sheepshead/euchre

- **Severity:** Critical (now the largest single blocker to adding a third game)
- **Location:** `client/src/games/sheepshead/GameTable.vue` (395 lines) and `client/src/games/euchre/GameTable.vue` (425 lines)
- **Why it matters:** Diff the two files: phase-toast, header (phase badge + dealer + trick counter), seat rail, `<trick-display>` invocation, my-hand section with role badges + your-turn glow, session scoreboard with progress bar, between-hand result, session-over with router-link to lobby, completed-trick history disclosure — all identical in structure with cosmetically different role-badge labels. The CSS blocks are essentially identical (~190 lines each). Every layout fix or design-token tweak now has to be made twice. A third game makes it three; a CSS migration touches all of them.
- **Refactor:** Extract a shared `<GameTableShell>` (or just put the layout in `views/GameView.vue` and demote `sheepshead/GameTable.vue` to a `<SheepsheadBidding>` slot). Concretely:
  ```
  views/GameView.vue
   ├── <GameHeader>             generic (reads phase, dealer, names)
   ├── <SeatRail>               accepts a slot for game-specific role badges
   ├── <TrickDisplay>           already exists
   ├── <component :is="biddingPanel" />     game-specific
   ├── <MyHandPanel>            accepts game-specific badges slot + sort-fn
   ├── <SessionScoreboard>      generic, reads victory_goal from server
   ├── <HandResult>             generic
   ├── <SessionResult>          generic
   ├── <CompletedTrickHistory>  generic
   └── <PhaseToast> + game-specific toasts
  ```
  Both Sheepshead and Euchre GameTable wrappers shrink to ~40 lines: just register the game-specific bidding panel and the badge slots.

### N2. Server-side bot dispatch by `match game_name` — games/mod.rs

- **Severity:** High (Open/Closed bypass via string match)
- **Location:** `server/src/games/mod.rs:16-30` — `bot_bid` and `bot_play` switch on `state.game_name.as_str()`.
- **Why it matters:** Adding Hearts requires editing this file, plus `get_game` at line 8 (which is unavoidable for now without a static registry, but `bot_bid`/`bot_play` are avoidable). This is exactly the abstraction bypass that C4 was supposed to close. The bot's per-game module already implements `bid_action`/`play_card` — those should be reachable through the `Game` trait (e.g., as a `&dyn BotStrategy` returned by `Game::bot_strategy(&self) -> &dyn BotStrategy`).
- **Refactor:** Either add `BotStrategy` as an associated type / trait-object on `Game`, or make `bot_bid`/`bot_play` two more required trait methods on `Game` directly. The room calls `self.game.bot_bid(state, seat)` and `self.game.bot_play(state, seat)` — no string dispatch needed. Delete `crate::bot::bid_action`/`play_card` wrappers, and the `games::bot_bid`/`bot_play` dispatchers along with them.

### N3. Server victory_goal/player_count disagree with client for Euchre

- **Severity:** High (active bug; Euchre rooms misconfigured at room creation)
- **Location:**
  - Server: `handler.rs:119, 121, 144` hardcode `lobby.create_room(game, 5, 24)`; `matchmaker.rs:80` hardcodes `lobby.create_room("sheepshead".into(), 5, 24)`.
  - Client: `client/src/games/sheepshead/GameTable.vue:34` `VICTORY_GOAL = 24`; `client/src/games/euchre/GameTable.vue:23` `VICTORY_GOAL = 10`.
- **Why it matters:** Creating a Euchre room today builds it with 5 seats (Euchre is 4-player) and victory_goal=24 server-side, while the client shows "first to 10". Either the server's session winner detection fires never (24-point victory is rare in Euchre), the seat count rejects valid Euchre player counts, or both. This is functional drift, not a smell.
- **Refactor:** Add `Game::default_victory_goal(&self) -> i32` and `Game::default_player_count(&self) -> usize` (or use `valid_player_counts()[0]`). Lobby creation reads from the trait. Server includes `victory_goal` and `player_count` in `Snapshot` (or a new `RoomConfig` message). Client reads from snapshot, deletes `VICTORY_GOAL` constants. `client/src/engine/games.ts` `GAMES` registry already has `playerCount` baked in client-side — that's another piece that should come from the server, not be duplicated.

### N4. Sheepshead trump table still lives in `engine/sort.ts` (asymmetry from Euchre)

- **Severity:** Medium (game-specific code in "engine" namespace)
- **Location:**
  - `client/src/engine/sort.ts:7-32` — Sheepshead trump rank table (queens/jacks per suit, diamond ranks)
  - `client/src/engine/sort.ts:75-104` — dead `trickWinnerIndex` (no longer called by the store)
  - Compare: `client/src/games/euchre/sort.ts:9-22` — Euchre's trump table correctly lives under `games/euchre/`
- **Why it matters:** `engine/` should be game-agnostic per CLAUDE.md. The Euchre work correctly placed Euchre's sort under `games/`. The Sheepshead equivalent didn't move. `HandComponent.vue:16` falls back to the "engine" sort when no `sortFn` prop is provided — and that fallback bakes in Sheepshead rules.
- **Refactor:** Move `sortHand` and the trump table to `client/src/games/sheepshead/sort.ts` (parallel to `euchre/sort.ts`). Update Sheepshead's `GameTable.vue` to pass `:sort-fn` to `HandComponent` like Euchre does. Delete `trickWinnerIndex` and `SUIT_ORDER` from `engine/sort.ts` (Euchre imports `SUIT_ORDER` from there — move that constant to a shared `engine/display.ts` or duplicate it under each game, since the fail-suit display order is itself a game-specific decision).

### N5. Sheepshead's `BiddingPanel.vue` not split (asymmetry from Euchre)

- **Severity:** High (SRP) — promoted from the second half of H4
- **Location:** `client/src/games/sheepshead/BiddingPanel.vue` (159 lines) still mixes picking, burying, and calling sub-phases. `burySelection` ref leaks across sub-phase boundaries.
- **Why it matters:** Euchre's `BiddingPanel.vue` is 40 lines because it dispatches into `OrderingPanel`/`DiscardingPanel`/`CallingPanel`. The Sheepshead equivalent should follow the same pattern. The asymmetry is what makes it noticeable — same conceptual layout, two different implementations.
- **Refactor:** Mirror Euchre's structure:
  ```
  client/src/games/sheepshead/
   ├── BiddingPanel.vue              ← shrinks to a switch
   └── bidding/
        ├── PickingPanel.vue         ← pick/pass buttons + waiting state
        ├── BuryingPanel.vue         ← owns burySelection ref, validates, submits
        └── CallingPanel.vue         ← callable suits + go-alone
  ```

### N6. Sheepshead-specific state still in `useGameStore` (asymmetry from Euchre)

- **Severity:** High (SRP, asymmetry, breaks the CLAUDE.md standard we just wrote)
- **Location:**
  - `client/src/stores/game.ts:32-39` — `picker`, `isPicker` computeds
  - `client/src/stores/game.ts:17` — `partnerRevealedSeat` ref
  - `client/src/stores/game.ts:152-158` — `partner_revealed` event handler with setTimeout
  - `client/src/games/sheepshead/GameTable.vue:41-44, 78-81` — recomputes `partnerSeat` and `calledSuit` ad-hoc in the component
- **Why it matters:** Euchre got its own `useEuchreState()` composable (`client/src/games/euchre/state.ts`) exposing all game-specific reads with type guards. Sheepshead never got the equivalent. The standard added in `b3c946a` says "game-specific state ... live in separate Pinia stores" — Sheepshead violates it because the work stopped at Euchre.
- **Refactor:** Create `client/src/games/sheepshead/state.ts` with `useSheepsheadState()` exposing `picker`, `isPicker`, `partnerSeat`, `calledSuit`, `isPickingPhase`, `isBuryPhase`, `isCallingPhase`, `callableSuits`, `partnerRevealedSeat`. Move the `partner_revealed` setTimeout into a `usePartnerRevealToast()` composable owned by the Sheepshead GameTable, not the global store. Strip those concepts out of `useGameStore`.

### N7. Two layers of bot dispatch indirection — bot.rs

- **Severity:** Medium (cleanup; depends on N2 being done first)
- **Location:** `server/src/bot.rs:151-158` — `bot::bid_action` calls `crate::games::bot_bid`; `bot::play_card` calls `crate::games::bot_play`. The room (`room.rs:578, 585`) calls `bot::bid_action`/`play_card`, which forwards to `games::bot_bid`, which switches on `game_name` and calls `sheepshead::bot::bid_action`. Three layers to get from caller to callee.
- **Refactor:** Once N2 lands (BotStrategy on the trait), delete both shim layers. `room.rs` calls `self.game.bot_bid(state, seat)`. Generic helpers (`build_bot_state`, `current_winner`, `min_winning_trump`, `lowest_card`, `highest_point_card`) move to `engine/bot_helpers.rs`. `server/src/bot.rs` disappears.

### N8. `point_value` in shared bot helpers is Sheepshead-specific

- **Severity:** Medium-High (potential ranking bug for Euchre bots)
- **Location:** `server/src/bot.rs:53-62` `point_value` table is `Ace=11, Ten=10, King=4, Queen=3, Jack=2` — Sheepshead's scoring. `lowest_card` and `highest_point_card` (bot.rs:116-144) use this table to sort cards for sluffing decisions.
- **Why it matters:** Euchre's `card_points` is uniform (Euchre scores tricks won, not card points). If `euchre::bot` calls into these shared helpers, defenders/partners will sluff cards ranked by *Sheepshead* points — wrong choice for Euchre. Verify whether `euchre/bot.rs` actually uses `lowest_card`/`highest_point_card`; if yes, fix by calling `game.card_points(card)` and treating Euchre's all-zeros table as "use rank as the tiebreaker."
- **Refactor:** Subsumed by M6 — fix by routing through `game.card_points`.

---

## Post-Merge Status Update — Re-Prioritized Rollout

Given the merge, here is the work re-sequenced. **The biggest change to the original sequence is promoting N1 (GameTable cloning) and N3 (Euchre config disagreement) above almost everything else**, because they're either active bugs or active blockers to the third game.

### Phase 0 — Active bug fixes (do first)

- [ ] **P0.1 — N3** Pipe `victory_goal`/`player_count` through the `Game` trait and the `Snapshot` payload so Euchre rooms get the right values. Delete client `VICTORY_GOAL` constants. (Resolves H6 too.)
- [ ] **P0.2 — N8/M6** Audit whether `euchre/bot.rs` reaches into `lowest_card`/`highest_point_card`; if so, fix the sluff-ranking by routing through `Game::card_points`.

### Phase 1 — Lock in symmetry between Sheepshead and Euchre

The Euchre refactor established a cleaner pattern in several places. Bring Sheepshead up to that pattern before adding game #3.

- [ ] **P1.1 — N5** Split Sheepshead's `BiddingPanel.vue` into `PickingPanel`/`BuryingPanel`/`CallingPanel` under `games/sheepshead/bidding/`. (Resolves H4 fully.)
- [ ] **P1.2 — N6** Create `useSheepsheadState()` mirroring `useEuchreState()`. Strip `picker`/`isPicker`/`partnerRevealedSeat`/`partner_revealed` handler from `useGameStore`. Move the partner-reveal setTimeout into a component-local `usePartnerRevealToast()` composable.
- [ ] **P1.3 — N4** Move Sheepshead's trump table from `engine/sort.ts` to `games/sheepshead/sort.ts`. Update `sheepshead/GameTable.vue` to pass `:sort-fn`. Delete dead `trickWinnerIndex`.
- [ ] **P1.4 — M8 remainder** Move Sheepshead's bidding actions (`pick`/`pass`/`bury`/`callAce`/`goAlone`) into `games/sheepshead/useSheepsheadBidding.ts` mirroring `useEuchreBidding.ts`.

### Phase 2 — Eliminate GameTable duplication (blocking for new games)

- [ ] **P2.1 — N1** Extract `<GameTableShell>` (or fold layout into `GameView.vue`). Both per-game GameTables shrink to ~40 lines that supply the bidding panel and any game-specific badge slots.
- [ ] **P2.2 — L6** Drop the `gameState!` non-null assertions in both GameTables; have `GameView.vue` guard before mounting.

### Phase 3 — Close the trait abstraction holes (blocking for game #3)

- [ ] **P3.1 — N2/C4 remainder** Add bot dispatch to the `Game` trait (`bot_bid`/`bot_play` or `bot_strategy() -> &dyn BotStrategy`). Delete `games::bot_bid`/`bot_play` string-match dispatchers.
- [ ] **P3.2 — N7** Move generic bot helpers to `engine/bot_helpers.rs`; delete `server/src/bot.rs`.
- [ ] **P3.3 — C3** Introduce `Game::Meta` associated type (or typed wrapper). Replace ~30 `state.meta["…"]` accesses in `sheepshead/rules.rs`, `euchre/rules.rs`, both per-game bot modules, and `room.rs` with typed reads.
- [ ] **P3.4 — H10** Fold `extra_piles` into the typed `Meta` (or a typed `PileKind` enum). Delete `"blind"` and `"kitty"` string literals.

### Phase 4 — Server hygiene (was Phase 1 / 2 in the original plan)

- [ ] **P4.1 — M2** Typed `BidResult.broadcast_payload` variants per game.
- [ ] **P4.2 — M3** Promote `Game::deal` from `assert!`-on-bad-input to `Result<DealResult, GameError>`. Removes Euchre's three new asserts at the same time.
- [ ] **P4.3 — M4** Introduce `engine::GameError` / `lobby::RoomError` via `thiserror` (already a dep).
- [ ] **P4.4 — H1** Move "if fill_bots then start" / "default victory goal" / "auto-create room if missing" out of `ws::handler::route` into the lobby/room layer.
- [ ] **P4.5 — M5** Remove file-level `#![allow(dead_code)]` from `room.rs`/`matchmaker.rs`; audit per-item allows.
- [ ] **P4.6 — H9** `ts-rs`/`typeshare` for protocol types.

### Phase 5 — Break up the Room god object (still big, still unchanged)

- [ ] **P5.1 — C5** Split `Room` into `SeatManager` / `LobbyChat` / `RejoinTracker` / `GameSession` / `SessionScorer` / `BotDriver` / `Broadcaster`.
- [ ] **P5.2 — H5** Consolidate mutexes (one `RwLock<RoomInner>`, or actor pattern).
- [ ] **P5.3 — H7** Persistent bot task driven by `tokio::sync::Notify`; remove the per-action spawns.
- [x] **P5.4 — H8** `GameState::redacted_for(seat, game)`; `Game::visible_extra_piles(state, seat)`; collapse both call sites. (Resolved in this branch; was Phase 5 but landed early as it had no dependencies on the larger Room split.)

### Phase 6 — Polish (unchanged from original)

- [ ] **P6.1 — M9** Centralize magic timeouts (server `config.rs`, client `config.ts`).
- [ ] **P6.2 — M1** Factor Sheepshead's schneider scoring helpers.
- [ ] **P6.3 — L1** WebSocket reconnect with backoff.
- [ ] **P6.4 — L2** CSS custom properties / design tokens. (Big leverage now that `<GameTableShell>` will have one CSS surface instead of two.)
- [ ] **P6.5 — L3** Accessibility pass.
- [ ] **P6.6 — L4-L10** small cleanups.
- [ ] **P6.7 — L7** Test coverage for `Room` paths.

### Resolution Summary

| Finding | Original Status | Post-Merge Status |
|---------|----------------|-------------------|
| C1 | Critical | ✅ Resolved |
| C2 | Critical | ⚠️ Mostly resolved (trickWinner index dead code remains) |
| C3 | Critical | ❌ Open (worse — Euchre adds more untyped accesses) |
| C4 | Critical | ⚠️ Half resolved (per-game modules; string dispatch remains, see N2) |
| C5 | Critical | ❌ Open |
| H1 | High | ❌ Open (now active Euchre bug, see N3) |
| H2 | High | ❌ Open (asymmetric, see N6) |
| H3 | High | ❌ Critical (duplicated, see N1) |
| H4 | High | ⚠️ Half resolved (Euchre split, Sheepshead not, see N5) |
| H5 | High | ❌ Open |
| H6 | Medium-High | ❌ Open (active bug, see N3) |
| H7 | High | ❌ Open |
| H8 | Medium-High | ✅ Resolved |
| H9 | High | ❌ Open |
| H10 | Medium-High | ❌ Open (plural now) |
| M1 | Medium | ❌ Open |
| M2 | Medium | ❌ Open (plural now) |
| M3 | Medium | ❌ Open (plural now) |
| M4 | Medium | ❌ Open |
| M5 | Medium | ❌ Open |
| M6 | Medium | ❌ Open (now a potential Euchre bug, see N8) |
| M7 | Medium | ⚠️ Partial (Euchre clean, Sheepshead not, see N4) |
| M8 | Medium | ⚠️ Half resolved (Euchre clean, Sheepshead not) |
| M9 | Medium | ❌ Open (plural now) |
| M10 | Medium | ✅ Resolved |
| N1 | — | 🆕 Critical (GameTable cloned) |
| N2 | — | 🆕 High (string dispatch in games::mod) |
| N3 | — | 🆕 High (active Euchre config bug) |
| N4 | — | 🆕 Medium (Sheepshead trump still in engine/) |
| N5 | — | 🆕 High (Sheepshead BiddingPanel not split) |
| N6 | — | 🆕 High (Sheepshead state still in useGameStore) |
| N7 | — | 🆕 Medium (two-layer bot dispatch shim) |
| N8 | — | 🆕 Medium-High (Sheepshead point_value used for Euchre sluffs) |

**One-sentence verdict:** The Euchre work made real progress on the critical findings (C1 ✅, C2 ⚠️, C4 ⚠️) but introduced cross-game asymmetry — Sheepshead is now the laggard for every pattern Euchre established correctly — and the GameTable was duplicated rather than decomposed, which is now the biggest barrier to adding a third game. The next sprint should be Phase 0 + Phase 1 + Phase 2 (active bug fixes + bring Sheepshead up to Euchre's structure + un-duplicate GameTable) before adding Hearts or Spades.

---

## What I'm Deliberately Not Recommending

A few things I considered and rejected as scope creep or premature:

- **Replacing the Pinia store with an event-sourced log of `StateUpdate`s.** Tempting for time-travel debugging, but adds complexity the current product doesn't need.
- **Switching `Room` to an actor pattern as a hard requirement.** It's a clean fix for H5 but heavier than a consolidated lock — leave it as an option if locking gets unwieldy after Phase 4.
- **Persisting game state to a database.** The product is ephemeral by design; persistence is a feature decision, not a refactor.
- **Replacing JSON with binary protocol or compact card encoding.** No measured perf issue yet; defer until there's pressure.
- **Adding integration tests over the WebSocket protocol.** Worth doing eventually, but the per-unit tests cover most paths once Phase 2 collapses the duplicated state.
