use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::engine::Card;

/// Flat bid payload covering all games.
/// The `action` field discriminates the bid type.
/// Games validate that only their own actions are accepted.
///
/// This type is used as the TypeScript-facing wire type for `ClientMessage::Bid.value`
/// and `StateUpdate::BidPlaced.value`.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BidPayload {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cards: Option<Vec<Card>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card: Option<Card>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alone: Option<bool>,
}
