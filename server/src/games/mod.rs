pub mod sheepshead;

use crate::engine::Game;

/// Look up a game implementation by name.
pub fn get_game(name: &str) -> Option<Box<dyn Game>> {
    match name {
        "sheepshead" => Some(Box::new(sheepshead::Sheepshead)),
        _ => None,
    }
}
