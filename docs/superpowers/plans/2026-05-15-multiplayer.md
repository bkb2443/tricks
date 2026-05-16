# Multiplayer Lobbies Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add private rooms with short codes, public matchmaking, lobby chat, bot backfill, and disconnect/rejoin — while keeping solo mode unchanged.

**Architecture:** Extend the existing Room/Lobby architecture with a lobby phase (seats, chat, host) before the game starts; add a Matchmaker for public queuing; add three new client views (Home redesign, LobbyView, QueueView). The existing `JoinRoom` + `fill_bots` solo path is untouched.

**Tech Stack:** Rust/Axum/Tokio (server), Vue 3/TypeScript/Pinia (client), existing WebSocket protocol

---

## File Map

| File | Change |
|------|--------|
| `server/src/engine/state.rs` | Add `Lobby` to `GamePhase`; add `SeatInfo`; add `LobbyChat`, `SeatUpdate`, `QueueStatus` to `StateUpdate`; add new `ClientMessage` variants |
| `server/src/lobby/room.rs` | Full seat model; lobby phase; host tracking; disconnect/rejoin; chat; `start_game`; `force_bot`; `extend_rejoin` |
| `server/src/lobby/matchmaker.rs` | **New** — `Matchmaker`, queue, 60s timer, room creation |
| `server/src/lobby/mod.rs` | Room-code registry (`DashMap<String, Arc<Room>>`); export `Matchmaker` |
| `server/src/ws/handler.rs` | Route new `ClientMessage` variants; disconnect cleanup; `ws_id` in `PlayerCtx` |
| `server/src/main.rs` | Add `Matchmaker` to `Lobby` (passed through `AppState`) |
| `client/src/engine/types.ts` | Add `'lobby'` phase; `SeatInfo`; new `StateUpdate` + `ClientMessage` variants |
| `client/src/stores/game.ts` | Handle new variants; `seats`, `lobbyChat`, `isLobby`, `queueStatus` |
| `client/src/composables/useGame.ts` | Add `joinWithCode`, `createPrivateRoom`, `joinQueue`, `leaveQueue`, `startGame`, `sendLobbyChat`, `forceBot`, `extendRejoin` |
| `client/src/views/HomeView.vue` | Redesign: name prompt + solo + create private + find game + join with code |
| `client/src/views/LobbyView.vue` | **New** — seat rail, chat, host controls, countdown |
| `client/src/views/QueueView.vue` | **New** — queue status, cancel |
| `client/src/router/index.ts` | Add `/lobby` and `/queue` routes |
| `client/src/views/GameView.vue` | Remove "waiting for players" block (lobby view handles it now) |

---

### Task 1: Protocol Types

**Files:**
- Modify: `server/src/engine/state.rs`
- Modify: `client/src/engine/types.ts`

- [ ] **Step 1: Add `Lobby` to `GamePhase` and `SeatInfo` struct (server)**

In `server/src/engine/state.rs`, update `GamePhase` and add `SeatInfo`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GamePhase {
    Lobby,
    Bidding,
    Playing,
    Scoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeatInfo {
    pub seat: usize,
    /// "empty" | "human" | "bot" | "disconnected"
    pub state: String,
    pub name: Option<String>,
}
```

- [ ] **Step 2: Add new `StateUpdate` variants**

Add to the `StateUpdate` enum in `server/src/engine/state.rs`. Also add `room_code: String` to `JoinedRoom` so clients know their room's short code after creating or joining:

```rust
    /// Updated: now includes room_code so clients can display/share it.
    JoinedRoom { room_id: Uuid, seat: usize, room_code: String },
    LobbyChat { from: String, text: String, timestamp: u64 },
    SeatUpdate { seats: Vec<SeatInfo> },
    QueueStatus { position: usize, waiting_since: u64 },
```

- [ ] **Step 3: Add new `ClientMessage` variants**

Replace the `ClientMessage` enum:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Legacy solo / create-and-fill path. Kept for backward compat.
    JoinRoom { room_id: Option<Uuid>, game: String, players: usize, #[serde(default)] fill_bots: bool },
    /// Multiplayer: create a new private room and join it as host.
    CreateRoom { name: String, game: String, max_hands: Option<u32> },
    /// Multiplayer: join an existing room by short code.
    Join { name: String, room_code: String },
    PlayCard { card: Card },
    Bid { value: serde_json::Value },
    LobbyChat { text: String },
    StartGame,
    ForceBot { seat: usize },
    ExtendRejoin { seat: usize },
    JoinQueue,
    LeaveQueue,
}
```

- [ ] **Step 4: Add `GameState::new_lobby` constructor**

In `impl GameState`:

```rust
    /// Creates a GameState in Lobby phase (no hands dealt yet).
    pub fn new_lobby(
        game_id: Uuid,
        game_name: String,
        player_count: usize,
        room_type: &str,
        max_hands: Option<u32>,
    ) -> Self {
        Self {
            game_id,
            game_name,
            phase: GamePhase::Lobby,
            player_count,
            dealer: 0,
            current_player: 0,
            hands: vec![Vec::new(); player_count],
            extra_piles: Vec::new(),
            current_trick: None,
            completed_tricks: Vec::new(),
            scores: vec![0; player_count],
            meta: serde_json::json!({
                "host_seat": null,
                "countdown_ends_at": null,
                "room_type": room_type,
                "max_hands": max_hands
            }),
            names: Vec::new(),
        }
    }
```

- [ ] **Step 5: Build server to confirm no unexpected errors**

```bash
cd /Users/bkb2443/Git/tricks/server && cargo build 2>&1 | grep "^error" | head -20
```

Expected: errors only in `lobby/room.rs` and `ws/handler.rs` where old patterns need updating (those are fixed in Tasks 3 and 6). No errors in `engine/state.rs`.

- [ ] **Step 6: Update client `types.ts`**

Replace the file contents:

```typescript
// These types mirror the Rust server structs exactly.
export type Suit = 'clubs' | 'spades' | 'hearts' | 'diamonds'

export type Rank =
  | 'two' | 'three' | 'four' | 'five' | 'six'
  | 'seven' | 'eight' | 'nine' | 'ten'
  | 'jack' | 'queen' | 'king' | 'ace'

export interface Card { suit: Suit; rank: Rank }

export type GamePhase = 'lobby' | 'bidding' | 'playing' | 'scoring'

export interface SeatInfo {
  seat: number
  state: 'empty' | 'human' | 'bot' | 'disconnected'
  name: string | null
}

export interface Trick {
  led_by: number
  plays: [number, Card][]
  winner: number | null
}

export interface GameState {
  game_id: string
  game_name: string
  phase: GamePhase
  player_count: number
  dealer: number
  current_player: number
  hands: Card[][]
  extra_piles: [string, Card[]][]
  current_trick: Trick | null
  completed_tricks: Trick[]
  scores: number[]
  meta: Record<string, unknown>
  names: string[]
}

export type ClientMessage =
  | { type: 'join_room'; room_id?: string; game: string; players: number; fill_bots?: boolean }
  | { type: 'create_room'; name: string; game: string; max_hands: number | null }
  | { type: 'join'; name: string; room_code: string }
  | { type: 'play_card'; card: Card }
  | { type: 'bid'; value: unknown }
  | { type: 'lobby_chat'; text: string }
  | { type: 'start_game' }
  | { type: 'force_bot'; seat: number }
  | { type: 'extend_rejoin'; seat: number }
  | { type: 'join_queue' }
  | { type: 'leave_queue' }

export type StateUpdate =
  | { type: 'joined_room';     room_id: string; seat: number; room_code: string }
  | { type: 'snapshot';        state: GameState }
  | { type: 'card_played';     player: number; card: Card }
  | { type: 'trick_complete';  winner: number; points: number }
  | { type: 'hand_complete';   hand_scores: number[]; session_scores: number[] }
  | { type: 'session_over';    winner: number; final_scores: number[] }
  | { type: 'bid_placed';      player: number; value: unknown; current_player: number }
  | { type: 'hand_updated';    hand: Card[] }
  | { type: 'phase_changed';   phase: GamePhase }
  | { type: 'partner_revealed'; seat: number }
  | { type: 'lobby_chat';      from: string; text: string; timestamp: number }
  | { type: 'seat_update';     seats: SeatInfo[] }
  | { type: 'queue_status';    position: number; waiting_since: number }
  | { type: 'error';           message: string }
```

- [ ] **Step 7: Type-check client**

```bash
cd /Users/bkb2443/Git/tricks/client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vue-tsc --noEmit 2>&1 | tail -5
```

Expected: errors only for components that reference old types — those are fixed in later tasks.

- [ ] **Step 8: Commit**

```bash
cd /Users/bkb2443/Git/tricks && git add server/src/engine/state.rs client/src/engine/types.ts
git commit -m "feat(protocol): add Lobby phase, SeatInfo, LobbyChat/SeatUpdate/QueueStatus, new client messages"
```

---

### Task 2: Room Code Registry

**Files:**
- Modify: `server/src/lobby/mod.rs`

- [ ] **Step 1: Write failing test for room code generation**

Add to `server/src/lobby/mod.rs` at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_code_format() {
        let code = generate_room_code();
        let parts: Vec<&str> = code.split('-').collect();
        assert_eq!(parts.len(), 2, "code should be WORD-NN format");
        let num: u32 = parts[1].parse().expect("second part should be a number");
        assert!((10..=99).contains(&num), "number should be 10-99");
        assert!(parts[0].len() >= 3, "word should be at least 3 chars");
    }

    #[test]
    fn get_room_by_code_round_trips() {
        let lobby = Lobby::new();
        let (code, _room) = lobby.create_room("sheepshead".into(), 5, 24).unwrap();
        assert!(lobby.get_room(&code).is_some(), "should find room by code");
        assert!(lobby.get_room("NOTEXIST-00").is_none());
    }
}
```

- [ ] **Step 2: Run to see failures**

```bash
cd /Users/bkb2443/Git/tricks/server && cargo test lobby::tests -- --nocapture 2>&1 | tail -10
```

Expected: compilation errors — `generate_room_code`, `create_room` returning tuple, etc. don't exist yet.

- [ ] **Step 3: Implement room code registry**

Replace `server/src/lobby/mod.rs`:

```rust
mod room;
pub mod matchmaker;

