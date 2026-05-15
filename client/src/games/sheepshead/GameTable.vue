<script setup lang="ts">
import { computed } from 'vue'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'
import TrickDisplay from '@/components/TrickDisplay.vue'
import HandComponent from '@/components/HandComponent.vue'
import BiddingPanel from './BiddingPanel.vue'
import type { Card } from '@/engine/types'

const store = useGameStore()
// Safe to call in template (reads reactive refs); do not cache the return value outside a template
const { playerName } = store
const { playCard } = useGame()

// eslint-disable-next-line @typescript-eslint/no-non-null-assertion
const state = computed(() => store.gameState!)
const seat  = computed(() => store.seat ?? 0)

const canPlay = computed(
  () => store.isMyTurn && state.value.phase === 'playing',
)

// Build the player list in seat order, starting from the viewer's seat
const playerOrder = computed(() => {
  const count = state.value.player_count
  return Array.from({ length: count }, (_, i) => (seat.value + i) % count)
})

function handlePlay(card: Card) {
  if (canPlay.value) playCard(card)
}

const VICTORY_GOAL = 24

// Score display: positive = win (green), negative = loss (red)
function scoreClass(s: number) {
  return s > 0 ? 'win' : s < 0 ? 'loss' : ''
}

const partnerSeat = computed<number | null>(() => {
  const p = state.value.meta?.partner
  return typeof p === 'number' ? p : null
})

const ORDINALS = ['1st', '2nd', '3rd', '4th', '5th']
function trickOrdinal(n: number) {
  return ORDINALS[n] ?? `${n + 1}th`
}
</script>

<template>
  <div class="sheepshead-table">
    <!-- ── Header: phase indicator + dealer badge ──────────────── -->
    <header class="table-header">
      <span class="phase-badge" :class="state.phase">{{ state.phase }}</span>
      <span class="dealer-badge">Dealer: {{ playerName(state.dealer) }}</span>
      <span class="trick-counter">
        Trick {{ state.completed_tricks.length + (state.current_trick ? 1 : 0) }} / 6
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
          <span v-if="p === store.picker" class="role-badge picker">Picker</span>
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
      :picker-seat="store.picker"
      :partner-seat="partnerSeat"
    />

    <!-- ── Bidding panel (only during Bidding phase) ──────────── -->
    <bidding-panel v-if="state.phase === 'bidding'" />

    <!-- ── My hand ────────────────────────────────────────────── -->
    <section v-if="state.phase !== 'scoring'" class="my-hand" :class="{ 'your-turn-glow': canPlay }">
      <div class="my-hand-label">
        Your hand (seat {{ seat }})
        <span v-if="store.isPicker" class="badge picker">Picker</span>
        <span v-if="seat === state.dealer" class="badge">Dealer</span>
        <span v-if="canPlay" class="your-turn">↑ Your turn</span>
      </div>
      <hand-component
        :cards="store.myHand"
        :selectable="canPlay"
        @select="handlePlay"
      />
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
.sheepshead-table {
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
  min-width: 60px;
}
.seat.active { border-color: #22c55e; }
.seat-label { display: flex; align-items: center; gap: 4px; font-weight: 600; }
.card-count { color: #9ca3af; }

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
.role-badge.picker { background: #7c3aed; color: #fff; }

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
</style>
