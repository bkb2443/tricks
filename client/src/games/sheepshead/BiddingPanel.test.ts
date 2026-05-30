import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { setActivePinia, createPinia } from 'pinia'
import { useGameStore } from '@/stores/game'
import BiddingPanel from './BiddingPanel.vue'
import type { GameState } from '@/engine/types'

function makeState(meta: object, currentPlayer = 1): GameState {
  return {
    game_id: 'test',
    game_name: 'sheepshead',
    phase: 'bidding',
    player_count: 5,
    dealer: 0,
    current_player: currentPlayer,
    hands: [[], [{ suit: 'clubs', rank: 'ace' }, { suit: 'clubs', rank: 'king' }], [], [], []],
    extra_piles: [],
    current_trick: null,
    completed_tricks: [],
    scores: [0, 0, 0, 0, 0],
    session_scores: [0, 0, 0, 0, 0],
    meta: {
      kind: 'sheepshead' as const,
      picker: null, sub_phase: 'picking', passed: 0,
      buried: [], leaster: false, callable_suits: [],
      called_suit: null, going_alone: false, partner: null,
      ...meta,
    },
    names: ['Bot 1', 'You', 'Bot 2', 'Bot 3', 'Bot 4'],
  }
}

describe('BiddingPanel sub-phase rendering', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('picking sub-phase shows pick/pass for the active player', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 1, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ picker: null, sub_phase: 'picking' }, 1) })
    const w = mount(BiddingPanel)
    expect(w.text()).toContain('Do you want to pick the blind?')
    expect(w.find('button.btn-pick').exists()).toBe(true)
    expect(w.find('button.btn-pass').exists()).toBe(true)
  })

  it('burying sub-phase shows bury UI for the picker', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 1, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ picker: 1, sub_phase: 'burying' }, 1) })
    const w = mount(BiddingPanel)
    expect(w.text()).toContain('Select')
    expect(w.text()).toContain('2 cards')
    expect(w.text()).not.toContain('Do you want to pick the blind?')
    expect(w.text()).not.toContain('Choose your partner card or go alone')
  })

  // BUG: before fix, isBuryPhase = (picker !== null && phase === 'bidding') stays true
  // after bury because it doesn't check sub_phase. The v-else-if for isBuryPhase fires
  // before isCallingPhase, so the calling UI never renders — picker is stuck.
  it('calling sub-phase shows partner-call UI, NOT bury UI', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 1, room_code: 'TEST01' })
    store.handleUpdate({
      type: 'snapshot',
      state: makeState({ picker: 1, sub_phase: 'calling', callable_suits: ['clubs'] }, 1),
    })
    const w = mount(BiddingPanel)
    // Bury UI must NOT be visible
    expect(w.text()).not.toContain('Select')
    // Calling UI must be visible
    expect(w.text()).toContain('Choose your partner card or go alone')
    expect(w.find('button.btn-alone').exists()).toBe(true)
  })

  it('waiting message shown to non-picker during burying sub-phase', () => {
    const store = useGameStore()
    store.handleUpdate({ type: 'joined_room', room_id: 'r', seat: 2, room_code: 'TEST01' })
    store.handleUpdate({ type: 'snapshot', state: makeState({ picker: 1, sub_phase: 'burying' }, 1) })
    const w = mount(BiddingPanel)
    expect(w.text()).toContain('Waiting')
    expect(w.find('button.btn-bury').exists()).toBe(false)
  })
})
