use std::collections::{HashMap, HashSet};

use crate::engine::{Card, GameState, Rank, Trick};
use crate::engine::game::{EffectiveSuit, Game};
use crate::games::sheepshead::Sheepshead;

// ---------------------------------------------------------------------------
// BotState — derived fresh from GameState on every decision
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct BotState {
    /// All cards visible in completed and current trick plays.
    pub played_cards: HashSet<Card>,
    /// Suits a player has shown they cannot follow (inferred from trick history).
    pub known_voids: HashMap<usize, HashSet<EffectiveSuit>>,
    /// Reserved for future partner-calling mechanic; always None for now.
    pub predicted_partner: Option<usize>,
}

pub fn build_bot_state(state: &GameState, game: &dyn Game) -> BotState {
    let mut played_cards = HashSet::new();
    let mut known_voids: HashMap<usize, HashSet<EffectiveSuit>> = HashMap::new();

    for trick in &state.completed_tricks {
        let Some(&(_, led_card)) = trick.plays.first() else { continue };
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

    BotState { played_cards, known_voids, predicted_partner: None }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

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

#[allow(dead_code)]
fn picker_seat(state: &GameState) -> Option<usize> {
    state.meta["picker"].as_u64().map(|v| v as usize)
}

#[allow(dead_code)]
fn trick_points(trick: &Trick, game: &dyn Game) -> u8 {
    trick.plays.iter().map(|(_, c)| game.card_points(*c)).sum()
}

/// Winner seat of a partial or complete trick (does not require trick to be full).
#[allow(dead_code)]
fn current_winner(trick: &Trick, game: &dyn Game, state: &GameState) -> usize {
    debug_assert!(!trick.plays.is_empty(), "current_winner called on empty trick");
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
fn min_winning_trump(candidates: &[Card], trick: &Trick, game: &dyn Game, state: &GameState) -> Option<Card> {
    let best_trump_in_trick = trick.plays.iter()
        .filter_map(|(_, c)| game.trump_rank(*c, state))
        .max();

    let mut winners: Vec<Card> = candidates.iter().filter(|&&c| {
        match (game.trump_rank(c, state), best_trump_in_trick) {
            (Some(my), Some(best)) => my > best,
            (Some(_), None) => true,
            _ => false,
        }
    }).copied().collect();

    winners.sort_by_key(|c| game.trump_rank(*c, state).unwrap());
    winners.into_iter().next()
}

// ---------------------------------------------------------------------------
// Bidding
// ---------------------------------------------------------------------------

/// Bid JSON the bot should submit.
// FIXME: bid logic is hardcoded to Sheepshead; needs game: &dyn Game param when second game is added
pub fn bid_action(state: &GameState, seat: usize) -> serde_json::Value {
    if state.meta["picker"].is_null() {
        // Pick/pass sub-phase
        let hand = &state.hands[seat];
        if should_pick(hand, state, &Sheepshead) {
            serde_json::json!({ "action": "pick" })
        } else {
            serde_json::json!({ "action": "pass" })
        }
    } else {
        // Bury sub-phase
        let bury = choose_bury(&state.hands[seat], state, &Sheepshead);
        serde_json::json!({ "action": "bury", "cards": bury })
    }
}

// ---------------------------------------------------------------------------
// Leading and following
// ---------------------------------------------------------------------------

/// Play a card for the bot.
pub fn play_card(state: &GameState, seat: usize, game: &dyn Game) -> Option<Card> {
    let hand = &state.hands[seat];
    if hand.is_empty() {
        return None;
    }
    let bot_state = build_bot_state(state, game);
    match &state.current_trick {
        None => Some(choose_lead(hand, seat, state, &bot_state, game)),
        Some(trick) if trick.plays.is_empty() => Some(choose_lead(hand, seat, state, &bot_state, game)),
        Some(trick) => {
            let legal = game.legal_plays(hand, trick, state);
            Some(choose_follow(&legal, seat, trick, state, game))
        }
    }
}

// ---- stubs to be replaced in Tasks 2-5 ----

fn should_pick(hand: &[Card], state: &GameState, game: &dyn Game) -> bool {
    let qj_pts: u8 = hand.iter().map(|c| match c.rank {
        Rank::Queen => 3,
        Rank::Jack => 2,
        _ => 0,
    }).sum();

    if qj_pts < 7 {
        return false;
    }

    // Must have at least one non-face trump (a diamond that isn't Q/J)
    hand.iter().any(|c| {
        c.rank != Rank::Queen
            && c.rank != Rank::Jack
            && game.trump_rank(*c, state).is_some()
    })
}
fn choose_bury(hand: &[Card], state: &GameState, game: &dyn Game) -> Vec<Card> {
    // Partition into fail (non-trump) and trump
    let mut fail: Vec<Card> = hand.iter()
        .filter(|c| game.trump_rank(**c, state).is_none())
        .copied()
        .collect();

    if fail.len() >= 2 {
        // Sort by point value descending
        fail.sort_by_key(|c| std::cmp::Reverse(point_value(*c)));
        let first = fail[0];
        // Prefer second card from same suit to create a void
        let same_suit: Vec<Card> = fail[1..].iter()
            .filter(|c| c.suit == first.suit)
            .copied()
            .collect();
        let second = if !same_suit.is_empty() {
            same_suit[0]
        } else {
            fail[1]
        };
        return vec![first, second];
    }

    if fail.len() == 1 {
        // One fail card + lowest trump
        let fail_card = fail[0];
        let mut trump: Vec<Card> = hand.iter()
            .filter(|c| game.trump_rank(**c, state).is_some())
            .copied()
            .collect();
        trump.sort_by_key(|c| game.trump_rank(*c, state).unwrap());
        return vec![fail_card, trump[0]];
    }

    // All trump — bury the two weakest
    let mut trump: Vec<Card> = hand.to_vec();
    trump.sort_by_key(|c| game.trump_rank(*c, state).unwrap());
    trump.into_iter().take(2).collect()
}
fn choose_lead(hand: &[Card], seat: usize, state: &GameState, bs: &BotState, game: &dyn Game) -> Card {
    let is_picker = picker_seat(state) == Some(seat);

    let mut trump: Vec<Card> = hand.iter()
        .filter(|c| game.trump_rank(**c, state).is_some())
        .copied()
        .collect();
    trump.sort_by(|a, b| {
        game.trump_rank(*b, state).cmp(&game.trump_rank(*a, state))
    });

    if is_picker {
        // Lead highest trump to draw out defenders
        if !trump.is_empty() {
            return trump[0];
        }
        return lead_best_fail(hand, state, bs, game);
    }

    // Defender: lead safe fail aces (skip suits where picker is void)
    let picker = picker_seat(state);
    let picker_voids = picker.and_then(|p| bs.known_voids.get(&p));
    let safe_fail_aces: Vec<Card> = hand.iter()
        .filter(|c| {
            c.rank == Rank::Ace
                && game.trump_rank(**c, state).is_none()
                && picker_voids.is_none_or(|v| !v.contains(&game.effective_suit(**c, state)))
        })
        .copied()
        .collect();
    if !safe_fail_aces.is_empty() {
        return safe_fail_aces[0];
    }

    lead_best_fail(hand, state, bs, game)
}

fn lead_best_fail(hand: &[Card], state: &GameState, bs: &BotState, game: &dyn Game) -> Card {
    let picker = picker_seat(state);
    let mut fail: Vec<Card> = hand.iter()
        .filter(|c| game.trump_rank(**c, state).is_none())
        .copied()
        .collect();

    // Avoid leading into picker's known void (they'll trump in)
    if let Some(p) = picker
        && let Some(voids) = bs.known_voids.get(&p) { fail.retain(|c| !voids.contains(&game.effective_suit(*c, state))); }

    fail.sort_by_key(|c| std::cmp::Reverse(point_value(*c)));
    if let Some(&c) = fail.first() {
        return c;
    }

    // No safe fail card — lead lowest trump
    let mut trump: Vec<Card> = hand.iter()
        .filter(|c| game.trump_rank(**c, state).is_some())
        .copied()
        .collect();
    trump.sort_by_key(|c| game.trump_rank(*c, state).unwrap());
    trump.into_iter().next().unwrap_or(hand[0])
}
fn choose_follow(legal: &[Card], seat: usize, trick: &Trick, state: &GameState, game: &dyn Game) -> Card {
    let is_picker = picker_seat(state) == Some(seat);
    let trick_pts = trick_points(trick, game);
    let winner_seat = current_winner(trick, game, state);

    if is_picker {
        follow_as_picker(legal, trick, trick_pts, winner_seat == seat, game, state)
    } else {
        let picker = picker_seat(state);
        let picker_has_played = picker.is_none_or(|p| trick.plays.iter().any(|(s, _)| *s == p));
        let picker_is_winning = picker.is_some_and(|p| winner_seat == p);
        follow_as_defender(legal, trick, trick_pts, picker_has_played, picker_is_winning, game, state)
    }
}

fn follow_as_picker(legal: &[Card], trick: &Trick, trick_pts: u8, i_am_winning: bool, game: &dyn Game, state: &GameState) -> Card {
    if i_am_winning {
        // Already winning — conserve resources, play lowest card
        return lowest_card(legal, game, state);
    }

    // Currently losing
    // High-value trick — play min trump to recapture
    if trick_pts >= 10
        && let Some(t) = min_winning_trump(legal, trick, game, state) {
            return t;
        }

    // Low value or can't beat — throw lowest
    lowest_card(legal, game, state)
}

fn follow_as_defender(legal: &[Card], trick: &Trick, trick_pts: u8, picker_has_played: bool, picker_is_winning: bool, game: &dyn Game, state: &GameState) -> Card {
    if picker_has_played && !picker_is_winning {
        // Picker is out and losing — a fellow defender is winning
        // Dump highest-point card to load the trick
        return highest_point_card(legal, game, state);
    }

    if picker_has_played && picker_is_winning {
        if trick_pts >= 10
            && let Some(t) = min_winning_trump(legal, trick, game, state) {
                return t;
            }
        return lowest_card(legal, game, state);
    }

    // Picker has not yet played — risk-weight
    // Higher threshold (14 vs 10) because picker still has an unknown response;
    // require at least an Ace + some card value before risking trump into the unknown
    if trick_pts >= 14
        && let Some(t) = min_winning_trump(legal, trick, game, state) {
            return t;
        }

    // Low-value or can't win — throw lowest
    lowest_card(legal, game, state)
}

fn lowest_card(cards: &[Card], game: &dyn Game, state: &GameState) -> Card {
    // Prefer lowest non-trump; if none, lowest trump
    let mut fail: Vec<Card> = cards.iter()
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

fn highest_point_card(cards: &[Card], game: &dyn Game, state: &GameState) -> Card {
    // Prefer highest-point fail card; fall back to highest-point trump
    let mut fail: Vec<Card> = cards.iter()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Card, Rank, Suit, Trick};
    use crate::games::sheepshead::Sheepshead;

    fn make_card(suit: Suit, rank: Rank) -> Card { Card::new(suit, rank) }
    fn sheepshead() -> Sheepshead { Sheepshead }

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
        trick.plays = vec![(0, c1), (1, c2), (2, make_card(Suit::Clubs, Rank::Nine)),
                           (3, make_card(Suit::Clubs, Rank::Eight)), (4, make_card(Suit::Clubs, Rank::King))];
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
        trick.plays = vec![(0, led), (1, sluff), (2, make_card(Suit::Clubs, Rank::King)),
                           (3, make_card(Suit::Clubs, Rank::Nine)), (4, make_card(Suit::Clubs, Rank::Eight))];
        trick.winner = Some(0);
        let state = state_with_tricks(vec![trick], 5);
        let bs = build_bot_state(&state, &game);
        use crate::engine::game::EffectiveSuit;
        assert!(bs.known_voids.get(&1)
            .map_or(false, |v| v.contains(&EffectiveSuit::Plain(Suit::Clubs))));
    }

    fn hand_from(cards: &[(Suit, Rank)]) -> Vec<Card> {
        cards.iter().map(|&(s, r)| Card::new(s, r)).collect()
    }

    fn state_with_picker(picker: usize) -> GameState {
        use uuid::Uuid;
        let mut s = GameState::new(Uuid::new_v4(), "sheepshead".into(), 5, 0);
        s.meta = serde_json::json!({ "picker": picker });
        s
    }

    #[test]
    fn picker_leads_highest_trump() {
        let state = state_with_picker(0);
        let bs = BotState { played_cards: HashSet::new(), known_voids: HashMap::new(), predicted_partner: None };
        let hand = hand_from(&[
            (Suit::Clubs, Rank::Queen),   // strongest trump
            (Suit::Spades, Rank::Jack),
            (Suit::Diamonds, Rank::Nine),
            (Suit::Clubs, Rank::Ace),
            (Suit::Spades, Rank::Ace),
            (Suit::Hearts, Rank::Ace),
        ]);
        let lead = choose_lead(&hand, 0, &state, &bs, &sheepshead());
        assert_eq!(lead, Card::new(Suit::Clubs, Rank::Queen));
    }

    #[test]
    fn defender_leads_fail_ace() {
        let state = state_with_picker(0);
        let bs = BotState { played_cards: HashSet::new(), known_voids: HashMap::new(), predicted_partner: None };
        // Seat 1 is a defender
        let hand = hand_from(&[
            (Suit::Clubs, Rank::Queen),  // trump
            (Suit::Clubs, Rank::Ace),    // fail ace — should lead this
            (Suit::Hearts, Rank::King),
            (Suit::Spades, Rank::Nine),
            (Suit::Hearts, Rank::Seven),
            (Suit::Spades, Rank::Eight),
        ]);
        let lead = choose_lead(&hand, 1, &state, &bs, &sheepshead());
        assert_eq!(lead, Card::new(Suit::Clubs, Rank::Ace));
    }

    #[test]
    fn defender_avoids_leading_into_picker_void() {
        let state = state_with_picker(0);
        // Mark picker (seat 0) as void in clubs
        let mut known_voids: HashMap<usize, HashSet<EffectiveSuit>> = HashMap::new();
        known_voids.insert(0, {
            let mut s = HashSet::new();
            s.insert(EffectiveSuit::Plain(Suit::Clubs));
            s
        });
        let bs = BotState { played_cards: HashSet::new(), known_voids, predicted_partner: None };
        let hand = hand_from(&[
            (Suit::Clubs, Rank::Ace),    // clubs — picker void, avoid
            (Suit::Hearts, Rank::Ace),   // hearts — safe, lead this
            (Suit::Clubs, Rank::King),
            (Suit::Hearts, Rank::King),
            (Suit::Spades, Rank::Nine),
            (Suit::Spades, Rank::Eight),
        ]);
        let lead = choose_lead(&hand, 1, &state, &bs, &sheepshead());
        // Should lead hearts ace, not clubs ace
        assert_eq!(lead, Card::new(Suit::Hearts, Rank::Ace));
    }

    fn trick_with_plays(plays: Vec<(usize, Suit, Rank)>) -> Trick {
        let mut t = Trick::new(plays[0].0);
        t.plays = plays.iter().map(|&(seat, suit, rank)| (seat, Card::new(suit, rank))).collect();
        t
    }

    #[test]
    fn picker_following_plays_min_trump_to_recapture_high_value_trick() {
        // Trick has A♠(11pts) + 10♠(10pts) = 21pts; picker(0) is next, currently losing
        // Legal plays: J♥ (trump 8) and Q♣ (trump 14) — should play J♥ (minimum winning trump)
        let state = state_with_picker(0);
        let trick = trick_with_plays(vec![
            (1, Suit::Spades, Rank::Ace),
            (2, Suit::Spades, Rank::Ten),
        ]);
        let legal = vec![
            Card::new(Suit::Hearts, Rank::Jack), // J♥ trump strength 8
            Card::new(Suit::Clubs, Rank::Queen),  // Q♣ trump strength 14
        ];
        let play = choose_follow(&legal, 0, &trick, &state, &sheepshead());
        // Trick pts = 21 (≥10), play min winning trump = J♥ (strength 8)
        assert_eq!(play, Card::new(Suit::Hearts, Rank::Jack));
    }

    #[test]
    fn defender_dumps_points_when_picker_already_lost() {
        // Picker(0) led K♦ (trump 4), seat 1 trumped over with Q♣ (trump 14).
        // Seat 2 (defender) is next — picker has played and is losing.
        // Seat 2 should dump highest-point card (A♣ fail = 11 pts), not waste trump.
        let state = state_with_picker(0);
        let trick = trick_with_plays(vec![
            (0, Suit::Diamonds, Rank::King),  // picker led K♦ (trump 4)
            (1, Suit::Clubs, Rank::Queen),    // defender trumped over with Q♣ (trump 14)
        ]);
        let legal = vec![
            Card::new(Suit::Diamonds, Rank::Jack), // J♦ is trump (strength 7)
            Card::new(Suit::Clubs, Rank::Ace),     // A♣ fail (11 pts)
        ];
        let play = choose_follow(&legal, 2, &trick, &state, &sheepshead());
        assert_eq!(play, Card::new(Suit::Clubs, Rank::Ace));
    }

    #[test]
    fn defender_plays_low_when_picker_winning_low_value_trick() {
        // Seat 1 led 7♣, picker(0) played Q♣ (trump 14, winning).
        // Seat 2 (defender) following. Trick is low value (3pts) — throw lowest, not trump.
        let state = state_with_picker(0);
        let trick = trick_with_plays(vec![
            (1, Suit::Clubs, Rank::Seven),
            (0, Suit::Clubs, Rank::Queen), // picker winning with Q♣ (trump 14)
        ]);
        let legal = vec![
            Card::new(Suit::Clubs, Rank::Eight),  // fail 0pts
            Card::new(Suit::Clubs, Rank::Nine),   // fail 0pts
        ];
        let play = choose_follow(&legal, 2, &trick, &state, &sheepshead());
        // Any low card is fine — just not trump
        assert!(sheepshead().trump_rank(play, &state).is_none());
    }

    fn base_state() -> GameState {
        use uuid::Uuid;
        GameState::new(Uuid::new_v4(), "sheepshead".into(), 5, 0)
    }

    #[test]
    fn pick_threshold_exactly_7() {
        // Q♣(3) + J♠(2) + J♥(2) = 7 — should pick
        let hand = hand_from(&[
            (Suit::Clubs, Rank::Queen),
            (Suit::Spades, Rank::Jack),
            (Suit::Hearts, Rank::Jack),
            (Suit::Clubs, Rank::Ace),    // fail ace
            (Suit::Spades, Rank::Ace),   // fail ace
            (Suit::Diamonds, Rank::Nine), // non-face trump (secondary check)
        ]);
        assert!(should_pick(&hand, &base_state(), &sheepshead()));
    }

    #[test]
    fn pick_threshold_below_7_fails() {
        // Q♣(3) + J♠(2) = 5 — should pass
        let hand = hand_from(&[
            (Suit::Clubs, Rank::Queen),
            (Suit::Spades, Rank::Jack),
            (Suit::Clubs, Rank::Ace),
            (Suit::Spades, Rank::Ace),
            (Suit::Hearts, Rank::Ace),
            (Suit::Diamonds, Rank::Nine),
        ]);
        assert!(!should_pick(&hand, &base_state(), &sheepshead()));
    }

    #[test]
    fn pick_requires_non_face_trump() {
        // Q♣(3) + Q♠(3) + J♠(2) = 8 but no non-face trump — should pass
        let hand = hand_from(&[
            (Suit::Clubs, Rank::Queen),
            (Suit::Spades, Rank::Queen),
            (Suit::Spades, Rank::Jack),
            (Suit::Clubs, Rank::Ace),
            (Suit::Clubs, Rank::Ten),
            (Suit::Clubs, Rank::King),
        ]);
        assert!(!should_pick(&hand, &base_state(), &sheepshead()));
    }

    #[test]
    fn bury_prefers_fail_aces_and_tens() {
        // Hand has fail ace, fail ten, and trump — should bury the ace and ten
        let hand = hand_from(&[
            (Suit::Clubs, Rank::Queen),   // trump — keep
            (Suit::Spades, Rank::Jack),   // trump — keep
            (Suit::Diamonds, Rank::Nine), // trump — keep
            (Suit::Clubs, Rank::Ace),     // fail ace — bury first
            (Suit::Clubs, Rank::Ten),     // fail ten — bury second
            (Suit::Hearts, Rank::King),   // fail king
        ]);
        let buried = choose_bury(&hand, &base_state(), &sheepshead());
        assert_eq!(buried.len(), 2);
        assert!(buried.contains(&Card::new(Suit::Clubs, Rank::Ace)));
        assert!(buried.contains(&Card::new(Suit::Clubs, Rank::Ten)));
    }

    #[test]
    fn bury_avoids_trump_when_fail_available() {
        let hand = hand_from(&[
            (Suit::Clubs, Rank::Queen),
            (Suit::Spades, Rank::Jack),
            (Suit::Diamonds, Rank::Nine),
            (Suit::Diamonds, Rank::Eight),
            (Suit::Clubs, Rank::Ace),    // bury this
            (Suit::Hearts, Rank::King),  // bury this
        ]);
        let buried = choose_bury(&hand, &base_state(), &sheepshead());
        // Neither buried card should be trump
        for c in &buried {
            assert!(sheepshead().trump_rank(*c, &base_state()).is_none(),
                "buried trump card {c}");
        }
    }

    #[test]
    fn bury_creates_void_when_possible() {
        // Two clubs available — burying both creates a clubs void
        let hand = hand_from(&[
            (Suit::Clubs, Rank::Queen),
            (Suit::Spades, Rank::Jack),
            (Suit::Diamonds, Rank::Nine),
            (Suit::Clubs, Rank::Ace),   // highest point fail
            (Suit::Clubs, Rank::King),  // same suit — prefer over Hearts king (creates void)
            (Suit::Hearts, Rank::King),
        ]);
        let buried = choose_bury(&hand, &base_state(), &sheepshead());
        assert_eq!(buried.len(), 2);
        // Both buried should be clubs (creating void)
        assert!(buried.iter().all(|c| c.suit == Suit::Clubs));
    }
}
