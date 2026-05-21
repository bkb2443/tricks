import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useGameStore } from './game'
import type { Card, GameState } from '@/engine/types'

function makeState(overrides: Partial<GameState> = {}): GameState {
  return {
    game_id: 'test-id',
    game_name: 'sheepshead',
    phase: 'bidding',
    player_count: 5,
    dealer: 0,
    current_player: 1,
    hands: [[], [], [], [], []],
    extra_piles: [],
    current_trick: null,
    completed_tricks: [],
    scores: [0, 0, 0, 0, 0],
    meta: { picker: null, passed: 0, buried: [], leaster: false },
    names: [],
    ...overrides,
  }
}

const ACE_CLUBS:  Card = { suit: 'clubs',    rank: 'ace'  }
const KING_CLUBS: Card = { suit: 'clubs',    rank: 'king' }
const ACE_HEARTS: Card = { suit: 'hearts',   rank: 'ace'  }
const TEN_SPADES: Card = { suit: 'spades',   rank: 'ten'  }
const NINE_SPADES: Card = { suit: 'spades',  rank: 'nine' }

describe('game store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  // ── Basic join / snapshot ───────────────────────────────────────────────────

  it('handles joined_room', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'abc-123', seat: 2, room_code: 'TEST01' })
    expect(store.roomId).toBe('abc-123')
    expect(store.seat).toBe(2)
  })

  it('populates myHand from snapshot for own seat', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 1, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ hands: [[], [ACE_CLUBS], [], [], []] }) })
    expect(store.myHand).toEqual([ACE_CLUBS])
  })

  it('isMyTurn when current_player matches seat', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 1, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ current_player: 1 }) })
    expect(store.isMyTurn).toBe(true)
  })

  it('isMyTurn is false when current_player differs from seat', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 3, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ current_player: 1 }) })
    expect(store.isMyTurn).toBe(false)
  })

  it('hand_updated replaces myHand', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TEST01' })
    store.handleUpdate({ type: 'hand_updated', hand: [KING_CLUBS] })
    expect(store.myHand).toEqual([KING_CLUBS])
  })

  // ── card_played — bugs fixed: current_player advancement + myHand removal ──

  it('card_played creates a new trick when none exists', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ phase: 'playing' }) })
    store.handleUpdate({ type: 'card_played', player: 1, card: ACE_CLUBS, next_player: 2 })
    expect(store.gameState?.current_trick?.plays).toHaveLength(1)
    expect(store.gameState?.current_trick?.led_by).toBe(1)
  })

  it('card_played appends to an in-progress trick', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TEST01' })
    store.handleUpdate({
      type: 'snapshot',
      state: makeState({
        phase: 'playing',
        current_trick: { led_by: 1, plays: [[1, ACE_CLUBS]], winner: null },
      }),
    })
    store.handleUpdate({ type: 'card_played', player: 2, card: KING_CLUBS, next_player: 3 })
    expect(store.gameState?.current_trick?.plays).toHaveLength(2)
  })

  it('card_played sets current_player from server-supplied next_player', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ phase: 'playing', current_player: 1 }) })

    store.handleUpdate({ type: 'card_played', player: 1, card: ACE_CLUBS, next_player: 2 })
    expect(store.gameState?.current_player).toBe(2)

    store.handleUpdate({ type: 'card_played', player: 2, card: KING_CLUBS, next_player: 3 })
    expect(store.gameState?.current_player).toBe(3)
  })

  it('isMyTurn becomes true once next_player reaches human seat', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 2, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ phase: 'playing', current_player: 0 }) })

    expect(store.isMyTurn).toBe(false)
    store.handleUpdate({ type: 'card_played', player: 0, card: ACE_CLUBS, next_player: 1 })
    expect(store.isMyTurn).toBe(false)
    store.handleUpdate({ type: 'card_played', player: 1, card: KING_CLUBS, next_player: 2 })
    expect(store.isMyTurn).toBe(true)
  })

  it('card_played for own card removes it from myHand', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 1, room_code: 'TEST01' })
    store.handleUpdate({
      type: 'snapshot',
      state: makeState({ phase: 'playing', current_player: 1, hands: [[], [ACE_CLUBS, KING_CLUBS], [], [], []] }),
    })
    expect(store.myHand).toHaveLength(2)

    store.handleUpdate({ type: 'card_played', player: 1, card: ACE_CLUBS, next_player: 2 })
    expect(store.myHand).toHaveLength(1)
    expect(store.myHand[0]).toEqual(KING_CLUBS)
  })

  it('card_played for another player does not change myHand', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 1, room_code: 'TEST01' })
    store.handleUpdate({
      type: 'snapshot',
      state: makeState({ phase: 'playing', hands: [[], [ACE_CLUBS], [], [], []] }),
    })

    store.handleUpdate({ type: 'card_played', player: 0, card: KING_CLUBS, next_player: 1 })
    expect(store.myHand).toHaveLength(1)
    expect(store.myHand[0]).toEqual(ACE_CLUBS)
  })

  it('card_played always updates current_player to server next_player', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TEST01' })
    store.handleUpdate({
      type: 'snapshot',
      state: makeState({
        phase: 'playing',
        current_player: 4,
        current_trick: {
          led_by: 0,
          plays: [[0, ACE_CLUBS], [1, KING_CLUBS], [2, ACE_HEARTS], [3, TEN_SPADES]],
          winner: null,
        },
      }),
    })

    store.handleUpdate({ type: 'card_played', player: 4, card: NINE_SPADES, next_player: 0 })
    // Server sends the trick winner as next_player; trick_complete will follow
    expect(store.gameState?.current_player).toBe(0)
  })

  // ── trick_complete ──────────────────────────────────────────────────────────

  it('trick_complete moves trick to completed, clears current, sets current_player', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TEST01' })
    store.handleUpdate({
      type: 'snapshot',
      state: makeState({
        phase: 'playing',
        current_trick: { led_by: 1, plays: [[1, ACE_CLUBS]], winner: null },
      }),
    })
    store.handleUpdate({ type: 'trick_complete', winner: 1, points: 11 })
    expect(store.gameState?.current_trick).toBeNull()
    expect(store.gameState?.completed_tricks).toHaveLength(1)
    expect(store.gameState?.completed_tricks[0].winner).toBe(1)
    expect(store.gameState?.current_player).toBe(1)
  })

  it('stores completedTrick for 1.5s after trick_complete', async () => {
    const store = useGameStore()
    expect(store.completedTrick).toBeNull()
  })

  // ── bid_placed — bug fixed: current_player must advance during picking ──────

  // BUG FIX: bid_placed must update current_player so that isMyTurn becomes
  // true when bots have all passed and it reaches the human's picking turn.
  it('bid_placed updates current_player', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 2, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ current_player: 1 }) })

    store.handleUpdate({ type: 'bid_placed', player: 1, value: { action: 'pass' }, current_player: 2 })
    expect(store.gameState?.current_player).toBe(2)
    expect(store.isMyTurn).toBe(true)
  })

  it('isMyTurn becomes true once bots have passed and bid_placed reaches human seat', () => {
    const store = useGameStore()
    // Dealer = 4, so picking order is 0 → 1 → 2 → 3 → 4; human is seat 3
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 3, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ dealer: 4, current_player: 0 }) })

    expect(store.isMyTurn).toBe(false)
    store.handleUpdate({ type: 'bid_placed', player: 0, value: { action: 'pass' }, current_player: 1 })
    expect(store.isMyTurn).toBe(false)
    store.handleUpdate({ type: 'bid_placed', player: 1, value: { action: 'pass' }, current_player: 2 })
    expect(store.isMyTurn).toBe(false)
    store.handleUpdate({ type: 'bid_placed', player: 2, value: { action: 'pass' }, current_player: 3 })
    expect(store.isMyTurn).toBe(true) // Human's turn to pick or pass
  })

  it('bid_placed for pick keeps current_player on picker for bury sub-phase', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 2, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ current_player: 2 }) })

    // Human picks; server keeps current_player = 2 (picker must bury next)
    store.handleUpdate({ type: 'bid_placed', player: 2, value: { action: 'pick' }, current_player: 2 })
    expect(store.gameState?.current_player).toBe(2)
    expect(store.isMyTurn).toBe(true)
  })

  // Server must broadcast picker/sub_phase in the pick payload so the client
  // transitions from picking sub-phase to burying sub-phase. Without this, meta.picker
  // stays null and the bury UI (isBuryPhase) never renders — user cannot proceed after picking.
  it('bid_placed for pick sets meta.picker so bury phase becomes active', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 2, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ current_player: 2 }) })

    // Server broadcasts { picker, sub_phase } as the payload (not raw { action: "pick" })
    store.handleUpdate({
      type: 'bid_placed',
      player: 2,
      value: { picker: 2, sub_phase: 'burying' },
      current_player: 2,
    })

    expect(store.picker).toBe(2)
    expect(store.isPicker).toBe(true)
    expect(store.gameState?.meta?.sub_phase).toBe('burying')
  })

  it('bid_placed for bury advances current_player to first trick leader', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 2, room_code: 'TEST01' })
    store.handleUpdate({
      type: 'snapshot',
      state: makeState({ dealer: 1, current_player: 2, meta: { picker: 2, passed: 0, buried: [], leaster: false } }),
    })

    // After bury, server sets current_player to (dealer+1) % 5 = 2
    store.handleUpdate({ type: 'bid_placed', player: 2, value: { action: 'bury', cards: [] }, current_player: 2 })
    expect(store.gameState?.current_player).toBe(2)
  })

  // ── phase_changed ───────────────────────────────────────────────────────────

  it('phase_changed updates the phase', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ phase: 'bidding' }) })
    store.handleUpdate({ type: 'phase_changed', phase: 'playing' })
    expect(store.phase).toBe('playing')
  })

  // ── hand_complete / session_over / error / picker / reset ───────────────────

  it('hand_complete updates hand scores and session scores', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState() })
    store.handleUpdate({
      type: 'hand_complete',
      hand_scores: [1, -1, -1, -1, -1],
      session_scores: [1, -1, -1, -1, -1],
    })
    expect(store.gameState?.scores).toEqual([1, -1, -1, -1, -1])
    expect(store.sessionScores).toEqual([1, -1, -1, -1, -1])
  })

  it('session_over sets winner and final scores', () => {
    const store = useGameStore()
    store.handleUpdate({
      type: 'session_over',
      winner: 2,
      final_scores: [5, 3, 11, -2, 4],
    })
    expect(store.sessionWinner).toBe(2)
    expect(store.sessionScores).toEqual([5, 3, 11, -2, 4])
  })

  it('error sets the error message', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'error', message: 'not your turn' })
    expect(store.error).toBe('not your turn')
  })

  it('picker computed returns null before pick', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 1, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState() })
    expect(store.picker).toBeNull()
  })

  it('picker computed returns seat after meta.picker is set', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 1, room_code: 'TEST01' })
    store.handleUpdate({
      type: 'snapshot',
      state: makeState({ meta: { picker: 1, passed: 0, buried: [], leaster: false } }),
    })
    expect(store.picker).toBe(1)
    expect(store.isPicker).toBe(true)
  })

  it('reset clears all state including isSolo', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 2, room_code: 'TEST01' })
    store.isSolo = true
    store.reset()
    expect(store.roomId).toBeNull()
    expect(store.seat).toBeNull()
    expect(store.gameState).toBeNull()
    expect(store.isSolo).toBe(false)
  })
})
