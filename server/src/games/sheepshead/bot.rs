use std::collections::{HashMap, HashSet};

use crate::engine::{Card, GameState, Rank, Suit, Trick};
use crate::engine::game::{EffectiveSuit, Game};
use crate::bot::{BotState, build_bot_state, point_value, current_winner, min_winning_trump,
                  trick_points, lowest_card, highest_point_card};
use crate::games::sheepshead::Sheepshead;

// ---------------------------------------------------------------------------
// Sheepshead-specific helpers
// ---------------------------------------------------------------------------

fn picker_seat(state: &GameState) -> Option<usize> {
    state.meta["picker"].as_u64().map(|v| v as usize)
}

#[allow(dead_code)]
fn predict_partner(
    state: &GameState,
    known_voids: &HashMap<usize, HashSet<EffectiveSuit>>,
) -> Option<usize> {
    // Definitive: partner is revealed in meta
    if let Some(p) = state.meta["partner"].as_u64() {
        return Some(p as usize);
    }

    // Going alone or leaster — no partner
    if state.meta["going_alone"].as_bool().unwrap_or(false)
        || state.meta["leaster"].as_bool().unwrap_or(false)
    {
        return None;
    }

    // Pre-revelation inference: if called suit is known, players void in it can't be partner
    let picker = state.meta["picker"].as_u64().map(|p| p as usize)?;
    let called_suit_str = state.meta["called_suit"].as_str()?;
    let called_suit: Suit =
        serde_json::from_str(&format!("\"{}\"", called_suit_str)).ok()?;
    let called_eff = EffectiveSuit::Plain(called_suit);

    let candidates: Vec<usize> = (0..state.player_count)
        .filter(|&s| {
            s != picker
                && !known_voids
                    .get(&s)
                    .is_some_and(|v| v.contains(&called_eff))
        })
        .collect();

    if candidates.len() == 1 {
        Some(candidates[0])
    } else {
        None
    }
}

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

