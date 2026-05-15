use serde::{Deserialize, Serialize};

use crate::engine::Card;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trick {
    /// Index of the player who led this trick.
    pub led_by: usize,
    /// Cards played in order: (player_index, card).
    pub plays: Vec<(usize, Card)>,
    /// Seat of the player who won this trick. Set when the trick completes.
    pub winner: Option<usize>,
}

impl Trick {
    pub fn new(led_by: usize) -> Self {
        Self { led_by, plays: Vec::new(), winner: None }
    }

    pub fn led_card(&self) -> Option<Card> {
        self.plays.first().map(|(_, c)| *c)
    }

    pub fn is_complete(&self, player_count: usize) -> bool {
        self.plays.len() == player_count
    }
}
