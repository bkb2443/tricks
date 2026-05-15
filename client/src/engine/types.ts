// These types mirror the Rust server structs exactly.
// Suit/Rank values are the serde(rename_all = "lowercase") serialisations.

export type Suit = 'clubs' | 'spades' | 'hearts' | 'diamonds'

export type Rank =
  | 'two' | 'three' | 'four' | 'five' | 'six'
  | 'seven' | 'eight' | 'nine' | 'ten'
  | 'jack' | 'queen' | 'king' | 'ace'

export interface Card {
  suit: Suit
  rank: Rank
}

export type GamePhase = 'bidding' | 'playing' | 'scoring'

export interface Trick {
  led_by: number
  plays: [number, Card][]
  winner: number | null
}

export interface GameState {
  game_id: string
  game_name: string
  phase: GamePhase
  player_count: number
  dealer: number
  current_player: number
  /** Only `hands[mySeat]` is populated; the rest are empty arrays. */
  hands: Card[][]
  /** Hidden from clients (e.g. blind) — always empty in received snapshots. */
  extra_piles: [string, Card[]][]
  current_trick: Trick | null
  completed_tricks: Trick[]
  scores: number[]
  /** Game-specific metadata (opaque; typed per-game where needed). */
  meta: Record<string, unknown>
}

// ---------------------------------------------------------------------------
// Messages: client → server
// ---------------------------------------------------------------------------

export type ClientMessage =
  | { type: 'join_room'; room_id?: string; game: string; players: number; fill_bots?: boolean }
  | { type: 'play_card'; card: Card }
  | { type: 'bid'; value: unknown }

// ---------------------------------------------------------------------------
// Messages: server → client
// ---------------------------------------------------------------------------

export type StateUpdate =
  | { type: 'joined_room';    room_id: string; seat: number }
  | { type: 'snapshot';       state: GameState }
  | { type: 'card_played';    player: number; card: Card }
  | { type: 'trick_complete'; winner: number; points: number }
  | { type: 'hand_complete';  hand_scores: number[]; session_scores: number[] }
  | { type: 'session_over';   winner: number; final_scores: number[] }
  | { type: 'bid_placed';     player: number; value: unknown; current_player: number }
  | { type: 'hand_updated';   hand: Card[] }
  | { type: 'phase_changed';  phase: GamePhase }
  | { type: 'error';          message: string }
