use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::engine::{Card, Trick};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub game_id: Uuid,
    pub game_name: String,
    pub phase: GamePhase,
    pub player_count: usize,
    /// Seat index of the player who dealt this hand.
    pub dealer: usize,
    /// Index of the player whose turn it is.
    pub current_player: usize,
    /// `hands[i]` = cards currently held by player i. Clients only see their own hand.
    pub hands: Vec<Vec<Card>>,
    /// Named side piles (e.g. Sheepshead blind, Euchre kitty). Hidden from clients
    /// until game-specific rules expose them (e.g. picker takes the blind).
    pub extra_piles: Vec<(String, Vec<Card>)>,
    pub current_trick: Option<Trick>,
    pub completed_tricks: Vec<Trick>,
    pub scores: Vec<i32>,
    /// Game-specific metadata (e.g. who picked the blind in Sheepshead, called trump in
    /// Euchre). Serialised as opaque JSON so the engine never needs to know the shape.
    pub meta: serde_json::Value,
    /// Display name for each seat. Populated by the room before the first Snapshot.
    #[serde(default)]
    pub names: Vec<String>,
}

impl GameState {
    /// Group completed tricks by the player who won them, for use in `score_game`.
    pub fn tricks_by_player(&self) -> Vec<Vec<Trick>> {
        let mut by_player = vec![Vec::new(); self.player_count];
        for trick in &self.completed_tricks {
            if let Some(w) = trick.winner {
                by_player[w].push(trick.clone());
            }
        }
        by_player
    }

    pub fn new(game_id: Uuid, game_name: String, player_count: usize, dealer: usize) -> Self {
        Self {
            game_id,
            game_name,
            phase: GamePhase::Bidding,
            player_count,
            dealer,
            current_player: (dealer + 1) % player_count,
            hands: vec![Vec::new(); player_count],
            extra_piles: Vec::new(),
            current_trick: None,
            completed_tricks: Vec::new(),
            scores: vec![0; player_count],
            meta: serde_json::Value::Null,
            names: Vec::new(),
        }
    }

    /// Creates a GameState in Lobby phase (no hands dealt yet).
    #[allow(dead_code)]
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
}

// ---------------------------------------------------------------------------
// Messages: client → server
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// Join an existing room by ID, or create a new one (omit room_id).
    /// Set `fill_bots: true` to have the server fill remaining seats with bots (useful for local dev).
    JoinRoom { room_id: Option<Uuid>, game: String, players: usize, #[serde(default)] fill_bots: bool },
    /// Multiplayer: create a new private room and join it as host.
    CreateRoom { name: String, game: String, max_hands: Option<u32> },
    /// Multiplayer: join an existing room by short code.
    Join { name: String, room_code: String },
    /// Play a card during the Playing phase.
    PlayCard { card: Card },
    /// Generic bid payload; shape is game-specific.
    Bid { value: serde_json::Value },
    /// Send a chat message in the lobby.
    LobbyChat { text: String },
    /// Host starts the game from the lobby.
    StartGame,
    /// Host forces a disconnected seat to become a bot.
    ForceBot { seat: usize },
    /// Host extends the rejoin window for a disconnected seat by 30 seconds.
    ExtendRejoin { seat: usize },
    /// Join the public matchmaking queue.
    JoinQueue,
    /// Leave the public matchmaking queue.
    LeaveQueue,
}

// ---------------------------------------------------------------------------
// Messages: server → client
// ---------------------------------------------------------------------------

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StateUpdate {
    /// Sent immediately after a successful JoinRoom. Tells the client its seat index.
    JoinedRoom { room_id: Uuid, seat: usize, room_code: String },
    /// Full state snapshot sent once dealing is complete. Only `hands[seat]` is
    /// populated; other hands are empty. `extra_piles` is also cleared (blind is hidden).
    Snapshot { state: GameState },
    /// `current_trick_winner` is the seat currently winning the in-progress trick,
    /// or `None` if the trick just completed (server will emit TrickComplete separately).
    CardPlayed { player: usize, card: Card, current_trick_winner: Option<usize> },
    TrickComplete { winner: usize, points: u8 },
    /// Per-hand result plus running session totals. Sent after each hand.
    HandComplete { hand_scores: Vec<i32>, session_scores: Vec<i32> },
    /// Sent when a player reaches the victory goal. No further hands will be dealt.
    SessionOver { winner: usize, final_scores: Vec<i32> },
    /// Includes `current_player` so clients can advance their local turn pointer
    /// without knowing game-specific bid-ordering rules.
    BidPlaced { player: usize, value: serde_json::Value, current_player: usize },
    /// Sent privately to the player whose hand changed (e.g. picker took the blind).
    HandUpdated { hand: Vec<Card> },
    /// Broadcast when the game transitions between phases (Bidding → Playing, etc.).
    PhaseChanged { phase: GamePhase },
    /// Broadcast when a partner is revealed (e.g., Sheepshead partner mechanics).
    PartnerRevealed { seat: usize },
    /// Lobby chat message broadcast to all players.
    LobbyChat { from: String, text: String, timestamp: u64 },
    /// Seat status update broadcast to all players in the lobby.
    SeatUpdate { seats: Vec<SeatInfo> },
    /// Queue status for waiting players.
    QueueStatus { position: usize, waiting_since: u64 },
    Error { message: String },
}
