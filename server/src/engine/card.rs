use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Suit {
    Clubs,
    Spades,
    Hearts,
    Diamonds,
}

/// Ranks present in a standard deck. Games that use fewer ranks (e.g. Sheepshead
/// drops 2–6) enforce this in `Game::build_deck`, not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Suit {
    pub fn as_str(self) -> &'static str {
        match self {
            Suit::Clubs => "clubs",
            Suit::Spades => "spades",
            Suit::Hearts => "hearts",
            Suit::Diamonds => "diamonds",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "clubs" => Some(Suit::Clubs),
            "spades" => Some(Suit::Spades),
            "hearts" => Some(Suit::Hearts),
            "diamonds" => Some(Suit::Diamonds),
            _ => None,
        }
    }
}

impl Card {
    pub fn new(suit: Suit, rank: Rank) -> Self {
        Self { suit, rank }
    }
}

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rank = match self.rank {
            Rank::Two => "2",
            Rank::Three => "3",
            Rank::Four => "4",
            Rank::Five => "5",
            Rank::Six => "6",
            Rank::Seven => "7",
            Rank::Eight => "8",
            Rank::Nine => "9",
            Rank::Ten => "10",
            Rank::Jack => "J",
            Rank::Queen => "Q",
            Rank::King => "K",
            Rank::Ace => "A",
        };
        let suit = match self.suit {
            Suit::Clubs => "♣",
            Suit::Spades => "♠",
            Suit::Hearts => "♥",
            Suit::Diamonds => "♦",
        };
        write!(f, "{rank}{suit}")
    }
}
