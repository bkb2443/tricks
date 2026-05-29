use crate::engine::meta::EuchreMeta;
use crate::engine::{
    BidResult, Card, DealResult, GameMeta, GamePhase, GameState, Rank, Suit, Trick,
    game::{EffectiveSuit, Game, PlayResult, apply_play_generic},
};

pub struct Euchre;

// ---------------------------------------------------------------------------
// Trump helpers
// ---------------------------------------------------------------------------

/// The suit of the same color (♣↔♠, ♥↔♦).
pub fn same_color_suit(suit: Suit) -> Suit {
    match suit {
        Suit::Clubs => Suit::Spades,
        Suit::Spades => Suit::Clubs,
        Suit::Hearts => Suit::Diamonds,
        Suit::Diamonds => Suit::Hearts,
    }
}

/// Trump strength for a given trump suit.
/// Right bower (Jack of trump suit): 8
/// Left bower (Jack of same-color suit): 7
/// A: 6, K: 5, Q: 4, 10: 3, 9: 2
pub(super) fn trump_strength_for_suit(card: Card, trump: Suit) -> Option<u8> {
    // Right bower
    if card.rank == Rank::Jack && card.suit == trump {
        return Some(8);
    }
    // Left bower
    if card.rank == Rank::Jack && card.suit == same_color_suit(trump) {
        return Some(7);
    }
    // Other trump suit cards (excluding Jack which is handled above)
    if card.suit == trump {
        return match card.rank {
            Rank::Ace => Some(6),
            Rank::King => Some(5),
            Rank::Queen => Some(4),
            Rank::Ten => Some(3),
            Rank::Nine => Some(2),
            _ => None,
        };
    }
    None
}

