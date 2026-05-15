use rand::seq::SliceRandom;

use crate::engine::{Card, GameState, Rank};
use crate::engine::game::Game;

fn point_value(card: Card) -> u8 {
    match card.rank {
        Rank::Ace => 11,
        Rank::Ten => 10,
        Rank::King => 4,
        Rank::Queen => 3,
        Rank::Jack => 2,
        _ => 0,
    }
}

/// Bid JSON the bot should submit.
/// Pick/pass sub-phase: always pass.
/// Bury sub-phase (bot somehow became picker): bury the 2 lowest-point cards.
pub fn bid_action(state: &GameState, seat: usize) -> serde_json::Value {
    if state.meta["picker"].is_null() {
        serde_json::json!({ "action": "pass" })
    } else {
        let mut hand = state.hands[seat].clone();
        hand.sort_by_key(|c| point_value(*c));
        let bury: Vec<Card> = hand.into_iter().take(2).collect();
        serde_json::json!({ "action": "bury", "cards": bury })
    }
}

/// Random legal card for the bot to play.
pub fn play_card(state: &GameState, seat: usize, game: &dyn Game) -> Option<Card> {
    let hand = &state.hands[seat];
    if hand.is_empty() {
        return None;
    }
    let legal = match &state.current_trick {
        Some(trick) => game.legal_plays(hand, trick, state),
        None => hand.to_vec(),
    };
    let mut rng = rand::thread_rng();
    legal.choose(&mut rng).copied()
}
