<script setup lang="ts">
import { computed, watch, ref, onUnmounted } from 'vue'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'
import TrickDisplay from '@/components/TrickDisplay.vue'
import HandComponent from '@/components/HandComponent.vue'
import EuchreBiddingPanel from './BiddingPanel.vue'
import type { Card } from '@/engine/types'
import { phaseLabel } from '@/engine/phases'
import { sortHandEuchre } from '@/games/euchre/sort'
import { useEuchreState } from '@/games/euchre/state'

const store = useGameStore()
// Safe to call in template (reads reactive refs); do not cache the return value outside a template
const { playerName } = store
const { playCard, startNextHand } = useGame()
const { callerSeat, sitsOut, calledSuit } = useEuchreState()

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const state = computed(() => store.gameState!)
const seat  = computed(() => store.seat ?? 0)

const VICTORY_GOAL = 10

const SUIT_SYMBOLS: Record<string, string> = { clubs: '♣', spades: '♠', hearts: '♥', diamonds: '♦' }

const canPlay = computed(
  () => store.isMyTurn && state.value.phase === 'playing' && seat.value !== sitsOut.value,
)

// Build the player list in seat order, starting from the viewer's seat
const playerOrder = computed(() => {
  const count = state.value.player_count
  return Array.from({ length: count }, (_, i) => (seat.value + i) % count)
})

function handlePlay(card: Card) {
  if (canPlay.value) playCard(card)
}

// Score display: positive = win (green), negative = loss (red)
function scoreClass(s: number) {
  return s > 0 ? 'win' : s < 0 ? 'loss' : ''
}

// Team check: seats 0&2 vs 1&3
function isTeammate(p: number): boolean {
  return p % 2 === seat.value % 2
}

function teamLabel(p: number): string {
  return isTeammate(p) ? '(Partner)' : '(Opponent)'
}

const completedTricks = computed(() => state.value.completed_tricks.length)
const currentTrickCount = computed(() => state.value.current_trick ? 1 : 0)

const ORDINALS = ['1st', '2nd', '3rd', '4th', '5th']
function trickOrdinal(n: number) {
  return ORDINALS[n] ?? `${n + 1}th`
}

const phaseToast = ref<string | null>(null)
let toastTimer: ReturnType<typeof setTimeout> | null = null

watch(
  () => state.value?.phase,
  (newPhase, oldPhase) => {
    if (newPhase && oldPhase && newPhase !== oldPhase) {
      if (toastTimer !== null) clearTimeout(toastTimer)
      phaseToast.value = phaseLabel(state.value?.game_name ?? '', newPhase)
      toastTimer = setTimeout(() => {
        phaseToast.value = null
        toastTimer = null
      }, 1500)
    }
  },
)

onUnmounted(() => {
  if (toastTimer !== null) { clearTimeout(toastTimer); toastTimer = null }
})

const euchreHandSortFn = computed(() => (cards: Card[]) => sortHandEuchre(cards, calledSuit.value))

const nextDealer = computed(() => (state.value.dealer + 1) % state.value.player_count)
const isNextDealer = computed(() => seat.value === nextDealer.value)
</script>

