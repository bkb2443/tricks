use crate::engine::{Card, GameState, GamePhase, Trick};

// ---------------------------------------------------------------------------
// Deal
// ---------------------------------------------------------------------------

/// Return value of `Game::deal`.
pub struct DealResult {
    /// `hands[i]` = cards dealt to player i.
    pub hands: Vec<Vec<Card>>,
    /// Named extra piles, e.g. `[("blind", [card1, card2])]` for Sheepshead.
    pub extra_piles: Vec<(String, Vec<Card>)>,
    /// Initial game-specific metadata stored in `GameState::meta`.
    pub initial_meta: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Bid
// ---------------------------------------------------------------------------

/// Outcome returned by `Game::apply_bid`.
pub struct BidResult {
    /// `true` when the bidding phase just finished (`GameState::phase` → `Playing`).
    /// The room should broadcast `PhaseChanged`.
    pub phase_complete: bool,
    /// Seat whose hand was modified (e.g. picker took the blind, or buried cards).
    /// The room should send that player a private `HandUpdated`.
    pub hand_updated_seat: Option<usize>,
    /// Optional override payload for the `BidPlaced` broadcast.
    /// When set, the room uses this instead of the raw client bid value.
    pub broadcast_payload: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Play
// ---------------------------------------------------------------------------

/// Outcome returned by `Game::apply_play`.
pub enum PlayResult {
    /// Trick still in progress; `GameState::current_player` has been advanced.
    Continuing,
    /// A trick just completed.
    TrickComplete { winner: usize, points: u8 },
    /// All tricks played; `GameState::phase` has been set to `Scoring`.
    GameOver { last_trick_winner: usize, last_trick_points: u8, scores: Vec<i32> },
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Every trick-taking game implements this trait. The engine calls through it;
/// no game-specific logic leaks into the engine layer.
#[allow(dead_code)]
pub trait Game: Send + Sync {
    fn name(&self) -> &'static str;
    fn valid_player_counts(&self) -> &'static [usize];

    /// Cards used by this game (may be a subset of a standard 52-card deck).
    fn build_deck(&self) -> Vec<Card>;

    /// Distribute `shuffled_deck` among `player_count` players. `dealer` is the
    /// dealing seat; games that deal starting left-of-dealer use it.
    fn deal(&self, shuffled_deck: Vec<Card>, player_count: usize, dealer: usize) -> DealResult;

    /// Trump strength of a card: `Some(n)` means trump with priority n (higher = stronger).
    /// `None` means not trump. `state` is provided so games with dynamic trump (Euchre)
    /// can consult the called suit stored in `meta`.
    fn trump_rank(&self, card: Card, state: &GameState) -> Option<u8>;

    /// Strength of a card within its plain (non-trump) suit. Only called when the card
    /// is not trump. Higher = stronger.
    fn plain_suit_rank(&self, card: Card) -> u8;

    /// Effective suit of a card for the purpose of following suit. Queens and Jacks are
    /// trump in Sheepshead, so their effective suit differs from `card.suit`.
    fn effective_suit(&self, card: Card, state: &GameState) -> EffectiveSuit;

    /// Cards in `hand` that are legal to play given the current trick and state.
    fn legal_plays(&self, hand: &[Card], trick: &Trick, state: &GameState) -> Vec<Card>;

    /// Points a card contributes when held in a player's trick pile.
    fn card_points(&self, card: Card) -> u8;

    /// Index within `trick.plays` of the winning play (called on a complete trick).
    fn trick_winner(&self, trick: &Trick, state: &GameState) -> usize;

    /// Final scores for all players given each player's collected tricks.
    /// `tricks_by_player[i]` is the list of complete tricks won by player i.
    fn score_game(&self, tricks_by_player: &[Vec<Trick>], state: &GameState) -> Vec<i32>;

    /// Whether this game has a bidding/calling phase before trick play begins.
    fn has_bidding(&self) -> bool {
        false
    }

    /// Apply a bid action from `seat`. Returns a `BidResult` on success or a
    /// user-facing error string. Default rejects all bids (for games without bidding).
    fn apply_bid(
        &self,
        state: &mut GameState,
        seat: usize,
        value: &serde_json::Value,
    ) -> Result<BidResult, String> {
        let _ = (state, seat, value);
        Err("this game has no bidding phase".into())
    }

    /// Validate and apply a card play. This default implementation handles the
    /// generic trick-taking flow (turn order, legality, trick completion, game end)
    /// by delegating game-specific decisions to `legal_plays`, `trick_winner`,
    /// `card_points`, and `score_game`. Games with unusual mechanics can override.
    fn apply_play(
        &self,
        state: &mut GameState,
        seat: usize,
        card: Card,
    ) -> Result<PlayResult, String> {
        if state.phase != GamePhase::Playing {
            return Err("game is not in the playing phase".into());
        }
        if state.current_player != seat {
            return Err(format!(
                "it is player {}'s turn, not player {seat}",
                state.current_player
            ));
        }
        if !state.hands[seat].contains(&card) {
            return Err(format!("{card} is not in your hand"));
        }

        // Open a new trick when the player is leading
        if state.current_trick.is_none() {
            state.current_trick = Some(Trick::new(seat));
        }

        // Legality check — clone to avoid a simultaneous borrow of state
        let legal = {
            let hand = state.hands[seat].clone();
            let trick = state.current_trick.as_ref().unwrap().clone();
            self.legal_plays(&hand, &trick, state)
        };
        if !legal.contains(&card) {
            return Err(format!("{card} is not a legal play"));
        }

        // Remove from hand, add to trick
        let pos = state.hands[seat].iter().position(|c| *c == card).unwrap();
        state.hands[seat].remove(pos);
        state.current_trick.as_mut().unwrap().plays.push((seat, card));

        // Not yet complete — advance to the next player in trick order
        let player_count = state.player_count;
        let plays_so_far = state.current_trick.as_ref().unwrap().plays.len();
        if plays_so_far < player_count {
            let led_by = state.current_trick.as_ref().unwrap().led_by;
            state.current_player = (led_by + plays_so_far) % player_count;
            return Ok(PlayResult::Continuing);
        }

        // Trick is complete
        let trick = state.current_trick.take().unwrap();
        let winner_idx = self.trick_winner(&trick, state);
        let winner_seat = trick.plays[winner_idx].0;
        let points: u8 = trick.plays.iter().map(|(_, c)| self.card_points(*c)).sum();

        let mut completed = trick;
        completed.winner = Some(winner_seat);
        state.completed_tricks.push(completed);
        state.current_player = winner_seat;

        // Game ends when every hand is empty
        if state.hands.iter().all(|h| h.is_empty()) {
            let tbp = state.tricks_by_player();
            let scores = self.score_game(&tbp, state);
            state.scores = scores.clone();
            state.phase = GamePhase::Scoring;
            return Ok(PlayResult::GameOver {
                last_trick_winner: winner_seat,
                last_trick_points: points,
                scores,
            });
        }

        Ok(PlayResult::TrickComplete { winner: winner_seat, points })
    }
}

/// The logical suit category of a card (distinguishes trump from plain suits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectiveSuit {
    Trump,
    Plain(crate::engine::Suit),
}
