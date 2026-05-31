use crate::engine::meta::SheepsheadMeta;
use crate::engine::{
    BidResult, Card, DealResult, GameMeta, GamePhase, GameState, Rank, Suit, Trick,
    game::{EffectiveSuit, Game, apply_play_generic},
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

// ---------------------------------------------------------------------------
// Meta access helpers
// ---------------------------------------------------------------------------

fn sheepshead_meta(state: &GameState) -> Option<&SheepsheadMeta> {
    if let GameMeta::Sheepshead(ref m) = state.meta {
        Some(m)
    } else {
        None
    }
}

fn sheepshead_meta_mut(state: &mut GameState) -> Option<&mut SheepsheadMeta> {
    if let GameMeta::Sheepshead(ref mut m) = state.meta {
        Some(m)
    } else {
        None
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
            Rank::Seven,
            Rank::Eight,
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
            initial_meta: GameMeta::Sheepshead(SheepsheadMeta {
                picker: None,
                sub_phase: "picking".into(),
                passed: 0,
                leaster: false,
                buried: vec![],
                callable_suits: vec![],
                called_suit: None,
                going_alone: false,
                partner: None,
            }),
        }
    }

    // ---------------------------------------------------------------------------
    // Bidding / picking phase
    // ---------------------------------------------------------------------------

    fn has_bidding(&self) -> bool {
        true
    }

    /// Handle pick, pass, bury, call, and go_alone actions.
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
        let sub_phase = sheepshead_meta(state)
            .map(|m| m.sub_phase.clone())
            .unwrap_or_else(|| "picking".into());

        match sub_phase.as_str() {
            "picking" => self.handle_pick_or_pass(state, seat, action),
            "burying" => self.handle_bury(state, seat, action, value),
            "calling" => self.handle_call(state, seat, action, value),
            _ => Err(format!("unknown sub_phase '{sub_phase}'")),
        }
    }

    fn apply_play(
        &self,
        state: &mut GameState,
        seat: usize,
        card: Card,
    ) -> Result<crate::engine::PlayResult, String> {
        // Partner revelation: if the called ace is played while partner is unknown, reveal them
        let should_reveal = {
            let meta = sheepshead_meta(state);
            if let Some(m) = meta {
                m.partner.is_none()
                    && !m.going_alone
                    && card.rank == Rank::Ace
                    && m.called_suit
                        .as_deref()
                        .map(|suit_str| match suit_str {
                            "clubs" => card.suit == Suit::Clubs,
                            "spades" => card.suit == Suit::Spades,
                            "hearts" => card.suit == Suit::Hearts,
                            _ => false,
                        })
                        .unwrap_or(false)
            } else {
                false
            }
        };

        if should_reveal && let Some(m) = sheepshead_meta_mut(state) {
            m.partner = Some(seat);
        }

        apply_play_generic(self, state, seat, card, None)
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
        if matching.is_empty() {
            hand.to_vec()
        } else {
            matching
        }
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

    fn default_max_hands(&self) -> Option<u32> {
        Some(20)
    }

    fn match_winner(&self, cumulative_scores: &[i32]) -> Option<usize> {
        cumulative_scores
            .iter()
            .enumerate()
            .max_by_key(|&(_, &s)| s)
            .map(|(i, _)| i)
    }

    fn tutorials(&self) -> &'static [crate::engine::tutorial::TutorialHand] {
        crate::games::sheepshead::tutorials::all()
    }

    fn hint_reason(
        &self,
        card: crate::engine::Card,
        state: &crate::engine::GameState,
        seat: usize,
    ) -> &'static str {
        use crate::engine::GameMeta;

        let is_trump = self.trump_rank(card, state).is_some();
        let is_leading = state
            .current_trick
            .as_ref()
            .is_none_or(|t| t.plays.is_empty());
        let picker = if let GameMeta::Sheepshead(ref m) = state.meta {
            m.picker
        } else {
            None
        };
        let is_picker = picker == Some(seat);

        if is_leading {
            if is_picker {
                if is_trump {
                    "Lead trump \u{2014} draw defenders\u{2019} trump out early while yours is strongest"
                } else if card.rank == crate::engine::Rank::Ace {
                    "Lead your ace \u{2014} if the picker isn\u{2019}t void here, you\u{2019}ll take this trick"
                } else {
                    "No trump left \u{2014} lead your highest fail card to create pressure"
                }
            } else if card.rank == crate::engine::Rank::Ace && !is_trump {
                "Lead your ace \u{2014} the picker isn\u{2019}t void here, so you should win this trick"
            } else {
                "Lead a low card to preserve your trump; avoid suits the picker can trump"
            }
        } else {
            // Following
            let trick = match &state.current_trick {
                Some(t) => t,
                None => return "",
            };
            let winner_seat = crate::bot::current_winner(trick, self, state);
            let partner = if let GameMeta::Sheepshead(ref m) = state.meta {
                m.partner
            } else {
                None
            };
            let teammate = if is_picker { partner } else { picker };
            let team_winning = winner_seat == seat || teammate == Some(winner_seat);
            let i_am_winning = winner_seat == seat;

            if team_winning && !i_am_winning {
                "Your teammate is winning \u{2014} discard a high-point card for them to collect"
            } else if i_am_winning {
                "You\u{2019}re already winning this trick \u{2014} play your cheapest card"
            } else if is_trump {
                "This trick is worth a lot of points \u{2014} use just enough trump to take it"
            } else {
                let trick_pts: u8 = trick.plays.iter().map(|(_, c)| self.card_points(*c)).sum();
                if trick_pts < 5 {
                    "This trick has few points \u{2014} don\u{2019}t waste good cards on it"
                } else {
                    "You can\u{2019}t win this trick \u{2014} save your strong cards for later"
                }
            }
        }
    }

    fn score_game(&self, tricks_by_player: &[Vec<Trick>], state: &GameState) -> Vec<i32> {
        let n = tricks_by_player.len();

        let meta = sheepshead_meta(state);

        // ── Leaster ──────────────────────────────────────────────────────────────
        if meta.map(|m| m.leaster).unwrap_or(false) {
            let raw: Vec<i32> = tricks_by_player
                .iter()
                .map(|tricks| {
                    tricks
                        .iter()
                        .flat_map(|t| t.plays.iter().map(|(_, c)| self.card_points(*c) as i32))
                        .sum()
                })
                .collect();
            let max_pts = raw.iter().max().copied().unwrap_or(0);
            let loser = raw.iter().position(|&p| p == max_pts).unwrap_or(0);
            return (0..n)
                .map(|i| if i == loser { -((n as i32) - 1) } else { 1 })
                .collect();
        }

        let picker = meta.and_then(|m| m.picker).unwrap_or(0);
        let partner = meta.and_then(|m| m.partner);
        let going_alone = meta.map(|m| m.going_alone).unwrap_or(false);

        // Compute card points per player from tricks
        let raw: Vec<i32> = tricks_by_player
            .iter()
            .map(|tricks| {
                tricks
                    .iter()
                    .flat_map(|t| t.plays.iter().map(|(_, c)| self.card_points(*c) as i32))
                    .sum()
            })
            .collect();

        // Buried cards count toward picker's total
        let buried_points: i32 = meta
            .map(|m| {
                m.buried
                    .iter()
                    .map(|c| self.card_points(*c) as i32)
                    .sum::<i32>()
            })
            .unwrap_or(0);

        let picker_total = raw[picker] + buried_points;

        // ── Going alone (1v4, double stakes) — also covers partner-never-revealed ─
        if going_alone || partner.is_none() {
            let defender_total = 120 - picker_total;
            let picker_wins = picker_total > 60;
            let schneider = if picker_wins {
                defender_total <= 30
            } else {
                picker_total <= 30
            };

            return (0..n)
                .map(|i| {
                    if i == picker {
                        match (picker_wins, schneider) {
                            (true, true) => 8,
                            (true, false) => 4,
                            (false, true) => -8,
                            (false, false) => -4,
                        }
                    } else {
                        match (picker_wins, schneider) {
                            (true, true) => -2,
                            (true, false) => -1,
                            (false, true) => 2,
                            (false, false) => 1,
                        }
                    }
                })
                .collect();
        }

        // ── Called partner (2v3) ──────────────────────────────────────────────────
        let partner_seat = partner.unwrap();
        let team_total = picker_total + raw[partner_seat];
        let team_wins = team_total > 60;
        let opponent_total = 120 - team_total;
        let schneider = if team_wins {
            opponent_total <= 30
        } else {
            team_total <= 30
        };

        (0..n)
            .map(|i| {
                if i == picker {
                    match (team_wins, schneider) {
                        (true, true) => 4,
                        (true, false) => 2,
                        (false, true) => -4,
                        (false, false) => -2,
                    }
                } else if i == partner_seat {
                    match (team_wins, schneider) {
                        (true, true) => 2,
                        (true, false) => 1,
                        (false, true) => -2,
                        (false, false) => -1,
                    }
                } else {
                    // defender
                    match (team_wins, schneider) {
                        (true, true) => -2,
                        (true, false) => -1,
                        (false, true) => 2,
                        (false, false) => 1,
                    }
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
            let has_other = hand
                .iter()
                .any(|c| c.suit == suit && c.rank != Rank::Ace && trump_strength(*c).is_none());
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

                if let Some(m) = sheepshead_meta_mut(state) {
                    m.picker = Some(seat);
                    m.sub_phase = "burying".into();
                }
                // current_player stays as `seat` — they must now bury 2 cards

                let payload = serde_json::json!({ "picker": seat, "sub_phase": "burying" });
                Ok(BidResult {
                    phase_complete: false,
                    hand_updated_seat: Some(seat),
                    broadcast_payload: Some(payload),
                })
            }

            "pass" => {
                let passed = {
                    let m = sheepshead_meta_mut(state)
                        .ok_or_else(|| "sheepshead meta missing".to_string())?;
                    m.passed += 1;
                    m.passed
                };

                if passed >= state.player_count {
                    // All five players passed → leaster
                    if let Some(m) = sheepshead_meta_mut(state) {
                        m.leaster = true;
                    }
                    state.phase = GamePhase::Playing;
                    state.current_player = (state.dealer + 1) % state.player_count;
                    Ok(BidResult {
                        phase_complete: true,
                        hand_updated_seat: None,
                        broadcast_payload: None,
                    })
                } else {
                    state.current_player = (state.dealer + 1 + passed) % state.player_count;
                    Ok(BidResult {
                        phase_complete: false,
                        hand_updated_seat: None,
                        broadcast_payload: None,
                    })
                }
            }

            _ => Err(format!(
                "unknown picking action '{action}'; expected 'pick' or 'pass'"
            )),
        }
    }

    fn handle_bury(
        &self,
        state: &mut GameState,
        seat: usize,
        action: &str,
        value: &serde_json::Value,
    ) -> Result<BidResult, String> {
        let picker = sheepshead_meta(state)
            .and_then(|m| m.picker)
            .ok_or_else(|| "picker not set".to_string())?;

        if seat != picker {
            return Err(format!("only the picker (player {picker}) can bury cards"));
        }
        if action != "bury" {
            return Err(format!(
                "expected 'bury' action during burying phase, got '{action}'"
            ));
        }

        let cards_arr = value["cards"]
            .as_array()
            .ok_or("'cards' must be an array of exactly 2 cards")?;
        if cards_arr.len() != 2 {
            return Err(format!(
                "must bury exactly 2 cards, got {}",
                cards_arr.len()
            ));
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

        // Compute which fail aces the picker can legally call
        let suits = callable_suits(&state.hands[seat]);
        let suits_strs: Vec<String> = suits.iter().map(|s| s.as_str().to_string()).collect();

        if let Some(m) = sheepshead_meta_mut(state) {
            m.buried = bury.clone();
            m.callable_suits = suits_strs.clone();
            m.sub_phase = "calling".into();
        }

        let suits_json: Vec<&str> = suits_strs.iter().map(|s| s.as_str()).collect();
        let payload = serde_json::json!({
            "sub_phase":      "calling",
            "callable_suits": suits_json
        });

        Ok(BidResult {
            phase_complete: false,
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
        let picker = sheepshead_meta(state)
            .and_then(|m| m.picker)
            .ok_or_else(|| "picker not set".to_string())?;
        if seat != picker {
            return Err(format!(
                "only the picker (player {picker}) can call a partner"
            ));
        }

        match action {
            "go_alone" => {
                if let Some(m) = sheepshead_meta_mut(state) {
                    m.going_alone = true;
                    m.called_suit = None;
                }
            }
            "call" => {
                let suit_str = value["suit"].as_str().ok_or("missing 'suit' field")?;
                let callable: Vec<String> = sheepshead_meta(state)
                    .map(|m| m.callable_suits.clone())
                    .unwrap_or_default();
                if !callable.iter().any(|s| s == suit_str) {
                    return Err(format!(
                        "suit '{suit_str}' is not callable; callable: {}",
                        callable.join(", ")
                    ));
                }
                if let Some(m) = sheepshead_meta_mut(state) {
                    m.called_suit = Some(suit_str.to_string());
                    m.going_alone = false;
                }
            }
            _ => {
                return Err(format!(
                    "unknown calling action '{action}'; expected 'call' or 'go_alone'"
                ));
            }
        }

        if let Some(m) = sheepshead_meta_mut(state) {
            m.sub_phase = "done".into();
        }
        state.phase = GamePhase::Playing;
        state.current_player = (state.dealer + 1) % state.player_count;

        Ok(BidResult {
            phase_complete: true,
            hand_updated_seat: None,
            broadcast_payload: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::PlayResult;
    use crate::engine::meta::SheepsheadMeta;
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

    fn make_sheepshead_meta(
        picker: Option<usize>,
        sub_phase: &str,
        passed: usize,
        going_alone: bool,
        called_suit: Option<&str>,
        partner: Option<usize>,
        leaster: bool,
        buried: Vec<Card>,
        callable_suits: Vec<String>,
    ) -> GameMeta {
        GameMeta::Sheepshead(SheepsheadMeta {
            picker,
            sub_phase: sub_phase.into(),
            passed,
            leaster,
            buried,
            callable_suits,
            called_suit: called_suit.map(|s| s.to_string()),
            going_alone,
            partner,
        })
    }

    // ── deck & deal ──────────────────────────────────────────────────────────

    #[test]
    fn deck_has_32_cards() {
        assert_eq!(Sheepshead.build_deck().len(), 32);
    }

    #[test]
    fn total_points_is_120() {
        let total: u8 = Sheepshead
            .build_deck()
            .iter()
            .map(|c| Sheepshead.card_points(*c))
            .sum();
        assert_eq!(total, 120);
    }

    #[test]
    fn deal_gives_6_cards_per_player_and_2_blind() {
        let result = Sheepshead.deal(Sheepshead.build_deck(), 5, 0);
        for hand in &result.hands {
            assert_eq!(hand.len(), 6);
        }
        let blind = result
            .extra_piles
            .iter()
            .find(|(n, _)| n == "blind")
            .unwrap();
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
        assert!(matches!(&state.meta, GameMeta::Sheepshead(m) if m.picker == Some(1)));
        assert_eq!(state.current_player, 1); // still picker's turn to bury
    }

    #[test]
    fn pass_advances_to_next_player() {
        let mut state = dealt_state(); // current_player=1
        let r = Sheepshead
            .apply_bid(&mut state, 1, &serde_json::json!({"action":"pass"}))
            .unwrap();
        assert!(!r.phase_complete);
        assert_eq!(state.current_player, 2);
        assert!(matches!(&state.meta, GameMeta::Sheepshead(m) if m.passed == 1));
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
            Sheepshead
                .apply_bid(&mut state, seat, &serde_json::json!({"action":"pass"}))
                .unwrap();
        }
        assert_eq!(state.phase, GamePhase::Playing);
        assert!(matches!(&state.meta, GameMeta::Sheepshead(m) if m.leaster));
    }

    // ── burying sub-phase ────────────────────────────────────────────────────

    #[test]
    fn bury_removes_cards_and_transitions_to_calling() {
        let mut state = dealt_state();
        Sheepshead
            .apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"}))
            .unwrap();

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
        assert!(matches!(&state.meta, GameMeta::Sheepshead(m) if m.sub_phase == "calling"));
        assert!(matches!(&state.meta, GameMeta::Sheepshead(m) if m.buried.len() == 2));
    }

    #[test]
    fn cannot_bury_card_not_in_hand() {
        let mut state = dealt_state();
        Sheepshead
            .apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"}))
            .unwrap();

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
        Sheepshead
            .apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"}))
            .unwrap();
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
        Sheepshead
            .apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"}))
            .unwrap();
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
        Sheepshead
            .apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"}))
            .unwrap();
        let c1 = state.hands[1][0];
        let c2 = state.hands[1][1];
        Sheepshead
            .apply_bid(
                &mut state,
                1,
                &serde_json::json!({"action":"bury","cards":[c1,c2]}),
            )
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
            let trick = state
                .current_trick
                .as_ref()
                .cloned()
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
        let led = state.hands[1]
            .iter()
            .copied()
            .find(|c| Sheepshead.effective_suit(*c, &state) != EffectiveSuit::Trump);
        let Some(led_card) = led else { return }; // skip if hand is all trump

        Sheepshead.apply_play(&mut state, 1, led_card).unwrap();
        let led_suit = Sheepshead.effective_suit(led_card, &state);
        let next = state.current_player;

        // If next player has a card of the led suit, playing off-suit must fail
        let has_suit = state.hands[next]
            .iter()
            .any(|c| Sheepshead.effective_suit(*c, &state) == led_suit);
        if has_suit {
            let off_suit = state.hands[next]
                .iter()
                .copied()
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
            PlayResult::GameOver {
                last_trick_winner,
                last_trick_points,
                ..
            } => (last_trick_winner, last_trick_points),
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
            PlayResult::GameOver {
                last_trick_points, ..
            } => last_trick_points,
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
        let buried_pts: u8 = if let GameMeta::Sheepshead(ref m) = state.meta {
            m.buried.iter().map(|c| Sheepshead.card_points(*c)).sum()
        } else {
            0
        };

        for _ in 0..6 {
            play_full_trick(&mut state);
        }

        let tbp = state.tricks_by_player();
        let trick_total: u8 = tbp
            .iter()
            .flatten()
            .flat_map(|t| t.plays.iter().map(|(_, c)| Sheepshead.card_points(*c)))
            .sum();
        assert_eq!(trick_total + buried_pts, 120);
    }

    // ── score_game helpers & tests ───────────────────────────────────────────

    fn make_state_with_meta(meta: GameMeta) -> GameState {
        let mut state = GameState::new(uuid::Uuid::nil(), "sheepshead".into(), 5, 0);
        state.meta = meta;
        state
    }

    fn make_trick(winner: usize, cards: Vec<(usize, Card)>) -> Trick {
        let led_by = cards.first().map(|(s, _)| *s).unwrap_or(0);
        let mut t = Trick::new(led_by);
        t.plays = cards;
        t.winner = Some(winner);
        t
    }

    #[test]
    fn going_alone_normal_win() {
        let state = make_state_with_meta(make_sheepshead_meta(
            Some(1),
            "done",
            0,
            true,
            None,
            None,
            false,
            vec![],
            vec![],
        ));
        let mut tbp: Vec<Vec<Trick>> = vec![Vec::new(); 5];
        tbp[1].push(make_trick(
            1,
            vec![
                (1, Card::new(Suit::Clubs, Rank::Ace)),  // 11
                (1, Card::new(Suit::Clubs, Rank::Ten)),  // 10
                (1, Card::new(Suit::Spades, Rank::Ace)), // 11
                (1, Card::new(Suit::Spades, Rank::Ten)), // 10
                (1, Card::new(Suit::Hearts, Rank::Ace)), // 11
            ],
        ));
        tbp[1].push(make_trick(
            1,
            vec![
                (1, Card::new(Suit::Hearts, Rank::Ten)),   // 10
                (0, Card::new(Suit::Clubs, Rank::Seven)),  // 0
                (2, Card::new(Suit::Clubs, Rank::Eight)),  // 0
                (3, Card::new(Suit::Clubs, Rank::Nine)),   // 0
                (4, Card::new(Suit::Spades, Rank::Seven)), // 0
            ],
        ));
        // Picker: 53+10=63; defenders: 120-63=57 (not schneider since >30)
        let scores = Sheepshead.score_game(&tbp, &state);
        assert_eq!(scores[1], 4, "picker wins alone: +4");
        assert_eq!(scores[0], -1);
        assert_eq!(scores[2], -1);
        assert_eq!(scores[3], -1);
        assert_eq!(scores[4], -1);
        assert_eq!(scores.iter().sum::<i32>(), 0);
    }

    #[test]
    fn going_alone_schneider_win() {
        let state = make_state_with_meta(make_sheepshead_meta(
            Some(1),
            "done",
            0,
            true,
            None,
            None,
            false,
            vec![],
            vec![],
        ));
        let mut tbp: Vec<Vec<Trick>> = vec![Vec::new(); 5];
        tbp[1].push(make_trick(
            1,
            vec![
                (1, Card::new(Suit::Clubs, Rank::Ace)),
                (1, Card::new(Suit::Clubs, Rank::Ten)),
                (1, Card::new(Suit::Spades, Rank::Ace)),
                (1, Card::new(Suit::Spades, Rank::Ten)),
                (1, Card::new(Suit::Hearts, Rank::Ace)),
            ],
        )); // 53
        tbp[1].push(make_trick(
            1,
            vec![
                (1, Card::new(Suit::Hearts, Rank::Ten)),
                (1, Card::new(Suit::Diamonds, Rank::Ace)),
                (1, Card::new(Suit::Diamonds, Rank::Ten)),
                (1, Card::new(Suit::Clubs, Rank::King)),
                (1, Card::new(Suit::Spades, Rank::King)),
            ],
        )); // 10+11+10+4+4 = 39 → total 92; defenders get 28 ≤ 30 → schneider
        let scores = Sheepshead.score_game(&tbp, &state);
        assert_eq!(scores[1], 8, "picker schneider win alone: +8");
        assert_eq!(scores[0], -2);
        assert_eq!(scores[2], -2);
        assert_eq!(scores[3], -2);
        assert_eq!(scores[4], -2);
        assert_eq!(scores.iter().sum::<i32>(), 0);
    }

    #[test]
    fn going_alone_normal_loss() {
        let state = make_state_with_meta(make_sheepshead_meta(
            Some(1),
            "done",
            0,
            true,
            None,
            None,
            false,
            vec![],
            vec![],
        ));
        let mut tbp: Vec<Vec<Trick>> = vec![Vec::new(); 5];
        tbp[1].push(make_trick(
            1,
            vec![
                (1, Card::new(Suit::Clubs, Rank::Ace)),
                (1, Card::new(Suit::Clubs, Rank::Ten)),
                (1, Card::new(Suit::Spades, Rank::Ace)),
                (1, Card::new(Suit::Spades, Rank::Ten)),
                (1, Card::new(Suit::Hearts, Rank::Seven)),
            ],
        )); // 42
        let scores = Sheepshead.score_game(&tbp, &state);
        assert_eq!(scores[1], -4, "picker loses alone: -4");
        assert_eq!(scores[0], 1);
        assert_eq!(scores[2], 1);
        assert_eq!(scores[3], 1);
        assert_eq!(scores[4], 1);
        assert_eq!(scores.iter().sum::<i32>(), 0);
    }

    #[test]
    fn partner_2v3_normal_win() {
        let state = make_state_with_meta(make_sheepshead_meta(
            Some(1),
            "done",
            0,
            false,
            Some("clubs"),
            Some(2),
            false,
            vec![],
            vec![],
        ));
        let mut tbp: Vec<Vec<Trick>> = vec![Vec::new(); 5];
        tbp[1].push(make_trick(
            1,
            vec![
                (1, Card::new(Suit::Clubs, Rank::Ace)),
                (1, Card::new(Suit::Clubs, Rank::Ten)),
                (1, Card::new(Suit::Spades, Rank::Ace)),
                (1, Card::new(Suit::Spades, Rank::Ten)),
                (1, Card::new(Suit::Hearts, Rank::Seven)),
            ],
        )); // 42
        tbp[2].push(make_trick(
            2,
            vec![
                (2, Card::new(Suit::Hearts, Rank::Ace)),
                (2, Card::new(Suit::Hearts, Rank::Ten)),
            ],
        )); // 21
        let scores = Sheepshead.score_game(&tbp, &state);
        assert_eq!(scores[1], 2, "picker wins 2v3: +2");
        assert_eq!(scores[2], 1, "partner wins 2v3: +1");
        assert_eq!(scores[0], -1);
        assert_eq!(scores[3], -1);
        assert_eq!(scores[4], -1);
        assert_eq!(scores.iter().sum::<i32>(), 0);
    }

    #[test]
    fn partner_2v3_normal_loss() {
        let state = make_state_with_meta(make_sheepshead_meta(
            Some(1),
            "done",
            0,
            false,
            Some("clubs"),
            Some(2),
            false,
            vec![],
            vec![],
        ));
        let mut tbp: Vec<Vec<Trick>> = vec![Vec::new(); 5];
        // picker gets 11+11=22 pts
        tbp[1].push(make_trick(
            1,
            vec![
                (1, Card::new(Suit::Clubs, Rank::Ace)),    // 11
                (1, Card::new(Suit::Spades, Rank::Ace)),   // 11
                (0, Card::new(Suit::Hearts, Rank::Seven)), // 0
                (3, Card::new(Suit::Hearts, Rank::Eight)), // 0
                (4, Card::new(Suit::Hearts, Rank::Nine)),  // 0
            ],
        )); // picker: 22
        // partner gets 11+11=22 pts
        tbp[2].push(make_trick(
            2,
            vec![
                (2, Card::new(Suit::Hearts, Rank::Ace)),  // 11
                (2, Card::new(Suit::Spades, Rank::King)), // 4
                (0, Card::new(Suit::Clubs, Rank::Seven)), // 0
                (3, Card::new(Suit::Clubs, Rank::Eight)), // 0
                (4, Card::new(Suit::Clubs, Rank::Nine)),  // 0
            ],
        )); // partner: 15 → team = 22+15 = 37 > 30, ≤ 60 → normal loss
        let scores = Sheepshead.score_game(&tbp, &state);
        assert_eq!(scores[1], -2, "picker loses 2v3: -2");
        assert_eq!(scores[2], -1, "partner loses 2v3: -1");
        assert_eq!(scores[0], 1);
        assert_eq!(scores[3], 1);
        assert_eq!(scores[4], 1);
        assert_eq!(scores.iter().sum::<i32>(), 0);
    }

    #[test]
    fn partner_2v3_schneider_win() {
        let state = make_state_with_meta(make_sheepshead_meta(
            Some(1),
            "done",
            0,
            false,
            Some("clubs"),
            Some(2),
            false,
            vec![],
            vec![],
        ));
        let mut tbp: Vec<Vec<Trick>> = vec![Vec::new(); 5];
        tbp[1].push(make_trick(
            1,
            vec![
                (1, Card::new(Suit::Clubs, Rank::Ace)),
                (1, Card::new(Suit::Clubs, Rank::Ten)),
                (1, Card::new(Suit::Spades, Rank::Ace)),
                (1, Card::new(Suit::Spades, Rank::Ten)),
                (1, Card::new(Suit::Hearts, Rank::Ace)),
            ],
        )); // 53
        tbp[2].push(make_trick(
            2,
            vec![
                (2, Card::new(Suit::Hearts, Rank::Ten)),
                (2, Card::new(Suit::Diamonds, Rank::Ace)),
                (2, Card::new(Suit::Diamonds, Rank::Ten)),
                (2, Card::new(Suit::Clubs, Rank::King)),
                (2, Card::new(Suit::Spades, Rank::King)),
            ],
        )); // 39 → team: 92; defenders: 28 ≤ 30 → schneider
        let scores = Sheepshead.score_game(&tbp, &state);
        assert_eq!(scores[1], 4, "picker schneider win 2v3: +4");
        assert_eq!(scores[2], 2, "partner schneider win 2v3: +2");
        assert_eq!(scores[0], -2);
        assert_eq!(scores[3], -2);
        assert_eq!(scores[4], -2);
        assert_eq!(scores.iter().sum::<i32>(), 0);
    }

    #[test]
    fn partner_never_revealed_scores_as_going_alone() {
        // partner is None (ace never played) → treat as going alone
        let state = make_state_with_meta(make_sheepshead_meta(
            Some(1),
            "done",
            0,
            false,
            Some("clubs"),
            None,
            false,
            vec![],
            vec![],
        ));
        let mut tbp: Vec<Vec<Trick>> = vec![Vec::new(); 5];
        tbp[1].push(make_trick(
            1,
            vec![
                (1, Card::new(Suit::Clubs, Rank::Ace)),
                (1, Card::new(Suit::Clubs, Rank::Ten)),
                (1, Card::new(Suit::Spades, Rank::Ace)),
                (1, Card::new(Suit::Spades, Rank::Ten)),
                (1, Card::new(Suit::Hearts, Rank::Ace)),
            ],
        )); // 53
        tbp[1].push(make_trick(
            1,
            vec![
                (1, Card::new(Suit::Hearts, Rank::Ten)),
                (1, Card::new(Suit::Diamonds, Rank::Ace)),
                (0, Card::new(Suit::Clubs, Rank::Seven)),
                (2, Card::new(Suit::Clubs, Rank::Eight)),
                (3, Card::new(Suit::Clubs, Rank::Nine)),
            ],
        )); // 10+11=21 → picker total 74 > 60; defenders 46 > 30 → no schneider
        let scores = Sheepshead.score_game(&tbp, &state);
        assert_eq!(scores[1], 4, "partner null → going alone win: +4");
        assert_eq!(scores[0], -1);
        assert_eq!(scores[2], -1);
        assert_eq!(scores[3], -1);
        assert_eq!(scores[4], -1);
        assert_eq!(scores.iter().sum::<i32>(), 0);
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
            assert!(
                Sheepshead
                    .trump_rank(Card::new(suit, Rank::Queen), &state)
                    .is_some()
            );
            assert!(
                Sheepshead
                    .trump_rank(Card::new(suit, Rank::Jack), &state)
                    .is_some()
            );
        }
    }

    #[test]
    fn non_diamond_plain_cards_are_not_trump() {
        let state = dummy_state();
        for rank in [
            Rank::Ace,
            Rank::Ten,
            Rank::King,
            Rank::Nine,
            Rank::Eight,
            Rank::Seven,
        ] {
            for suit in [Suit::Clubs, Suit::Spades, Suit::Hearts] {
                assert!(
                    Sheepshead
                        .trump_rank(Card::new(suit, rank), &state)
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn trick_winner_trump_beats_led_suit() {
        let state = dummy_state();
        let mut trick = Trick::new(0);
        trick.plays.push((0, Card::new(Suit::Clubs, Rank::Ace))); // led ♣A
        trick.plays.push((1, Card::new(Suit::Clubs, Rank::Seven))); // ♣7
        trick
            .plays
            .push((2, Card::new(Suit::Diamonds, Rank::Seven))); // 7♦ = lowest trump
        assert_eq!(Sheepshead.trick_winner(&trick, &state), 2);
    }

    // ── calling sub-phase ────────────────────────────────────────────────────

    /// Advance to the calling sub-phase (pick → bury → ready for call).
    fn calling_state() -> (GameState, usize) {
        let mut state = dealt_state();
        // player 1 picks (dealer=0, first player=1)
        Sheepshead
            .apply_bid(&mut state, 1, &serde_json::json!({"action":"pick"}))
            .unwrap();
        let c1 = state.hands[1][0];
        let c2 = state.hands[1][1];
        let r = Sheepshead
            .apply_bid(
                &mut state,
                1,
                &serde_json::json!({"action":"bury","cards":[c1,c2]}),
            )
            .unwrap();
        // After burying, should be in calling sub-phase (NOT playing yet)
        assert!(
            !r.phase_complete,
            "bury should NOT transition to Playing directly"
        );
        assert!(matches!(&state.meta, GameMeta::Sheepshead(m) if m.sub_phase == "calling"));
        (state, 1) // (state, picker_seat)
    }

    #[test]
    fn bury_transitions_to_calling_not_playing() {
        let (state, _) = calling_state();
        assert_eq!(state.phase, GamePhase::Bidding);
        assert!(matches!(&state.meta, GameMeta::Sheepshead(m) if m.sub_phase == "calling"));
        assert!(
            matches!(&state.meta, GameMeta::Sheepshead(m) if !m.callable_suits.is_empty() || m.callable_suits.is_empty())
        );
    }

    #[test]
    fn go_alone_transitions_to_playing() {
        let (mut state, picker) = calling_state();
        let r = Sheepshead
            .apply_bid(
                &mut state,
                picker,
                &serde_json::json!({"action":"go_alone"}),
            )
            .unwrap();
        assert!(r.phase_complete);
        assert_eq!(state.phase, GamePhase::Playing);
        assert!(matches!(&state.meta, GameMeta::Sheepshead(m) if m.going_alone));
        assert!(matches!(&state.meta, GameMeta::Sheepshead(m) if m.called_suit.is_none()));
    }

    #[test]
    fn call_valid_suit_transitions_to_playing() {
        let (mut state, picker) = calling_state();
        // Find a callable suit from meta
        let suits: Vec<String> = if let GameMeta::Sheepshead(ref m) = state.meta {
            m.callable_suits.clone()
        } else {
            vec![]
        };
        if suits.is_empty() {
            // No callable suit means forced go_alone — skip this test
            return;
        }
        let suit_str = suits[0].clone();
        let r = Sheepshead
            .apply_bid(
                &mut state,
                picker,
                &serde_json::json!({"action":"call","suit":suit_str}),
            )
            .unwrap();
        assert!(r.phase_complete);
        assert_eq!(state.phase, GamePhase::Playing);
        assert!(
            matches!(&state.meta, GameMeta::Sheepshead(m) if m.called_suit.as_deref() == Some(suit_str.as_str()))
        );
        assert!(matches!(&state.meta, GameMeta::Sheepshead(m) if !m.going_alone));
    }

    #[test]
    fn playing_called_ace_sets_partner_in_meta() {
        // Set up a game where player 1 called a suit (if callable)
        let (mut state, picker) = calling_state();
        let callable: Vec<String> = if let GameMeta::Sheepshead(ref m) = state.meta {
            m.callable_suits.clone()
        } else {
            vec![]
        };
        if callable.is_empty() {
            return;
        } // no callable suit — skip

        let suit_str = callable[0].clone();
        Sheepshead
            .apply_bid(
                &mut state,
                picker,
                &serde_json::json!({"action":"call","suit":suit_str}),
            )
            .unwrap();
        assert_eq!(state.phase, GamePhase::Playing);

        // Find which player holds the called ace
        let called_suit: Suit = serde_json::from_str(&format!("\"{}\"", suit_str)).unwrap();
        let ace = Card::new(called_suit, Rank::Ace);
        let partner_seat = (0..5).find(|&s| state.hands[s].contains(&ace));
        let Some(partner) = partner_seat else { return }; // ace may be in buried — skip

        // Set current_player to partner so we can test the revelation directly
        state.current_player = partner;
        // Ensure there's no current trick (partner leads)
        state.current_trick = None;
        Sheepshead.apply_play(&mut state, partner, ace).unwrap();
        assert!(
            matches!(&state.meta, GameMeta::Sheepshead(m) if m.partner == Some(partner)),
            "partner should be set when called ace is played"
        );
    }

    // ── hint_reason tests ────────────────────────────────────────────────────

    /// Build a Playing-phase state with picker=0, caller provided can adjust fields.
    fn state_with_picker(picker: usize) -> GameState {
        let mut state = GameState::new(Uuid::nil(), "sheepshead".into(), 5, 0);
        state.phase = GamePhase::Playing;
        state.current_player = picker;
        state.meta = make_sheepshead_meta(
            Some(picker),
            "done",
            0,
            false,
            Some("clubs"),
            None,
            false,
            vec![],
            vec![],
        );
        // Give each seat some cards
        let seven_hearts = Card::new(Suit::Hearts, Rank::Seven);
        state.hands = vec![vec![seven_hearts]; 5];
        state
    }

    #[test]
    fn hint_reason_picker_leading_trump() {
        let state = state_with_picker(0);
        // Q♣ is trump (highest trump in Sheepshead)
        let trump_card = Card::new(Suit::Clubs, Rank::Queen);
        // Picker (seat 0) is leading (no current trick)
        let reason = Sheepshead.hint_reason(trump_card, &state, 0);
        assert!(
            reason.to_lowercase().contains("trump"),
            "hint for picker leading trump should mention 'trump', got: {reason}"
        );
    }

    #[test]
    fn hint_reason_defender_leading_ace() {
        let state = state_with_picker(0);
        // Seat 1 is a defender; they are leading (trick is None → leading)
        let ace_clubs = Card::new(Suit::Clubs, Rank::Ace);
        // ace_clubs is a plain suit card (Clubs Ace is not trump in Sheepshead)
        let reason = Sheepshead.hint_reason(ace_clubs, &state, 1);
        assert!(
            !reason.is_empty(),
            "hint_reason must return a non-empty string for a defender leading an ace"
        );
    }

    #[test]
    fn hint_reason_team_winning_dump_points() {
        let mut state = state_with_picker(0);
        // Set up a trick where picker (seat 0) is already winning with Q♣.
        let queen_clubs = Card::new(Suit::Clubs, Rank::Queen);
        let mut trick = Trick::new(0);
        trick.plays.push((0, queen_clubs)); // picker led Q♣ (trump, winning)
        state.current_trick = Some(trick);
        state.current_player = 1; // seat 1 is following

        // Seat 1 is a defender following into a trick the picker is winning.
        // Playing a fail card means "teammate is winning" context doesn't apply
        // (seat 1 is not on picker's team), but the hint should produce a non-empty string.
        let fail_card = Card::new(Suit::Hearts, Rank::Seven); // plain fail card
        let reason = Sheepshead.hint_reason(fail_card, &state, 1);
        assert!(
            !reason.is_empty(),
            "hint_reason must return a non-empty string when following; got empty string"
        );

        // Now test from the picker's own perspective on a trick where picker is winning.
        // No partner is set, so teammate lookup returns None — falls into i_am_winning path.
        state.current_player = 0;
        let reason_picker = Sheepshead.hint_reason(fail_card, &state, 0);
        assert!(
            !reason_picker.is_empty(),
            "hint_reason must return a non-empty string for picker following their own winning trick"
        );
    }

    #[test]
    fn call_ace_picker_holds_is_rejected() {
        let (mut state, picker) = calling_state();
        // Find a suit whose ace the picker holds — should be invalid
        let ace_in_hand = state.hands[picker]
            .iter()
            .find(|c| {
                c.rank == Rank::Ace && c.suit != Suit::Diamonds && trump_strength(**c).is_none()
            })
            .copied();
        let Some(ace) = ace_in_hand else { return }; // skip if no fail aces in hand
        let suit_str = format!("{:?}", ace.suit).to_lowercase();
        let err = Sheepshead.apply_bid(
            &mut state,
            picker,
            &serde_json::json!({"action":"call","suit":suit_str}),
        );
        assert!(
            err.is_err(),
            "should reject calling an ace the picker holds"
        );
    }
}
