use std::collections::{HashMap, HashSet};

use crate::engine::game::{EffectiveSuit, Game};
use crate::engine::{Card, GameState, Rank, Trick};

// ---------------------------------------------------------------------------
// BotState — derived fresh from GameState on every decision
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct BotState {
    /// All cards visible in completed and current trick plays.
    pub played_cards: HashSet<Card>,
    /// Suits a player has shown they cannot follow (inferred from trick history).
    pub known_voids: HashMap<usize, HashSet<EffectiveSuit>>,
    /// Predicted partner seat, inferred from void tracking or meta revelation.
    pub predicted_partner: Option<usize>,
}

pub fn build_bot_state(state: &GameState, game: &dyn Game) -> BotState {
    let mut played_cards = HashSet::new();
    let mut known_voids: HashMap<usize, HashSet<EffectiveSuit>> = HashMap::new();

    for trick in &state.completed_tricks {
        let Some(&(_, led_card)) = trick.plays.first() else {
            continue;
        };
        let led_suit = game.effective_suit(led_card, state);

        for &(seat, card) in &trick.plays {
            played_cards.insert(card);
            let card_suit = game.effective_suit(card, state);
            if card_suit != led_suit {
                known_voids.entry(seat).or_default().insert(led_suit);
            }
        }
    }

    if let Some(trick) = &state.current_trick {
        for &(_, card) in &trick.plays {
            played_cards.insert(card);
        }
    }

    // No partner prediction at the generic level (game-specific)
    let predicted_partner = None;

    BotState {
        played_cards,
        known_voids,
        predicted_partner,
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

pub fn point_value(card: Card) -> u8 {
    match card.rank {
        Rank::Ace => 11,
        Rank::Ten => 10,
        Rank::King => 4,
        Rank::Queen => 3,
        Rank::Jack => 2,
        _ => 0,
    }
}

#[allow(dead_code)]
pub fn trick_points(trick: &Trick, game: &dyn Game) -> u8 {
    trick.plays.iter().map(|(_, c)| game.card_points(*c)).sum()
}

/// Winner seat of a partial or complete trick (does not require trick to be full).
#[allow(dead_code)]
pub fn current_winner(trick: &Trick, game: &dyn Game, state: &GameState) -> usize {
    debug_assert!(
        !trick.plays.is_empty(),
        "current_winner called on empty trick"
    );
    let mut best = 0usize;
    let mut best_trump = game.trump_rank(trick.plays[0].1, state);
    let led_suit = game.effective_suit(trick.plays[0].1, state);

    for (i, &(_, card)) in trick.plays.iter().enumerate().skip(1) {
        let t = game.trump_rank(card, state);
        let beats = match (best_trump, t) {
            (None, Some(_)) => true,
            (Some(b), Some(my)) => my > b,
            (None, None) => {
                game.effective_suit(card, state) == led_suit
                    && game.plain_suit_rank(card) > game.plain_suit_rank(trick.plays[best].1)
            }
            (Some(_), None) => false,
        };
        if beats {
            best = i;
            best_trump = t;
        }
    }

    trick.plays[best].0
}

/// Lowest trump in `candidates` that beats every play in `trick`. Returns None if none can win.
#[allow(dead_code)]
pub fn min_winning_trump(
    candidates: &[Card],
    trick: &Trick,
    game: &dyn Game,
    state: &GameState,
) -> Option<Card> {
    let best_trump_in_trick = trick
        .plays
        .iter()
        .filter_map(|(_, c)| game.trump_rank(*c, state))
        .max();

    let mut winners: Vec<Card> = candidates
        .iter()
        .filter(
            |&&c| match (game.trump_rank(c, state), best_trump_in_trick) {
                (Some(my), Some(best)) => my > best,
                (Some(_), None) => true,
                _ => false,
            },
        )
        .copied()
        .collect();

    winners.sort_by_key(|c| game.trump_rank(*c, state).unwrap());
    winners.into_iter().next()
}

pub fn lowest_card(cards: &[Card], game: &dyn Game, state: &GameState) -> Card {
    // Prefer lowest non-trump; if none, lowest trump
    let mut fail: Vec<Card> = cards
        .iter()
        .filter(|c| game.trump_rank(**c, state).is_none())
        .copied()
        .collect();
    fail.sort_by_key(|c| point_value(*c));
    if let Some(&c) = fail.first() {
        return c;
    }
    let mut trump: Vec<Card> = cards.to_vec();
    trump.sort_by_key(|c| game.trump_rank(*c, state).unwrap_or(255));
    trump.into_iter().next().unwrap_or(cards[0])
}

pub fn highest_point_card(cards: &[Card], game: &dyn Game, state: &GameState) -> Card {
    // Prefer highest-point fail card; fall back to highest-point trump
    let mut fail: Vec<Card> = cards
        .iter()
        .filter(|c| game.trump_rank(**c, state).is_none())
        .copied()
        .collect();
    fail.sort_by_key(|c| std::cmp::Reverse(point_value(*c)));
    if let Some(&c) = fail.first() {
        return c;
    }
    let mut all = cards.to_vec();
    all.sort_by_key(|c| std::cmp::Reverse(point_value(*c)));
    all.into_iter().next().unwrap_or(cards[0])
}

// ---------------------------------------------------------------------------
// Public dispatchers (called by lobby/room.rs)
// ---------------------------------------------------------------------------

/// Bid JSON the bot should submit. Dispatches to the per-game bot module.
pub fn bid_action(state: &GameState, seat: usize) -> serde_json::Value {
    crate::games::bot_bid(state, seat)
}

/// Play a card for the bot. Dispatches to the per-game bot module.
pub fn play_card(state: &GameState, seat: usize, game: &dyn Game) -> Option<Card> {
    crate::games::bot_play(state, seat, game)
}

// ---------------------------------------------------------------------------
// Tests (generic only)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Card, Rank, Suit, Trick};
    use crate::games::sheepshead::Sheepshead;

    fn make_card(suit: Suit, rank: Rank) -> Card {
        Card::new(suit, rank)
    }
    fn sheepshead() -> Sheepshead {
        Sheepshead
    }

    fn state_with_tricks(tricks: Vec<Trick>, player_count: usize) -> GameState {
        use uuid::Uuid;
        let mut s = GameState::new(Uuid::new_v4(), "sheepshead".into(), player_count, 0);
        s.completed_tricks = tricks;
        s
    }

    #[test]
    fn bot_state_tracks_played_cards() {
        let game = sheepshead();
        let c1 = make_card(Suit::Clubs, Rank::Ace);
        let c2 = make_card(Suit::Hearts, Rank::Seven);
        let mut trick = Trick::new(0);
        trick.plays = vec![
            (0, c1),
            (1, c2),
            (2, make_card(Suit::Clubs, Rank::Nine)),
            (3, make_card(Suit::Clubs, Rank::Eight)),
            (4, make_card(Suit::Clubs, Rank::King)),
        ];
        trick.winner = Some(0);
        let state = state_with_tricks(vec![trick], 5);
        let bs = build_bot_state(&state, &game);
        assert!(bs.played_cards.contains(&c1));
        assert!(bs.played_cards.contains(&c2));
    }

    #[test]
    fn bot_state_infers_void() {
        let game = sheepshead();
        let led = make_card(Suit::Clubs, Rank::Ace);
        let sluff = make_card(Suit::Hearts, Rank::Seven);
        let mut trick = Trick::new(0);
        trick.plays = vec![
            (0, led),
            (1, sluff),
            (2, make_card(Suit::Clubs, Rank::King)),
            (3, make_card(Suit::Clubs, Rank::Nine)),
            (4, make_card(Suit::Clubs, Rank::Eight)),
        ];
        trick.winner = Some(0);
        let state = state_with_tricks(vec![trick], 5);
        let bs = build_bot_state(&state, &game);
        use crate::engine::game::EffectiveSuit;
        assert!(
            bs.known_voids
                .get(&1)
                .map_or(false, |v| v.contains(&EffectiveSuit::Plain(Suit::Clubs)))
        );
    }
}
