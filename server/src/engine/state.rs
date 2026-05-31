use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::engine::game::Game;
use crate::engine::meta::{GameMeta, LobbyMeta};
use crate::engine::{Card, Trick};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ChatMessage {
    pub from: String,
    pub text: String,
    #[ts(type = "number")]
    pub timestamp: u64,
}

/// A hint card suggested to the player in training mode, paired with a human-readable reason.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HintCard {
    pub card: Card,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum GamePhase {
    Lobby,
    Bidding,
    Playing,
    Scoring,
    /// Between hands: waiting for the next dealer to start the next hand.
    Intermission,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SeatInfo {
    pub seat: usize,
    /// "empty" | "human" | "bot" | "disconnected"
    pub state: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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
    /// Cumulative per-player scores across all hands played so far in this session.
    #[serde(default)]
    pub session_scores: Vec<i32>,
    /// Game-specific metadata. Typed via `GameMeta` so the TypeScript client
    /// receives a discriminated union rather than `unknown`.
    pub meta: GameMeta,
    /// Display name for each seat. Populated by the room before the first Snapshot.
    #[serde(default)]
    pub names: Vec<String>,
    /// True when this room is in training mode (solo rooms only).
    #[serde(default)]
    pub training_mode: bool,
    /// True when the player has enabled best-card hint suggestions.
    #[serde(default)]
    pub hint_enabled: bool,
    /// Legal cards for the current player's seat (populated only in training mode,
    /// only for the snapshot sent to that player).
    #[serde(default)]
    pub legal_cards: Vec<Card>,
    /// Best-card hint (populated in training mode when hint_enabled is true and
    /// it is the player's turn). Not populated by `redacted_for`; the room populates
    /// it before sending a private snapshot.
    #[serde(default)]
    pub hint: Option<HintCard>,
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
            session_scores: vec![0; player_count],
            meta: GameMeta::None,
            names: Vec::new(),
            training_mode: false,
            hint_enabled: false,
            legal_cards: Vec::new(),
            hint: None,
        }
    }

    /// Build a snapshot of this state as viewed by `seat`:
    ///   * every hand other than `seat`'s is cleared
    ///   * any extra pile not in `game.visible_extra_piles(self, seat)` is removed
    ///   * in training mode, `legal_cards` is populated for the requested seat
    ///
    /// Single source of truth for snapshot redaction. The room sends the result to
    /// one player as a `Snapshot` message; nothing else should re-implement either
    /// the hand-clearing or the pile-filtering inline.
    ///
    /// Note: `hint` is NOT populated here. The room populates it before sending the
    /// private snapshot (so it can call `bot::play_card` without a borrow conflict).
    pub fn redacted_for(&self, seat: usize, game: &dyn Game) -> GameState {
        let mut view = self.clone();
        for (i, hand) in view.hands.iter_mut().enumerate() {
            if i != seat {
                hand.clear();
            }
        }
        let visible = game.visible_extra_piles(self, seat);
        view.extra_piles
            .retain(|(name, _)| visible.contains(&name.as_str()));

        // In training mode, populate legal_cards for this seat.
        view.legal_cards = if self.training_mode && self.phase == GamePhase::Playing {
            match &self.current_trick {
                None => self.hands[seat].clone(),
                Some(trick) if trick.plays.is_empty() => self.hands[seat].clone(),
                Some(trick) => {
                    let hand = self.hands[seat].clone();
                    let trick_clone = trick.clone();
                    game.legal_plays(&hand, &trick_clone, self)
                }
            }
        } else {
            Vec::new()
        };

        // hint is always cleared here; the room sets it after calling bot::play_card.
        view.hint = None;

        view
    }

    /// Build a snapshot for a spectator: all hands, extra piles, legal_cards, and hint cleared.
    pub fn redacted_for_spectator(&self) -> GameState {
        let mut view = self.clone();
        for hand in view.hands.iter_mut() {
            hand.clear();
        }
        view.extra_piles.clear();
        view.legal_cards.clear();
        view.hint = None;
        view
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
            session_scores: vec![0; player_count],
            meta: GameMeta::Lobby(LobbyMeta {
                host_seat: None,
                countdown_ends_at: None,
                room_type: room_type.to_string(),
                max_hands,
            }),
            names: Vec::new(),
            training_mode: false,
            hint_enabled: false,
            legal_cards: Vec::new(),
            hint: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Messages: client → server
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum ClientMessage {
    /// Join an existing room by ID, or create a new one (omit room_id).
    /// Set `fill_bots: true` to have the server fill remaining seats with bots (useful for local dev).
    /// Set `training_mode: true` to enable training mode for this session (solo only).
    /// Optionally set `training_tutorial_id` to load a specific scripted tutorial hand.
    JoinRoom {
        room_id: Option<Uuid>,
        game: String,
        players: usize,
        #[serde(default)]
        fill_bots: bool,
        #[serde(default)]
        training_mode: bool,
        #[serde(default)]
        training_tutorial_id: Option<String>,
    },
    /// Multiplayer: create a new private room and join it as host.
    CreateRoom {
        name: String,
        game: String,
        max_hands: Option<u32>,
    },
    /// Multiplayer: join an existing room by short code.
    Join { name: String, room_code: String },
    /// Join an existing room as a spectator (no seat, no game actions).
    Spectate { name: String, room_code: String },
    /// Play a card during the Playing phase.
    PlayCard { card: Card },
    /// Generic bid payload; shape is game-specific.
    /// The server uses `serde_json::Value` internally so it can forward the raw JSON
    /// to each game's `apply_bid` without ambiguity. The client should send a `BidPayload`.
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
    /// Next dealer advances from Intermission to start the next hand.
    StartNextHand,
    /// Toggle the best-card hint on/off (training mode only). No fields — server flips hint_enabled.
    ToggleHint,
}

// ---------------------------------------------------------------------------
// Messages: server → client
// ---------------------------------------------------------------------------

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export)]
pub enum StateUpdate {
    /// Sent immediately after a successful JoinRoom. Tells the client its seat index.
    JoinedRoom {
        room_id: Uuid,
        seat: usize,
        room_code: String,
    },
    /// Sent immediately after a successful Spectate. No seat assigned.
    JoinedAsSpectator {
        room_id: Uuid,
        room_code: String,
    },
    /// Full state snapshot sent once dealing is complete. Only `hands[seat]` is
    /// populated; other hands are empty. `extra_piles` is also cleared (blind is hidden).
    Snapshot {
        state: GameState,
    },
    /// `current_trick_winner` is the seat currently winning the in-progress trick,
    /// or `None` if the trick just completed (server will emit TrickComplete separately).
    /// `next_player` is the seat whose turn it is after this play.
    CardPlayed {
        player: usize,
        card: Card,
        current_trick_winner: Option<usize>,
        next_player: usize,
    },
    TrickComplete {
        winner: usize,
        points: u8,
    },
    /// Per-hand result plus running session totals. Sent after each hand.
    HandComplete {
        hand_scores: Vec<i32>,
        session_scores: Vec<i32>,
    },
    /// Sent when a player reaches the victory goal. No further hands will be dealt.
    SessionOver {
        winner: usize,
        final_scores: Vec<i32>,
    },
    /// Includes `current_player` so clients can advance their local turn pointer
    /// without knowing game-specific bid-ordering rules.
    BidPlaced {
        player: usize,
        value: serde_json::Value,
        current_player: usize,
    },
    /// Sent privately to the player whose hand changed (e.g. picker took the blind).
    HandUpdated {
        hand: Vec<Card>,
    },
    /// Broadcast when the game transitions between phases (Bidding → Playing, etc.).
    PhaseChanged {
        phase: GamePhase,
    },
    /// Broadcast when a partner is revealed (e.g., Sheepshead partner mechanics).
    PartnerRevealed {
        seat: usize,
    },
    /// Lobby chat message broadcast to all players.
    LobbyChat {
        from: String,
        text: String,
        #[ts(type = "number")]
        timestamp: u64,
    },
    /// Seat status update broadcast to all players in the lobby.
    SeatUpdate {
        seats: Vec<SeatInfo>,
        #[serde(default)]
        spectator_count: usize,
    },
    /// Queue status for waiting players.
    QueueStatus {
        position: usize,
        #[ts(type = "number")]
        waiting_since: u64,
    },
    Error {
        message: String,
    },
    /// Sent privately to the human player during a tutorial to display narration text.
    TutorialNarration {
        text: String,
    },
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Suit;
    use crate::engine::card::Rank;
    use crate::engine::game::{BidResult, DealResult, EffectiveSuit, PlayResult};
    use crate::engine::meta::SheepsheadMeta;

