# Multiplayer Lobbies Design

**Date:** 2026-05-15  
**Status:** Approved  
**Scope:** Private rooms with room codes, public matchmaking, lobby chat, bot backfill, disconnection/rejoin handling, guest identity

## Context

The platform currently supports solo play (one human + four bots). This spec adds real multiplayer: humans join rooms and bots fill empty seats. Solo mode is unchanged — multiplayer is purely additive.

---

## 1. Room Lifecycle

Rooms now have a pre-game lobby phase:

```
Lobby → [Countdown] → Bidding → Playing → Scoring → Done
```

**Lobby:** Players join, chat, and see the live seat list. The game has not started; hands are not dealt.

**Countdown:** A brief delay before the game starts. For private rooms the host triggers it (or starts immediately). For public rooms it fires automatically.

**Done:** Room is removed from the registry when the session ends and all WebSocket connections close, or after a 10-minute idle timeout in lobby.

**Room registry:** `HashMap<String, Arc<Room>>` in `AppState`, keyed by a short room code (two words + two digits, e.g. `"WOLF-42"`; ~1M combinations). Room codes are generated server-side on creation.

---

## 2. Player Identity & Seat Model

### Guest identity

On first visit the client prompts for a display name. The name is stored in `localStorage` as `guestName` and sent to the server on every WebSocket join handshake. No accounts or passwords required.

### Seat states

Each of the 5 seats is one of:

| State | Description |
|-------|-------------|
| `Empty` | Not yet claimed |
| `Human { name, ws_id }` | Connected player |
| `Bot` | Filled by bot engine |
| `Disconnected { name, rejoin_deadline }` | Human dropped; within rejoin window |

Seat assignment is server-authoritative. Humans claim the first empty seat on connect.

### Host

The first human in a private room is the host. If the host disconnects, host role passes to the next connected human. Public rooms have no host.

### WebSocket handshake

Client sends on connect:
```json
{ "type": "join", "name": "Alice", "room_code": "WOLF-42" }
```

Server responds with a `Snapshot` containing the full seat list, lobby chat history, and lobby metadata.

---

## 3. New StateUpdate Variants

```rust
/// A player sent a lobby chat message.
LobbyChat { from: String, text: String, timestamp: u64 },

/// The seat list changed (join, leave, bot fill, reconnect).
SeatUpdate { seats: Vec<SeatInfo> },

/// Player joined the matchmaking queue.
QueueStatus { position: usize, waiting_since: u64 },
```

Where:
```rust
pub struct SeatInfo {
    pub seat: usize,
    pub state: String,   // "empty" | "human" | "bot" | "disconnected"
    pub name: Option<String>,
}
```

`LobbyChat` and `SeatUpdate` are broadcast to all seats in the room. Chat history (last 50 messages) is stored in the room and replayed in the initial `Snapshot`.

---

## 4. GameState Lobby Phase

`GamePhase` gains a `Lobby` variant. In lobby phase:
- `hands` are empty
- `current_player` is unused
- `meta` carries: `host_seat`, `countdown_ends_at` (Unix ms, null if not started), `room_type` (`"private"` | `"public"`), `max_hands` (null for play-to-threshold, integer for fixed-hand sessions)

The `GameState.names` field (already present) is populated from seat names in lobby and carries into the game.

---

## 5. Client Actions (new message types)

| Action | Sender | Payload |
|--------|--------|---------|
| `join` | Client | `{ name, room_code }` |
| `lobby_chat` | Client | `{ text }` (max 200 chars, non-empty) |
| `start_game` | Host | `{}` — starts countdown / fills bots immediately |
| `force_bot` | Host | `{ seat }` — skips rejoin window for a disconnected seat |
| `extend_rejoin` | Host | `{ seat }` — extends window 30s (once per player per game) |
| `join_queue` | Client | `{}` — joins public matchmaking |
| `leave_queue` | Client | `{}` — leaves queue |

---

## 6. Matchmaking Queue

A `Matchmaker` struct is held in `AppState` alongside the room registry.

**Flow:**
1. Player sends `join_queue`; server adds them to the queue and responds with `QueueStatus`
2. If queue reaches 5: create room immediately, assign all 5, game starts in ~3 seconds (short countdown)
3. If 60 seconds elapse with 2–4 players: create room with current humans, fill remaining seats with bots
4. Server sends `JoinedRoom` to each queued player when their room is created
5. `leave_queue` or WebSocket disconnect removes the player from the queue

**Public room session mode:** Fixed 8 hands (`max_hands: 8`). Session ends after 8 hands; final standings broadcast.

