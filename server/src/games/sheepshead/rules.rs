use crate::engine::{
    BidResult, Card, DealResult, GamePhase, GameState, Rank, Suit, Trick,
    game::{EffectiveSuit, Game},
};

pub struct Sheepshead;

// ---------------------------------------------------------------------------
// Trump ordering (higher = stronger)
//
// Trump cards (14 total):
//   14 ♣Q  13 ♠Q  12 ♥Q  11 ♦Q
//   10 ♣J   9 ♠J   8 ♥J   7 ♦J
//    6 A♦   5 10♦   4 K♦   3 9♦   2 8♦   1 7♦
// ---------------------------------------------------------------------------
fn trump_strength(card: Card) -> Option<u8> {
    match (card.rank, card.suit) {
        (Rank::Queen, Suit::Clubs) => Some(14),
        (Rank::Queen, Suit::Spades) => Some(13),
        (Rank::Queen, Suit::Hearts) => Some(12),
        (Rank::Queen, Suit::Diamonds) => Some(11),
        (Rank::Jack, Suit::Clubs) => Some(10),
        (Rank::Jack, Suit::Spades) => Some(9),
        (Rank::Jack, Suit::Hearts) => Some(8),
        (Rank::Jack, Suit::Diamonds) => Some(7),
        (Rank::Ace, Suit::Diamonds) => Some(6),
        (Rank::Ten, Suit::Diamonds) => Some(5),
        (Rank::King, Suit::Diamonds) => Some(4),
        (Rank::Nine, Suit::Diamonds) => Some(3),
        (Rank::Eight, Suit::Diamonds) => Some(2),
        (Rank::Seven, Suit::Diamonds) => Some(1),
        _ => None,
    }
}

// Plain-suit rank (A > 10 > K > 9 > 8 > 7). Q/J never appear here (always trump).
fn plain_strength(card: Card) -> u8 {
    match card.rank {
        Rank::Ace => 6,
        Rank::Ten => 5,
        Rank::King => 4,
        Rank::Nine => 3,
        Rank::Eight => 2,
        Rank::Seven => 1,
        _ => 0,
    }
}