<template>
  <div class="euchre-table">
    <!-- ── Phase change toast ──────────────────────────────────── -->
    <Transition name="toast">
      <div v-if="phaseToast" class="phase-toast">{{ phaseToast }}</div>
    </Transition>

    <!-- ── Header: phase indicator + dealer badge ──────────────── -->
    <header class="table-header">
      <span class="phase-badge" :class="state.phase">{{ phaseLabel(state.game_name, state.phase) }}</span>
      <span class="dealer-badge">Dealer: {{ playerName(state.dealer) }}</span>
      <span class="trick-counter">
        Trick {{ completedTricks + currentTrickCount }} / 5
      </span>
    </header>

    <!-- ── Seat rail (other players) ──────────────────────────── -->
    <div class="seats">
      <div
        v-for="p in playerOrder.slice(1)"
        :key="p"
        class="seat"
        :class="{ active: state.current_player === p }"
      >
        <span class="seat-label">
          {{ playerName(p) }}
          <span v-if="p === state.dealer" class="badge">D</span>
          <span v-if="p === callerSeat" class="role-badge caller">Caller</span>
          <span v-if="p === sitsOut" class="role-badge sits-out">Sits Out</span>
        </span>
        <span class="team-label" :class="{ partner: isTeammate(p), opponent: !isTeammate(p) }">
          {{ teamLabel(p) }}
        </span>
        <span class="card-count">{{ state.hands[p].length }} cards</span>
      </div>
    </div>

    <!-- ── Current trick ───────────────────────────────────────── -->
    <trick-display
      :trick="state.current_trick"
      :completed-trick="store.completedTrick"
      :my-seat="seat"
      :names="state.names ?? []"
      :picker-seat="callerSeat"
      :partner-seat="null"
      :current-winner-seat="store.currentTrickWinner"
    />

    <!-- ── Bidding panel (only during Bidding phase) ──────────── -->
    <euchre-bidding-panel v-if="state.phase === 'bidding'" />

    <!-- ── My hand ────────────────────────────────────────────── -->
    <section
      v-if="state.phase !== 'scoring' && state.phase !== 'intermission' && seat !== sitsOut"
      class="my-hand"
      :class="{ 'your-turn-glow': canPlay }"
    >
      <div class="my-hand-label">
        Your hand (seat {{ seat }})
        <span v-if="seat === callerSeat" class="badge caller-badge">Caller</span>
        <span v-if="seat === callerSeat && state.meta?.going_alone" class="badge alone-badge">Alone</span>
        <span v-if="seat === state.dealer" class="badge">Dealer</span>
        <span v-if="calledSuit" class="badge trump-badge">
          Trump: {{ SUIT_SYMBOLS[calledSuit] ?? calledSuit }}
        </span>
        <span v-if="canPlay" class="your-turn">↑ Your turn</span>
      </div>
      <hand-component
        :cards="store.myHand"
        :selectable="canPlay"
        :sort-fn="euchreHandSortFn"
        @select="handlePlay"
      />
    </section>

    <!-- ── "Sits out" notice ───────────────────────────────────── -->
    <section v-if="seat === sitsOut" class="sits-out-notice">
      <p>You are sitting out this hand (partner is going alone).</p>
    </section>

    <!-- ── Session scoreboard (visible once scores accumulate) ── -->
    <section v-if="store.sessionScores.length" class="session-scores">
      <h3>Session Scores <span class="goal-label">(first to {{ VICTORY_GOAL }})</span></h3>
      <ul class="score-list">
        <li
          v-for="(score, i) in store.sessionScores"
          :key="i"
          class="score-row"
          :class="scoreClass(score)"
        >
          <span>{{ i === seat ? 'You (' + playerName(i) + ')' : playerName(i) }}</span>
          <span class="score-value">{{ score > 0 ? '+' : '' }}{{ score }}</span>
          <span class="progress-bar-wrap">
            <span
              class="progress-bar"
              :style="{ width: Math.max(0, Math.min(100, (score / VICTORY_GOAL) * 100)) + '%' }"
            />
          </span>
        </li>
      </ul>
    </section>

    <!-- ── Hand result (shown during scoring phase) ──────────── -->
    <section v-if="state.phase === 'scoring' && !store.sessionWinner" class="game-over">
      <h2>Hand Complete</h2>
      <ul class="score-list">
        <li
          v-for="(score, i) in state.scores"
          :key="i"
          class="score-row"
          :class="scoreClass(score)"
        >
          <span>{{ i === seat ? 'You (' + playerName(i) + ')' : playerName(i) }}</span>
          <span class="score-value">{{ score > 0 ? '+' : '' }}{{ score }}</span>
        </li>
      </ul>
      <p class="next-hand-hint">Next hand starting…</p>
    </section>

    <!-- ── Intermission (between hands) ──────────────────────── -->
    <section v-if="state.phase === 'intermission' && !store.sessionWinner" class="game-over intermission-panel">
      <h2>Hand Complete</h2>
      <ul class="score-list">
        <li
          v-for="(score, i) in state.scores"
          :key="i"
          class="score-row"
          :class="scoreClass(score)"
        >
          <span>{{ i === seat ? 'You (' + playerName(i) + ')' : playerName(i) }}</span>
          <span class="score-value">{{ score > 0 ? '+' : '' }}{{ score }}</span>
        </li>
      </ul>
      <div class="next-hand-controls">
        <button v-if="isNextDealer" class="deal-button" @click="startNextHand">Deal Next Hand</button>
        <p v-else class="next-hand-hint">Waiting for {{ playerName(nextDealer) }} to deal…</p>
      </div>
    </section>

    <!-- ── Catch-up panel (shown to rejoining player) ────────── -->
    <div v-if="store.showCatchUp" class="catchup-overlay">
      <div class="catchup-panel">
        <h3>You've rejoined</h3>
        <p v-if="calledSuit">Trump: {{ SUIT_SYMBOLS[calledSuit] ?? calledSuit }}</p>
        <p v-if="callerSeat !== null">Caller: {{ playerName(callerSeat) }}</p>
        <div class="catchup-scores">
          <h4>Current hand scores</h4>
          <ul class="score-list">
            <li v-for="(s, i) in state.scores" :key="i" class="score-row" :class="scoreClass(s)">
              <span>{{ i === seat ? 'You' : playerName(i) }}</span>
              <span class="score-value">{{ s > 0 ? '+' : '' }}{{ s }}</span>
            </li>
          </ul>
        </div>
        <button @click="store.dismissCatchUp()">Continue</button>
      </div>
    </div>

    <!-- ── Session over ──────────────────────────────────────── -->
    <section v-if="store.sessionWinner !== null" class="game-over session-over">
      <h2>{{ store.sessionWinner === seat ? '🏆 You Win!' : playerName(store.sessionWinner!) + ' Wins!' }}</h2>
      <ul class="score-list">
        <li
          v-for="(score, i) in store.sessionScores"
          :key="i"
          :class="['score-row', scoreClass(score), { winner: i === store.sessionWinner }]"
        >
          <span>{{ i === seat ? 'You (' + playerName(i) + ')' : playerName(i) }}</span>
          <span class="score-value">{{ score > 0 ? '+' : '' }}{{ score }}</span>
        </li>
      </ul>
      <router-link to="/"><button>Back to Lobby</button></router-link>
    </section>

    <!-- ── Completed tricks summary ───────────────────────────── -->
    <details v-if="state.completed_tricks.length" class="history">
      <summary>Completed tricks ({{ state.completed_tricks.length }})</summary>
      <ol>
        <li v-for="(trick, i) in state.completed_tricks" :key="i">
          {{ trickOrdinal(i) }} trick — won by P{{ trick.winner }}
          ({{ trick.plays.map(([, c]) => `${c.rank[0].toUpperCase()}${c.suit[0]}`).join(' ') }})
        </li>
      </ol>
    </details>
  </div>