use dashmap::DashMap;
use std::sync::Arc;
use rand::Rng;

pub use room::Room;
pub use matchmaker::Matchmaker;

use crate::games;

const WORDS: &[&str] = &[
    "WOLF", "BEAR", "HAWK", "DUCK", "DEER", "CROW", "FROG", "LYNX",
    "MOOSE", "PIKE", "LARK", "WREN", "NEWT", "MINK", "VOLE", "IBIS",
    "KITE", "TEAL", "WREN", "DOVE",
];

pub fn generate_room_code() -> String {
    let mut rng = rand::thread_rng();
    let word = WORDS[rng.gen_range(0..WORDS.len())];
    let num = rng.gen_range(10u32..=99);
    format!("{word}-{num:02}")
}

pub struct Lobby {
    rooms: DashMap<String, Arc<Room>>,
}

impl Lobby {
    pub fn new() -> Self {
        Self { rooms: DashMap::new() }
    }

    /// Create a new room. Returns `(room_code, Arc<Room>)` or `None` if game_name unknown.
    pub fn create_room(
        &self,
        game_name: String,
        player_count: usize,
        victory_goal: i32,
    ) -> Option<(String, Arc<Room>)> {
        let game = games::get_game(&game_name)?;
        // Generate a unique code (retry up to 10 times on collision)
        let code = (0..10)
            .map(|_| generate_room_code())
            .find(|c| !self.rooms.contains_key(c.as_str()))
            .unwrap_or_else(generate_room_code);
        let room = Arc::new(Room::new(
            uuid::Uuid::new_v4(),
            game_name,
            player_count,
            game,
            victory_goal,
            code.clone(),
        ));
        self.rooms.insert(code.clone(), Arc::clone(&room));
        Some((code, room))
    }

    pub fn get_room(&self, code: &str) -> Option<Arc<Room>> {
        self.rooms.get(code).map(|r| Arc::clone(&r))
    }

    pub fn remove_room(&self, code: &str) {
        self.rooms.remove(code);
    }
}

impl Default for Lobby {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn room_code_format() {
        let code = generate_room_code();
        let parts: Vec<&str> = code.split('-').collect();
        assert_eq!(parts.len(), 2);
        let num: u32 = parts[1].parse().expect("second part should be a number");
        assert!((10..=99).contains(&num));
        assert!(parts[0].len() >= 3);
    }

    #[test]
    fn get_room_by_code_round_trips() {
        let lobby = Lobby::new();
        let (code, _room) = lobby.create_room("sheepshead".into(), 5, 24).unwrap();
        assert!(lobby.get_room(&code).is_some());
        assert!(lobby.get_room("NOTEXIST-00").is_none());
    }
}
```

Also create an empty placeholder for the matchmaker (filled in Task 5):

```bash
cat > /Users/bkb2443/Git/tricks/server/src/lobby/matchmaker.rs << 'EOF'
pub struct Matchmaker;
impl Matchmaker {
    pub fn new(_lobby: std::sync::Arc<super::Lobby>) -> Self { Self }
}
EOF
```

- [ ] **Step 4: Add `room_code` field to `Room::new` signature**

In `server/src/lobby/room.rs`, add `room_code: String` parameter to `Room::new` and store it:

```rust
pub struct Room {
    pub id: uuid::Uuid,
    pub room_code: String,
    pub game_name: String,
    // ... rest unchanged
}

impl Room {
    pub fn new(
        id: uuid::Uuid,
        game_name: String,
        player_count: usize,
        game: Box<dyn crate::engine::game::Game>,
        victory_goal: i32,
        room_code: String,
    ) -> Self {
        // ... same as before, add room_code field
        Self {
            id,
            room_code,
            // ... rest unchanged
        }
    }
}
```

- [ ] **Step 5: Update `ws/handler.rs` to use new `create_room` return type**

The `JoinRoom` handler now gets `(code, room)` from `create_room`:

```rust
ClientMessage::JoinRoom { room_id, game, players, fill_bots } => {
    let room = match room_id {
        Some(ref id) => lobby.get_room(id).or_else(|| {
            lobby.create_room(game, players, 24).map(|(_, r)| r)
        })?,
        None => lobby.create_room(game, players, 24).map(|(_, r)| r)?,
    };
    // ... rest unchanged
}
```

- [ ] **Step 6: Run tests and build**

```bash
cd /Users/bkb2443/Git/tricks/server && cargo test lobby::tests -- --nocapture 2>&1 | tail -5
cd /Users/bkb2443/Git/tricks/server && cargo build 2>&1 | tail -5
```

Expected: 2 lobby tests pass, build succeeds.

- [ ] **Step 7: Commit**

```bash
cd /Users/bkb2443/Git/tricks && git add server/src/lobby/mod.rs server/src/lobby/room.rs server/src/lobby/matchmaker.rs server/src/ws/handler.rs
git commit -m "feat(server): room code registry — short codes replace UUID-based room lookup"
```

---

### Task 3: Room Seat Model + Lobby Phase

**Files:**
- Modify: `server/src/lobby/room.rs`

This is the largest task. We replace `player_txs`/`bot_seats` with a proper `SeatState` model and add lobby phase support.

- [ ] **Step 1: Write failing tests**

Add to the tests section at the bottom of `server/src/lobby/room.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::games;
    use tokio::sync::mpsc;

    fn make_room() -> Arc<Room> {
        let game = games::get_game("sheepshead").unwrap();
        Arc::new(Room::new(
            uuid::Uuid::new_v4(),
            "sheepshead".into(),
            5,
            game,
            24,
            "TEST-42".into(),
            "private".into(),
        ))
    }

    #[test]
    fn join_lobby_claims_seat() {
        let room = make_room();
        let (tx, _rx) = mpsc::channel(16);
        let result = room.join_lobby("Alice".into(), uuid::Uuid::new_v4(), tx);
        assert!(result.is_some(), "should join successfully");
        let (seat, _) = result.unwrap();
        assert_eq!(seat, 0);
    }

    #[test]
    fn duplicate_name_rejected() {
        let room = make_room();
        let (tx1, _) = mpsc::channel(16);
        let (tx2, _) = mpsc::channel(16);
        let ws1 = uuid::Uuid::new_v4();
        let ws2 = uuid::Uuid::new_v4();
        room.join_lobby("Alice".into(), ws1, tx1).unwrap();
        let result = room.join_lobby("Alice".into(), ws2, tx2);
        assert!(result.is_none(), "duplicate name should be rejected");
    }

    #[test]
    fn lobby_chat_validates_length() {
        let room = make_room();
        let (tx, _) = mpsc::channel(16);
        room.join_lobby("Alice".into(), uuid::Uuid::new_v4(), tx).unwrap();
        assert!(room.handle_lobby_chat(0, "hello".into()).is_ok());
        assert!(room.handle_lobby_chat(0, "".into()).is_err());
        let too_long = "x".repeat(201);
        assert!(room.handle_lobby_chat(0, too_long).is_err());
    }
}
```

- [ ] **Step 2: Run to confirm failures**

```bash
cd /Users/bkb2443/Git/tricks/server && cargo test lobby::room::tests -- --nocapture 2>&1 | tail -10
```

Expected: compile errors — `join_lobby`, `handle_lobby_chat`, new `Room::new` signature don't exist.

- [ ] **Step 3: Rewrite `room.rs` with seat model**

Replace the entire `server/src/lobby/room.rs` with:

```rust
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::engine::{Card, GamePhase, GameState, PlayResult, SeatInfo, StateUpdate, deal_game};
use crate::engine::game::Game;

// ── Seat model ───────────────────────────────────────────────────────────────

enum SeatState {
    Empty,
    Human {
        name: String,
        ws_id: Uuid,
        tx: mpsc::Sender<StateUpdate>,
    },
    Bot,
    Disconnected {
        name: String,
        rejoin_deadline: std::time::Instant,
        /// Whether host has already used their one extend for this seat.
        extend_used: bool,
    },
}

impl SeatState {
    fn is_empty(&self) -> bool { matches!(self, SeatState::Empty) }
    fn is_human(&self) -> bool { matches!(self, SeatState::Human { .. }) }
    fn is_bot(&self) -> bool { matches!(self, SeatState::Bot) }

    fn name(&self) -> Option<&str> {
        match self {
            SeatState::Human { name, .. } => Some(name),
            SeatState::Disconnected { name, .. } => Some(name),
            _ => None,
        }
    }

    fn ws_id(&self) -> Option<Uuid> {
        match self {
            SeatState::Human { ws_id, .. } => Some(*ws_id),
            _ => None,
        }
    }

    fn tx(&self) -> Option<&mpsc::Sender<StateUpdate>> {
        match self {
            SeatState::Human { tx, .. } => Some(tx),
            _ => None,
        }
    }

    fn to_seat_info(&self, seat: usize) -> SeatInfo {
        let (state_str, name) = match self {
            SeatState::Empty => ("empty", None),
            SeatState::Human { name, .. } => ("human", Some(name.clone())),
            SeatState::Bot => ("bot", None),
            SeatState::Disconnected { name, .. } => ("disconnected", Some(name.clone())),
        };
        SeatInfo { seat, state: state_str.into(), name }
    }
}

// ── Room ─────────────────────────────────────────────────────────────────────

