import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { useGameStore } from '@/stores/game'
import { useEuchreState } from './state'
import type { GameState } from '@/engine/types'

function makeEuchreState(meta: Record<string, unknown> = {}): GameState {
  return {
    game_id: 'test-id',
    game_name: 'euchre',
    phase: 'bidding',
    player_count: 4,
    dealer: 0,
    current_player: 1,
    hands: [[], [], [], []],
    extra_piles: [],
    current_trick: null,
    completed_tricks: [],
    scores: [0, 0, 0, 0],
    session_scores: [0, 0, 0, 0],
    meta: {
      kind: 'euchre' as const,
      turned_up_card: null,
      sub_phase: 'ordering',
      passed_round1: 0,
      passed_round2: 0,
      caller_seat: null,
      called_suit: null,
      going_alone: false,
      sits_out: null,
      ...meta,
    },
    names: [],
    training_mode: false,
    hint_enabled: false,
    legal_cards: [],
    hint: null,
  }
}

describe('useEuchreState', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('returns nulls when no game state', () => {
    const { callerSeat, sitsOut, calledSuit, turnedUpCard, subPhase } = useEuchreState()
    expect(callerSeat.value).toBeNull()
    expect(sitsOut.value).toBeNull()
    expect(calledSuit.value).toBeNull()
    expect(turnedUpCard.value).toBeNull()
    expect(subPhase.value).toBeNull()
  })

  it('returns nulls for a non-euchre game', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TST001' })
    store.handleUpdate({
      type: 'snapshot',
      state: {
        ...makeEuchreState(),
        game_name: 'sheepshead',
        player_count: 5,
        hands: [[], [], [], [], []],
        scores: [0, 0, 0, 0, 0],
        session_scores: [0, 0, 0, 0, 0],
        meta: { kind: 'sheepshead' as const, picker: null, passed: 0, buried: [], leaster: false, sub_phase: 'picking', callable_suits: [], called_suit: null, going_alone: false, partner: null },
      },
    })
    const { calledSuit, subPhase } = useEuchreState()
    expect(calledSuit.value).toBeNull()
    expect(subPhase.value).toBeNull()
  })

  it('reads subPhase from meta', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TST001' })
    store.handleUpdate({ type: 'snapshot', state: makeEuchreState({ sub_phase: 'calling' }) })
    const { subPhase } = useEuchreState()
    expect(subPhase.value).toBe('calling')
  })

  it('reads callerSeat and calledSuit after bidding completes', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TST001' })
    store.handleUpdate({
      type: 'snapshot',
      state: makeEuchreState({ caller_seat: 2, called_suit: 'hearts', sub_phase: 'done' }),
    })
    const { callerSeat, calledSuit } = useEuchreState()
    expect(callerSeat.value).toBe(2)
    expect(calledSuit.value).toBe('hearts')
  })

  it('reads turnedUpCard from meta', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TST001' })
    store.handleUpdate({
      type: 'snapshot',
      state: makeEuchreState({ turned_up_card: { suit: 'clubs', rank: 'jack' } }),
    })
    const { turnedUpCard } = useEuchreState()
    expect(turnedUpCard.value).toEqual({ suit: 'clubs', rank: 'jack' })
  })

  it('reads sitsOut seat', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 0, room_code: 'TST001' })
    store.handleUpdate({ type: 'snapshot', state: makeEuchreState({ sits_out: 3 }) })
    const { sitsOut } = useEuchreState()
    expect(sitsOut.value).toBe(3)
  })
})
