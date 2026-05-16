use std::sync::Arc;

use crate::lobby::Lobby;

/// Public matchmaking queue. Fully implemented in Task 4.
pub struct Matchmaker {
    _lobby: Arc<Lobby>,
}

impl Matchmaker {
    pub fn new(lobby: Arc<Lobby>) -> Self {
        Self { _lobby: lobby }
    }
}
