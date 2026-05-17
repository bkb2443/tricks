import { computed } from 'vue'
import { useGameStore } from '@/stores/game'
import type { Card, Suit } from '@/engine/types'

export function useEuchreState() {
  const store = useGameStore()

  const callerSeat = computed<number | null>(() => {
    if (store.gameState?.game_name !== 'euchre') return null
    const c = store.gameState?.meta?.caller_seat
    return typeof c === 'number' ? c : null
  })

  const sitsOut = computed<number | null>(() => {
    if (store.gameState?.game_name !== 'euchre') return null
    const so = store.gameState?.meta?.sits_out
    return typeof so === 'number' ? so : null
  })

  const calledSuit = computed<Suit | null>(() => {
    if (store.gameState?.game_name !== 'euchre') return null
    const cs = store.gameState?.meta?.called_suit
    return typeof cs === 'string' ? cs as Suit : null
  })

  const turnedUpCard = computed<Card | null>(() => {
    if (store.gameState?.game_name !== 'euchre') return null
    const tc = store.gameState?.meta?.turned_up_card
    return (tc && typeof tc === 'object' && 'suit' in (tc as object)) ? tc as Card : null
  })

  const subPhase = computed<string | null>(() => {
    if (store.gameState?.game_name !== 'euchre') return null
    const sp = store.gameState?.meta?.sub_phase
    return typeof sp === 'string' ? sp : null
  })

  return { callerSeat, sitsOut, calledSuit, turnedUpCard, subPhase }
}
