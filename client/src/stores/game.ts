import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Card, GamePhase, GameState, SeatInfo, StateUpdate, Trick } from '@/engine/types'

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
  const currentTrickWinner = ref<number>(-1)
  const showCatchUp = ref<boolean>(false)
  const partnerRevealedSeat = ref<number | null>(null)
  const seats          = ref<SeatInfo[]>([])
  const lobbyChat      = ref<Array<{ from: string; text: string; timestamp: number }>>([])
  const queueStatus    = ref<{ position: number; waiting_since: number } | null>(null)
  const roomCode       = ref<string | null>(null)
  const isSpectator    = ref<boolean>(false)
  const spectatorCount = ref<number>(0)
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

  const isLobby = computed(() => gameState.value?.phase === 'lobby')


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
        roomId.value      = update.room_id
        seat.value        = update.seat
        roomCode.value    = update.room_code || null
        isSpectator.value = false
        break

      case 'joined_as_spectator':
        roomId.value      = update.room_id
        seat.value        = null
        isSpectator.value = true
        roomCode.value    = update.room_code || null
        break

      case 'snapshot':
        // Detect mid-game rejoin: already have state, new snapshot arrives during active play
        if (gameState.value !== null && gameState.value.phase !== 'lobby' &&
            update.state.phase !== 'lobby' && update.state.phase !== 'intermission') {
          showCatchUp.value = true
        }
        gameState.value = update.state
        // The snapshot only populates our own hand slot; sync myHand from it.
        // Spectators have no seat, so their hand is always empty.
        myHand.value = seat.value !== null ? (update.state.hands[seat.value] ?? []) : []
        currentTrickWinner.value = -1
        // Sync session scores from snapshot (server now includes them)
        if (update.state.session_scores?.length) {
          sessionScores.value = update.state.session_scores
        }
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
        // The server sends the authoritative next player — no client-side turn arithmetic needed.
        s.current_player = update.next_player
        // Remove the card from the local hand if we played it.
        if (update.player === seat.value) {
          const idx = myHand.value.findIndex(
            c => c.suit === update.card.suit && c.rank === update.card.rank,
          )
          if (idx !== -1) myHand.value.splice(idx, 1)
        }
        // Track the currently winning seat from the server-authoritative value.
        currentTrickWinner.value = update.current_trick_winner ?? -1
        break
      }

      case 'trick_complete': {
        if (!gameState.value?.current_trick) break
        const t = gameState.value.current_trick
        t.winner = update.winner
        gameState.value.completed_tricks.push(t)
        gameState.value.current_trick  = null
        gameState.value.current_player = update.winner
        currentTrickWinner.value = -1
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
        if (gameState.value) {
          gameState.value.current_player = update.current_player
          // Merge any game-metadata included in the bid payload (e.g. sub_phase, callable_suits
          // when transitioning to the calling sub-phase after bury).
          if (typeof update.value === 'object' && update.value !== null) {
            gameState.value.meta = {
              ...gameState.value.meta,
              ...(update.value as Record<string, unknown>),
            }
          }
        }
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

      case 'seat_update':
        seats.value = update.seats
        spectatorCount.value = update.spectator_count ?? 0
        break

      case 'lobby_chat':
        lobbyChat.value = [...lobbyChat.value, {
          from: update.from,
          text: update.text,
          timestamp: update.timestamp,
        }].slice(-50)
        break

      case 'queue_status':
        queueStatus.value = { position: update.position, waiting_since: update.waiting_since }
        break

      case 'error':
        error.value = update.message
        break
    }
  }

  function dismissCatchUp(): void {
    showCatchUp.value = false
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
    currentTrickWinner.value = -1
    partnerRevealedSeat.value = null
    seats.value          = []
    lobbyChat.value      = []
    queueStatus.value    = null
    roomCode.value       = null
    showCatchUp.value    = false
    isSpectator.value    = false
    spectatorCount.value = 0
    if (pauseTimer !== null) { clearTimeout(pauseTimer); pauseTimer = null }
  }

  return {
    // state
    roomId, seat, gameState, myHand, error, isSolo, sessionScores, sessionWinner, completedTrick,
    partnerRevealedSeat, seats, lobbyChat, queueStatus, roomCode, showCatchUp,
    isSpectator, spectatorCount,
    // derived
    phase, isMyTurn, picker, isPicker, gameStarted, currentTrickWinner, playerName, isLobby,
    // actions
    handleUpdate, reset, dismissCatchUp,
  }
})