fn choose_call(state: &GameState, seat: usize) -> serde_json::Value {
    let hand = &state.hands[seat];

    // Go alone if holding 5+ high trump (Jacks and Queens — trump rank ≥ 7)
    let high_trump_count = hand
        .iter()
        .filter(|c| Sheepshead.trump_rank(**c, state).is_some_and(|r| r >= 7))
        .count();

    if high_trump_count >= 5 {
        return serde_json::json!({ "action": "go_alone" });
    }

    // Call the suit where we hold the most non-trump cards (best chance to lead it)
    let callable: Vec<String> = state.meta["callable_suits"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if callable.is_empty() {
        return serde_json::json!({ "action": "go_alone" });
    }

    // Pick the suit with the most non-trump, non-ace cards in our hand
    let best_suit = callable.iter().max_by_key(|suit_str| {
        let suit: Option<Suit> =
            serde_json::from_str(&format!("\"{}\"", suit_str)).ok();
        suit.map_or(0, |s| {
            hand.iter()
                .filter(|c| {
                    c.suit == s
                        && c.rank != Rank::Ace
                        && Sheepshead.trump_rank(**c, state).is_none()
                })
                .count()
        })
    });

    match best_suit {
        Some(suit) => serde_json::json!({ "action": "call", "suit": suit }),
        None => serde_json::json!({ "action": "go_alone" }),
    }
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

fn follow_as_team_member(
    legal: &[Card],
    trick: &Trick,
    team_winning: bool,
    i_am_winning: bool,
    game: &dyn Game,
    state: &GameState,
) -> Card {
    let trick_pts = trick_points(trick, game);

    if team_winning && !i_am_winning {
        // Teammate is winning — dump highest-point card
        return highest_point_card(legal, game, state);
    }
    if i_am_winning {
        return lowest_card(legal, game, state);
    }
    // Team is losing — try to recapture if worthwhile
    if trick_pts >= 10
        && let Some(t) = min_winning_trump(legal, trick, game, state) {
            return t;
        }
    lowest_card(legal, game, state)
}

fn follow_as_defender_ext(
    legal: &[Card],
    trick: &Trick,
    picker_has_played: bool,
    partner_has_played: bool,
    enemy_winning: bool,
    game: &dyn Game,
    state: &GameState,
) -> Card {
    let trick_pts = trick_points(trick, game);
    let all_enemies_played = picker_has_played && partner_has_played;

    if all_enemies_played && !enemy_winning {
        // All enemies out, defender is winning — dump points
        return highest_point_card(legal, game, state);
    }

    if all_enemies_played && enemy_winning {
        // Enemy winning, all have played — beat if valuable
        if trick_pts >= 10
            && let Some(t) = min_winning_trump(legal, trick, game, state) {
                return t;
            }
        return lowest_card(legal, game, state);
    }

    // Some enemies haven't played yet — be cautious
    if trick_pts >= 14
        && let Some(t) = min_winning_trump(legal, trick, game, state) {
            return t;
        }
    lowest_card(legal, game, state)
}

fn choose_follow(
    legal: &[Card],
    seat: usize,
    trick: &Trick,
    state: &GameState,
    bs: &BotState,
    game: &dyn Game,
) -> Card {
    let picker = picker_seat(state);
    let is_picker = picker == Some(seat);
    let is_partner = bs.predicted_partner == Some(seat);

    if is_picker || is_partner {
        // On the winning team
        let teammate = if is_picker {
            bs.predicted_partner
        } else {
            picker
        };
        let winner_seat = current_winner(trick, game, state);
        let team_winning = winner_seat == seat
            || (teammate == Some(winner_seat));
        follow_as_team_member(legal, trick, team_winning, winner_seat == seat, game, state)
    } else {
        // Defender
        let picker_has_played = picker
            .is_none_or(|p| trick.plays.iter().any(|(s, _)| *s == p));
        let partner_has_played = bs
            .predicted_partner
            .is_none_or(|p| trick.plays.iter().any(|(s, _)| *s == p));
        let winner_seat = current_winner(trick, game, state);
        let enemy_winning = picker
            .into_iter()
            .chain(bs.predicted_partner)
            .any(|e| winner_seat == e);
        follow_as_defender_ext(
            legal,
            trick,
            picker_has_played,
            partner_has_played,
            enemy_winning,
            game,
            state,
        )
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn bid_action(state: &GameState, seat: usize) -> serde_json::Value {
    let sub_phase = state.meta["sub_phase"].as_str().unwrap_or("picking");

    match sub_phase {
        "picking" => {
            if should_pick(&state.hands[seat], state, &Sheepshead) {
                serde_json::json!({ "action": "pick" })
            } else {
                serde_json::json!({ "action": "pass" })
            }
        }
        "burying" => {
            let bury = choose_bury(&state.hands[seat], state, &Sheepshead);
            serde_json::json!({ "action": "bury", "cards": bury })
        }
        "calling" => choose_call(state, seat),
        _ => serde_json::json!({ "action": "pass" }),
    }
}

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
            Some(choose_follow(&legal, seat, trick, state, &bot_state, game))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Card, Rank, Suit, Trick};
    use crate::games::sheepshead::Sheepshead;

    fn sheepshead() -> Sheepshead { Sheepshead }

    fn state_with_picker(picker: usize) -> GameState {
        use uuid::Uuid;
        let mut s = GameState::new(Uuid::new_v4(), "sheepshead".into(), 5, 0);
        s.meta = serde_json::json!({ "picker": picker });
        s
    }

    fn hand_from(cards: &[(Suit, Rank)]) -> Vec<Card> {
        cards.iter().map(|&(s, r)| Card::new(s, r)).collect()
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
        let bs = BotState { played_cards: HashSet::new(), known_voids: HashMap::new(), predicted_partner: None };
        let play = choose_follow(&legal, 0, &trick, &state, &bs, &sheepshead());
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
        let bs = BotState { played_cards: HashSet::new(), known_voids: HashMap::new(), predicted_partner: None };
        let play = choose_follow(&legal, 2, &trick, &state, &bs, &sheepshead());
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
        let bs = BotState { played_cards: HashSet::new(), known_voids: HashMap::new(), predicted_partner: None };
        let play = choose_follow(&legal, 2, &trick, &state, &bs, &sheepshead());
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