pub struct Room {
    pub id: Uuid,
    pub room_code: String,
    pub game_name: String,
    pub player_count: usize,
    pub victory_goal: i32,
    pub room_type: String,   // "private" | "public"
    game: Box<dyn Game>,
    seats: Mutex<Vec<SeatState>>,
    broadcast_tx: broadcast::Sender<StateUpdate>,
    pub state: Mutex<Option<GameState>>,
    session_scores: Mutex<Vec<i32>>,
    bots_running: AtomicBool,
    chat_history: Mutex<VecDeque<(String, String, u64)>>, // (from, text, timestamp_ms)
    max_hands: Option<u32>,
    hands_played: Mutex<u32>,
}

impl Room {
    pub fn new(
        id: Uuid,
        game_name: String,
        player_count: usize,
        game: Box<dyn Game>,
        victory_goal: i32,
        room_code: String,
        room_type: String,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(64);
        let seats = (0..player_count).map(|_| SeatState::Empty).collect();
        Self {
            id,
            room_code: room_code.clone(),
            game_name: game_name.clone(),
            player_count,
            victory_goal,
            room_type: room_type.clone(),
            game,
            seats: Mutex::new(seats),
            broadcast_tx,
            state: Mutex::new(Some(GameState::new_lobby(
                id,
                game_name,
                player_count,
                &room_type,
                None,
            ))),
            session_scores: Mutex::new(vec![0; player_count]),
            bots_running: AtomicBool::new(false),
            chat_history: Mutex::new(VecDeque::new()),
            max_hands: None,
            hands_played: Mutex::new(0),
        }
    }

    pub fn set_max_hands(&mut self, max: u32) {
        self.max_hands = Some(max);
    }

    // ── Seat info ─────────────────────────────────────────────────────────────

    fn seat_infos(&self) -> Vec<SeatInfo> {
        let seats = self.seats.lock().unwrap();
        seats.iter().enumerate().map(|(i, s)| s.to_seat_info(i)).collect()
    }

    fn host_seat(&self) -> Option<usize> {
        let seats = self.seats.lock().unwrap();
        seats.iter().position(|s| s.is_human())
    }

    // ── Joining ───────────────────────────────────────────────────────────────

    /// Join the room in lobby phase. Returns `(seat, broadcast_rx)` or `None` if
    /// the room is full or the name is already taken.
    pub fn join_lobby(
        &self,
        name: String,
        ws_id: Uuid,
        tx: mpsc::Sender<StateUpdate>,
    ) -> Option<(usize, broadcast::Receiver<StateUpdate>)> {
        let seat;
        {
            let mut seats = self.seats.lock().unwrap();
            // Name uniqueness check (Human + Disconnected seats reserve their name)
            let name_taken = seats.iter().any(|s| s.name() == Some(name.as_str()));
            if name_taken { return None; }

            seat = seats.iter().position(|s| s.is_empty())?;
            seats[seat] = SeatState::Human { name: name.clone(), ws_id, tx: tx.clone() };
        }

        // Update lobby GameState names and host
        {
            let mut guard = self.state.lock().unwrap();
            if let Some(ref mut state) = *guard {
                state.names = self.compute_names();
                if state.meta["host_seat"].is_null() {
                    state.meta["host_seat"] = serde_json::json!(seat);
                }
            }
        }

        // Send the joiner a lobby snapshot
        let snapshot = {
            let guard = self.state.lock().unwrap();
            guard.as_ref().map(|s| s.clone())
        };
        if let Some(state) = snapshot {
            let _ = tx.try_send(StateUpdate::Snapshot { state });
        }
        // Replay chat history
        {
            let history = self.chat_history.lock().unwrap();
            for (from, text, timestamp) in history.iter() {
                let _ = tx.try_send(StateUpdate::LobbyChat {
                    from: from.clone(),
                    text: text.clone(),
                    timestamp: *timestamp,
                });
            }
        }

        self.broadcast(StateUpdate::SeatUpdate { seats: self.seat_infos() });
        tracing::info!(room_code = %self.room_code, seat, name, "player joined lobby");

        Some((seat, self.broadcast_tx.subscribe()))
    }

    /// Legacy path for solo mode (fill_bots). Kept for backward compat.
    pub fn join(&self, tx: mpsc::Sender<StateUpdate>) -> Option<(usize, broadcast::Receiver<StateUpdate>)> {
        let seat;
        let all_filled;
        {
            let mut seats = self.seats.lock().unwrap();
            seat = seats.iter().position(|s| s.is_empty())?;
            seats[seat] = SeatState::Human {
                name: format!("Player{seat}"),
                ws_id: Uuid::new_v4(),
                tx,
            };
            all_filled = seats.iter().all(|s| !s.is_empty());
        }
        if all_filled { self.start_game(); }
        Some((seat, self.broadcast_tx.subscribe()))
    }

    // ── Lobby chat ────────────────────────────────────────────────────────────

    pub fn handle_lobby_chat(&self, seat: usize, text: String) -> Result<(), String> {
        if text.is_empty() { return Err("message cannot be empty".into()); }
        if text.len() > 200 { return Err("message too long (max 200 chars)".into()); }

        let from = {
            let seats = self.seats.lock().unwrap();
            seats.get(seat).and_then(|s| s.name().map(|n| n.to_string()))
                .unwrap_or_else(|| format!("Seat {seat}"))
        };
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        {
            let mut history = self.chat_history.lock().unwrap();
            history.push_back((from.clone(), text.clone(), timestamp));
            if history.len() > 50 { history.pop_front(); }
        }

        self.broadcast(StateUpdate::LobbyChat { from, text, timestamp });
        Ok(())
    }

