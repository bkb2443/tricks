use crate::engine::tutorial::{StepTrigger, TutorialHand, TutorialStep};
use crate::engine::{Rank, Suit};

// ---------------------------------------------------------------------------
// All Euchre tutorials
// ---------------------------------------------------------------------------

static TUTORIALS: [TutorialHand; 2] = [
    // ── Tutorial 1: Calling Trump ────────────────────────────────────────────
    TutorialHand {
        id: "euchre-calling-trump",
        title: "Calling Trump",
        description: "Order up the turned card as trump, then lead trump as the maker.",
        dealer: 3,
        player_seat: 0,
        hands: &[
            // Seat 0 (player/maker): A♣ K♣ Q♣ J♠ A♠
            // (with clubs as trump: A♣, K♣, Q♣, J♠=left bower, A♠ fail)
            &[
                (Suit::Clubs, Rank::Ace),
                (Suit::Clubs, Rank::King),
                (Suit::Clubs, Rank::Queen),
                (Suit::Spades, Rank::Jack),
                (Suit::Spades, Rank::Ace),
            ],
            // Seat 1: A♥ K♥ Q♥ J♥ 10♥
            &[
                (Suit::Hearts, Rank::Ace),
                (Suit::Hearts, Rank::King),
                (Suit::Hearts, Rank::Queen),
                (Suit::Hearts, Rank::Jack),
                (Suit::Hearts, Rank::Ten),
            ],
            // Seat 2 (partner): A♦ K♦ Q♦ J♦ 10♦
            &[
                (Suit::Diamonds, Rank::Ace),
                (Suit::Diamonds, Rank::King),
                (Suit::Diamonds, Rank::Queen),
                (Suit::Diamonds, Rank::Jack),
                (Suit::Diamonds, Rank::Ten),
            ],
            // Seat 3 (dealer): 10♣ K♠ Q♠ 10♠ 9♠
            &[
                (Suit::Clubs, Rank::Ten),
                (Suit::Spades, Rank::King),
                (Suit::Spades, Rank::Queen),
                (Suit::Spades, Rank::Ten),
                (Suit::Spades, Rank::Nine),
            ],
        ],
        // Kitty: J♣ (turned up), 9♣ 9♥ 9♦ (rest, hidden)
        // Turned-up is J♣ — ordering it up makes clubs trump.
        // After ordering, dealer (seat 3) gets J♣, discards 9♠ → hand: 10♣ K♠ Q♠ 10♠ J♣
        extra_pile: &[
            (Suit::Clubs, Rank::Jack),
            (Suit::Clubs, Rank::Nine),
            (Suit::Hearts, Rank::Nine),
            (Suit::Diamonds, Rank::Nine),
        ],
        steps: &[
            TutorialStep {
                trigger: StepTrigger::PlayerTurn { phase: "ordering" },
                narration: "The dealer turned up J\u{2663}. If clubs is trump, J\u{2663} is the Right Bower \u{2014} the strongest card in the game. You already hold A\u{2663}, K\u{2663}, Q\u{2663}, and J\u{2660} (the Left Bower when clubs is trump). That\u{2019}s four trump cards. Order it up.",
            },
            TutorialStep {
                trigger: StepTrigger::TrickComplete { trick_index: 0 },
                narration: "Leading trump early is classic maker strategy: if opponents can\u{2019}t follow trump, they can\u{2019}t beat your trump leads.",
            },
            TutorialStep {
                trigger: StepTrigger::PlayerTurn { phase: "playing" },
                narration: "You still have strong trump left. Keep leading it \u{2014} you\u{2019}re drawing out the opponents\u{2019} remaining trump so your partner can win plain-suit tricks later.",
            },
            TutorialStep {
                trigger: StepTrigger::TrickComplete { trick_index: 4 },
                narration: "Makers need 3 of 5 tricks to score. If you swept all 5 that\u{2019}s a march \u{2014} 2 points each instead of 1. Try Tutorial 2 to see what going alone looks like.",
            },
            TutorialStep {
                trigger: StepTrigger::GameEnd,
                narration: "As the maker, your trump advantage sets up the whole hand. Try the Going Alone tutorial to play for 4 points \u{2014} but it\u{2019}s riskier.",
            },
        ],
    },
    // ── Tutorial 2: Going Alone ──────────────────────────────────────────────
    TutorialHand {
        id: "euchre-going-alone",
        title: "Going Alone",
        description: "Recognize a dominant trump hand and go alone for 4 points.",
        dealer: 3,
        player_seat: 0,
        hands: &[
            // Seat 0 (player/solo maker): J♥ J♦ K♥ Q♥ 10♥
            // (hearts trump: J♥=right bower, J♦=left bower, K♥, Q♥, 10♥)
            &[
                (Suit::Hearts, Rank::Jack),
                (Suit::Diamonds, Rank::Jack),
                (Suit::Hearts, Rank::King),
                (Suit::Hearts, Rank::Queen),
                (Suit::Hearts, Rank::Ten),
            ],
            // Seat 1: A♣ K♣ Q♣ J♣ 10♣
            &[
                (Suit::Clubs, Rank::Ace),
                (Suit::Clubs, Rank::King),
                (Suit::Clubs, Rank::Queen),
                (Suit::Clubs, Rank::Jack),
                (Suit::Clubs, Rank::Ten),
            ],
            // Seat 2 (partner, sits out): A♠ K♠ Q♠ J♠ 10♠
            &[
                (Suit::Spades, Rank::Ace),
                (Suit::Spades, Rank::King),
                (Suit::Spades, Rank::Queen),
                (Suit::Spades, Rank::Jack),
                (Suit::Spades, Rank::Ten),
            ],
            // Seat 3 (dealer): A♦ 10♦ 9♦ 9♣ 9♠
            &[
                (Suit::Diamonds, Rank::Ace),
                (Suit::Diamonds, Rank::Ten),
                (Suit::Diamonds, Rank::Nine),
                (Suit::Clubs, Rank::Nine),
                (Suit::Spades, Rank::Nine),
            ],
        ],
        // Kitty: A♥ (turned up), 9♥ K♦ Q♦ (rest)
        // After ordering alone: dealer gets A♥, discards 9♦ → hand: A♦ 10♦ 9♣ 9♠ A♥
        extra_pile: &[
            (Suit::Hearts, Rank::Ace),
            (Suit::Hearts, Rank::Nine),
            (Suit::Diamonds, Rank::King),
            (Suit::Diamonds, Rank::Queen),
        ],
        steps: &[
            TutorialStep {
                trigger: StepTrigger::PlayerTurn { phase: "ordering" },
                narration: "The turned-up card is A\u{2665}. You already hold J\u{2665} (Right Bower \u{2014} strongest in the game), J\u{2666} (Left Bower \u{2014} second strongest), K\u{2665}, Q\u{2665}, and 10\u{2665}. That\u{2019}s five hearts trump. With a hand this dominant, go alone for 4 points instead of the usual 1.",
            },
            TutorialStep {
                trigger: StepTrigger::TrickComplete { trick_index: 0 },
                narration: "Your partner (seat 2) is sitting out \u{2014} they can\u{2019}t help you. But you don\u{2019}t need them. Lead your highest trump to clear the field.",
            },
            TutorialStep {
                trigger: StepTrigger::PlayerTurn { phase: "playing" },
                narration: "One trick done. Opponents have no trump left to challenge you. Continue leading trump or your top plain-suit cards.",
            },
            TutorialStep {
                trigger: StepTrigger::TrickComplete { trick_index: 4 },
                narration: "All 5 tricks \u{2014} a march going alone! That\u{2019}s 4 points, compared to 1 for a normal win. Going alone is risky (you must take 3 of 5 unassisted) but the reward is high when your hand is this strong.",
            },
            TutorialStep {
                trigger: StepTrigger::GameEnd,
                narration: "You now know the three big Euchre moves: ordering up, calling trump in round 2, and going alone. Head back to try Sheepshead tutorials or play a full game.",
            },
        ],
    },
];

