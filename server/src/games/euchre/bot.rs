use crate::engine::{Card, GameState, Rank, Suit, Trick};
use crate::engine::game::Game;
use crate::bot::{BotState, build_bot_state, current_winner, min_winning_trump, lowest_card};
use crate::games::euchre::rules::{plain_strength, trump_strength_for_suit};

fn is_trump(card: Card, suit: Suit) -> bool {
    trump_strength_for_suit(card, suit).is_some()
}

// ---------------------------------------------------------------------------
// Bidding
// ---------------------------------------------------------------------------

pub fn bid_action(state: &GameState, seat: usize) -> serde_json::Value {
    let sub_phase = state.meta["sub_phase"].as_str().unwrap_or("ordering");

    match sub_phase {
        "ordering" => bid_ordering(state, seat),
        "discarding" => bid_discarding(state, seat),
        "calling" => bid_calling(state, seat),
        _ => serde_json::json!({"action": "pass"}),
    }
}

fn bid_ordering(state: &GameState, seat: usize) -> serde_json::Value {
    let hand = &state.hands[seat];
    let turned_up: Card = match serde_json::from_value(state.meta["turned_up_card"].clone()) {
        Ok(c) => c,
        Err(_) => return serde_json::json!({"action": "pass"}),
    };
    let turned_suit = turned_up.suit;
    let dealer = state.dealer;

    // Count trump cards if turned-up suit were called.
    // For the dealer, also count the turned-up card itself (they'd pick it up).
    let mut trump_count = hand.iter().filter(|&&c| is_trump(c, turned_suit)).count();
    if seat == dealer && (!is_trump(turned_up, turned_suit) || !hand.contains(&turned_up)) {
        // turned_up card is always trump for its own suit (it's a card of that suit)
        // The dealer would receive it
        trump_count += 1;
    }

    if trump_count >= 5 {
        serde_json::json!({"action": "order_up", "alone": true})
    } else if trump_count >= 3 {
        serde_json::json!({"action": "order_up"})
    } else {
        serde_json::json!({"action": "pass"})
    }
}

fn bid_discarding(state: &GameState, seat: usize) -> serde_json::Value {
    let hand = &state.hands[seat];
    let called_suit = state.meta["called_suit"].as_str().and_then(Suit::from_str);

    // Discard the lowest plain (non-trump) card by rank.
    // If all trump, discard the weakest trump.
    if let Some(trump) = called_suit {
        let mut plain: Vec<Card> = hand.iter()
            .filter(|&&c| !is_trump(c, trump))
            .copied()
            .collect();
        if !plain.is_empty() {
            plain.sort_by_key(|c| plain_strength(*c));
            let discard = plain[0];
            return serde_json::json!({"action": "discard", "card": discard});
        }
        // All trump — discard weakest trump
        let mut trump_cards: Vec<Card> = hand.to_vec();
        trump_cards.sort_by_key(|c| trump_strength_for_suit(*c, trump));
        let discard = trump_cards[0];
        return serde_json::json!({"action": "discard", "card": discard});
    }

    // Fallback: discard first card
    serde_json::json!({"action": "discard", "card": hand[0]})
}

fn bid_calling(state: &GameState, seat: usize) -> serde_json::Value {
    let hand = &state.hands[seat];
    let turned_up: Card = match serde_json::from_value(state.meta["turned_up_card"].clone()) {
        Ok(c) => c,
        Err(_) => return serde_json::json!({"action": "pass"}),
    };
    let turned_suit = turned_up.suit;
    let passed_round2 = state.meta["passed_round2"].as_u64().unwrap_or(0) as usize;
    let dealer = state.dealer;
    let is_stuck = passed_round2 >= 3 && seat == dealer;

    let all_suits = [Suit::Clubs, Suit::Spades, Suit::Hearts, Suit::Diamonds];
    let callable_suits: Vec<Suit> = all_suits.iter()
        .filter(|&&s| s != turned_suit)
        .copied()
        .collect();

    // Count trump in hand for each callable suit
    let best = callable_suits.iter().max_by_key(|&&s| {
        hand.iter().filter(|&&c| is_trump(c, s)).count()
    });

    if let Some(&best_suit) = best {
        let count = hand.iter().filter(|&&c| is_trump(c, best_suit)).count();
        if is_stuck || count >= 2 {
            let alone = count >= 5;
            if alone {
                return serde_json::json!({"action": "call", "suit": best_suit.as_str(), "alone": true});
            }
            return serde_json::json!({"action": "call", "suit": best_suit.as_str()});
        }
    }

    if is_stuck {
        // Must call something — pick the best suit even with 0
        if let Some(&fallback) = callable_suits.first() {
            return serde_json::json!({"action": "call", "suit": fallback.as_str()});
        }
    }

    serde_json::json!({"action": "pass"})
}

// ---------------------------------------------------------------------------
// Card play
// ---------------------------------------------------------------------------