    /// Minimal `Game` impl for testing the redactor. `visible_extra_piles` returns
    /// whatever the test configured at construction time.
    struct MockGame {
        visible: Vec<&'static str>,
    }

    impl Game for MockGame {
        fn name(&self) -> &'static str {
            "mock"
        }
        fn valid_player_counts(&self) -> &'static [usize] {
            &[3]
        }
        fn build_deck(&self) -> Vec<Card> {
            Vec::new()
        }
        fn deal(&self, _: Vec<Card>, _: usize, _: usize) -> DealResult {
            DealResult {
                hands: Vec::new(),
                extra_piles: Vec::new(),
                initial_meta: GameMeta::None,
            }
        }
        fn trump_rank(&self, _: Card, _: &GameState) -> Option<u8> {
            None
        }
        fn plain_suit_rank(&self, _: Card) -> u8 {
            0
        }
        fn effective_suit(&self, c: Card, _: &GameState) -> EffectiveSuit {
            EffectiveSuit::Plain(c.suit)
        }
        fn legal_plays(&self, hand: &[Card], _: &Trick, _: &GameState) -> Vec<Card> {
            hand.to_vec()
        }
        fn card_points(&self, _: Card) -> u8 {
            0
        }
        fn trick_winner(&self, _: &Trick, _: &GameState) -> usize {
            0
        }
        fn score_game(&self, _: &[Vec<Trick>], _: &GameState) -> Vec<i32> {
            Vec::new()
        }
        fn apply_bid(
            &self,
            _: &mut GameState,
            _: usize,
            _: &serde_json::Value,
        ) -> Result<BidResult, String> {
            Err("no bidding".into())
        }
        fn apply_play(&self, _: &mut GameState, _: usize, _: Card) -> Result<PlayResult, String> {
            Ok(PlayResult::Continuing)
        }
        fn visible_extra_piles(&self, _: &GameState, _: usize) -> Vec<&'static str> {
            self.visible.clone()
        }
    }

    fn populated_state() -> GameState {
        let mut s = GameState::new(Uuid::nil(), "mock".into(), 3, 0);
        let c = Card::new(Suit::Clubs, Rank::Ace);
        s.hands = vec![vec![c, c], vec![c], vec![c, c, c]];
        s.extra_piles = vec![
            ("blind".to_string(), vec![c]),
            ("kitty".to_string(), vec![c, c]),
        ];
        s
    }

    #[test]
    fn redacted_for_clears_other_hands() {
        let state = populated_state();
        let game = MockGame { visible: vec![] };
        let view = state.redacted_for(1, &game);
        assert!(view.hands[0].is_empty());
        assert_eq!(view.hands[1].len(), 1, "viewer's own hand is preserved");
        assert!(view.hands[2].is_empty());
    }

    #[test]
    fn redacted_for_hides_extra_piles_by_default() {
        let state = populated_state();
        let game = MockGame { visible: vec![] };
        let view = state.redacted_for(0, &game);
        assert!(
            view.extra_piles.is_empty(),
            "default trait impl hides all piles"
        );
    }

    #[test]
    fn redacted_for_keeps_piles_listed_as_visible() {
        let state = populated_state();
        let game = MockGame {
            visible: vec!["blind"],
        };
        let view = state.redacted_for(0, &game);
        assert_eq!(view.extra_piles.len(), 1);
        assert_eq!(view.extra_piles[0].0, "blind");
    }

    #[test]
    fn redacted_for_keeps_multiple_visible_piles_and_drops_others() {
        let state = populated_state();
        let game = MockGame {
            visible: vec!["blind", "kitty", "graveyard"],
        };
        let view = state.redacted_for(2, &game);
        let names: Vec<&str> = view.extra_piles.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec!["blind", "kitty"],
            "only piles that exist AND are visible survive"
        );
    }

    #[test]
    fn redacted_for_does_not_mutate_source() {
        let state = populated_state();
        let game = MockGame { visible: vec![] };
        let _ = state.redacted_for(0, &game);
        assert_eq!(state.hands[1].len(), 1, "source hands unchanged");
        assert_eq!(state.hands[2].len(), 3, "source hands unchanged");
        assert_eq!(state.extra_piles.len(), 2, "source extra_piles unchanged");
    }

    #[test]
    fn redacted_for_preserves_meta_and_other_fields() {
        let mut state = populated_state();
        state.meta = GameMeta::Sheepshead(SheepsheadMeta {
            picker: Some(1),
            sub_phase: "done".into(),
            passed: 0,
            leaster: false,
            buried: vec![],
            callable_suits: vec![],
            called_suit: Some("clubs".into()),
            going_alone: false,
            partner: None,
        });
        state.current_player = 2;
        state.dealer = 1;
        let game = MockGame { visible: vec![] };
        let view = state.redacted_for(0, &game);
        // meta is not redacted by this helper — verify the variant is preserved
        assert!(matches!(&view.meta, GameMeta::Sheepshead(m) if m.picker == Some(1)));
        assert_eq!(view.current_player, 2);
        assert_eq!(view.dealer, 1);
    }
}
