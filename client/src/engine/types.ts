// These types mirror the Rust server structs exactly.
export type Suit = 'clubs' | 'spades' | 'hearts' | 'diamonds'

export type Rank =
  | 'two' | 'three' | 'four' | 'five' | 'six'
  | 'seven' | 'eight' | 'nine' | 'ten'
  | 'jack' | 'queen' | 'king' | 'ace'

export interface Card { suit: Suit; rank: Rank }

export type GamePhase = 'lobby' | 'bidding' | 'playing' | 'scoring'

export interface SeatInfo {
  seat: number
  state: 'empty' | 'human' | 'bot' | 'disconnected'
  name: string | null
}

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
  hands: Card[][]
  extra_piles: [string, Card[]][]
  current_trick: Trick | null
  completed_tricks: Trick[]
  scores: number[]
  meta: Record<string, unknown>
  names: string[]
}

export type ClientMessage =
  | { type: 'join_room'; room_id?: string; game: string; players: number; fill_bots?: boolean }
  | { type: 'create_room'; name: string; game: string; max_hands: number | null }
  | { type: 'join'; name: string; room_code: string }
  | { type: 'play_card'; card: Card }
  | { type: 'bid'; value: unknown }
  | { type: 'lobby_chat'; text: string }
  | { type: 'start_game' }
  | { type: 'force_bot'; seat: number }
  | { type: 'extend_rejoin'; seat: number }
  | { type: 'join_queue' }
  | { type: 'leave_queue' }

export type StateUpdate =
  | { type: 'joined_room';     room_id: string; seat: number; room_code: string }
  | { type: 'snapshot';        state: GameState }
  | { type: 'card_played';     player: number; card: Card }
  | { type: 'trick_complete';  winner: number; points: number }
  | { type: 'hand_complete';   hand_scores: number[]; session_scores: number[] }
  | { type: 'session_over';    winner: number; final_scores: number[] }
  | { type: 'bid_placed';      player: number; value: unknown; current_player: number }
  | { type: 'hand_updated';    hand: Card[] }
  | { type: 'phase_changed';   phase: GamePhase }
  | { type: 'partner_revealed'; seat: number }
  | { type: 'lobby_chat';      from: string; text: string; timestamp: number }
  | { type: 'seat_update';     seats: SeatInfo[] }
  | { type: 'queue_status';    position: number; waiting_since: number }
  | { type: 'error';           message: string }
