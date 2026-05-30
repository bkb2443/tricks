pub mod euchre;
pub mod sheepshead;

use crate::engine::game::Game;
use crate::engine::{Card, GameState};

/// Look up a game implementation by name.
pub fn get_game(name: &str) -> Option<Box<dyn Game>> {
    match name {
        "sheepshead" => Some(Box::new(sheepshead::Sheepshead)),
        "euchre" => Some(Box::new(euchre::Euchre)),
        _ => None,
    }
}

pub fn bot_bid(state: &GameState, seat: usize) -> serde_json::Value {
    match state.game_name.as_str() {
        "sheepshead" => sheepshead::bot::bid_action(state, seat),
        "euchre" => euchre::bot::bid_action(state, seat),
        _ => serde_json::json!({"action": "pass"}),
    }
}

pub fn bot_play(state: &GameState, seat: usize, game: &dyn Game) -> Option<Card> {
    match state.game_name.as_str() {
        "sheepshead" => sheepshead::bot::play_card(state, seat, game),
        "euchre" => euchre::bot::play_card(state, seat, game),
        _ => None,
    }
}
