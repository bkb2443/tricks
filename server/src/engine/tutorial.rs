use crate::engine::{Rank, Suit};

// Tutorial types are defined here for the engine layer; game implementations
// populate concrete instances. Allow dead_code until the game-rules agent wires them in.

/// A scripted, fixed-hand tutorial scenario for a game.
#[allow(dead_code)]
pub struct TutorialHand {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub dealer: usize,
    pub player_seat: usize,
    /// Fixed hands[i] = cards for seat i (as (Suit, Rank) pairs, converted to Card at runtime).
    pub hands: &'static [&'static [(Suit, Rank)]],
    /// Extra pile (blind for Sheepshead, kitty for Euchre).
    pub extra_pile: &'static [(Suit, Rank)],
    pub steps: &'static [TutorialStep],
}

/// A single narration step within a tutorial, fired when its trigger condition is met.
#[allow(dead_code)]
pub struct TutorialStep {
    pub trigger: StepTrigger,
    pub narration: &'static str,
}

/// Condition that causes a `TutorialStep` to fire.
#[derive(Clone, PartialEq)]
#[allow(dead_code)]
pub enum StepTrigger {
    /// Fires when the human player's turn arrives in a given bid sub_phase
    /// ("picking" | "burying" | "calling") or in the "playing" phase.
    PlayerTurn { phase: &'static str },
    /// Fires after the bot at `seat` plays in trick number `trick_index`
    /// (0-indexed completed tricks count).
    BotActed { seat: usize, trick_index: usize },
    /// Fires after trick `trick_index` (0-indexed) completes.
    TrickComplete { trick_index: usize },
    /// Fires when the hand ends (GameOver / Scoring phase).
    GameEnd,
}
