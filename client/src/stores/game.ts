import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Card, GamePhase, GameState, StateUpdate, Trick } from '@/engine/types'
import { trickWinnerIndex } from '@/engine/sort'

export const useGameStore = defineStore('game', () => {
  // ── State ─────────────────────────────────────────────────────────────────
  const roomId        = ref<string | null>(null)
  const seat          = ref<number | null>(null)
  const gameState     = ref<GameState | null>(null)
  const myHand        = ref<Card[]>([])
  const error         = ref<string | null>(null)
  const isSolo        = ref<boolean>(false)
  const sessionScores = ref<number[]>([])
  const sessionWinner = ref<number | null>(null)
  const completedTrick = ref<Trick | null>(null)
  const partnerRevealedSeat = ref<number | null>(null)
  let pauseTimer: ReturnType<typeof setTimeout> | null = null

  // ── Derived ───────────────────────────────────────────────────────────────
  const phase = computed<GamePhase | null>(() => gameState.value?.phase ?? null)

  const isMyTurn = computed(
    () => seat.value !== null && gameState.value?.current_player === seat.value,
  )

  /** Seat of the Sheepshead picker, or null if nobody has picked yet. */
  const picker = computed<number | null>(() => {
    const p = gameState.value?.meta?.picker
    return typeof p === 'number' ? p : null
  })

  const isPicker = computed(
    () => picker.value !== null && picker.value === seat.value,
  )

  /** True once all seats are filled and the server has sent the first Snapshot. */
  const gameStarted = computed(() => gameState.value !== null)

  const isCallingPhase = computed(() =>
    gameState.value?.meta?.sub_phase === 'calling' && gameState.value?.phase === 'bidding'
  )

  const callableSuits = computed<string[]>(() => {
    const cs = gameState.value?.meta?.callable_suits
    return Array.isArray(cs) ? (cs as string[]) : []
  })

  const calledSuit = computed<string | null>(() => {
    const cs = gameState.value?.meta?.called_suit
    return typeof cs === 'string' ? cs : null
  })

  /** Index within current_trick.plays of the currently winning card, or -1 if no trick in progress. */
  const currentTrickWinner = computed<number>(() => {
    const trick = gameState.value?.current_trick
    if (!trick || trick.plays.length === 0) return -1
    return trickWinnerIndex(trick)
  })

  /** Returns "You" for the local player's seat, the server-assigned name otherwise,
   *  falling back to "P{seat}" if names haven't loaded yet. */
  function playerName(s: number): string {
    if (s === seat.value) return 'You'
    return gameState.value?.names?.[s] || `P${s}`
  }

  // ── Update handler ────────────────────────────────────────────────────────

  function handleUpdate(update: StateUpdate): void {
    error.value = null

    switch (update.type) {
      case 'joined_room':
        roomId.value = update.room_id
        seat.value   = update.seat
        break

      case 'snapshot':
        gameState.value = update.state
        // The snapshot only populates our own hand slot; sync myHand from it.
        myHand.value = update.state.hands[seat.value!] ?? []
        break

      case 'hand_updated':
        // Private message: server updated our hand (picker took blind / buried).
        myHand.value = update.hand
        break

      case 'card_played': {
        if (!gameState.value) break
        const s = gameState.value
        if (s.current_trick) {
          s.current_trick.plays.push([update.player, update.card])
        } else {
          // New trick starting — cancel any pending completion display
          if (pauseTimer !== null) {
            clearTimeout(pauseTimer)
            pauseTimer = null
          }
          completedTrick.value = null
          s.current_trick = { led_by: update.player, plays: [[update.player, update.card]], winner: null }
        }
        // Advance current_player to the next in trick order. The server does this
        // internally but only sends CardPlayed, not the new current_player.
        const trick = s.current_trick!
        if (trick.plays.length < s.player_count) {
          s.current_player = (trick.led_by + trick.plays.length) % s.player_count
        }
        // Remove the card from the local hand if we played it.
        if (update.player === seat.value) {
          const idx = myHand.value.findIndex(
            c => c.suit === update.card.suit && c.rank === update.card.rank,
          )
          if (idx !== -1) myHand.value.splice(idx, 1)
        }
        break
      }

      case 'trick_complete': {
        if (!gameState.value?.current_trick) break
        const t = gameState.value.current_trick
        t.winner = update.winner
        gameState.value.completed_tricks.push(t)
        gameState.value.current_trick  = null
        gameState.value.current_player = update.winner
        // Hold the completed trick visible for 1.5s
        completedTrick.value = { ...t }
        if (pauseTimer !== null) clearTimeout(pauseTimer)
        pauseTimer = setTimeout(() => {
          completedTrick.value = null
          pauseTimer = null
        }, 1500)
        break
      }

      case 'phase_changed':
        if (gameState.value) gameState.value.phase = update.phase
        break

      case 'bid_placed':
        if (gameState.value) gameState.value.current_player = update.current_player
        break

      case 'hand_complete':
        if (gameState.value) gameState.value.scores = update.hand_scores
        sessionScores.value = update.session_scores
        break

      case 'session_over':
        sessionScores.value = update.final_scores
        sessionWinner.value = update.winner
        break

      case 'partner_revealed':
        if (gameState.value) {
          gameState.value.meta = { ...gameState.value.meta, partner: update.seat }
        }
        partnerRevealedSeat.value = update.seat
        setTimeout(() => { partnerRevealedSeat.value = null }, 2000)
        break

      case 'error':
        error.value = update.message
        break
    }
  }

  function reset(): void {
    roomId.value        = null
    seat.value          = null
    gameState.value     = null
    myHand.value        = []
    error.value         = null
    isSolo.value        = false
    sessionScores.value = []
    sessionWinner.value = null
    completedTrick.value = null
    partnerRevealedSeat.value = null
    if (pauseTimer !== null) { clearTimeout(pauseTimer); pauseTimer = null }
  }

  return {
    // state
    roomId, seat, gameState, myHand, error, isSolo, sessionScores, sessionWinner, completedTrick,
    partnerRevealedSeat,
    // derived
    phase, isMyTurn, picker, isPicker, gameStarted, currentTrickWinner, playerName,
    isCallingPhase, callableSuits, calledSuit,
    // actions
    handleUpdate, reset,
  }
})