impl Game for Sheepshead {
    fn name(&self) -> &'static str {
        "sheepshead"
    }

    fn valid_player_counts(&self) -> &'static [usize] {
        &[5]
    }

    fn build_deck(&self) -> Vec<Card> {
        // 32-card deck: ranks 7–A across all four suits
        let ranks = [
            Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten,
            Rank::Jack, Rank::Queen, Rank::King, Rank::Ace,
        ];
        let suits = [Suit::Clubs, Suit::Spades, Suit::Hearts, Suit::Diamonds];
        suits
            .iter()
            .flat_map(|&suit| ranks.iter().map(move |&rank| Card::new(suit, rank)))
            .collect()
    }

    /// Traditional 3–blind–3 packet deal, starting left of the dealer.
    ///
    /// Sequence: 3 cards each → 2 cards to blind → 3 cards each.
    /// Total: 5×6 + 2 = 32 cards.
    fn deal(&self, shuffled_deck: Vec<Card>, player_count: usize, dealer: usize) -> DealResult {
        assert_eq!(player_count, 5, "Sheepshead requires exactly 5 players");
        assert_eq!(shuffled_deck.len(), 32, "Sheepshead deck must be 32 cards");

        let mut deck = shuffled_deck.into_iter();
        let mut hands: Vec<Vec<Card>> = vec![Vec::new(); player_count];

        // First packet: 3 cards to each player, starting left of dealer
        for offset in 0..player_count {
            let seat = (dealer + 1 + offset) % player_count;
            hands[seat].extend(deck.by_ref().take(3));
        }

        let blind: Vec<Card> = deck.by_ref().take(2).collect();

        // Second packet: 3 more cards to each player, same order
        for offset in 0..player_count {
            let seat = (dealer + 1 + offset) % player_count;
            hands[seat].extend(deck.by_ref().take(3));
        }

        DealResult {
            hands,
            extra_piles: vec![("blind".to_string(), blind)],
            // `picker` null  → picking sub-phase (current_player = left of dealer)
            // `picker` set   → burying sub-phase (current_player = picker)
            // `passed`       → how many players have passed so far
            // `buried`       → two cards the picker chose to set aside
            // `leaster`      → true when all five players pass
            initial_meta: serde_json::json!({
                "picker":         null,
                "sub_phase":      "picking",
                "passed":         0,
                "buried":         [],
                "leaster":        false,
                "callable_suits": [],
                "called_suit":    null,
                "going_alone":    false,
                "partner":        null
            }),
        }
    }

    // ---------------------------------------------------------------------------
    // Bidding / picking phase
    // ---------------------------------------------------------------------------

    fn has_bidding(&self) -> bool {
        true
    }

    /// Handle pick, pass, and bury actions.
    ///
    /// Picking sub-phase  (`meta.picker == null`):
    ///   `{ "action": "pick" }`  — take the blind; move to burying sub-phase.
    ///   `{ "action": "pass" }`  — decline; advance to next player (or leaster).
    ///
    /// Burying sub-phase  (`meta.picker` is set):
    ///   `{ "action": "bury", "cards": [<card>, <card>] }`  — discard 2 cards,
    ///   store in `meta.buried`, transition to `Playing`.
    fn apply_bid(
        &self,
        state: &mut GameState,
        seat: usize,
        value: &serde_json::Value,
    ) -> Result<BidResult, String> {
        if state.phase != GamePhase::Bidding {
            return Err("game is not in the bidding phase".into());
        }

        let action = value["action"].as_str().ok_or("missing 'action' field")?;
        let sub_phase = state.meta["sub_phase"].as_str().unwrap_or("picking");

        match sub_phase {
            "picking" => self.handle_pick_or_pass(state, seat, action),
            "burying" => self.handle_bury(state, seat, action, value),
            "calling" => self.handle_call(state, seat, action, value),
            _ => Err(format!("unknown sub_phase '{sub_phase}'")),
        }
    }

    fn trump_rank(&self, card: Card, _state: &GameState) -> Option<u8> {
        trump_strength(card)
    }

    fn plain_suit_rank(&self, card: Card) -> u8 {
        plain_strength(card)
    }

    fn effective_suit(&self, card: Card, _state: &GameState) -> EffectiveSuit {
        if trump_strength(card).is_some() {
            EffectiveSuit::Trump
        } else {
            EffectiveSuit::Plain(card.suit)
        }
    }

    fn legal_plays(&self, hand: &[Card], trick: &Trick, state: &GameState) -> Vec<Card> {
        let Some(led) = trick.led_card() else {
            return hand.to_vec();
        };
        let led_suit = self.effective_suit(led, state);
        let matching: Vec<Card> = hand
            .iter()
            .filter(|&&c| self.effective_suit(c, state) == led_suit)
            .copied()
            .collect();
        if matching.is_empty() { hand.to_vec() } else { matching }
    }

    fn card_points(&self, card: Card) -> u8 {
        match card.rank {
            Rank::Ace => 11,
            Rank::Ten => 10,
            Rank::King => 4,
            Rank::Queen => 3,
            Rank::Jack => 2,
            _ => 0,
        }
    }

    fn trick_winner(&self, trick: &Trick, state: &GameState) -> usize {
        let led = trick.plays[0].1;
        let led_suit = self.effective_suit(led, state);
        trick
            .plays
            .iter()
            .enumerate()
            .max_by_key(|(_, (_, card))| {
                if let Some(t) = self.trump_rank(*card, state) {
                    1000 + t as u32
                } else if self.effective_suit(*card, state) == led_suit {
                    self.plain_suit_rank(*card) as u32
                } else {
                    0
                }
            })
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    fn score_game(&self, tricks_by_player: &[Vec<Trick>], state: &GameState) -> Vec<i32> {
        let raw: Vec<i32> = tricks_by_player
            .iter()
            .map(|tricks| {
                tricks
                    .iter()
                    .flat_map(|t| t.plays.iter().map(|(_, c)| self.card_points(*c) as i32))
                    .sum()
            })
            .collect();

        let n = raw.len() as i32;

        if state.meta["leaster"].as_bool().unwrap_or(false) {
            // Leaster: the player with the most card points loses; all others gain 1 VP.
            // Zero-sum: loser -(n-1), each winner +1.
            let max_pts = raw.iter().max().copied().unwrap_or(0);
            let loser = raw.iter().position(|&p| p == max_pts).unwrap_or(0);
            return (0..raw.len())
                .map(|i| if i == loser { -(n - 1) } else { 1 })
                .collect();
        }

        let picker = state.meta["picker"].as_u64().unwrap_or(0) as usize;
        let picker_points = raw[picker];

        // Buried cards count toward the picker's total at scoring time.
        let buried_points: i32 = state.meta["buried"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| serde_json::from_value::<Card>(v.clone()).ok())
            .map(|c| self.card_points(c) as i32)
            .sum();

        let picker_total = picker_points + buried_points;
        let defender_total = 120 - picker_total;
        let picker_wins = picker_total > 60;

        // Schneider: the losing side has ≤30 card points.
        let schneider = if picker_wins {
            defender_total <= 30
        } else {
            picker_total <= 30
        };

        // Base multiplier: 1 for a normal result, 2 when schneider.
        // Zero-sum exchange model (n=5):
        //   Normal win:      picker +(n-1),     each defender -1
        //   Schneider win:   picker +2(n-1),    each defender -2
        //   Normal loss:     picker -2(n-1),    each defender +2
        //   Schneider loss:  picker -4(n-1),    each defender +4
        let base = if schneider { 2_i32 } else { 1_i32 };
        let defenders = n - 1;

        (0..raw.len())
            .map(|i| {
                if i == picker {
                    if picker_wins { base * defenders } else { -(base * 2 * defenders) }
                } else {
                    if picker_wins { -base } else { base * 2 }
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Returns the fail suits whose ace the picker can legally call.
/// Callable if: picker doesn't hold the ace AND picker holds at least one
/// other non-trump card of that suit.
fn callable_suits(hand: &[Card]) -> Vec<Suit> {
    [Suit::Clubs, Suit::Spades, Suit::Hearts]
        .iter()
        .copied()
        .filter(|&suit| {
            let has_ace = hand.contains(&Card::new(suit, Rank::Ace));
            let has_other = hand.iter().any(|c| {
                c.suit == suit && c.rank != Rank::Ace && trump_strength(*c).is_none()
            });
            !has_ace && has_other
        })
        .collect()
}

impl Sheepshead {
    fn handle_pick_or_pass(
        &self,
        state: &mut GameState,
        seat: usize,
        action: &str,
    ) -> Result<BidResult, String> {
        if state.current_player != seat {
            return Err(format!(
                "it is player {}'s turn to decide, not player {seat}",
                state.current_player
            ));
        }

        match action {
            "pick" => {
                // Move blind into picker's hand (hand grows to 8 cards temporarily)
                let blind_pos = state
                    .extra_piles
                    .iter()
                    .position(|(name, _)| name == "blind")
                    .ok_or("blind not found — has it already been taken?")?;
                let (_, blind) = state.extra_piles.remove(blind_pos);
                state.hands[seat].extend(blind);

                state.meta["picker"] = serde_json::json!(seat);
                state.meta["sub_phase"] = serde_json::json!("burying");
                // current_player stays as `seat` — they must now bury 2 cards

                Ok(BidResult { phase_complete: false, hand_updated_seat: Some(seat), broadcast_payload: None })
            }

            "pass" => {
                let passed = state.meta["passed"].as_u64().unwrap_or(0) as usize + 1;
                state.meta["passed"] = serde_json::json!(passed);

                if passed >= state.player_count {
                    // All five players passed → leaster
                    state.meta["leaster"] = serde_json::json!(true);
                    state.phase = GamePhase::Playing;
                    state.current_player = (state.dealer + 1) % state.player_count;
                    Ok(BidResult { phase_complete: true, hand_updated_seat: None, broadcast_payload: None })
                } else {
                    state.current_player = (state.dealer + 1 + passed) % state.player_count;
                    Ok(BidResult { phase_complete: false, hand_updated_seat: None, broadcast_payload: None })
                }
            }

            _ => Err(format!("unknown picking action '{action}'; expected 'pick' or 'pass'")),
        }
    }

    fn handle_bury(
        &self,
        state: &mut GameState,
        seat: usize,
        action: &str,
        value: &serde_json::Value,
    ) -> Result<BidResult, String> {
        let picker = state.meta["picker"]
            .as_u64()
            .ok_or_else(|| "picker not set".to_string())?
            as usize;

        if seat != picker {
            return Err(format!("only the picker (player {picker}) can bury cards"));
        }
        if action != "bury" {
            return Err(format!("expected 'bury' action during burying phase, got '{action}'"));
        }

        let cards_arr = value["cards"]
            .as_array()
            .ok_or("'cards' must be an array of exactly 2 cards")?;
        if cards_arr.len() != 2 {
            return Err(format!("must bury exactly 2 cards, got {}", cards_arr.len()));
        }

        let mut bury: Vec<Card> = Vec::with_capacity(2);
        for cv in cards_arr {
            let card: Card =
                serde_json::from_value(cv.clone()).map_err(|e| format!("invalid card: {e}"))?;
            if !state.hands[seat].contains(&card) {
                return Err(format!("card {card} is not in your hand"));
            }
            if bury.contains(&card) {
                return Err(format!("cannot bury the same card twice ({card})"));
            }
            bury.push(card);
        }

        // Remove buried cards from picker's hand (back to 6 cards)
        for card in &bury {
            let pos = state.hands[seat]
                .iter()
                .position(|c| c == card)
                .ok_or_else(|| format!("card {card} disappeared from hand during removal"))?;
            state.hands[seat].remove(pos);
        }

        state.meta["buried"] = serde_json::to_value(&bury).unwrap();

        // Compute which fail aces the picker can legally call
        let suits = callable_suits(&state.hands[seat]);
        let suits_json: Vec<&str> = suits.iter().map(|s| match s {
            Suit::Clubs    => "clubs",
            Suit::Spades   => "spades",
            Suit::Hearts   => "hearts",
            Suit::Diamonds => "diamonds",
        }).collect();
        state.meta["callable_suits"] = serde_json::json!(suits_json);
        state.meta["sub_phase"]      = serde_json::json!("calling");

        let payload = serde_json::json!({
            "sub_phase":      "calling",
            "callable_suits": suits_json
        });

        Ok(BidResult {
            phase_complete:    false,
            hand_updated_seat: Some(seat),
            broadcast_payload: Some(payload),
        })
    }

    fn handle_call(
        &self,
        state: &mut GameState,
        seat: usize,
        action: &str,
        value: &serde_json::Value,
    ) -> Result<BidResult, String> {
        let picker = state.meta["picker"]
            .as_u64()
            .ok_or_else(|| "picker not set".to_string())?
            as usize;
        if seat != picker {
            return Err(format!("only the picker (player {picker}) can call a partner"));
        }

        match action {
            "go_alone" => {
                state.meta["going_alone"] = serde_json::json!(true);
                state.meta["called_suit"] = serde_json::Value::Null;
            }
            "call" => {
                let suit_str = value["suit"].as_str().ok_or("missing 'suit' field")?;
                let callable = state.meta["callable_suits"]
                    .as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default();
                if !callable.contains(&suit_str) {
                    return Err(format!(
                        "suit '{suit_str}' is not callable; callable: {}",
                        callable.join(", ")
                    ));
                }
                state.meta["called_suit"] = serde_json::json!(suit_str);
                state.meta["going_alone"] = serde_json::json!(false);
            }
            _ => return Err(format!("unknown calling action '{action}'; expected 'call' or 'go_alone'")),
        }

        state.meta["sub_phase"] = serde_json::json!("done");
        state.phase = GamePhase::Playing;
        state.current_player = (state.dealer + 1) % state.player_count;

        Ok(BidResult { phase_complete: true, hand_updated_seat: None, broadcast_payload: None })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::PlayResult;
    use uuid::Uuid;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn dealt_state() -> GameState {
        let deck = Sheepshead.build_deck();
        let result = Sheepshead.deal(deck, 5, 0);
        let mut state = GameState::new(Uuid::nil(), "sheepshead".into(), 5, 0);
        state.hands = result.hands;
        state.extra_piles = result.extra_piles;
        state.meta = result.initial_meta;
        state.phase = GamePhase::Bidding;
        state
    }

    fn dummy_state() -> GameState {
        GameState::new(Uuid::nil(), "sheepshead".into(), 5, 0)
    }

    // ── deck & deal ──────────────────────────────────────────────────────────

    #[test]
    fn deck_has_32_cards() {
        assert_eq!(Sheepshead.build_deck().len(), 32);
    }

    #[test]
    fn total_points_is_120() {
        let total: u8 = Sheepshead.build_deck().iter().map(|c| Sheepshead.card_points(*c)).sum();
        assert_eq!(total, 120);
    }

    #[test]
    fn deal_gives_6_cards_per_player_and_2_blind() {
        let result = Sheepshead.deal(Sheepshead.build_deck(), 5, 0);
        for hand in &result.hands {
            assert_eq!(hand.len(), 6);
        }
        let blind = result.extra_piles.iter().find(|(n, _)| n == "blind").unwrap();
        assert_eq!(blind.1.len(), 2);
    }

    #[test]
    fn deal_is_complete_no_duplicates() {
        use std::collections::HashSet;
        let result = Sheepshead.deal(Sheepshead.build_deck(), 5, 2);
        let mut seen = HashSet::new();
        for hand in &result.hands {
            for &card in hand {
                assert!(seen.insert(card), "duplicate in hand: {card}");
            }
        }
        for (_, pile) in &result.extra_piles {
            for &card in pile {
                assert!(seen.insert(card), "duplicate in blind: {card}");
            }
        }
        assert_eq!(seen.len(), 32);
    }

    #[test]
    fn deal_first_card_goes_to_player_left_of_dealer() {
        let deck = Sheepshead.build_deck();
        let first = deck[0];
        let result = Sheepshead.deal(deck, 5, 0);
        assert_eq!(result.hands[1][0], first); // dealer=0 → first to player 1
    }

    // ── picking sub-phase ────────────────────────────────────────────────────

    #[test]
    fn pick_moves_blind_to_hand_and_sets_picker() {
        let mut state = dealt_state(); // dealer=0, current_player=1
        let blind_count = state.extra_piles[0].1.len();
        let result = Sheepshead.apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"}));
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(!r.phase_complete);
        assert_eq!(r.hand_updated_seat, Some(1));
        assert_eq!(state.hands[1].len(), 6 + blind_count); // 8 cards
        assert!(state.extra_piles.iter().all(|(n, _)| n != "blind")); // blind gone
        assert_eq!(state.meta["picker"], serde_json::json!(1));
        assert_eq!(state.current_player, 1); // still picker's turn to bury
    }

    #[test]
    fn pass_advances_to_next_player() {
        let mut state = dealt_state(); // current_player=1
        let r = Sheepshead.apply_bid(&mut state, 1, &serde_json::json!({"action":"pass"})).unwrap();
        assert!(!r.phase_complete);
        assert_eq!(state.current_player, 2);
        assert_eq!(state.meta["passed"], serde_json::json!(1));
    }

    #[test]
    fn wrong_player_cannot_pick() {
        let mut state = dealt_state(); // current_player=1
        let err = Sheepshead.apply_bid(&mut state, 3, &serde_json::json!({"action":"pick"}));
        assert!(err.is_err());
    }

    #[test]
    fn all_pass_triggers_leaster_and_playing_phase() {
        let mut state = dealt_state(); // dealer=0, current_player=1
        for player in 1..=5 {
            let seat = if player <= 4 { player } else { 0 };
            Sheepshead.apply_bid(&mut state, seat, &serde_json::json!({"action":"pass"})).unwrap();
        }
        assert_eq!(state.phase, GamePhase::Playing);
        assert_eq!(state.meta["leaster"], serde_json::json!(true));
    }

    // ── burying sub-phase ────────────────────────────────────────────────────

    #[test]
    fn bury_removes_cards_and_transitions_to_calling() {
        let mut state = dealt_state();
        Sheepshead.apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"})).unwrap();

        let card1 = state.hands[1][0];
        let card2 = state.hands[1][1];
        let bury_msg = serde_json::json!({
            "action": "bury",
            "cards": [card1, card2]
        });
        let r = Sheepshead.apply_bid(&mut state, 1, &bury_msg).unwrap();

        assert!(!r.phase_complete); // stays in Bidding until call/go_alone
        assert_eq!(r.hand_updated_seat, Some(1));
        assert_eq!(state.hands[1].len(), 6); // back to 6 after bury
        assert_eq!(state.phase, GamePhase::Bidding);
        assert_eq!(state.meta["sub_phase"].as_str(), Some("calling"));
        assert_eq!(state.meta["buried"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn cannot_bury_card_not_in_hand() {
        let mut state = dealt_state();
        Sheepshead.apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"})).unwrap();

        // player 2's first card (not in player 1's hand)
        let other_card = state.hands[2][0];
        let in_hand = state.hands[1][0];
        let err = Sheepshead.apply_bid(
            &mut state,
            1,
            &serde_json::json!({"action":"bury","cards":[in_hand, other_card]}),
        );
        assert!(err.is_err());
    }

    #[test]
    fn cannot_bury_same_card_twice() {
        let mut state = dealt_state();
        Sheepshead.apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"})).unwrap();
        let card = state.hands[1][0];
        let err = Sheepshead.apply_bid(
            &mut state,
            1,
            &serde_json::json!({"action":"bury","cards":[card, card]}),
        );
        assert!(err.is_err());
    }

    #[test]
    fn non_picker_cannot_bury() {
        let mut state = dealt_state();
        Sheepshead.apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"})).unwrap();
        let c1 = state.hands[2][0];
        let c2 = state.hands[2][1];
        let err = Sheepshead.apply_bid(
            &mut state,
            2,
            &serde_json::json!({"action":"bury","cards":[c1,c2]}),
        );
        assert!(err.is_err());
    }

    // ── playing phase ────────────────────────────────────────────────────────

    /// Advance through bidding and reach Playing phase with player 1 as picker.
    fn playing_state() -> GameState {
        let mut state = dealt_state();
        Sheepshead.apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"})).unwrap();
        let c1 = state.hands[1][0];
        let c2 = state.hands[1][1];
        Sheepshead
            .apply_bid(&mut state, 1, &serde_json::json!({"action":"bury","cards":[c1,c2]}))
            .unwrap();
        // Must now call or go_alone before reaching Playing phase
        Sheepshead
            .apply_bid(&mut state, 1, &serde_json::json!({"action":"go_alone"}))
            .unwrap();
        // phase == Playing, current_player == 1 (dealer=0, left of dealer leads)
        state
    }

    /// Play one full trick using the first legal card for whoever's turn it is.
    fn play_full_trick(state: &mut GameState) -> PlayResult {
        let mut result = PlayResult::Continuing;
        for _ in 0..5 {
            let player = state.current_player;
            let hand = state.hands[player].clone();
            let trick = state.current_trick.as_ref().cloned()
                .unwrap_or_else(|| Trick::new(player));
            let legal = Sheepshead.legal_plays(&hand, &trick, state);
            result = Sheepshead.apply_play(state, player, legal[0]).unwrap();
        }
        result
    }

    #[test]
    fn play_card_removes_from_hand_and_adds_to_trick() {
        let mut state = playing_state(); // current_player = 1
        let card = state.hands[1][0];
        Sheepshead.apply_play(&mut state, 1, card).unwrap();
        assert!(!state.hands[1].contains(&card));
        let trick = state.current_trick.as_ref().unwrap();
        assert_eq!(trick.plays, vec![(1, card)]);
        assert_eq!(state.hands[1].len(), 5);
    }

    #[test]
    fn wrong_player_cannot_play() {
        let mut state = playing_state(); // current_player = 1
        let card = state.hands[0][0];
        assert!(Sheepshead.apply_play(&mut state, 0, card).is_err());
    }

    #[test]
    fn card_not_in_hand_rejected() {
        let mut state = playing_state();
        let other = state.hands[2][0]; // belongs to player 2
        assert!(Sheepshead.apply_play(&mut state, 1, other).is_err());
    }

    #[test]
    fn must_follow_suit_when_possible() {
        let mut state = playing_state();
        // Player 1 leads a plain-suit card (if they have one)
        let led = state.hands[1].iter().copied()
            .find(|c| Sheepshead.effective_suit(*c, &state) != EffectiveSuit::Trump);
        let Some(led_card) = led else { return }; // skip if hand is all trump

        Sheepshead.apply_play(&mut state, 1, led_card).unwrap();
        let led_suit = Sheepshead.effective_suit(led_card, &state);
        let next = state.current_player;

        // If next player has a card of the led suit, playing off-suit must fail
        let has_suit = state.hands[next].iter()
            .any(|c| Sheepshead.effective_suit(*c, &state) == led_suit);
        if has_suit {
            let off_suit = state.hands[next].iter().copied()
                .find(|c| Sheepshead.effective_suit(*c, &state) != led_suit);
            if let Some(bad) = off_suit {
                assert!(Sheepshead.apply_play(&mut state, next, bad).is_err());
            }
        }
    }

    #[test]
    fn complete_trick_records_winner_and_advances_lead() {
        let mut state = playing_state();
        let result = play_full_trick(&mut state);
        let (winner, _points) = match result {
            PlayResult::TrickComplete { winner, points } => (winner, points),
            PlayResult::GameOver { last_trick_winner, last_trick_points, .. } => {
                (last_trick_winner, last_trick_points)
            }
            PlayResult::Continuing => panic!("expected trick to complete"),
        };
        assert_eq!(state.completed_tricks.len(), 1);
        assert_eq!(state.completed_tricks[0].winner, Some(winner));
        assert_eq!(state.current_player, winner); // winner leads next trick
    }

    #[test]
    fn trick_points_sum_correctly() {
        let mut state = playing_state();
        let result = play_full_trick(&mut state);
        let points = match result {
            PlayResult::TrickComplete { points, .. } => points,
            PlayResult::GameOver { last_trick_points, .. } => last_trick_points,
            _ => panic!("expected trick completion"),
        };
        let played: u8 = state.completed_tricks[0]
            .plays
            .iter()
            .map(|(_, c)| Sheepshead.card_points(*c))
            .sum();
        assert_eq!(points, played);
    }

    #[test]
    fn six_tricks_empties_all_hands_and_ends_game() {
        let mut state = playing_state();
        let mut final_result = PlayResult::Continuing;
        for _ in 0..6 {
            final_result = play_full_trick(&mut state);
        }
        assert!(matches!(final_result, PlayResult::GameOver { .. }));
        assert_eq!(state.phase, GamePhase::Scoring);
        assert!(state.hands.iter().all(|h| h.is_empty()));
        assert_eq!(state.completed_tricks.len(), 6);
    }

    #[test]
    fn final_scores_sum_to_120_plus_buried() {
        // After a full game, raw point totals across all tricks must equal 120.
        // Buried cards are NOT in completed_tricks, so raw trick points = 120 - buried.
        let mut state = playing_state();
        let buried_pts: u8 = state.meta["buried"]
            .as_array().unwrap()
            .iter()
            .filter_map(|v| serde_json::from_value::<Card>(v.clone()).ok())
            .map(|c| Sheepshead.card_points(c))
            .sum();

        for _ in 0..6 { play_full_trick(&mut state); }

        let tbp = state.tricks_by_player();
        let trick_total: u8 = tbp.iter().flatten()
            .flat_map(|t| t.plays.iter().map(|(_, c)| Sheepshead.card_points(*c)))
            .sum();
        assert_eq!(trick_total + buried_pts, 120);
    }

    // ── trump & trick logic ──────────────────────────────────────────────────

    #[test]
    fn club_queen_beats_jack_of_clubs() {
        let state = dummy_state();
        let cq = Card::new(Suit::Clubs, Rank::Queen);
        let cj = Card::new(Suit::Clubs, Rank::Jack);
        assert!(Sheepshead.trump_rank(cq, &state) > Sheepshead.trump_rank(cj, &state));
    }

    #[test]
    fn all_queens_and_jacks_are_trump() {
        let state = dummy_state();
        for suit in [Suit::Clubs, Suit::Spades, Suit::Hearts, Suit::Diamonds] {
            assert!(Sheepshead.trump_rank(Card::new(suit, Rank::Queen), &state).is_some());
            assert!(Sheepshead.trump_rank(Card::new(suit, Rank::Jack), &state).is_some());
        }
    }

    #[test]
    fn non_diamond_plain_cards_are_not_trump() {
        let state = dummy_state();
        for rank in [Rank::Ace, Rank::Ten, Rank::King, Rank::Nine, Rank::Eight, Rank::Seven] {
            for suit in [Suit::Clubs, Suit::Spades, Suit::Hearts] {
                assert!(Sheepshead.trump_rank(Card::new(suit, rank), &state).is_none());
            }
        }
    }

    #[test]
    fn trick_winner_trump_beats_led_suit() {
        let state = dummy_state();
        let mut trick = Trick::new(0);
        trick.plays.push((0, Card::new(Suit::Clubs, Rank::Ace)));      // led ♣A
        trick.plays.push((1, Card::new(Suit::Clubs, Rank::Seven)));    // ♣7
        trick.plays.push((2, Card::new(Suit::Diamonds, Rank::Seven))); // 7♦ = lowest trump
        assert_eq!(Sheepshead.trick_winner(&trick, &state), 2);
    }

    // ── calling sub-phase ────────────────────────────────────────────────────

    /// Advance to the calling sub-phase (pick → bury → ready for call).
    fn calling_state() -> (GameState, usize) {
        let mut state = dealt_state();
        // player 1 picks (dealer=0, first player=1)
        Sheepshead.apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"})).unwrap();
        let c1 = state.hands[1][0];
        let c2 = state.hands[1][1];
        let r = Sheepshead
            .apply_bid(&mut state, 1, &serde_json::json!({"action":"bury","cards":[c1,c2]}))
            .unwrap();
        // After burying, should be in calling sub-phase (NOT playing yet)
        assert!(!r.phase_complete, "bury should NOT transition to Playing directly");
        assert_eq!(state.meta["sub_phase"].as_str(), Some("calling"));
        (state, 1) // (state, picker_seat)
    }

    #[test]
    fn bury_transitions_to_calling_not_playing() {
        let (state, _) = calling_state();
        assert_eq!(state.phase, GamePhase::Bidding);
        assert_eq!(state.meta["sub_phase"].as_str(), Some("calling"));
        assert!(state.meta["callable_suits"].is_array());
    }

    #[test]
    fn go_alone_transitions_to_playing() {
        let (mut state, picker) = calling_state();
        let r = Sheepshead
            .apply_bid(&mut state, picker, &serde_json::json!({"action":"go_alone"}))
            .unwrap();
        assert!(r.phase_complete);
        assert_eq!(state.phase, GamePhase::Playing);
        assert_eq!(state.meta["going_alone"].as_bool(), Some(true));
        assert!(state.meta["called_suit"].is_null());
    }

    #[test]
    fn call_valid_suit_transitions_to_playing() {
        let (mut state, picker) = calling_state();
        // Find a callable suit from meta
        let suits = state.meta["callable_suits"].as_array().unwrap().clone();
        if suits.is_empty() {
            // No callable suit means forced go_alone — skip this test
            return;
        }
        let suit_str = suits[0].as_str().unwrap().to_string();
        let r = Sheepshead
            .apply_bid(&mut state, picker, &serde_json::json!({"action":"call","suit":suit_str}))
            .unwrap();
        assert!(r.phase_complete);
        assert_eq!(state.phase, GamePhase::Playing);
        assert_eq!(state.meta["called_suit"].as_str(), Some(suit_str.as_str()));
        assert_eq!(state.meta["going_alone"].as_bool(), Some(false));
    }

    #[test]
    fn call_ace_picker_holds_is_rejected() {
        let (mut state, picker) = calling_state();
        // Find a suit whose ace the picker holds — should be invalid
        let ace_in_hand = state.hands[picker].iter()
            .find(|c| c.rank == Rank::Ace && c.suit != Suit::Diamonds
                    && trump_strength(**c).is_none())
            .copied();
        let Some(ace) = ace_in_hand else { return }; // skip if no fail aces in hand
        let suit_str = format!("{:?}", ace.suit).to_lowercase();
        let err = Sheepshead
            .apply_bid(&mut state, picker, &serde_json::json!({"action":"call","suit":suit_str}));
        assert!(err.is_err(), "should reject calling an ace the picker holds");
    }
}
