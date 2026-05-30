import { computed } from 'vue'
import { useGameStore } from '@/stores/game'
import type { Card, Suit } from '@/engine/types'

export function useEuchreState() {
  const store = useGameStore()

  const em = computed(() => {
    const m = store.gameState?.meta
    return m?.kind === 'euchre' ? m : null
  })

  const callerSeat = computed<number | null>(() => em.value?.caller_seat ?? null)

  const sitsOut = computed<number | null>(() => em.value?.sits_out ?? null)

  const calledSuit = computed<Suit | null>(() => (em.value?.called_suit ?? null) as Suit | null)

  const turnedUpCard = computed<Card | null>(() => em.value?.turned_up_card ?? null)

  const subPhase = computed<string | null>(() => em.value?.sub_phase ?? null)

  return { callerSeat, sitsOut, calledSuit, turnedUpCard, subPhase }
}