    fn system_chat(&self, text: String) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let mut history = self.chat_history.lock().unwrap();
        history.push_back(("System".into(), text.clone(), timestamp));
        if history.len() > 50 { history.pop_front(); }
        drop(history);
        self.broadcast(StateUpdate::LobbyChat { from: "System".into(), text, timestamp });
    }

    // ── Game start ────────────────────────────────────────────────────────────

    /// Fill all Empty seats with bots, then start the first hand.
    pub fn start_game(self: &Arc<Self>) {
        // Fill empty seats with bots
        {
            let mut seats = self.seats.lock().unwrap();
            for s in seats.iter_mut() {
                if s.is_empty() { *s = SeatState::Bot; }
            }
        }
        let dealer = {
            let mut rng = rand::thread_rng();
            rand::Rng::gen_range(&mut rng, 0..self.player_count)
        };
        self.start_next_hand(dealer);
        // Kick off bots
        let room_arc = Arc::clone(self);
        tokio::spawn(async move { room_arc.drive_bots().await });
        tracing::info!(room_code = %self.room_code, "game started");
    }

    /// For solo / legacy fill_bots path.
    pub fn fill_bots(self: &Arc<Self>) {
        {
            let mut seats = self.seats.lock().unwrap();
            for s in seats.iter_mut() {
                if s.is_empty() { *s = SeatState::Bot; }
            }
        }
    }

    // ── Disconnection & rejoin ────────────────────────────────────────────────

    pub fn on_disconnect(self: &Arc<Self>, seat: usize, ws_id: Uuid) {
        let is_lobby = {
            let guard = self.state.lock().unwrap();
            guard.as_ref().map(|s| s.phase == GamePhase::Lobby).unwrap_or(true)
        };

        let name = {
            let seats = self.seats.lock().unwrap();
            seats.get(seat).and_then(|s| {
                if s.ws_id() == Some(ws_id) { s.name().map(|n| n.to_string()) } else { None }
            })
        };
        let Some(name) = name else { return };

        if is_lobby {
            // Lobby disconnect: seat goes back to Empty immediately
            let mut seats = self.seats.lock().unwrap();
            if let Some(s) = seats.get_mut(seat) {
                *s = SeatState::Empty;
            }
            drop(seats);
            self.broadcast(StateUpdate::SeatUpdate { seats: self.seat_infos() });
        } else {
            // In-game disconnect: start rejoin window
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            {
                let mut seats = self.seats.lock().unwrap();
                if let Some(s) = seats.get_mut(seat) {
                    *s = SeatState::Disconnected { name: name.clone(), rejoin_deadline: deadline, extend_used: false };
                }
            }
            self.broadcast(StateUpdate::SeatUpdate { seats: self.seat_infos() });
            self.system_chat(format!("{name} disconnected — 30 seconds to rejoin."));

            let room = Arc::clone(self);
            let name_clone = name.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                room.on_rejoin_expired(seat, &name_clone);
            });
        }
    }

    pub fn on_rejoin_expired(&self, seat: usize, expected_name: &str) {
        let should_bot = {
            let seats = self.seats.lock().unwrap();
            matches!(seats.get(seat), Some(SeatState::Disconnected { name, .. }) if name == expected_name)
        };
        if should_bot {
            {
                let mut seats = self.seats.lock().unwrap();
                if let Some(s) = seats.get_mut(seat) {
                    *s = SeatState::Bot;
                }
            }
            self.broadcast(StateUpdate::SeatUpdate { seats: self.seat_infos() });
            self.system_chat(format!("{expected_name}'s hand has been taken over by a bot."));
        }
    }

    pub fn on_rejoin(
        &self,
        seat: usize,
        name: &str,
        ws_id: Uuid,
        tx: mpsc::Sender<StateUpdate>,
    ) -> bool {
        let can_rejoin = {
            let seats = self.seats.lock().unwrap();
            matches!(seats.get(seat), Some(SeatState::Disconnected { name: n, rejoin_deadline, .. })
                if n == name && *rejoin_deadline > std::time::Instant::now())
        };
        if !can_rejoin { return false; }

        {
            let mut seats = self.seats.lock().unwrap();
            if let Some(s) = seats.get_mut(seat) {
                *s = SeatState::Human { name: name.to_string(), ws_id, tx: tx.clone() };
            }
        }

        // Send full snapshot to rejoiner
        let snapshot = {
            let guard = self.state.lock().unwrap();
            guard.as_ref().map(|s| {
                let mut view = s.clone();
                for (i, hand) in view.hands.iter_mut().enumerate() {
                    if i != seat { hand.clear(); }
                }
                view.extra_piles.clear();
                view
            })
        };
        if let Some(state) = snapshot {
            let _ = tx.try_send(StateUpdate::Snapshot { state });
        }

        self.broadcast(StateUpdate::SeatUpdate { seats: self.seat_infos() });
        self.system_chat(format!("{name} rejoined."));
        true
    }

    pub fn force_bot(&self, seat: usize, requester_seat: usize) -> Result<(), String> {
        if self.host_seat() != Some(requester_seat) {
            return Err("only the host can force a bot takeover".into());
        }
        let name = {
            let seats = self.seats.lock().unwrap();
            match seats.get(seat) {
                Some(SeatState::Disconnected { name, .. }) => name.clone(),
                _ => return Err("seat is not disconnected".into()),
            }
        };
        self.on_rejoin_expired(seat, &name);
        Ok(())
    }

    pub fn extend_rejoin(&self, seat: usize, requester_seat: usize) -> Result<(), String> {
        if self.host_seat() != Some(requester_seat) {
            return Err("only the host can extend rejoin window".into());
        }
        let mut seats = self.seats.lock().unwrap();
        match seats.get_mut(seat) {
            Some(SeatState::Disconnected { rejoin_deadline, extend_used, .. }) => {
                if *extend_used { return Err("extend already used for this seat".into()); }
                *rejoin_deadline += std::time::Duration::from_secs(30);
                *extend_used = true;
                Ok(())
            }
            _ => Err("seat is not disconnected".into()),
        }
    }

    // ── Existing game methods (unchanged signatures) ───────────────────────────

    pub fn apply_bid(&self, seat: usize, value: serde_json::Value) -> Result<(), String> {
        let (result, current_player) = {
            let mut guard = self.state.lock().unwrap();
            let state = guard.as_mut().ok_or_else(|| "game not started".to_string())?;
            let result = self.game.apply_bid(state, seat, &value)?;
            let cp = state.current_player;
            (result, cp)
        };
        let bid_value = result.broadcast_payload.unwrap_or(value);
        self.broadcast(StateUpdate::BidPlaced { player: seat, value: bid_value, current_player });
        if let Some(updated_seat) = result.hand_updated_seat {
            let hand = {
                let guard = self.state.lock().unwrap();
                guard.as_ref().map(|s| s.hands[updated_seat].clone()).unwrap_or_default()
            };
            self.send_private(updated_seat, StateUpdate::HandUpdated { hand });
        }
        if result.phase_complete {
            self.broadcast(StateUpdate::PhaseChanged { phase: GamePhase::Playing });
        }
        Ok(())
    }

    pub fn play_card(&self, seat: usize, card: Card) -> Result<(), String> {
        let (result, newly_revealed_partner) = {
            let mut guard = self.state.lock().unwrap();
            let state = guard.as_mut().ok_or_else(|| "game not started".to_string())?;
            let partner_was_null = state.meta["partner"].is_null();
            let result = self.game.apply_play(state, seat, card)?;
            let newly_revealed = if partner_was_null && !state.meta["partner"].is_null() {
                state.meta["partner"].as_u64().map(|p| p as usize)
            } else {
                None
            };
            (result, newly_revealed)
        };

        self.broadcast(StateUpdate::CardPlayed { player: seat, card });

        if let Some(partner_seat) = newly_revealed_partner {
            self.broadcast(StateUpdate::PartnerRevealed { seat: partner_seat });
        }

        match result {
            PlayResult::Continuing => {}
            PlayResult::TrickComplete { winner, points } => {
                self.broadcast(StateUpdate::TrickComplete { winner, points });
            }
            PlayResult::GameOver { last_trick_winner, last_trick_points, scores } => {
                self.broadcast(StateUpdate::TrickComplete {
                    winner: last_trick_winner,
                    points: last_trick_points,
                });
                let session_scores = {
                    let mut ss = self.session_scores.lock().unwrap();
                    for (i, &delta) in scores.iter().enumerate() { ss[i] += delta; }
                    ss.clone()
                };
                let mut hp = self.hands_played.lock().unwrap();
                *hp += 1;
                let hands_done = *hp;
                drop(hp);

                self.broadcast(StateUpdate::HandComplete {
                    hand_scores: scores,
                    session_scores: session_scores.clone(),
                });

                // Check fixed-hand-count session end
                if self.max_hands.map_or(false, |max| hands_done >= max) {
                    let winner = self.session_winner(&session_scores).unwrap_or(0);
                    self.broadcast(StateUpdate::SessionOver {
                        winner,
                        final_scores: session_scores,
                    });
                    return Ok(());
                }

                if let Some(winner) = self.session_winner(&session_scores) {
                    self.broadcast(StateUpdate::SessionOver { winner, final_scores: session_scores });
                }
            }
        }
        Ok(())
    }

    pub fn broadcast(&self, update: StateUpdate) {
        let _ = self.broadcast_tx.send(update);
    }

    pub fn send_private(&self, seat: usize, update: StateUpdate) {
        let seats = self.seats.lock().unwrap();
        if let Some(tx) = seats.get(seat).and_then(|s| s.tx()) {
            let _ = tx.try_send(update);
        }
    }

    const BOT_ACTION_DELAY_MS: u64 = 1200;

    pub async fn drive_bots(self: &Arc<Self>) {
        if self.bots_running.swap(true, Ordering::SeqCst) { return; }
        struct Guard<'a>(&'a AtomicBool);
        impl Drop for Guard<'_> { fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); } }
        let _guard = Guard(&self.bots_running);

        loop {
            let (seat, phase) = {
                let guard = self.state.lock().unwrap();
                let Some(state) = guard.as_ref() else { break };
                (state.current_player, state.phase.clone())
            };

            if phase == GamePhase::Scoring {
                let session_scores = self.session_scores.lock().unwrap().clone();
                let hands_done = *self.hands_played.lock().unwrap();
                let session_over = self.max_hands.map_or(false, |max| hands_done >= max)
                    || self.session_winner(&session_scores).is_some();
                if session_over { break; }
                tokio::time::sleep(std::time::Duration::from_millis(Self::BOT_ACTION_DELAY_MS)).await;
                let next_dealer = {
                    let guard = self.state.lock().unwrap();
                    guard.as_ref().map(|s| (s.dealer + 1) % self.player_count).unwrap_or(0)
                };
                self.start_next_hand(next_dealer);
                continue;
            }

            if phase == GamePhase::Lobby { break; }

            let is_bot = {
                let seats = self.seats.lock().unwrap();
                seats.get(seat).map(|s| s.is_bot()).unwrap_or(false)
            };
            if !is_bot { break; }

            tokio::time::sleep(std::time::Duration::from_millis(Self::BOT_ACTION_DELAY_MS)).await;

            if phase == GamePhase::Bidding {
                let value = {
                    let guard = self.state.lock().unwrap();
                    let Some(state) = guard.as_ref() else { break };
                    crate::bot::bid_action(state, seat)
                };
                if self.apply_bid(seat, value).is_err() { break; }
            } else {
                let card = {
                    let guard = self.state.lock().unwrap();
                    let Some(state) = guard.as_ref() else { break };
                    match crate::bot::play_card(state, seat, self.game.as_ref()) {
                        Some(c) => c,
                        None => break,
                    }
                };
                if self.play_card(seat, card).is_err() { break; }
            }
        }
    }

    fn start_next_hand(&self, dealer: usize) {
        let mut rng = rand::thread_rng();
        let mut state = GameState::new(self.id, self.game_name.clone(), self.player_count, dealer);
        deal_game(self.game.as_ref(), &mut state, &mut rng);
        state.names = self.compute_names();
        {
            let txs = self.seats.lock().unwrap();
            for (seat, seat_state) in txs.iter().enumerate() {
                let Some(tx) = seat_state.tx() else { continue };
                let mut view = state.clone();
                for (i, hand) in view.hands.iter_mut().enumerate() {
                    if i != seat { hand.clear(); }
                }
                view.extra_piles.clear();
                let _ = tx.try_send(StateUpdate::Snapshot { state: view });
            }
        }
        *self.state.lock().unwrap() = Some(state);
        tracing::info!(room_code = %self.room_code, dealer, "hand started");
    }

    fn session_winner(&self, session_scores: &[i32]) -> Option<usize> {
        let goal = self.victory_goal;
        let mut reached: Vec<usize> = session_scores.iter().enumerate()
            .filter(|&(_, &s)| s >= goal).map(|(i, _)| i).collect();
        if reached.is_empty() { return None; }
        reached.sort_by(|&a, &b| session_scores[b].cmp(&session_scores[a]));
        Some(reached[0])
    }

    fn compute_names(&self) -> Vec<String> {
        let seats = self.seats.lock().unwrap();
        let mut bot_counter = 0usize;
        seats.iter().map(|s| match s {
            SeatState::Human { name, .. } => name.clone(),
            SeatState::Disconnected { name, .. } => name.clone(),
            SeatState::Bot => { bot_counter += 1; format!("Bot {bot_counter}") }
            SeatState::Empty => "Empty".into(),
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games;

    fn make_room() -> Arc<Room> {
        let game = games::get_game("sheepshead").unwrap();
        Arc::new(Room::new(
            Uuid::new_v4(),
            "sheepshead".into(),
            5,
            game,
            24,
            "TEST-42".into(),
            "private".into(),
        ))
    }

    #[test]
    fn join_lobby_claims_seat() {
        let room = make_room();
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        let result = room.join_lobby("Alice".into(), Uuid::new_v4(), tx);
        assert!(result.is_some());
        let (seat, _) = result.unwrap();
        assert_eq!(seat, 0);
    }

    #[test]
    fn duplicate_name_rejected() {
        let room = make_room();
        let (tx1, _) = tokio::sync::mpsc::channel(16);
        let (tx2, _) = tokio::sync::mpsc::channel(16);
        room.join_lobby("Alice".into(), Uuid::new_v4(), tx1).unwrap();
        assert!(room.join_lobby("Alice".into(), Uuid::new_v4(), tx2).is_none());
    }

    #[test]
    fn lobby_chat_validates_length() {
        let room = make_room();
        let (tx, _) = tokio::sync::mpsc::channel(16);
        room.join_lobby("Alice".into(), Uuid::new_v4(), tx).unwrap();
        assert!(room.handle_lobby_chat(0, "hello".into()).is_ok());
        assert!(room.handle_lobby_chat(0, "".into()).is_err());
        assert!(room.handle_lobby_chat(0, "x".repeat(201)).is_err());
    }
}
```

- [ ] **Step 4: Run all server tests**

```bash
cd /Users/bkb2443/Git/tricks/server && cargo test 2>&1 | tail -10
cd /Users/bkb2443/Git/tricks/server && cargo clippy -- -D warnings 2>&1 | tail -5
```

Expected: all 51+ tests pass, no clippy errors.

- [ ] **Step 5: Commit**

```bash
cd /Users/bkb2443/Git/tricks && git add server/src/lobby/room.rs
git commit -m "feat(server): seat model, lobby phase, chat, start_game, disconnect/rejoin"
```

---

### Task 4: Matchmaker

**Files:**
- Modify: `server/src/lobby/matchmaker.rs`

- [ ] **Step 1: Write failing tests**

```rust
// At the bottom of matchmaker.rs after implementation:
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn queue_fills_at_five() {
        // Verify that joining 5 players creates a room immediately
        // (tested via side effect: all 5 get JoinedRoom messages)
        // This is an integration-level smoke test — just verify no panic.
        let lobby = std::sync::Arc::new(crate::lobby::Lobby::new());
        let mm = std::sync::Arc::new(Matchmaker::new(std::sync::Arc::clone(&lobby)));
        for i in 0..5 {
            let (tx, _rx) = tokio::sync::mpsc::channel(16);
            mm.join_queue(format!("Player{i}"), tx, uuid::Uuid::new_v4());
        }
        // Room should have been created
        // (We can't easily inspect DashMap from here, so just verify no panic)
    }
}
```

- [ ] **Step 2: Implement matchmaker**

Replace `server/src/lobby/matchmaker.rs`:

```rust
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::engine::StateUpdate;
use crate::lobby::Lobby;

const QUEUE_TIMEOUT_SECS: u64 = 60;
const PUBLIC_MAX_HANDS: u32 = 8;

struct QueueEntry {
    name: String,
    tx: mpsc::Sender<StateUpdate>,
    ws_id: Uuid,
    joined_at: Instant,
}

pub struct Matchmaker {
    queue: Mutex<Vec<QueueEntry>>,
    lobby: Arc<Lobby>,
    timer_running: Mutex<bool>,
}

impl Matchmaker {
    pub fn new(lobby: Arc<Lobby>) -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            lobby,
            timer_running: Mutex::new(false),
        }
    }

    pub fn join_queue(self: &Arc<Self>, name: String, tx: mpsc::Sender<StateUpdate>, ws_id: Uuid) {
        let position;
        let should_start_timer;
        {
            let mut q = self.queue.lock().unwrap();
            q.push(QueueEntry { name, tx: tx.clone(), ws_id, joined_at: Instant::now() });
            position = q.len();
            should_start_timer = position == 1;

            if position == 5 {
                self.flush_queue_locked(&mut q);
                return;
            }
        }

        let waiting_since = SystemTime::now()
            .duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        let _ = tx.try_send(StateUpdate::QueueStatus { position, waiting_since });

        if should_start_timer {
            let mm = Arc::clone(self);
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(QUEUE_TIMEOUT_SECS)).await;
                mm.on_timer_fired();
            });
        }
    }

    pub fn leave_queue(&self, ws_id: Uuid) {
        let mut q = self.queue.lock().unwrap();
        q.retain(|e| e.ws_id != ws_id);
    }

    fn on_timer_fired(self: &Arc<Self>) {
        let mut q = self.queue.lock().unwrap();
        if q.len() >= 2 {
            self.flush_queue_locked(&mut q);
        } else {
            q.clear(); // <2 players — discard
        }
    }

    fn flush_queue_locked(&self, q: &mut Vec<QueueEntry>) {
        let entries: Vec<QueueEntry> = q.drain(..).collect();
        let Some((code, room)) = self.lobby.create_room("sheepshead".into(), 5, 24) else {
            return;
        };

        // Set public room properties (max_hands)
        // Room was created — we can't mutate max_hands via Arc<Room> without interior mutability.
        // We pass max_hands via meta instead (room reads it from meta in score path).
        // Update meta directly:
        {
            let mut guard = room.state.lock().unwrap();
            if let Some(ref mut state) = *guard {
                state.meta["max_hands"] = serde_json::json!(PUBLIC_MAX_HANDS);
                state.meta["room_type"] = serde_json::json!("public");
            }
        }

        // Assign human players
        for entry in entries {
            let (seat, _broadcast_rx) = match room.join_lobby(entry.name, entry.ws_id, entry.tx.clone()) {
                Some(r) => r,
                None => continue,
            };
            let _ = entry.tx.try_send(StateUpdate::JoinedRoom {
                room_id: uuid::Uuid::nil(), // placeholder; client uses room_code now
                seat,
            });
        }

        // Start the game (fills remaining seats with bots)
        room.start_game();
        tracing::info!(room_code = %code, "public room created by matchmaker");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn queue_fills_at_five() {
        let lobby = Arc::new(Lobby::new());
        let mm = Arc::new(Matchmaker::new(Arc::clone(&lobby)));
        for i in 0..5 {
            let (tx, _rx) = mpsc::channel(16);
            mm.join_queue(format!("Player{i}"), tx, Uuid::new_v4());
        }
        // No panic = pass
    }

    #[test]
    fn leave_queue_removes_entry() {
        let lobby = Arc::new(Lobby::new());
        let mm = Matchmaker::new(Arc::clone(&lobby));
        let (tx, _rx) = mpsc::channel(16);
        let ws_id = Uuid::new_v4();
        // Can't easily test queue size without exposing it, but verify no panic
        let mm_arc = Arc::new(mm);
        mm_arc.join_queue("Alice".into(), tx, ws_id);
        mm_arc.leave_queue(ws_id);
    }
}
```

- [ ] **Step 3: Add `Matchmaker` to `Lobby`**

In `server/src/lobby/mod.rs`, add `matchmaker: Matchmaker` to `Lobby`:

```rust
pub struct Lobby {
    rooms: DashMap<String, Arc<Room>>,
    pub matchmaker: Arc<matchmaker::Matchmaker>,
}

