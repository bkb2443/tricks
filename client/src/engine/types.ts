// AUTO-GENERATED — do not edit by hand.
// Run `cargo test export_typescript_bindings` in server/ to regenerate.

type JsonValue = number | string | boolean | null | JsonValue[] | { [key: string]: JsonValue };

export type Suit = "clubs" | "spades" | "hearts" | "diamonds";
export type Rank = "two" | "three" | "four" | "five" | "six" | "seven" | "eight" | "nine" | "ten" | "jack" | "queen" | "king" | "ace";
export type Card = { suit: Suit, rank: Rank, };
export type Trick = {
/**
 * Index of the player who led this trick.
 */
led_by: number,
/**
 * Cards played in order: (player_index, card).
 */
plays: Array<[number, Card]>,
/**
 * Seat of the player who won this trick. Set when the trick completes.
 */
winner: number | null, };
export type GamePhase = "lobby" | "bidding" | "playing" | "scoring" | "intermission";
export type SeatInfo = { seat: number,
/**
 * "empty" | "human" | "bot" | "disconnected"
 */
state: string, name: string | null, };
export type LobbyMeta = { host_seat: number | null, countdown_ends_at: number | null, room_type: string, max_hands: number | null, };
export type SheepsheadMeta = { picker: number | null,
/**
 * "picking" | "burying" | "calling" | "done"
 */
sub_phase: string, passed: number, leaster: boolean, buried: Array<Card>, callable_suits: Array<string>, called_suit: string | null, going_alone: boolean, partner: number | null, };
export type EuchreMeta = { turned_up_card: Card | null,
/**
 * "ordering" | "discarding" | "calling" | "done"
 */
sub_phase: string, passed_round1: number, passed_round2: number, caller_seat: number | null, called_suit: string | null, going_alone: boolean, sits_out: number | null, };
export type GameMeta = { "kind": "none" } | { "kind": "lobby" } & LobbyMeta | { "kind": "sheepshead" } & SheepsheadMeta | { "kind": "euchre" } & EuchreMeta;
export type GameState = { game_id: string, game_name: string, phase: GamePhase, player_count: number,
/**
 * Seat index of the player who dealt this hand.
 */
dealer: number,
/**
 * Index of the player whose turn it is.
 */
current_player: number,
/**
 * `hands[i]` = cards currently held by player i. Clients only see their own hand.
 */
hands: Array<Array<Card>>,
/**
 * Named side piles (e.g. Sheepshead blind, Euchre kitty). Hidden from clients
 * until game-specific rules expose them (e.g. picker takes the blind).
 */
extra_piles: Array<[string, Array<Card>]>, current_trick: Trick | null, completed_tricks: Array<Trick>, scores: Array<number>,
/**
 * Cumulative per-player scores across all hands played so far in this session.
 */
session_scores: Array<number>,
/**
 * Game-specific metadata. Typed via `GameMeta` so the TypeScript client
 * receives a discriminated union rather than `unknown`.
 */
meta: GameMeta,
/**
 * Display name for each seat. Populated by the room before the first Snapshot.
 */
names: Array<string>, };
export type BidPayload = { action: string, cards: Array<Card> | null, suit: string | null, card: Card | null, alone: boolean | null, };
export type ClientMessage = { "type": "join_room", room_id: string | null, game: string, players: number, fill_bots: boolean, } | { "type": "create_room", name: string, game: string, max_hands: number | null, } | { "type": "join", name: string, room_code: string, } | { "type": "spectate", name: string, room_code: string, } | { "type": "play_card", card: Card, } | { "type": "bid", value: JsonValue, } | { "type": "lobby_chat", text: string, } | { "type": "start_game" } | { "type": "force_bot", seat: number, } | { "type": "extend_rejoin", seat: number, } | { "type": "join_queue" } | { "type": "leave_queue" } | { "type": "start_next_hand" };
export type StateUpdate = { "type": "joined_room", room_id: string, seat: number, room_code: string, } | { "type": "joined_as_spectator", room_id: string, room_code: string, } | { "type": "snapshot", state: GameState, } | { "type": "card_played", player: number, card: Card, current_trick_winner: number | null, next_player: number, } | { "type": "trick_complete", winner: number, points: number, } | { "type": "hand_complete", hand_scores: Array<number>, session_scores: Array<number>, } | { "type": "session_over", winner: number, final_scores: Array<number>, } | { "type": "bid_placed", player: number, value: JsonValue, current_player: number, } | { "type": "hand_updated", hand: Array<Card>, } | { "type": "phase_changed", phase: GamePhase, } | { "type": "partner_revealed", seat: number, } | { "type": "lobby_chat", from: string, text: string, timestamp: number, } | { "type": "seat_update", seats: Array<SeatInfo>, spectator_count: number, } | { "type": "queue_status", position: number, waiting_since: number, } | { "type": "error", message: string, };