</template>

<style scoped>
.euchre-table {
  max-width: 700px;
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

/* Header */
.table-header {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  flex-wrap: wrap;
}
.phase-badge {
  padding: 0.2rem 0.6rem;
  border-radius: 999px;
  font-size: 0.75rem;
  text-transform: uppercase;
  font-weight: 700;
  background: #4b5563;
}
.phase-badge.bidding { background: #7c3aed; }
.phase-badge.playing { background: #15803d; }
.phase-badge.scoring { background: #b45309; }
.phase-badge.intermission { background: #1d4ed8; }
.dealer-badge, .trick-counter { font-size: 0.8rem; color: #9ca3af; }

/* Seat rail */
.seats {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}
.seat {
  background: rgba(0,0,0,0.2);
  border: 1px solid transparent;
  border-radius: 6px;
  padding: 0.35rem 0.6rem;
  font-size: 0.8rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  min-width: 70px;
}
.seat.active { border-color: #22c55e; }
.seat-label { display: flex; align-items: center; gap: 4px; font-weight: 600; }
.card-count { color: #9ca3af; }
.team-label { font-size: 0.7rem; }
.team-label.partner  { color: #34d399; }
.team-label.opponent { color: #f87171; }

.badge {
  font-size: 0.65rem;
  background: #4b5563;
  padding: 0 4px;
  border-radius: 3px;
}
.role-badge {
  font-size: 0.6rem;
  padding: 1px 5px;
  border-radius: 999px;
  font-weight: 600;
  letter-spacing: 0.02em;
}
.role-badge.caller   { background: #b45309; color: #fff; }
.role-badge.sits-out { background: #4b5563; color: #d1d5db; }

/* My hand */
.my-hand {
  background: rgba(0,0,0,0.2);
  border-radius: 8px;
  padding: 0.75rem;
}
.my-hand-label {
  font-size: 0.8rem;
  color: #9ca3af;
  margin-bottom: 0.5rem;
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
.your-turn { color: #22c55e; font-weight: 600; }

.caller-badge { background: #b45309; color: #fff; font-size: 0.65rem; padding: 0 5px; border-radius: 3px; }
.alone-badge  { background: #9333ea; color: #fff; font-size: 0.65rem; padding: 0 5px; border-radius: 3px; }
.trump-badge  { background: #0284c7; color: #fff; font-size: 0.65rem; padding: 0 5px; border-radius: 3px; }

/* Sits-out notice */
.sits-out-notice {
  background: rgba(0,0,0,0.2);
  border-radius: 8px;
  padding: 0.75rem 1rem;
  text-align: center;
  color: #9ca3af;
  font-style: italic;
}
.sits-out-notice p { margin: 0; }

/* Session scoreboard */
.session-scores {
  background: rgba(0,0,0,0.25);
  border-radius: 8px;
  padding: 0.75rem 1rem;
}
.session-scores h3 { margin: 0 0 0.5rem; font-size: 0.9rem; }
.goal-label { color: #9ca3af; font-weight: 400; font-size: 0.8rem; }
.progress-bar-wrap {
  flex: 1;
  margin-left: 0.75rem;
  background: rgba(255,255,255,0.08);
  border-radius: 999px;
  height: 6px;
  overflow: hidden;
  align-self: center;
}
.progress-bar {
  display: block;
  height: 100%;
  background: #6366f1;
  border-radius: 999px;
  transition: width 0.4s ease;
}

/* Hand result / session over */
.game-over {
  background: rgba(0,0,0,0.3);
  border-radius: 8px;
  padding: 1.25rem;
  text-align: center;
}
.game-over h2 { margin: 0 0 1rem; }
.next-hand-hint { font-size: 0.8rem; color: #9ca3af; margin: 0; }
.session-over { border: 1px solid rgba(99, 102, 241, 0.5); }
.score-list { list-style: none; padding: 0; margin: 0 0 1rem; }
.score-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.25rem 0;
  border-bottom: 1px solid rgba(255,255,255,0.08);
}
.score-value { font-weight: 700; margin-left: auto; }
.win  .score-value { color: #4ade80; }
.loss .score-value { color: #f87171; }
.score-row.winner { background: rgba(99, 102, 241, 0.15); border-radius: 4px; padding: 0.25rem 0.4rem; }

/* History */
.history {
  font-size: 0.8rem;
  color: #9ca3af;
  background: rgba(0,0,0,0.2);
  border-radius: 6px;
  padding: 0.5rem 0.75rem;
}
.history summary { cursor: pointer; }
.history ol { margin: 0.5rem 0 0; padding-left: 1.2rem; }
.history li { margin-bottom: 0.2rem; }

/* Your-turn glow pulse */
@keyframes your-turn-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(34, 197, 94, 0); }
  50%       { box-shadow: 0 0 0 8px rgba(34, 197, 94, 0.35); }
}
.your-turn-glow {
  outline: 2px solid rgba(34, 197, 94, 0.7);
  outline-offset: 2px;
  animation: your-turn-pulse 1.2s ease-in-out infinite;
}

/* Phase change toast */
.toast-enter-active, .toast-leave-active { transition: opacity 0.2s ease; }
.toast-enter-from, .toast-leave-to { opacity: 0; }
.phase-toast {
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  background: rgba(0, 0, 0, 0.78);
  color: #fff;
  font-size: 2rem;
  font-weight: 700;
  padding: 1rem 2.5rem;
  border-radius: 12px;
  pointer-events: none;
  z-index: 100;
  letter-spacing: 0.04em;
}

/* Intermission panel */
.intermission-panel { border: 1px solid rgba(99, 102, 241, 0.3); }
.next-hand-controls { margin-top: 1rem; }
.deal-button {
  padding: 0.6rem 1.6rem;
  background: #6366f1;
  color: #fff;
  border: none;
  border-radius: 6px;
  font-size: 1rem;
  font-weight: 600;
  cursor: pointer;
}
.deal-button:hover { background: #4f46e5; }

/* Catch-up overlay */
.catchup-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}
.catchup-panel {
  background: #1f2937;
  border-radius: 12px;
  padding: 1.5rem 2rem;
  max-width: 360px;
  width: 90%;
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.catchup-panel h3 { margin: 0; font-size: 1.2rem; }
.catchup-panel h4 { margin: 0; font-size: 0.9rem; color: #9ca3af; }
.catchup-panel p { margin: 0; font-size: 0.9rem; color: #d1d5db; }
.catchup-scores { text-align: left; }
.catchup-panel button {
  padding: 0.5rem 1.5rem;
  background: #6366f1;
  color: #fff;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.95rem;
  align-self: center;
}
.catchup-panel button:hover { background: #4f46e5; }
</style>