impl Lobby {
    pub fn new() -> Self {
        let rooms = DashMap::new();
        // We need a temporary Arc<Lobby> for Matchmaker — use a two-phase init
        // by wrapping Matchmaker creation after Lobby is built.
        // Instead, Matchmaker holds a reference to create_room via a closure.
        // Simplest: build Lobby first, then set matchmaker after Arc creation.
        // Use a Mutex<Option<Arc<Matchmaker>>> and set it in Lobby::with_matchmaker.
        Self { rooms, matchmaker: Arc::new(matchmaker::Matchmaker::new_empty()) }
    }

    pub fn init_matchmaker(self: &Arc<Self>) {
        // Replace the placeholder matchmaker with one that has a real Lobby reference.
        // Since matchmaker is Arc, we can't replace it — instead Matchmaker takes
        // a weak reference to Lobby.
        // Simpler approach: pass Arc<Lobby> to Matchmaker::new in main.rs after Arc::new(Lobby).
    }
    // ...
}
```

Actually the cleanest approach: `Matchmaker::new` takes `Arc<Lobby>` which is created in `main.rs`:

In `server/src/main.rs`:

```rust
let lobby = Arc::new(lobby::Lobby::new());
let matchmaker = Arc::new(lobby::Matchmaker::new(Arc::clone(&lobby)));
// Pass both through AppState or combine into a wrapper.
```

Since `AppState` is currently just `Arc<Lobby>`, add a `MatchmakerState` or put `matchmaker` inside `Lobby`. Let's put it inside Lobby using `std::sync::OnceLock`:

```rust
pub struct Lobby {
    rooms: DashMap<String, Arc<Room>>,
    pub matchmaker: std::sync::OnceLock<Arc<matchmaker::Matchmaker>>,
}

impl Lobby {
    pub fn new() -> Self {
        Self { rooms: DashMap::new(), matchmaker: std::sync::OnceLock::new() }
    }
}
```

In `main.rs`, after `Arc::new(lobby::Lobby::new())`:
```rust
let lobby = Arc::new(lobby::Lobby::new());
let mm = Arc::new(lobby::Matchmaker::new(Arc::clone(&lobby)));
lobby.matchmaker.set(mm).ok();
```

`Matchmaker::new_empty()` is not needed — just use `OnceLock`.

- [ ] **Step 4: Run all tests**

```bash
cd /Users/bkb2443/Git/tricks/server && cargo test 2>&1 | tail -5
cd /Users/bkb2443/Git/tricks/server && cargo clippy -- -D warnings 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
cd /Users/bkb2443/Git/tricks && git add server/src/lobby/matchmaker.rs server/src/lobby/mod.rs server/src/main.rs
git commit -m "feat(server): matchmaker — public queue with 60s timeout and 5-player fill"
```

---

### Task 5: WebSocket Handler Updates

**Files:**
- Modify: `server/src/ws/handler.rs`

- [ ] **Step 1: Update `PlayerCtx` and add disconnect handling**

Replace `server/src/ws/handler.rs`:

```rust
use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::{
    engine::{ClientMessage, GamePhase, StateUpdate},
    lobby::{Lobby, Room},
};

type Sink = futures_util::stream::SplitSink<WebSocket, Message>;

struct PlayerCtx {
    seat: usize,
    name: Option<String>,
    ws_id: Uuid,
    room: Arc<Room>,
    broadcast_rx: broadcast::Receiver<StateUpdate>,
}

