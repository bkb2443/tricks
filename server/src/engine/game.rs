use crate::engine::{Card, GameMeta, GamePhase, GameState, Trick};

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
    pub initial_meta: GameMeta,
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
    GameOver {
        last_trick_winner: usize,
        last_trick_points: u8,
        scores: Vec<i32>,
    },
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Shared trick-taking logic. Used by the default `apply_play` and by game implementations
/// that need pre/post hooks (e.g., Sheepshead partner revelation).
///
/// `active_seats`: when `Some`, only those seats participate (going-alone support).
/// The trick completes after `active_seats.len()` plays and next-player advancement
/// skips any seat not in the list. When `None`, all seats participate normally.
pub fn apply_play_generic(
    game: &dyn Game,
    state: &mut GameState,
    seat: usize,
    card: Card,
    active_seats: Option<&[usize]>,
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
        game.legal_plays(&hand, &trick, state)
    };
    if !legal.contains(&card) {
        return Err(format!("{card} is not a legal play"));
    }

    // Remove from hand, add to trick
    let pos = state.hands[seat].iter().position(|c| *c == card).unwrap();
    state.hands[seat].remove(pos);
    state
        .current_trick
        .as_mut()
        .unwrap()
        .plays
        .push((seat, card));

    let player_count = state.player_count;
    let trick_size = active_seats.map(|s| s.len()).unwrap_or(player_count);
    let plays_so_far = state.current_trick.as_ref().unwrap().plays.len();

    // Not yet complete — advance to the next active player
    if plays_so_far < trick_size {
        let led_by = state.current_trick.as_ref().unwrap().led_by;
        state.current_player = match active_seats {
            None => (led_by + plays_so_far) % player_count,
            Some(active) => next_active_player(led_by, plays_so_far, active, player_count),
        };
        return Ok(PlayResult::Continuing);
    }

    // Trick is complete
    let trick = state.current_trick.take().unwrap();
    let winner_idx = game.trick_winner(&trick, state);
    let winner_seat = trick.plays[winner_idx].0;
    let points: u8 = trick.plays.iter().map(|(_, c)| game.card_points(*c)).sum();

    let mut completed = trick;
    completed.winner = Some(winner_seat);
    state.completed_tricks.push(completed);
    state.current_player = winner_seat;

    // Game ends when all participating hands are empty
    let game_over = match active_seats {
        None => state.hands.iter().all(|h| h.is_empty()),
        Some(active) => active.iter().all(|&s| state.hands[s].is_empty()),
    };
    if game_over {
        let tbp = state.tricks_by_player();
        let scores = game.score_game(&tbp, state);
        state.scores = scores.clone();
        state.phase = GamePhase::Scoring;
        return Ok(PlayResult::GameOver {
            last_trick_winner: winner_seat,
            last_trick_points: points,
            scores,
        });
    }

    Ok(PlayResult::TrickComplete {
        winner: winner_seat,
        points,
    })
}

/// Returns the seat of the `plays_so_far`-th player in active-seat rotation from `led_by`.
fn next_active_player(
    led_by: usize,
    plays_so_far: usize,
    active: &[usize],
    player_count: usize,
) -> usize {
    let mut count = 0usize;
    let mut seat = led_by;
    loop {
        seat = (seat + 1) % player_count;
        if !active.contains(&seat) {
            continue;
        }
        if count + 1 == plays_so_far {
            return seat;
        }
        count += 1;
    }
}

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
    ) -> Result<PlayResult, String>;

    /// Names of extra piles that should be visible to `seat` in a snapshot.
    /// Default: no piles are visible (i.e. the kitty/blind is hidden from everyone).
    /// Games override this when a pile becomes visible to a given seat —
    /// e.g. a Sheepshead variant that reveals the blind face-up after picking.
    fn visible_extra_piles(&self, state: &GameState, seat: usize) -> Vec<&'static str> {
        let _ = (state, seat);
        Vec::new()
    }

    /// Default maximum number of hands per match. `None` means no hand-count limit
    /// (termination is score-based via `match_over`).
    fn default_max_hands(&self) -> Option<u32> {
        None
    }

    /// Returns `true` when the match should end based on cumulative scores or hand
    /// count. Called after every hand completes. For score-based termination (e.g.
    /// Euchre reaching 10 points). The hand-count limit is checked separately by
    /// the room — this method is for score-based termination only.
    fn match_over(&self, cumulative_scores: &[i32], hands_played: usize) -> bool {
        let _ = (cumulative_scores, hands_played);
        false
    }

    /// Returns the winning seat index when the match is over. Called only when
    /// `match_over` returned `true` or the hand limit was reached. Returns `None`
    /// if no clear winner can be determined (room falls back to highest score).
    fn match_winner(&self, cumulative_scores: &[i32]) -> Option<usize> {
        let _ = cumulative_scores;
        None
    }

    /// Returns scripted tutorial hands for this game. Default: none.
    fn tutorials(&self) -> &'static [crate::engine::tutorial::TutorialHand] {
        &[]
    }

    /// Given the card the bot chose to play (or the hint card), return a short human-readable
    /// reason string for why this card is good. Used in training mode hint display.
    /// Default: empty string (hint text suppressed).
    fn hint_reason(&self, card: Card, state: &GameState, seat: usize) -> &'static str {
        let _ = (card, state, seat);
        ""
    }
}

/// The logical suit category of a card (distinguishes trump from plain suits).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectiveSuit {
    Trump,
    Plain(crate::engine::Suit),
}
