use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::engine::Card;

/// Lobby-phase metadata.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LobbyMeta {
    pub host_seat: Option<usize>,
    #[ts(type = "number | null")]
    pub countdown_ends_at: Option<u64>,
    pub room_type: String,
    pub max_hands: Option<u32>,
}

/// Sheepshead game-phase metadata (used during Bidding, Playing, Scoring, Intermission).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SheepsheadMeta {
    pub picker: Option<usize>,
    /// "picking" | "burying" | "calling" | "done"
    pub sub_phase: String,
    pub passed: usize,
    pub leaster: bool,
    pub buried: Vec<Card>,
    pub callable_suits: Vec<String>,
    pub called_suit: Option<String>,
    pub going_alone: bool,
    pub partner: Option<usize>,
}

/// Euchre game-phase metadata.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EuchreMeta {
    pub turned_up_card: Option<Card>,
    /// "ordering" | "discarding" | "calling" | "done"
    pub sub_phase: String,
    pub passed_round1: usize,
    pub passed_round2: usize,
    pub caller_seat: Option<usize>,
    pub called_suit: Option<String>,
    pub going_alone: bool,
    pub sits_out: Option<usize>,
}

/// Game-specific metadata stored in `GameState::meta`.
/// Tagged with `"kind"` discriminant for TypeScript discriminated union generation.
#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[ts(export)]
pub enum GameMeta {
    #[default]
    None,
    Lobby(LobbyMeta),
    Sheepshead(SheepsheadMeta),
    Euchre(EuchreMeta),
}