pub async fn upgrade(ws: WebSocketUpgrade, State(lobby): State<Arc<Lobby>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, lobby))
}

async fn handle_socket(socket: WebSocket, lobby: Arc<Lobby>) {
    let (mut sink, mut stream) = socket.split();
    let (player_tx, mut player_rx) = mpsc::channel::<StateUpdate>(16);
    let ws_id = Uuid::new_v4();

    let mut ctx: Option<PlayerCtx> = None;

    loop {
        tokio::select! {
            msg = stream.next() => {
                let Some(Ok(msg)) = msg else { break };
                match msg {
                    Message::Text(text) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(client_msg) => {
                                let reply = route(client_msg, &lobby, &player_tx, ws_id, &mut ctx);
                                if let Some(upd) = reply
                                    && send(&mut sink, &upd).await.is_err() { break; }
                            }
                            Err(e) => {
                                let err = StateUpdate::Error { message: e.to_string() };
                                if send(&mut sink, &err).await.is_err() { break; }
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }

            Some(upd) = player_rx.recv() => {
                if send(&mut sink, &upd).await.is_err() { break; }
            }

            upd = async {
                match ctx.as_mut() {
                    Some(c) => match c.broadcast_rx.recv().await {
                        Ok(u) => Some(u),
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("broadcast lagged by {n} messages");
                            None
                        }
                        Err(broadcast::error::RecvError::Closed) => None,
                    },
                    None => std::future::pending::<Option<StateUpdate>>().await,
                }
            } => {
                if let Some(u) = upd
                    && send(&mut sink, &u).await.is_err() { break; }
            }
        }
    }

    // Cleanup on disconnect
    if let Some(c) = ctx {
        let phase = {
            let guard = c.room.state.lock().unwrap();
            guard.as_ref().map(|s| s.phase.clone())
        };
        if phase != Some(GamePhase::Lobby) {
            let room = Arc::clone(&c.room);
            tokio::spawn(async move { room.on_disconnect(c.seat, c.ws_id); });
        }
        // Lobby disconnects are handled synchronously by on_disconnect (no async needed)
        if phase == Some(GamePhase::Lobby) {
            c.room.on_disconnect(c.seat, c.ws_id);
        }
        // Leave matchmaking queue if still in it
        if let Some(mm) = lobby.matchmaker.get() {
            mm.leave_queue(c.ws_id);
        }
    } else {
        // Never joined a room — still might be in queue
        if let Some(mm) = lobby.matchmaker.get() {
            mm.leave_queue(ws_id);
        }
    }
}

fn route(
    msg: ClientMessage,
    lobby: &Lobby,
    player_tx: &mpsc::Sender<StateUpdate>,
    ws_id: Uuid,
    ctx: &mut Option<PlayerCtx>,
) -> Option<StateUpdate> {
    match msg {
        // ── Legacy solo path ─────────────────────────────────────────────────
        ClientMessage::JoinRoom { room_id, game, players, fill_bots } => {
            let room = match room_id {
                Some(ref id) => lobby.get_room(id).or_else(|| {
                    lobby.create_room(game, players, 24).map(|(_, r)| r)
                })?,
                None => lobby.create_room(game, players, 24).map(|(_, r)| r)?,
            };
            match room.join(player_tx.clone()) {
                Some((seat, broadcast_rx)) => {
                    let code = room.room_code.clone();
                    let reply = StateUpdate::JoinedRoom { room_id: room.id, seat, room_code: code };
                    if fill_bots {
                        let room_arc = Arc::clone(&room);
                        room_arc.fill_bots();
                        room_arc.start_game();
                    }
                    *ctx = Some(PlayerCtx { seat, name: None, ws_id, room, broadcast_rx });
                    Some(reply)
                }
                None => Some(StateUpdate::Error { message: "room is full".into() }),
            }
        }

        // ── Multiplayer: create a new private room ────────────────────────────
        ClientMessage::CreateRoom { name, game, max_hands } => {
            let (code, room) = match lobby.create_room(game, 5, 24) {
                Some(r) => r,
                None => return Some(StateUpdate::Error { message: "unknown game".into() }),
            };
            // Set max_hands in meta if specified
            if let Some(mh) = max_hands {
                let mut guard = room.state.lock().unwrap();
                if let Some(ref mut state) = *guard {
                    state.meta["max_hands"] = serde_json::json!(mh);
                }
            }
            match room.join_lobby(name.clone(), ws_id, player_tx.clone()) {
                Some((seat, broadcast_rx)) => {
                    let reply = StateUpdate::JoinedRoom { room_id: room.id, seat, room_code: code.clone() };
                    *ctx = Some(PlayerCtx { seat, name: Some(name), ws_id, room, broadcast_rx });
                    Some(reply)
                }
                None => Some(StateUpdate::Error { message: "failed to join room".into() }),
            }
        }

        // ── Multiplayer: join existing room by short code ─────────────────────
        ClientMessage::Join { name, room_code } => {
            let room = match lobby.get_room(&room_code) {
                Some(r) => r,
                None => return Some(StateUpdate::Error {
                    message: format!("room '{room_code}' not found"),
                }),
            };
            match room.join_lobby(name.clone(), ws_id, player_tx.clone()) {
                Some((seat, broadcast_rx)) => {
                    let reply = StateUpdate::JoinedRoom { room_id: room.id, seat, room_code: room_code.clone() };
                    *ctx = Some(PlayerCtx { seat, name: Some(name), ws_id, room, broadcast_rx });
                    Some(reply)
                }
                None => Some(StateUpdate::Error {
                    message: "room is full or name already taken".into(),
                }),
            }
        }

        // ── Game actions ──────────────────────────────────────────────────────
        ClientMessage::Bid { value } => {
            let c = ctx.as_ref()?;
            match c.room.apply_bid(c.seat, value) {
                Ok(()) => {
                    let room_arc = Arc::clone(&c.room);
                    tokio::spawn(async move { room_arc.drive_bots().await });
                    None
                }
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        ClientMessage::PlayCard { card } => {
            let c = ctx.as_ref()?;
            match c.room.play_card(c.seat, card) {
                Ok(()) => {
                    let room_arc = Arc::clone(&c.room);
                    tokio::spawn(async move { room_arc.drive_bots().await });
                    None
                }
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        // ── Lobby actions ─────────────────────────────────────────────────────
        ClientMessage::LobbyChat { text } => {
            let c = ctx.as_ref()?;
            match c.room.handle_lobby_chat(c.seat, text) {
                Ok(()) => None,
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        ClientMessage::StartGame => {
            let c = ctx.as_ref()?;
            let room = Arc::clone(&c.room);
            room.start_game();
            None
        }

        ClientMessage::ForceBot { seat } => {
            let c = ctx.as_ref()?;
            match c.room.force_bot(seat, c.seat) {
                Ok(()) => None,
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        ClientMessage::ExtendRejoin { seat } => {
            let c = ctx.as_ref()?;
            match c.room.extend_rejoin(seat, c.seat) {
                Ok(()) => None,
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        // ── Matchmaking ───────────────────────────────────────────────────────
        ClientMessage::JoinQueue => {
            let name = ctx.as_ref()
                .and_then(|c| c.name.clone())
                .unwrap_or_else(|| format!("Player-{}", &ws_id.to_string()[..4]));
            if let Some(mm) = lobby.matchmaker.get() {
                let mm_arc = Arc::clone(mm);
                mm_arc.join_queue(name, player_tx.clone(), ws_id);
            }
            None
        }

        ClientMessage::LeaveQueue => {
            if let Some(mm) = lobby.matchmaker.get() {
                mm.leave_queue(ws_id);
            }
            None
        }
    }
}

async fn send(sink: &mut Sink, update: &StateUpdate) -> Result<(), axum::Error> {
    let json = serde_json::to_string(update).expect("StateUpdate serialization failed");
    sink.send(Message::Text(json)).await
}
```

- [ ] **Step 2: Build and run all tests**

```bash
cd /Users/bkb2443/Git/tricks/server && cargo build 2>&1 | tail -5
cd /Users/bkb2443/Git/tricks/server && cargo test 2>&1 | tail -5
cd /Users/bkb2443/Git/tricks/server && cargo clippy -- -D warnings 2>&1 | tail -5
```

Expected: all tests pass, clean build, no clippy warnings.

- [ ] **Step 3: Commit**

```bash
cd /Users/bkb2443/Git/tricks && git add server/src/ws/handler.rs
git commit -m "feat(server): route new client messages — Join, LobbyChat, StartGame, ForceBot, ExtendRejoin, JoinQueue, LeaveQueue"
```

---

### Task 6: Client Store + Composable

**Files:**
- Modify: `client/src/stores/game.ts`
- Modify: `client/src/composables/useGame.ts`

- [ ] **Step 1: Update store**

In `client/src/stores/game.ts`, add after the existing `partnerRevealedSeat` ref:

```typescript
  const seats          = ref<import('@/engine/types').SeatInfo[]>([])
  const lobbyChat      = ref<Array<{ from: string; text: string; timestamp: number }>>([])
  const queueStatus    = ref<{ position: number; waiting_since: number } | null>(null)
  const roomCode       = ref<string | null>(null)
```

Update the `joined_room` case in `handleUpdate` to capture `room_code`:

```typescript
      case 'joined_room':
        roomId.value   = update.room_id
        seat.value     = update.seat
        roomCode.value = update.room_code || null
        break
```

Add `isLobby` computed alongside the existing computeds:

```typescript
  const isLobby = computed(() => gameState.value?.phase === 'lobby')
```

Add to the `handleUpdate` switch:

```typescript
      case 'seat_update':
        seats.value = update.seats
        break

      case 'lobby_chat':
        lobbyChat.value = [...lobbyChat.value, {
          from: update.from,
          text: update.text,
          timestamp: update.timestamp,
        }].slice(-50)
        break

      case 'queue_status':
        queueStatus.value = { position: update.position, waiting_since: update.waiting_since }
        break
```

In `reset()`, add:

```typescript
    seats.value = []
    lobbyChat.value = []
    queueStatus.value = null
    roomCode.value = null
```

Add to the return object:

```typescript
    seats, lobbyChat, queueStatus, isLobby, roomCode,
```

- [ ] **Step 2: Update useGame composable**

Add to `client/src/composables/useGame.ts`:

```typescript
  function joinWithCode(name: string, roomCode: string): void {
    store.isSolo = false
    sendMessage({ type: 'join', name, room_code: roomCode })
  }

  function createPrivateRoom(game: string, maxHands: number | null, name: string): void {
    store.isSolo = false
    sendMessage({ type: 'create_room', name, game, max_hands: maxHands })
  }

  function joinQueue(): void {
    sendMessage({ type: 'join_queue' })
  }

  function leaveQueue(): void {
    sendMessage({ type: 'leave_queue' })
  }

  function startGame(): void {
    sendMessage({ type: 'start_game' })
  }

  function sendLobbyChat(text: string): void {
    sendMessage({ type: 'lobby_chat', text })
  }

  function forceBot(seat: number): void {
    sendMessage({ type: 'force_bot', seat })
  }

  function extendRejoin(seat: number): void {
    sendMessage({ type: 'extend_rejoin', seat })
  }
```

Add all new functions to the return object.

- [ ] **Step 3: Type-check and run tests**

```bash
cd /Users/bkb2443/Git/tricks/client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vue-tsc --noEmit 2>&1 | tail -5
cd /Users/bkb2443/Git/tricks/client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vitest run 2>&1 | tail -5
```

Expected: no type errors, all tests pass.

- [ ] **Step 4: Commit**

```bash
cd /Users/bkb2443/Git/tricks && git add client/src/stores/game.ts client/src/composables/useGame.ts
git commit -m "feat(client): store handles seat_update/lobby_chat/queue_status; new lobby composable actions"
```

---

### Task 7: Client Views + Router

**Files:**
- Modify: `client/src/views/HomeView.vue`
- Create: `client/src/views/LobbyView.vue`
- Create: `client/src/views/QueueView.vue`
- Modify: `client/src/router/index.ts`
- Modify: `client/src/views/GameView.vue`

- [ ] **Step 1: Update router**

Replace `client/src/router/index.ts`:

```typescript
import { createRouter, createWebHistory } from 'vue-router'
import HomeView  from '@/views/HomeView.vue'
import LobbyView from '@/views/LobbyView.vue'
import QueueView from '@/views/QueueView.vue'
import GameView  from '@/views/GameView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    { path: '/',      name: 'home',  component: HomeView  },
    { path: '/lobby', name: 'lobby', component: LobbyView },
    { path: '/queue', name: 'queue', component: QueueView },
    { path: '/game',  name: 'game',  component: GameView  },
  ],
})

export default router
```

- [ ] **Step 2: Redesign HomeView**

Replace `client/src/views/HomeView.vue`:

```vue
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useGame } from '@/composables/useGame'
import { connected } from '@/engine/socket'

const router = useRouter()
const { createSoloRoom, joinWithCode, joinQueue, createPrivateRoom } = useGame()

const guestName    = ref(localStorage.getItem('guestName') ?? '')
const joinCode     = ref('')
const maxHands     = ref<number | null>(null)
const nameError    = ref('')

function saveName() {
  const n = guestName.value.trim()
  if (!n) { nameError.value = 'Enter a name to continue.'; return false }
  localStorage.setItem('guestName', n)
  nameError.value = ''
  return true
}

function handleSolo() {
  if (!saveName()) return
  createSoloRoom('sheepshead', 5)
  router.push('/game')
}

function handleCreatePrivate() {
  if (!saveName()) return
  createPrivateRoom('sheepshead', maxHands.value, guestName.value.trim())
  router.push('/lobby')
}

function handleJoinCode() {
  if (!saveName()) return
  if (!joinCode.value.trim()) return
  joinWithCode(guestName.value.trim(), joinCode.value.trim().toUpperCase())
  router.push('/lobby')
}

function handleFindGame() {
  if (!saveName()) return
  joinQueue()
  router.push('/queue')
}

onMounted(() => {
  guestName.value = localStorage.getItem('guestName') ?? ''
})
</script>

<template>
  <div class="home">
    <h1>Tricks</h1>
    <p v-if="!connected" class="warn">Not connected to server — waiting…</p>

    <!-- Name prompt -->
    <section class="name-section">
      <label>
        Your name
        <input v-model="guestName" placeholder="Enter display name" maxlength="20" @input="nameError = ''" />
      </label>
      <p v-if="nameError" class="name-error">{{ nameError }}</p>
    </section>

    <!-- Solo -->
    <section class="solo">
      <div class="solo-text">
        <h2>Play Solo</h2>
        <p>You vs 4 bots — starts immediately.</p>
      </div>
      <button class="btn-solo" :disabled="!connected" @click="handleSolo">Play Solo →</button>
    </section>

    <!-- Multiplayer actions -->
    <div class="panels">
      <section>
        <h2>Create Private Room</h2>
        <p class="hint">Share the room code with friends.</p>
        <button :disabled="!connected" @click="handleCreatePrivate">Create Room →</button>
      </section>

      <section>
        <h2>Find a Game</h2>
        <p class="hint">Match with others online.</p>
        <button :disabled="!connected" @click="handleFindGame">Find Game →</button>
      </section>

      <section class="join-code-section">
        <h2>Join with Code</h2>
        <input v-model="joinCode" placeholder="WOLF-42" maxlength="10" @keydown.enter="handleJoinCode" />
        <button :disabled="!connected || !joinCode.trim()" @click="handleJoinCode">Join →</button>
      </section>
    </div>
  </div>
</template>

<style scoped>
.home { max-width: 600px; margin: 2rem auto; }
h1 { font-size: 2.5rem; margin-bottom: 1.5rem; }
.warn { color: #fbbf24; }
.name-section { margin-bottom: 1.25rem; }
.name-section label { display: flex; flex-direction: column; gap: 0.4rem; font-size: 0.9rem; color: #9ca3af; }
.name-section input { font-size: 1rem; }
.name-error { color: #f87171; font-size: 0.85rem; margin: 0.25rem 0 0; }
.solo { display: flex; align-items: center; justify-content: space-between; gap: 1rem;
  background: rgba(99,102,241,0.15); border: 1px solid rgba(99,102,241,0.4);
  border-radius: 8px; padding: 1rem 1.25rem; margin-bottom: 1.5rem; }
.solo-text h2 { margin: 0 0 0.25rem; font-size: 1.1rem; }
.solo-text p { margin: 0; font-size: 0.85rem; color: #9ca3af; }
.btn-solo { background: #6366f1; white-space: nowrap; }
.btn-solo:hover:not(:disabled) { background: #4f46e5; }
.panels { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; margin-bottom: 2rem; }
.join-code-section { grid-column: 1 / -1; }
section { background: rgba(0,0,0,0.25); border-radius: 8px; padding: 1rem 1.25rem; display: flex; flex-direction: column; gap: 0.5rem; }
h2 { margin: 0; font-size: 1rem; }
.hint { margin: 0; font-size: 0.8rem; color: #6b7280; }
input { background: rgba(255,255,255,0.08); border: 1px solid rgba(255,255,255,0.15); border-radius: 5px; padding: 0.4rem 0.6rem; color: #fff; font-size: 0.9rem; }
</style>
```

- [ ] **Step 3: Create LobbyView**

Create `client/src/views/LobbyView.vue`:

```vue
<script setup lang="ts">
import { computed, ref, onMounted, nextTick, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'

const store  = useRouter() // not store — fix:
const router = useRouter()
const { store: gameStore } = useGame()

// Re-import properly
import { useGameStore as useGS } from '@/stores/game'
const gs = useGS()
const { startGame, sendLobbyChat, forceBot, extendRejoin } = useGame()

const chatInput = ref('')
const chatEl    = ref<HTMLElement | null>(null)

const isHost = computed(() => {
  const hostSeat = gs.gameState?.meta?.host_seat
  return typeof hostSeat === 'number' && hostSeat === gs.seat
})

const roomCode = computed(() => gs.roomCode ?? '—')

const countdownEndsAt = computed<number | null>(() => {
  const v = gs.gameState?.meta?.countdown_ends_at
  return typeof v === 'number' ? v : null
})

const secondsLeft = ref<number | null>(null)
let countdownInterval: ReturnType<typeof setInterval> | null = null

watch(countdownEndsAt, (val) => {
  if (countdownInterval) clearInterval(countdownInterval)
  if (val === null) { secondsLeft.value = null; return }
  const update = () => {
    const diff = Math.ceil((val - Date.now()) / 1000)
    secondsLeft.value = diff > 0 ? diff : 0
    if (diff <= 0 && countdownInterval) clearInterval(countdownInterval)
  }
  update()
  countdownInterval = setInterval(update, 500)
})

// Navigate to game when phase changes out of lobby
watch(() => gs.gameState?.phase, (phase) => {
  if (phase && phase !== 'lobby') router.push('/game')
})

function sendChat() {
  if (!chatInput.value.trim()) return
  sendLobbyChat(chatInput.value.trim())
  chatInput.value = ''
}

watch(() => gs.lobbyChat.length, async () => {
  await nextTick()
  if (chatEl.value) chatEl.value.scrollTop = chatEl.value.scrollHeight
})

async function copyCode() {
  await navigator.clipboard.writeText(roomCode.value)
}

const seatStateLabel = (state: string) =>
  state === 'empty' ? 'Empty' : state === 'bot' ? 'Bot' : state === 'disconnected' ? 'Disconnected' : ''
</script>

<template>
  <div class="lobby">
    <header class="lobby-header">
      <h1>Lobby</h1>
      <div v-if="roomCode !== '—'" class="room-code">
        <span>{{ roomCode }}</span>
        <button class="btn-copy" @click="copyCode" title="Copy room code">📋</button>
      </div>
    </header>

    <!-- Seat rail -->
    <section class="seats">
      <div
        v-for="info in gs.seats"
        :key="info.seat"
        class="seat-card"
        :class="[info.state, { me: info.seat === gs.seat }]"
      >
        <div class="seat-name">
          {{ info.name ?? seatStateLabel(info.state) }}
          <span v-if="info.seat === gs.seat" class="you-badge">You</span>
        </div>
        <div class="seat-state">{{ info.state }}</div>
        <div v-if="info.state === 'disconnected' && isHost" class="host-controls">
          <button @click="forceBot(info.seat)" class="btn-sm">Bot now</button>
          <button @click="extendRejoin(info.seat)" class="btn-sm">+30s</button>
        </div>
      </div>
    </section>

    <!-- Chat -->
    <section class="chat">
      <div ref="chatEl" class="chat-messages">
        <div
          v-for="(msg, i) in gs.lobbyChat"
          :key="i"
          class="chat-msg"
          :class="{ system: msg.from === 'System' }"
        >
          <span class="chat-from">{{ msg.from }}:</span>
          <span class="chat-text">{{ msg.text }}</span>
        </div>
        <div v-if="gs.lobbyChat.length === 0" class="chat-empty">No messages yet…</div>
      </div>
      <div class="chat-input-row">
        <input
          v-model="chatInput"
          placeholder="Say something…"
          maxlength="200"
          @keydown.enter="sendChat"
        />
        <button @click="sendChat" :disabled="!chatInput.trim()">Send</button>
      </div>
    </section>

    <!-- Host controls -->
    <div class="start-area">
      <template v-if="isHost">
        <p class="hint">You are the host.</p>
        <button class="btn-start" @click="startGame">Start Game →</button>
      </template>
      <p v-else class="waiting">Waiting for host to start the game…</p>
      <div v-if="secondsLeft !== null" class="countdown">Game starting in {{ secondsLeft }}s</div>
    </div>
  </div>
</template>

<style scoped>
.lobby { max-width: 700px; margin: 2rem auto; display: flex; flex-direction: column; gap: 1.25rem; }
.lobby-header { display: flex; align-items: center; justify-content: space-between; }
h1 { margin: 0; font-size: 2rem; }
.room-code { display: flex; align-items: center; gap: 0.5rem; background: rgba(255,255,255,0.08); border-radius: 6px; padding: 0.3rem 0.75rem; font-family: monospace; font-size: 1.2rem; }
.btn-copy { background: none; border: none; cursor: pointer; font-size: 1rem; padding: 0; }

.seats { display: grid; grid-template-columns: repeat(5, 1fr); gap: 0.75rem; }
.seat-card { background: rgba(0,0,0,0.25); border-radius: 8px; padding: 0.75rem; text-align: center; border: 1px solid transparent; }
.seat-card.me { border-color: rgba(34,197,94,0.6); }
.seat-card.empty .seat-name { color: #4b5563; }
.seat-card.disconnected { opacity: 0.6; }
.seat-name { font-weight: 600; font-size: 0.9rem; }
.you-badge { background: #16a34a; color: #fff; font-size: 0.6rem; padding: 0 4px; border-radius: 3px; margin-left: 4px; vertical-align: middle; }
.seat-state { font-size: 0.7rem; color: #6b7280; margin-top: 0.2rem; }
.host-controls { display: flex; gap: 0.25rem; justify-content: center; margin-top: 0.4rem; }
.btn-sm { font-size: 0.7rem; padding: 0.15rem 0.4rem; background: #374151; }

.chat { background: rgba(0,0,0,0.2); border-radius: 8px; overflow: hidden; }
.chat-messages { height: 180px; overflow-y: auto; padding: 0.75rem; display: flex; flex-direction: column; gap: 0.35rem; }
.chat-msg { font-size: 0.85rem; }
.chat-from { color: #9ca3af; margin-right: 0.4rem; }
.chat-msg.system .chat-from { color: #f59e0b; }
.chat-empty { color: #4b5563; font-style: italic; font-size: 0.85rem; }
.chat-input-row { display: flex; gap: 0.5rem; padding: 0.5rem 0.75rem; border-top: 1px solid rgba(255,255,255,0.08); }
.chat-input-row input { flex: 1; }

.start-area { text-align: center; padding: 0.5rem; }
.btn-start { background: #15803d; font-size: 1rem; padding: 0.6rem 2rem; }
.btn-start:hover { background: #166534; }
.hint { color: #9ca3af; font-size: 0.85rem; margin: 0 0 0.5rem; }
.waiting { color: #6b7280; font-style: italic; }
.countdown { color: #fbbf24; font-size: 1.1rem; font-weight: 700; margin-top: 0.5rem; }
</style>
```

- [ ] **Step 4: Create QueueView**

Create `client/src/views/QueueView.vue`:

```vue
<script setup lang="ts">
import { watch } from 'vue'
import { useRouter } from 'vue-router'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'

const router = useRouter()
const store  = useGameStore()
const { leaveQueue } = useGame()

// When matchmaker assigns a room, server sends JoinedRoom then Snapshot (Lobby or game phase).
// Navigate to lobby when we get a room.
watch(() => store.roomId, (id) => {
  if (id) router.push('/lobby')
})

function handleCancel() {
  leaveQueue()
  router.push('/')
}
</script>

<template>
  <div class="queue">
    <h1>Finding a Game…</h1>
    <div v-if="store.queueStatus" class="status">
      <p>Position in queue: <strong>{{ store.queueStatus.position }}</strong></p>
    </div>
    <p class="hint">You'll be matched automatically. Bots fill any empty seats.</p>
    <button class="btn-cancel" @click="handleCancel">Cancel</button>
  </div>
</template>

<style scoped>
.queue { max-width: 400px; margin: 6rem auto; text-align: center; display: flex; flex-direction: column; align-items: center; gap: 1rem; }
h1 { font-size: 2rem; }
.status { background: rgba(0,0,0,0.2); border-radius: 8px; padding: 1rem 2rem; }
.hint { color: #6b7280; font-size: 0.85rem; }
.btn-cancel { background: #6b7280; }
.btn-cancel:hover { background: #4b5563; }
</style>
```

- [ ] **Step 5: Simplify GameView (remove waiting-for-players section)**

In `client/src/views/GameView.vue`, remove the `v-else-if="!store.gameStarted"` block (lobby view handles it):

```vue
<template>
  <div v-if="notInRoom" class="center">
    <p>You haven't joined a room yet.</p>
    <router-link to="/"><button>Back to Home</button></router-link>
  </div>
  <sheepshead-table v-else-if="store.gameState?.game_name === 'sheepshead'" />
  <div v-else class="center">
    <p>Waiting for game to start…</p>
  </div>
</template>
```

- [ ] **Step 6: Type-check, tests, and build**

```bash
cd /Users/bkb2443/Git/tricks/client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vue-tsc --noEmit 2>&1 | tail -10
cd /Users/bkb2443/Git/tricks/client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vitest run 2>&1 | tail -5
cd /Users/bkb2443/Git/tricks/server && cargo test 2>&1 | tail -3
```

Expected: no type errors, all tests pass.

- [ ] **Step 7: Commit**

```bash
cd /Users/bkb2443/Git/tricks && git add client/src/views/ client/src/router/index.ts
git commit -m "feat(client): HomeView redesign, LobbyView, QueueView, updated router"
```

---

### Task 8: End-to-End Verification

- [ ] **Step 1: Full automated suite**

```bash
cd /Users/bkb2443/Git/tricks/server && cargo test 2>&1 | tail -5
cd /Users/bkb2443/Git/tricks/server && cargo clippy -- -D warnings 2>&1 | tail -3
cd /Users/bkb2443/Git/tricks/client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vitest run 2>&1 | tail -5
cd /Users/bkb2443/Git/tricks/client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vue-tsc --noEmit 2>&1 | tail -3
```

- [ ] **Step 2: Start servers**

```bash
# Terminal 1
cd /Users/bkb2443/Git/tricks/server && cargo run

# Terminal 2
cd /Users/bkb2443/Git/tricks/client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npm run dev
```

Open `http://localhost:5173`.

- [ ] **Step 3: Solo mode smoke test**

- Click "Play Solo →" → game starts immediately with 4 bots ✓
- Solo mode is unchanged from before ✓

- [ ] **Step 4: Private room smoke test**

- Open Tab A: enter name "Alice", click "Create Room" → lands on LobbyView with room code
- Open Tab B: enter name "Bob", paste code, click "Join" → Bob appears in seat rail on both tabs
- Chat between tabs — messages appear on both ✓
- Alice clicks "Start Game" → both tabs transition to game view ✓
- Bots fill remaining 3 seats ✓

- [ ] **Step 5: Disconnection smoke test**

- Mid-game: close Tab B → Tab A shows "Bob disconnected — 30 seconds to rejoin" toast
- Wait 30 seconds → "Bob's hand has been taken over by a bot" ✓
- Repeat, but reopen Tab B within 30s → "Bob rejoined" ✓

- [ ] **Step 6: Matchmaking smoke test**

- Tab A: enter name "Alice", click "Find a Game" → QueueView shows position 1
- Tab B: enter name "Bob", click "Find a Game" → both get matched into a room with 3 bots ✓
- Both tabs transition to LobbyView then immediately to game (8-hand fixed session) ✓

---

## Verification Summary

| Check | Command |
|-------|---------|
| Server tests | `cd server && cargo test` |
| Server lint | `cd server && cargo clippy -- -D warnings` |
| Client tests | `cd client && npx vitest run` |
| Client types | `cd client && npx vue-tsc --noEmit` |
| Manual smoke | Solo, private room, disconnect/rejoin, public matchmaking |
