use rand::seq::SliceRandom;

use crate::engine::{Game, GamePhase, GameState};

/// Shuffle the deck, deal via the game's rules, and populate `state`.
/// Transitions the phase to `Bidding` or `Playing` depending on `game.has_bidding()`.
pub fn deal_game(game: &dyn Game, state: &mut GameState, rng: &mut impl rand::Rng) {
    let mut deck = game.build_deck();
    deck.shuffle(rng);

    let result = game.deal(deck, state.player_count, state.dealer);
    state.hands = result.hands;
    state.extra_piles = result.extra_piles;
    state.meta = result.initial_meta;
    state.phase = if game.has_bidding() { GamePhase::Bidding } else { GamePhase::Playing };
}