**Private room session mode:** Host chooses at creation — play-to-threshold (existing behavior, `max_hands: null`) or fixed hands (4 / 8 / 12).

---

## 7. Disconnection & Rejoin

### During play

1. `on_close` fires → seat marked `Disconnected { rejoin_deadline: now + 30s }`
2. `SeatUpdate` broadcast + system `LobbyChat`: `"Alice disconnected — 30 seconds to rejoin."`
3. If player reconnects within window: seat restored to `Human`, `Snapshot` sent, system chat: `"Alice rejoined."`
4. If window expires: seat flips to `Bot`, system chat: `"Alice's hand has been taken over by a bot."`

**Rejoin matching:** name + room_code (no token). Names must be unique within a room — the server rejects a join handshake if the name is already taken by a connected seat, returning an `Error` message prompting the player to choose a different name. A `Disconnected` seat's name is reserved for rejoin; a new player cannot claim it.

**Host overrides (private rooms only):**
- "Bot now" button → `force_bot` skips remainder of window
- "Extend" button → `extend_rejoin` adds 30s, usable once per player per game

### During lobby

Disconnected seats return to `Empty` immediately — no rejoin window.

---

## 8. Client UI

### Home screen

- Guest name prompt (if no `guestName` in localStorage) — shown above the three action buttons
- **Create Private Room** → session mode picker (play-to-threshold or fixed hands 4/8/12) → lobby screen
- **Find a Game** → joins matchmaking queue → queue screen (position, estimated wait, cancel)
- **Join with Code** → text input for room code → lobby screen

### Lobby screen (new)

- Seat rail: 5 seats showing name / "Empty" / "Bot" — updates live via `SeatUpdate`
- Room code top-right with copy button (private rooms only)
- Chat panel: scrolling message list + text input
- **Private:** host sees "Start Now" button + optional countdown timer (30s / 60s / 120s); non-hosts see "Waiting for host to start…"
- **Public:** countdown timer visible to all once fired; no host controls

### In-game additions

- Disconnection toast when a player drops mid-game (reuses existing phase toast pattern)
- Host sees "Bot now" / "Extend" buttons on disconnected seat (shown in seat rail overlay)

### Unchanged

`GameTable.vue`, `BiddingPanel.vue`, `TrickDisplay.vue`, `HandComponent.vue` — no changes.

---

## 9. Files Modified / Created

| File | Change |
|------|--------|
| `server/src/engine/state.rs` | Add `Lobby` to `GamePhase`; add `LobbyChat`, `SeatUpdate`, `QueueStatus` to `StateUpdate`; add `SeatInfo` struct |
| `server/src/lobby/room.rs` | Lobby phase state; seat model; host tracking; disconnect/rejoin timer; `force_bot` / `extend_rejoin` handlers; `lobby_chat` handler; `start_game` handler |
| `server/src/lobby/matchmaker.rs` | **New** — `Matchmaker` struct; queue management; room creation on fill or timeout |
| `server/src/lobby/mod.rs` | Export `matchmaker` module |
| `server/src/ws/handler.rs` | Route new client action types; pass `Matchmaker` ref from `AppState` |
| `server/src/main.rs` | Add `Matchmaker` to `AppState` |
| `client/src/engine/types.ts` | Add `Lobby` phase; add `LobbyChat`, `SeatUpdate`, `QueueStatus` variants; add `SeatInfo` type |
| `client/src/stores/game.ts` | Handle new update variants; expose `seats`, `lobbyChat`, `isLobby`, `queueStatus` |
| `client/src/views/HomeView.vue` | **New** — name prompt, three action buttons |
| `client/src/views/LobbyView.vue` | **New** — seat rail, chat, start controls, room code |
| `client/src/views/QueueView.vue` | **New** — queue status, cancel |
| `client/src/router/index.ts` | Add routes for lobby, queue views |

---

## 10. Verification

- **Server unit tests:** seat state transitions; rejoin window timer fires correctly; matchmaking queue fills and times out; `force_bot` and `extend_rejoin` logic; `lobby_chat` validation (empty / too long)
- **Client unit tests:** store handles `SeatUpdate`, `LobbyChat`, `QueueStatus`; `isLobby` computed; `lobbyChat` history
- **Manual smoke:**
  - Open two browser tabs, create private room in one, join with code in other → both see each other in seat list
  - Chat between tabs in lobby
  - Host starts game → both play as humans, bots fill remaining 3 seats
  - Disconnect one tab mid-game → disconnection toast + 30s countdown → bot takes over
  - Reconnect within window → player resumes their hand
  - Public matchmaking: two tabs both click "Find a Game" → matched into same room with 3 bots