/// Plain suit rank: A=6, K=5, Q=4, J=3, 10=2, 9=1
pub(super) fn plain_strength(card: Card) -> u8 {
    match card.rank {
        Rank::Ace => 6,
        Rank::King => 5,
        Rank::Queen => 4,
        Rank::Jack => 3,
        Rank::Ten => 2,
        Rank::Nine => 1,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// Meta access helpers
// ---------------------------------------------------------------------------

fn euchre_meta(state: &GameState) -> Option<&EuchreMeta> {
    if let GameMeta::Euchre(ref m) = state.meta {
        Some(m)
    } else {
        None
    }
}

fn euchre_meta_mut(state: &mut GameState) -> Option<&mut EuchreMeta> {
    if let GameMeta::Euchre(ref mut m) = state.meta {
        Some(m)
    } else {
        None
    }
}

/// Returns the called trump suit from state meta, if present.
fn called_suit(state: &GameState) -> Option<Suit> {
    euchre_meta(state)
        .and_then(|m| m.called_suit.as_deref())
        .and_then(Suit::from_str)
}

/// Get the first active player left of the dealer, skipping sits_out.
fn first_active_after_dealer(dealer: usize, sits_out: Option<usize>, player_count: usize) -> usize {
    for offset in 1..=player_count {
        let seat = (dealer + offset) % player_count;
        if sits_out != Some(seat) {
            return seat;
        }
    }
    (dealer + 1) % player_count
}

impl Game for Euchre {
    fn name(&self) -> &'static str {
        "euchre"
    }

    fn valid_player_counts(&self) -> &'static [usize] {
        &[4]
    }

    fn build_deck(&self) -> Vec<Card> {
        // 24-card deck: ranks 9, Ten, Jack, Queen, King, Ace in all 4 suits
        let ranks = [
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
        ];
        let suits = [Suit::Clubs, Suit::Spades, Suit::Hearts, Suit::Diamonds];
        suits
            .iter()
            .flat_map(|&suit| ranks.iter().map(move |&rank| Card::new(suit, rank)))
            .collect()
    }

    /// Deal 3-2 pattern (starting left of dealer), kitty = 4 cards.
    /// First kitty card is the "turned up" card stored in meta.
    fn deal(&self, shuffled_deck: Vec<Card>, player_count: usize, dealer: usize) -> DealResult {
        assert_eq!(player_count, 4, "Euchre requires exactly 4 players");
        assert_eq!(shuffled_deck.len(), 24, "Euchre deck must be 24 cards");

        let mut deck = shuffled_deck.into_iter();
        let mut hands: Vec<Vec<Card>> = vec![Vec::new(); player_count];

        // Round 1: 3 cards to each player starting left of dealer
        for offset in 0..player_count {
            let seat = (dealer + 1 + offset) % player_count;
            hands[seat].extend(deck.by_ref().take(3));
        }

        // Round 2: 2 cards to each player starting left of dealer
        for offset in 0..player_count {
            let seat = (dealer + 1 + offset) % player_count;
            hands[seat].extend(deck.by_ref().take(2));
        }

        // Remaining 4 cards form the kitty
        let kitty: Vec<Card> = deck.collect();
        assert_eq!(kitty.len(), 4, "Euchre kitty must be 4 cards");

        let turned_up = kitty[0];

        DealResult {
            hands,
            extra_piles: vec![("kitty".to_string(), kitty)],
            initial_meta: GameMeta::Euchre(EuchreMeta {
                turned_up_card: Some(turned_up),
                sub_phase: "ordering".into(),
                passed_round1: 0,
                passed_round2: 0,
                caller_seat: None,
                called_suit: None,
                going_alone: false,
                sits_out: None,
            }),
        }
    }

    fn has_bidding(&self) -> bool {
        true
    }

    fn apply_bid(
        &self,
        state: &mut GameState,
        seat: usize,
        value: &serde_json::Value,
    ) -> Result<BidResult, String> {
        if state.phase != GamePhase::Bidding {
            return Err("game is not in the bidding phase".into());
        }

        let sub_phase = euchre_meta(state)
            .map(|m| m.sub_phase.clone())
            .unwrap_or_else(|| "ordering".into());

        match sub_phase.as_str() {
            "ordering" => self.handle_ordering(state, seat, value),
            "discarding" => self.handle_discarding(state, seat, value),
            "calling" => self.handle_calling(state, seat, value),
            _ => Err(format!("unknown sub_phase '{sub_phase}'")),
        }
    }

    fn apply_play(
        &self,
        state: &mut GameState,
        seat: usize,
        card: Card,
    ) -> Result<PlayResult, String> {
        let sits_out = euchre_meta(state).and_then(|m| m.sits_out);
        let active: Vec<usize> = (0..state.player_count)
            .filter(|&s| sits_out != Some(s))
            .collect();
        apply_play_generic(self, state, seat, card, Some(&active))
    }

    fn trump_rank(&self, card: Card, state: &GameState) -> Option<u8> {
        let trump = called_suit(state)?;
        trump_strength_for_suit(card, trump)
    }

    fn plain_suit_rank(&self, card: Card) -> u8 {
        plain_strength(card)
    }

    fn effective_suit(&self, card: Card, state: &GameState) -> EffectiveSuit {
        if let Some(trump) = called_suit(state)
            && trump_strength_for_suit(card, trump).is_some()
        {
            return EffectiveSuit::Trump;
        }
        EffectiveSuit::Plain(card.suit)
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
        if matching.is_empty() {
            hand.to_vec()
        } else {
            matching
        }
    }

    fn card_points(&self, _card: Card) -> u8 {
        0
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

    fn match_over(&self, cumulative_scores: &[i32], _hands_played: usize) -> bool {
        if cumulative_scores.len() < 4 {
            return false;
        }
        let team_a = cumulative_scores[0] + cumulative_scores[2];
        let team_b = cumulative_scores[1] + cumulative_scores[3];
        team_a >= 10 || team_b >= 10
    }

    fn match_winner(&self, cumulative_scores: &[i32]) -> Option<usize> {
        if cumulative_scores.len() < 4 {
            return None;
        }
        let team_a = cumulative_scores[0] + cumulative_scores[2];
        let team_b = cumulative_scores[1] + cumulative_scores[3];
        if team_a >= 10 && team_a >= team_b {
            if cumulative_scores[0] >= cumulative_scores[2] {
                Some(0)
            } else {
                Some(2)
            }
        } else if team_b >= 10 {
            if cumulative_scores[1] >= cumulative_scores[3] {
                Some(1)
            } else {
                Some(3)
            }
        } else {
            None
        }
    }

    fn score_game(&self, tricks_by_player: &[Vec<Trick>], state: &GameState) -> Vec<i32> {
        let n = tricks_by_player.len();
        let mut scores = vec![0i32; n];

        let meta = euchre_meta(state);
        let caller_seat = meta.and_then(|m| m.caller_seat).unwrap_or(0);
        let going_alone = meta.map(|m| m.going_alone).unwrap_or(false);
        let caller_partner = (caller_seat + 2) % 4;

        let maker_tricks = tricks_by_player[caller_seat].len()
            + if going_alone {
                0
            } else {
                tricks_by_player[caller_partner].len()
            };

        if maker_tricks >= 3 {
            if going_alone {
                if maker_tricks == 5 {
                    // Alone march: caller gets +4
                    scores[caller_seat] = 4;
                } else {
                    // Alone 3-4 tricks: caller gets +1
                    scores[caller_seat] = 1;
                }
            } else if maker_tricks == 5 {
                // March: caller and partner each get +2
                scores[caller_seat] = 2;
                scores[caller_partner] = 2;
            } else {
                // 3-4 tricks: caller and partner each get +1
                scores[caller_seat] = 1;
                scores[caller_partner] = 1;
            }
        } else {
            // Euchred: all non-maker seats get +2
            for (i, score) in scores.iter_mut().enumerate() {
                if i != caller_seat && (going_alone || i != caller_partner) {
                    *score = 2;
                }
            }
        }

        scores
    }
}

// ---------------------------------------------------------------------------
// Bidding sub-phase handlers
// ---------------------------------------------------------------------------

impl Euchre {
    // ── Ordering sub-phase (round 1) ─────────────────────────────────────────

    fn handle_ordering(
        &self,
        state: &mut GameState,
        seat: usize,
        value: &serde_json::Value,
    ) -> Result<BidResult, String> {
        if state.current_player != seat {
            return Err(format!(
                "it is player {}'s turn, not {seat}",
                state.current_player
            ));
        }

        let action = value["action"].as_str().ok_or("missing 'action' field")?;

        match action {
            "order_up" => {
                let alone = value["alone"].as_bool().unwrap_or(false);
                let turned_up: Card = euchre_meta(state)
                    .and_then(|m| m.turned_up_card)
                    .ok_or("turned_up_card not set")?;
                let called_suit_val = turned_up.suit;
                let dealer = state.dealer;

                // Give turned_up card to dealer's hand
                state.hands[dealer].push(turned_up);

                let sits_out = if alone {
                    let partner = (seat + 2) % 4;
                    Some(partner)
                } else {
                    None
                };

                if let Some(m) = euchre_meta_mut(state) {
                    m.caller_seat = Some(seat);
                    m.called_suit = Some(called_suit_val.as_str().to_string());
                    m.going_alone = alone;
                    m.sits_out = sits_out;
                    m.sub_phase = "discarding".into();
                }
                state.current_player = dealer;

                Ok(BidResult {
                    phase_complete: false,
                    hand_updated_seat: Some(dealer),
                    broadcast_payload: None,
                })
            }

            "pass" => {
                let passed = {
                    let m =
                        euchre_meta_mut(state).ok_or_else(|| "euchre meta missing".to_string())?;
                    m.passed_round1 += 1;
                    m.passed_round1
                };

                if passed >= 4 {
                    // All 4 passed round 1 — move to calling.
                    let turned_up: Card = euchre_meta(state)
                        .and_then(|m| m.turned_up_card)
                        .ok_or("turned_up_card not set")?;
                    let callable_suits: Vec<&str> = ["clubs", "spades", "hearts", "diamonds"]
                        .iter()
                        .copied()
                        .filter(|&s| s != turned_up.suit.as_str())
                        .collect();

                    if let Some(m) = euchre_meta_mut(state) {
                        m.sub_phase = "calling".into();
                    }
                    state.current_player = (state.dealer + 1) % 4;
                    Ok(BidResult {
                        phase_complete: false,
                        hand_updated_seat: None,
                        broadcast_payload: Some(serde_json::json!({
                            "sub_phase": "calling",
                            "callable_suits": callable_suits
                        })),
                    })
                } else {
                    state.current_player = (state.dealer + 1 + passed) % 4;
                    Ok(BidResult {
                        phase_complete: false,
                        hand_updated_seat: None,
                        broadcast_payload: None,
                    })
                }
            }

            _ => Err(format!(
                "unknown ordering action '{action}'; expected 'order_up' or 'pass'"
            )),
        }
    }

    // ── Discarding sub-phase ──────────────────────────────────────────────────

    fn handle_discarding(
        &self,
        state: &mut GameState,
        seat: usize,
        value: &serde_json::Value,
    ) -> Result<BidResult, String> {
        let dealer = state.dealer;
        if seat != dealer {
            return Err(format!("only the dealer (seat {dealer}) discards"));
        }
        if state.current_player != seat {
            return Err(format!(
                "it is player {}'s turn, not {seat}",
                state.current_player
            ));
        }

        let action = value["action"].as_str().ok_or("missing 'action' field")?;
        if action != "discard" {
            return Err(format!("expected 'discard' action, got '{action}'"));
        }

        let card: Card = serde_json::from_value(value["card"].clone())
            .map_err(|e| format!("invalid card: {e}"))?;

        if !state.hands[dealer].contains(&card) {
            return Err(format!("{card} is not in your hand"));
        }

        // Remove card from dealer's hand
        let pos = state.hands[dealer].iter().position(|c| *c == card).unwrap();
        state.hands[dealer].remove(pos);

        // Transition to Playing phase
        let sits_out = euchre_meta(state).and_then(|m| m.sits_out);
        if let Some(m) = euchre_meta_mut(state) {
            m.sub_phase = "done".into();
        }
        state.phase = GamePhase::Playing;
        state.current_player = first_active_after_dealer(dealer, sits_out, state.player_count);

        Ok(BidResult {
            phase_complete: true,
            hand_updated_seat: Some(dealer),
            broadcast_payload: None,
        })
    }

    // ── Calling sub-phase (round 2) ───────────────────────────────────────────

    fn handle_calling(
        &self,
        state: &mut GameState,
        seat: usize,
        value: &serde_json::Value,
    ) -> Result<BidResult, String> {
        if state.current_player != seat {
            return Err(format!(
                "it is player {}'s turn, not {seat}",
                state.current_player
            ));
        }

        let action = value["action"].as_str().ok_or("missing 'action' field")?;
        let turned_up_suit: Suit = euchre_meta(state)
            .and_then(|m| m.turned_up_card)
            .map(|c| c.suit)
            .ok_or("turned_up_card not set")?;
        let dealer = state.dealer;

        match action {
            "pass" => {
                let passed2 = {
                    let m =
                        euchre_meta_mut(state).ok_or_else(|| "euchre meta missing".to_string())?;
                    m.passed_round2 += 1;
                    m.passed_round2
                };

                // Stick-the-dealer: if 3 others passed and this is the dealer, they must call
                if passed2 >= 3 && seat == dealer {
                    // Revert the increment since we're returning an error
                    if let Some(m) = euchre_meta_mut(state) {
                        m.passed_round2 -= 1;
                    }
                    return Err("dealer must call (stick the dealer rule)".into());
                }

                state.current_player = (dealer + 1 + passed2) % 4;
                Ok(BidResult {
                    phase_complete: false,
                    hand_updated_seat: None,
                    broadcast_payload: None,
                })
            }

            "call" => {
                let suit_str_val = value["suit"].as_str().ok_or("missing 'suit' field")?;
                let suit = Suit::from_str(suit_str_val)
                    .ok_or_else(|| format!("unknown suit '{suit_str_val}'"))?;

                if suit == turned_up_suit {
                    return Err(format!(
                        "cannot call '{}' — it was the turned-up suit (use a different suit)",
                        suit_str_val
                    ));
                }

                let alone = value["alone"].as_bool().unwrap_or(false);
                let sits_out = if alone {
                    let partner = (seat + 2) % 4;
                    Some(partner)
                } else {
                    None
                };

                if let Some(m) = euchre_meta_mut(state) {
                    m.caller_seat = Some(seat);
                    m.called_suit = Some(suit.as_str().to_string());
                    m.going_alone = alone;
                    m.sits_out = sits_out;
                    m.sub_phase = "done".into();
                }

                state.phase = GamePhase::Playing;
                state.current_player =
                    first_active_after_dealer(dealer, sits_out, state.player_count);

                Ok(BidResult {
                    phase_complete: true,
                    hand_updated_seat: None,
                    broadcast_payload: None,
                })
            }

            _ => Err(format!(
                "unknown calling action '{action}'; expected 'call' or 'pass'"
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::meta::EuchreMeta;
    use uuid::Uuid;

    fn dealt_state() -> GameState {
        let deck = Euchre.build_deck();
        let result = Euchre.deal(deck, 4, 0);
        let mut state = GameState::new(Uuid::nil(), "euchre".into(), 4, 0);
        state.hands = result.hands;
        state.extra_piles = result.extra_piles;
        state.meta = result.initial_meta;
        state.phase = GamePhase::Bidding;
        state
    }

    fn state_with_trump(trump: Suit) -> GameState {
        let mut state = GameState::new(Uuid::nil(), "euchre".into(), 4, 0);
        state.meta = GameMeta::Euchre(EuchreMeta {
            turned_up_card: None,
            sub_phase: "done".into(),
            passed_round1: 0,
            passed_round2: 0,
            caller_seat: None,
            called_suit: Some(trump.as_str().to_string()),
            going_alone: false,
            sits_out: None,
        });
        state
    }

    fn state_with_meta_values(
        caller_seat: Option<usize>,
        going_alone: bool,
        called_suit: Option<&str>,
        sits_out: Option<usize>,
    ) -> GameState {
        let mut state = GameState::new(Uuid::nil(), "euchre".into(), 4, 0);
        state.meta = GameMeta::Euchre(EuchreMeta {
            turned_up_card: None,
            sub_phase: "done".into(),
            passed_round1: 0,
            passed_round2: 0,
            caller_seat,
            called_suit: called_suit.map(|s| s.to_string()),
            going_alone,
            sits_out,
        });
        state
    }

    #[test]
    fn deck_has_24_cards() {
        assert_eq!(Euchre.build_deck().len(), 24);
    }

    #[test]
    fn deal_gives_5_cards_per_player_and_4_kitty() {
        let result = Euchre.deal(Euchre.build_deck(), 4, 0);
        for hand in &result.hands {
            assert_eq!(hand.len(), 5);
        }
        let kitty = result
            .extra_piles
            .iter()
            .find(|(n, _)| n == "kitty")
            .unwrap();
        assert_eq!(kitty.1.len(), 4);
    }

    #[test]
    fn right_bower_is_highest_trump() {
        // ♣J when clubs is trump → rank 8
        let state = state_with_trump(Suit::Clubs);
        let right = Card::new(Suit::Clubs, Rank::Jack);
        assert_eq!(Euchre.trump_rank(right, &state), Some(8));
    }

    #[test]
    fn left_bower_is_second_trump() {
        // ♠J when clubs is trump → rank 7
        let state = state_with_trump(Suit::Clubs);
        let left = Card::new(Suit::Spades, Rank::Jack);
        assert_eq!(Euchre.trump_rank(left, &state), Some(7));
    }

    #[test]
    fn left_bower_effective_suit_is_trump() {
        // ♠J effective_suit == Trump when clubs is trump
        let state = state_with_trump(Suit::Clubs);
        let left = Card::new(Suit::Spades, Rank::Jack);
        assert_eq!(Euchre.effective_suit(left, &state), EffectiveSuit::Trump);
    }

    #[test]
    fn non_trump_jack_is_plain() {
        // ♥J is plain (not trump) when clubs is trump
        let state = state_with_trump(Suit::Clubs);
        let jack = Card::new(Suit::Hearts, Rank::Jack);
        assert!(Euchre.trump_rank(jack, &state).is_none());
        assert_eq!(
            Euchre.effective_suit(jack, &state),
            EffectiveSuit::Plain(Suit::Hearts)
        );
    }

    #[test]
    fn trick_winner_right_bower_beats_ace_of_trump() {
        let state = state_with_trump(Suit::Clubs);
        let mut trick = Trick::new(0);
        trick.plays.push((0, Card::new(Suit::Clubs, Rank::Ace))); // led A♣ (trump rank 6)
        trick.plays.push((1, Card::new(Suit::Clubs, Rank::Jack))); // J♣ right bower (rank 8)
        // J♣ should win
        assert_eq!(Euchre.trick_winner(&trick, &state), 1);
    }

    fn make_trick(winner_seat: usize, cards: Vec<(usize, Card)>) -> Trick {
        let led_by = cards.first().map(|(s, _)| *s).unwrap_or(0);
        let mut t = Trick::new(led_by);
        t.plays = cards;
        t.winner = Some(winner_seat);
        t
    }

    #[test]
    fn euchre_scoring_makers_win_3_tricks() {
        // caller=0, partner=2, 3+0=3 tricks → each +1
        let state = state_with_meta_values(Some(0), false, Some("clubs"), None);
        let mut tbp: Vec<Vec<Trick>> = vec![Vec::new(); 4];
        tbp[0].push(make_trick(
            0,
            vec![
                (0, Card::new(Suit::Clubs, Rank::Ace)),
                (1, Card::new(Suit::Spades, Rank::Nine)),
            ],
        ));
        tbp[0].push(make_trick(
            0,
            vec![
                (0, Card::new(Suit::Clubs, Rank::King)),
                (1, Card::new(Suit::Spades, Rank::Ten)),
            ],
        ));
        tbp[2].push(make_trick(
            2,
            vec![
                (2, Card::new(Suit::Clubs, Rank::Queen)),
                (3, Card::new(Suit::Spades, Rank::Jack)),
            ],
        ));
        let scores = Euchre.score_game(&tbp, &state);
        assert_eq!(scores[0], 1);
        assert_eq!(scores[2], 1);
        assert_eq!(scores[1], 0);
        assert_eq!(scores[3], 0);
    }

    #[test]
    fn euchre_scoring_march() {
        // caller=0 & partner=2 win all 5 → each +2
        let state = state_with_meta_values(Some(0), false, Some("clubs"), None);
        let mut tbp: Vec<Vec<Trick>> = vec![Vec::new(); 4];
        let dummy_cards = vec![
            (0, Card::new(Suit::Clubs, Rank::Ace)),
            (1, Card::new(Suit::Spades, Rank::Nine)),
        ];
        tbp[0].push(make_trick(0, dummy_cards.clone()));
        tbp[0].push(make_trick(0, dummy_cards.clone()));
        tbp[0].push(make_trick(0, dummy_cards.clone()));
        tbp[2].push(make_trick(2, dummy_cards.clone()));
        tbp[2].push(make_trick(2, dummy_cards.clone()));
        let scores = Euchre.score_game(&tbp, &state);
        assert_eq!(scores[0], 2);
        assert_eq!(scores[2], 2);
        assert_eq!(scores[1], 0);
        assert_eq!(scores[3], 0);
    }

    #[test]
    fn euchre_scoring_euchred() {
        // Makers (caller=0 + partner=2) get only 2 tricks → defenders each +2
        let state = state_with_meta_values(Some(0), false, Some("clubs"), None);
        let mut tbp: Vec<Vec<Trick>> = vec![Vec::new(); 4];
        let dummy_cards = vec![
            (1, Card::new(Suit::Spades, Rank::Nine)),
            (0, Card::new(Suit::Clubs, Rank::Nine)),
        ];
        tbp[0].push(make_trick(0, dummy_cards.clone()));
        tbp[0].push(make_trick(0, dummy_cards.clone()));
        tbp[1].push(make_trick(1, dummy_cards.clone()));
        tbp[1].push(make_trick(1, dummy_cards.clone()));
        tbp[3].push(make_trick(3, dummy_cards.clone()));
        let scores = Euchre.score_game(&tbp, &state);
        assert_eq!(scores[0], 0);
        assert_eq!(scores[2], 0);
        assert_eq!(scores[1], 2);
        assert_eq!(scores[3], 2);
    }

    #[test]
    fn euchre_scoring_alone_march() {
        // caller=0 goes alone, wins all 5 → caller +4
        let state = state_with_meta_values(Some(0), true, Some("clubs"), Some(2));
        let mut tbp: Vec<Vec<Trick>> = vec![Vec::new(); 4];
        let dummy = vec![
            (0, Card::new(Suit::Clubs, Rank::Ace)),
            (1, Card::new(Suit::Spades, Rank::Nine)),
        ];
        for _ in 0..5 {
            tbp[0].push(make_trick(0, dummy.clone()));
        }
        let scores = Euchre.score_game(&tbp, &state);
        assert_eq!(scores[0], 4);
        assert_eq!(scores[1], 0);
        assert_eq!(scores[2], 0);
        assert_eq!(scores[3], 0);
    }

    #[test]
    fn ordering_up_gives_dealer_the_turned_card() {
        let mut state = dealt_state(); // dealer=0, current_player=1
        let turned_up: Card = euchre_meta(&state).and_then(|m| m.turned_up_card).unwrap();
        let dealer_hand_before = state.hands[0].len();
        let result = Euchre.apply_bid(&mut state, 1, &serde_json::json!({"action": "order_up"}));
        assert!(result.is_ok());
        assert_eq!(state.hands[0].len(), dealer_hand_before + 1);
        assert!(state.hands[0].contains(&turned_up));
    }

    #[test]
    fn discarding_transitions_to_playing() {
        let mut state = dealt_state(); // dealer=0, current_player=1
        // Order up
        Euchre
            .apply_bid(&mut state, 1, &serde_json::json!({"action": "order_up"}))
            .unwrap();
        // Now current_player should be dealer (0), sub_phase "discarding"
        assert!(matches!(&state.meta, GameMeta::Euchre(m) if m.sub_phase == "discarding"));
        assert_eq!(state.current_player, 0);
        let card_to_discard = state.hands[0][0];
        let result = Euchre.apply_bid(
            &mut state,
            0,
            &serde_json::json!({"action": "discard", "card": card_to_discard}),
        );
        assert!(result.is_ok());
        let r = result.unwrap();
        assert!(r.phase_complete);
        assert_eq!(state.phase, GamePhase::Playing);
        assert_eq!(state.hands[0].len(), 5);
    }

    #[test]
    fn cannot_call_turned_up_suit_in_round_2() {
        let mut state = dealt_state();
        // All 4 players pass round 1
        for i in 1..=4 {
            let seat = i % 4;
            let _ = Euchre.apply_bid(&mut state, seat, &serde_json::json!({"action": "pass"}));
        }
        assert!(matches!(&state.meta, GameMeta::Euchre(m) if m.sub_phase == "calling"));

        // Get the turned-up suit
        let turned_up: Card = euchre_meta(&state).and_then(|m| m.turned_up_card).unwrap();
        let turned_suit_str = turned_up.suit.as_str();

        // Try to call the turned-up suit — should fail
        let cp = state.current_player;
        let result = Euchre.apply_bid(
            &mut state,
            cp,
            &serde_json::json!({"action": "call", "suit": turned_suit_str}),
        );
        assert!(result.is_err());
    }

    #[test]
    fn stick_the_dealer_forces_call() {
        let mut state = dealt_state(); // dealer=0
        // Pass all round 1
        for i in 1..=4 {
            let seat = i % 4;
            let _ = Euchre.apply_bid(&mut state, seat, &serde_json::json!({"action": "pass"}));
        }
        assert!(matches!(&state.meta, GameMeta::Euchre(m) if m.sub_phase == "calling"));

        // Pass 3 players in round 2 (seats 1, 2, 3)
        let turned_up: Card = euchre_meta(&state).and_then(|m| m.turned_up_card).unwrap();
        // Find a non-turned-up suit to try calling
        let other_suits = [Suit::Clubs, Suit::Spades, Suit::Hearts, Suit::Diamonds]
            .iter()
            .filter(|&&s| s != turned_up.suit)
            .copied()
            .collect::<Vec<_>>();

        // Pass seats 1, 2, 3
        for &seat in &[1usize, 2, 3] {
            let _ = Euchre.apply_bid(&mut state, seat, &serde_json::json!({"action": "pass"}));
        }

        // Now it's the dealer's (0) turn — they must call
        assert_eq!(state.current_player, 0);
        let pass_result = Euchre.apply_bid(&mut state, 0, &serde_json::json!({"action": "pass"}));
        assert!(pass_result.is_err(), "dealer must call (stick the dealer)");

        // But calling a valid suit should succeed
        let valid_suit = other_suits[0].as_str();
        let call_result = Euchre.apply_bid(
            &mut state,
            0,
            &serde_json::json!({"action": "call", "suit": valid_suit}),
        );
        assert!(call_result.is_ok());
    }
}