pub fn play_card(state: &GameState, seat: usize, game: &dyn Game) -> Option<Card> {
    let hand = &state.hands[seat];
    if hand.is_empty() {
        return None;
    }

    let bot_state = build_bot_state(state, game);

    let caller_seat = state.meta["caller_seat"].as_u64().map(|v| v as usize);
    let going_alone = state.meta["going_alone"].as_bool().unwrap_or(false);
    let caller_partner = caller_seat.map(|cs| (cs + 2) % 4);

    let is_maker = caller_seat == Some(seat)
        || (!going_alone && caller_partner == Some(seat));

    let teammate: Option<usize> = if is_maker {
        if going_alone { None } else { Some((seat + 2) % 4) }
    } else {
        // Defenders: teammate is the other defender
        Some((seat + 2) % 4)
    };

    match &state.current_trick {
        None => Some(choose_lead(hand, is_maker, game, state)),
        Some(trick) if trick.plays.is_empty() => Some(choose_lead(hand, is_maker, game, state)),
        Some(trick) => {
            let legal = game.legal_plays(hand, trick, state);
            Some(choose_follow(&legal, seat, trick, is_maker, teammate, &bot_state, game, state))
        }
    }
}

fn choose_lead(hand: &[Card], is_maker: bool, game: &dyn Game, state: &GameState) -> Card {
    if is_maker {
        // Lead highest trump
        let mut trump: Vec<Card> = hand.iter()
            .filter(|&&c| game.trump_rank(c, state).is_some())
            .copied()
            .collect();
        if !trump.is_empty() {
            trump.sort_by_key(|b| std::cmp::Reverse(game.trump_rank(*b, state)));
            return trump[0];
        }
    } else {
        // Defender: lead a plain-suit ace if available, otherwise lowest plain card
        let plain_ace = hand.iter().find(|&&c| {
            c.rank == Rank::Ace && game.trump_rank(c, state).is_none()
        }).copied();
        if let Some(ace) = plain_ace {
            return ace;
        }
    }

    lowest_card(hand, game, state)
}

#[allow(clippy::too_many_arguments)]
fn choose_follow(
    legal: &[Card],
    seat: usize,
    trick: &Trick,
    is_maker: bool,
    teammate: Option<usize>,
    _bot_state: &BotState,
    game: &dyn Game,
    state: &GameState,
) -> Card {
    let winner_seat = current_winner(trick, game, state);
    let team_winning = winner_seat == seat || teammate == Some(winner_seat);

    if team_winning {
        // Dump lowest card (no points in Euchre)
        return lowest_card(legal, game, state);
    }

    // Losing — try to win with minimum trump if possible
    if let Some(t) = min_winning_trump(legal, trick, game, state) {
        return t;
    }

    // Can't win — play lowest
    let _ = is_maker; // unused but kept for symmetry
    lowest_card(legal, game, state)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Card, Rank, Suit};
    use crate::engine::GameState;
    use uuid::Uuid;

    #[test]
    fn bot_orders_up_with_3_trump() {
        // Hand has 3 trump when clubs is the turned-up suit
        let mut state = GameState::new(Uuid::nil(), "euchre".into(), 4, 0);
        state.dealer = 0;
        let turned_up = Card::new(Suit::Clubs, Rank::Nine); // clubs is turned-up suit
        state.meta = serde_json::json!({
            "turned_up_card": turned_up,
            "sub_phase": "ordering",
            "passed_round1": 0,
            "passed_round2": 0,
            "caller_seat": null,
            "called_suit": null,
            "going_alone": false,
            "sits_out": null
        });
        // Seat 1 holds 3 clubs cards (all trump) + 2 non-trump
        state.hands[1] = vec![
            Card::new(Suit::Clubs, Rank::Ace),
            Card::new(Suit::Clubs, Rank::King),
            Card::new(Suit::Clubs, Rank::Queen),
            Card::new(Suit::Hearts, Rank::Nine),
            Card::new(Suit::Diamonds, Rank::Nine),
        ];
        state.current_player = 1;

        let action = bid_action(&state, 1);
        assert_eq!(action["action"].as_str(), Some("order_up"));
    }

    #[test]
    fn bot_passes_with_weak_hand() {
        let mut state = GameState::new(Uuid::nil(), "euchre".into(), 4, 0);
        state.dealer = 0;
        let turned_up = Card::new(Suit::Clubs, Rank::Nine);
        state.meta = serde_json::json!({
            "turned_up_card": turned_up,
            "sub_phase": "ordering",
            "passed_round1": 0,
            "passed_round2": 0,
            "caller_seat": null,
            "called_suit": null,
            "going_alone": false,
            "sits_out": null
        });
        // No clubs in hand (only 1 trump if ♠J is left bower, but no Jack here)
        state.hands[1] = vec![
            Card::new(Suit::Hearts, Rank::Nine),
            Card::new(Suit::Hearts, Rank::Ten),
            Card::new(Suit::Diamonds, Rank::Nine),
            Card::new(Suit::Diamonds, Rank::Ten),
            Card::new(Suit::Spades, Rank::Nine),
        ];
        state.current_player = 1;

        let action = bid_action(&state, 1);
        assert_eq!(action["action"].as_str(), Some("pass"));
    }
}