pub fn all() -> &'static [TutorialHand] {
    &TUTORIALS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Card;
    use std::collections::HashSet;

    fn all_cards_in_tutorial(tutorial: &crate::engine::tutorial::TutorialHand) -> Vec<Card> {
        let mut cards = Vec::new();
        for hand in tutorial.hands {
            for &(suit, rank) in *hand {
                cards.push(Card::new(suit, rank));
            }
        }
        for &(suit, rank) in tutorial.extra_pile {
            cards.push(Card::new(suit, rank));
        }
        cards
    }

    fn assert_no_duplicates(tutorial: &crate::engine::tutorial::TutorialHand, name: &str) {
        let cards = all_cards_in_tutorial(tutorial);
        let mut seen = HashSet::new();
        for c in &cards {
            assert!(seen.insert(*c), "{name}: duplicate card {c}");
        }
    }

    #[test]
    fn calling_trump_has_24_cards_no_duplicates() {
        let tutorial = &all()[0]; // euchre-calling-trump
        assert_eq!(
            all_cards_in_tutorial(tutorial).len(),
            24,
            "Euchre uses a 24-card deck"
        );
        assert_no_duplicates(tutorial, "calling-trump");
    }

    #[test]
    fn going_alone_has_24_cards_no_duplicates() {
        let tutorial = &all()[1]; // euchre-going-alone
        assert_eq!(
            all_cards_in_tutorial(tutorial).len(),
            24,
            "Euchre uses a 24-card deck"
        );
        assert_no_duplicates(tutorial, "going-alone");
    }

    #[test]
    fn all_tutorials_have_nonzero_steps() {
        for t in all() {
            assert!(!t.steps.is_empty(), "{} has no steps", t.id);
        }
    }
}
