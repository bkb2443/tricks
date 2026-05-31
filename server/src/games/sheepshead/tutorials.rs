use crate::engine::tutorial::{TutorialHand, TutorialStep, StepTrigger};
use crate::engine::{Suit, Rank};

// ---------------------------------------------------------------------------
// All Sheepshead tutorials
// ---------------------------------------------------------------------------

static TUTORIALS: [TutorialHand; 3] = [
    // ── Tutorial 1: Picking Hand ─────────────────────────────────────────────
    TutorialHand {
        id: "sheepshead-picking",
        title: "Picking Hand",
        description: "Pick the blind, bury for points, call a partner, and lead trump.",
        dealer: 4,
        player_seat: 0,
        hands: &[
            // Seat 0 (player): Q♣ Q♠ J♥ J♦ 9♦ 9♣
            &[
                (Suit::Clubs, Rank::Queen),
                (Suit::Spades, Rank::Queen),
                (Suit::Hearts, Rank::Jack),
                (Suit::Diamonds, Rank::Jack),
                (Suit::Diamonds, Rank::Nine),
                (Suit::Clubs, Rank::Nine),
            ],
            // Seat 1: Q♥ J♣ A♦ A♠ 10♠ A♥
            &[
                (Suit::Hearts, Rank::Queen),
                (Suit::Clubs, Rank::Jack),
                (Suit::Diamonds, Rank::Ace),
                (Suit::Spades, Rank::Ace),
                (Suit::Spades, Rank::Ten),
                (Suit::Hearts, Rank::Ace),
            ],
            // Seat 2 (partner, holds A♣): Q♦ J♠ 10♦ A♣ K♠ K♥
            &[
                (Suit::Diamonds, Rank::Queen),
                (Suit::Spades, Rank::Jack),
                (Suit::Diamonds, Rank::Ten),
                (Suit::Clubs, Rank::Ace),
                (Suit::Spades, Rank::King),
                (Suit::Hearts, Rank::King),
            ],
            // Seat 3: K♦ 8♦ 8♣ 9♠ 8♠ 9♥
            &[
                (Suit::Diamonds, Rank::King),
                (Suit::Diamonds, Rank::Eight),
                (Suit::Clubs, Rank::Eight),
                (Suit::Spades, Rank::Nine),
                (Suit::Spades, Rank::Eight),
                (Suit::Hearts, Rank::Nine),
            ],
            // Seat 4: 7♦ 10♣ 7♣ 7♠ 8♥ 7♥
            &[
                (Suit::Diamonds, Rank::Seven),
                (Suit::Clubs, Rank::Ten),
                (Suit::Clubs, Rank::Seven),
                (Suit::Spades, Rank::Seven),
                (Suit::Hearts, Rank::Eight),
                (Suit::Hearts, Rank::Seven),
            ],
        ],
        // Blind: K♣ 10♥
        extra_pile: &[
            (Suit::Clubs, Rank::King),
            (Suit::Hearts, Rank::Ten),
        ],
        steps: &[
            TutorialStep {
                trigger: StepTrigger::PlayerTurn { phase: "picking" },
                narration: "You hold Q\u{2663}, Q\u{2660}, J\u{2665}, J\u{2666}, and 9\u{2666} \u{2014} five trump cards including two Queens and two Jacks. That\u{2019}s a strong hand. Pick the blind to grab 2 more cards and then bury 2 of your weakest.",
            },
            TutorialStep {
                trigger: StepTrigger::PlayerTurn { phase: "burying" },
                narration: "You picked up K\u{2663} and 10\u{2665}. Bury them both \u{2014} they\u{2019}re worth 14 points that count toward your score even sitting in the buried pile. This also empties your hand of fail cards, giving you a pure trump hand.",
            },
            TutorialStep {
                trigger: StepTrigger::PlayerTurn { phase: "calling" },
                narration: "You have 9\u{2663} in your hand \u{2014} a clubs card that isn\u{2019}t the ace. Call clubs: whoever holds A\u{2663} becomes your secret partner. They\u{2019}ll reveal themselves the moment they play that ace.",
            },
            TutorialStep {
                trigger: StepTrigger::TrickComplete { trick_index: 0 },
                narration: "Your Q\u{2663} \u{2014} the highest card in Sheepshead \u{2014} took trick 1 along with all the trump others were forced to play. Stripping trump from defenders weakens them for later tricks.",
            },
            TutorialStep {
                trigger: StepTrigger::BotActed { seat: 2, trick_index: 2 },
                narration: "Seat 2 just played A\u{2663} \u{2014} that\u{2019}s the called ace! Seat 2 is now revealed as your partner. Watch for opportunities to dump high-point cards onto tricks they\u{2019}re winning.",
            },
            TutorialStep {
                trigger: StepTrigger::TrickComplete { trick_index: 5 },
                narration: "Hand over. Count your team\u{2019}s points: picker\u{2019}s tricks + partner\u{2019}s tricks + your buried cards. You need more than 60 to win. Your Q\u{2663} and Q\u{2660} alone are worth 6 points \u{2014} trump wins tricks and carries points.",
            },
            TutorialStep {
                trigger: StepTrigger::GameEnd,
                narration: "Well played. As picker, your trump strength let you control the hand. Try the Partner Hand tutorial to see the game from the defender\u{2019}s side.",
            },
        ],
    },
    // ── Tutorial 2: Defender/Partner Hand ───────────────────────────────────
    TutorialHand {
        id: "sheepshead-partner",
        title: "Defender Hand",
        description: "Play as the secret partner \u{2014} pass the pick, hold your called ace, and reveal at the right moment.",
        dealer: 4,
        player_seat: 0,
        hands: &[
            // Seat 0 (player/partner, holds A♣): J♦ 9♦ A♣ K♠ 9♠ A♥
            &[
                (Suit::Diamonds, Rank::Jack),
                (Suit::Diamonds, Rank::Nine),
                (Suit::Clubs, Rank::Ace),
                (Suit::Spades, Rank::King),
                (Suit::Spades, Rank::Nine),
                (Suit::Hearts, Rank::Ace),
            ],
            // Seat 1: Q♥ J♣ K♦ 10♠ 8♣ 7♥
            &[
                (Suit::Hearts, Rank::Queen),
                (Suit::Clubs, Rank::Jack),
                (Suit::Diamonds, Rank::King),
                (Suit::Spades, Rank::Ten),
                (Suit::Clubs, Rank::Eight),
                (Suit::Hearts, Rank::Seven),
            ],
            // Seat 2 (picker): Q♣ Q♠ J♥ A♦ 7♣ K♥
            &[
                (Suit::Clubs, Rank::Queen),
                (Suit::Spades, Rank::Queen),
                (Suit::Hearts, Rank::Jack),
                (Suit::Diamonds, Rank::Ace),
                (Suit::Clubs, Rank::Seven),
                (Suit::Hearts, Rank::King),
            ],
            // Seat 3: Q♦ J♠ 8♦ K♣ 9♣ 10♥
            &[
                (Suit::Diamonds, Rank::Queen),
                (Suit::Spades, Rank::Jack),
                (Suit::Diamonds, Rank::Eight),
                (Suit::Clubs, Rank::King),
                (Suit::Clubs, Rank::Nine),
                (Suit::Hearts, Rank::Ten),
            ],
            // Seat 4: 7♦ 10♣ 8♠ 7♠ A♠ 9♥
            &[
                (Suit::Diamonds, Rank::Seven),
                (Suit::Clubs, Rank::Ten),
                (Suit::Spades, Rank::Eight),
                (Suit::Spades, Rank::Seven),
                (Suit::Spades, Rank::Ace),
                (Suit::Hearts, Rank::Nine),
            ],
        ],
        // Blind: 10♦ 8♥
        extra_pile: &[
            (Suit::Diamonds, Rank::Ten),
            (Suit::Hearts, Rank::Eight),
        ],
        steps: &[
            TutorialStep {
                trigger: StepTrigger::PlayerTurn { phase: "picking" },
                narration: "Your hand has J\u{2666} and 9\u{2666} \u{2014} only two trump, and just 2 queen/jack points. That\u{2019}s too weak to pick. Pass and let a stronger player take it.",
            },
            TutorialStep {
                trigger: StepTrigger::TrickComplete { trick_index: 0 },
                narration: "Seat 2 picked and called clubs, making you their secret partner. You hold A\u{2663} \u{2014} that\u{2019}s the card that reveals you. Don\u{2019}t play it too early: the picker may not need you yet, and revealing yourself too soon tells the defenders who to target.",
            },
            TutorialStep {
                trigger: StepTrigger::PlayerTurn { phase: "playing" },
                narration: "You\u{2019}re a defender for now \u{2014} your identity is secret. Play safe fail cards to dump zero-point cards. Hold A\u{2663} until a trick where the picker is already winning, then play it to score big points for your team.",
            },
            TutorialStep {
                trigger: StepTrigger::BotActed { seat: 2, trick_index: 2 },
                narration: "Seat 2 (picker) is winning this trick. Now is a good moment to play A\u{2663} \u{2014} you\u{2019}ll reveal yourself as the partner AND load the trick with 11 points for your team.",
            },
            TutorialStep {
                trigger: StepTrigger::TrickComplete { trick_index: 2 },
                narration: "A\u{2663} revealed you as the partner. Now both you and the picker know who\u{2019}s on which team. Work together: dump high-point cards onto tricks your partner is winning.",
            },
            TutorialStep {
                trigger: StepTrigger::GameEnd,
                narration: "As a partner, your role is to support the picker and score points together. Try the Leaster tutorial to play a hand where everyone is on their own.",
            },
        ],
    },
    // ── Tutorial 3: Leaster ──────────────────────────────────────────────────
    TutorialHand {
        id: "sheepshead-leaster",
        title: "Leaster",
        description: "All players pass \u{2014} avoid taking the most card points or you lose 4 points.",
        dealer: 4,
        player_seat: 0,
        hands: &[
            // Seat 0 (player): J♥ 9♦ A♣ K♠ 9♠ 8♥
            &[
                (Suit::Hearts, Rank::Jack),
                (Suit::Diamonds, Rank::Nine),
                (Suit::Clubs, Rank::Ace),
                (Suit::Spades, Rank::King),
                (Suit::Spades, Rank::Nine),
                (Suit::Hearts, Rank::Eight),
            ],
            // Seat 1: J♦ 8♦ 10♣ A♠ K♥ 7♥
            &[
                (Suit::Diamonds, Rank::Jack),
                (Suit::Diamonds, Rank::Eight),
                (Suit::Clubs, Rank::Ten),
                (Suit::Spades, Rank::Ace),
                (Suit::Hearts, Rank::King),
                (Suit::Hearts, Rank::Seven),
            ],
            // Seat 2: Q♥ 7♦ K♣ 10♠ 9♥ 8♠
            &[
                (Suit::Hearts, Rank::Queen),
                (Suit::Diamonds, Rank::Seven),
                (Suit::Clubs, Rank::King),
                (Suit::Spades, Rank::Ten),
                (Suit::Hearts, Rank::Nine),
                (Suit::Spades, Rank::Eight),
            ],
            // Seat 3: Q♦ K♦ A♥ 10♥ 9♣ 7♣
            &[
                (Suit::Diamonds, Rank::Queen),
                (Suit::Diamonds, Rank::King),
                (Suit::Hearts, Rank::Ace),
                (Suit::Hearts, Rank::Ten),
                (Suit::Clubs, Rank::Nine),
                (Suit::Clubs, Rank::Seven),
            ],
            // Seat 4: Q♠ A♦ 10♦ 8♣ 7♠ J♠
            &[
                (Suit::Spades, Rank::Queen),
                (Suit::Diamonds, Rank::Ace),
                (Suit::Diamonds, Rank::Ten),
                (Suit::Clubs, Rank::Eight),
                (Suit::Spades, Rank::Seven),
                (Suit::Spades, Rank::Jack),
            ],
        ],
        // Blind (unused in leaster): Q♣ J♣
        extra_pile: &[
            (Suit::Clubs, Rank::Queen),
            (Suit::Clubs, Rank::Jack),
        ],
        steps: &[
            TutorialStep {
                trigger: StepTrigger::PlayerTurn { phase: "picking" },
                narration: "Your hand has J\u{2665} and 9\u{2666} \u{2014} only 2 trump-point cards. That\u{2019}s not enough to pick. Pass and see what the others do.",
            },
            TutorialStep {
                trigger: StepTrigger::TrickComplete { trick_index: 0 },
                narration: "All five players passed \u{2014} this is a Leaster. There\u{2019}s no picker or partner; everyone plays alone. The player who takes the most card points at the end loses 4 points. Your goal: take as few points as possible, especially avoid tricks with aces and tens.",
            },
            TutorialStep {
                trigger: StepTrigger::PlayerTurn { phase: "playing" },
                narration: "You have A\u{2663} \u{2014} an 11-point card. In a normal hand that\u{2019}s an asset; in a Leaster it\u{2019}s a liability. Try to dump it onto a trick someone else is winning.",
            },
            TutorialStep {
                trigger: StepTrigger::TrickComplete { trick_index: 2 },
                narration: "Notice how bots are playing low cards to avoid taking tricks. In a Leaster, your lowest cards are your best cards.",
            },
            TutorialStep {
                trigger: StepTrigger::TrickComplete { trick_index: 5 },
                narration: "Leaster is over. The player with the most points loses 4; everyone else gains 1. In a Leaster, winning tricks means losing the hand.",
            },
            TutorialStep {
                trigger: StepTrigger::GameEnd,
                narration: "Sheepshead has three modes: normal game (pick), partnership (call), and Leaster (all pass). Each rewards a completely different strategy. Head back to the tutorial list to try the Partner tutorial.",
            },
        ],
    },
];

pub fn all() -> &'static [TutorialHand] {
    &TUTORIALS
}
