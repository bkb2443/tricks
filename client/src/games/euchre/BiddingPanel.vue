<script setup lang="ts">
import { ref, computed } from 'vue'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'
import HandComponent from '@/components/HandComponent.vue'
import type { Card } from '@/engine/types'
import { sortHandEuchre } from '@/engine/sort'

const store = useGameStore()
// Safe to call in template (reads reactive refs); do not cache the return value outside a template
const { playerName } = store
const { orderUp, euchrePass, discard, callSuit } = useGame()

const state = computed(() => store.gameState!)
const seat  = computed(() => store.seat ?? 0)

const subPhase    = computed(() => store.euchreSubPhase)
const turnedUpCard = computed(() => store.euchreTurnedUpCard)
const calledSuit  = computed(() => store.euchreCalledSuit)

const RANK_LABEL: Record<string, string> = {
  ace: 'A', two: '2', three: '3', four: '4', five: '5', six: '6',
  seven: '7', eight: '8', nine: '9', ten: '10', jack: 'J', queen: 'Q', king: 'K',
}

const SUIT_SYMBOLS: Record<string, string> = { clubs: '♣', spades: '♠', hearts: '♥', diamonds: '♦' }

const SUIT_LABELS: Record<string, string> = {
  clubs:    '♣ Clubs',
  spades:   '♠ Spades',
  hearts:   '♥ Hearts',
  diamonds: '♦ Diamonds',
}

// ── "Ordering" sub-phase ──────────────────────────────────────────────────

const goAloneOrdering = ref(false)

function handleOrderUp() {
  orderUp(goAloneOrdering.value)
  goAloneOrdering.value = false
}

function handlePassOrdering() {
  euchrePass()
}

// ── "Discarding" sub-phase ────────────────────────────────────────────────

const discardSelection = ref<Card[]>([])

function toggleDiscard(card: Card) {
  const idx = discardSelection.value.findIndex(
    (c) => c.suit === card.suit && c.rank === card.rank,
  )
  if (idx >= 0) {
    discardSelection.value.splice(idx, 1)
  } else {
    // Only one card at a time
    discardSelection.value = [card]
  }
}

function submitDiscard() {
  if (discardSelection.value.length !== 1) return
  discard(discardSelection.value[0])
  discardSelection.value = []
}

const discardSortFn = computed(() => (cards: Card[]) => sortHandEuchre(cards, calledSuit.value))

// ── "Calling" sub-phase ───────────────────────────────────────────────────

const goAloneCalling = ref(false)

// Suits available for round 2 — all suits except the turned-up card's suit
const callableSuits = computed<string[]>(() => {
  const exclude = turnedUpCard.value?.suit
  return ['clubs', 'spades', 'hearts', 'diamonds'].filter(s => s !== exclude)
})

// "Stick the dealer" rule: dealer cannot pass in round 2 once 3 others have passed
const mustCall = computed<boolean>(() => {
  if (subPhase.value !== 'calling') return false
  if (seat.value !== state.value.dealer) return false
  const passed2 = state.value.meta?.passed_round2
  return typeof passed2 === 'number' && passed2 >= 3
})

function handleCallSuit(suit: string) {
  callSuit(suit, goAloneCalling.value)
  goAloneCalling.value = false
}

function handlePassCalling() {
  euchrePass()
}
</script>

<template>
  <div class="bidding-panel" :class="{ 'your-turn-glow': store.isMyTurn }">

    <!-- ── "ordering" sub-phase: round 1 ─────────────────────────────── -->
    <template v-if="subPhase === 'ordering'">
      <div class="turned-up" v-if="turnedUpCard">
        <span class="turned-label">Turn:</span>
        <span class="turned-card" :class="{ red: turnedUpCard.suit === 'hearts' || turnedUpCard.suit === 'diamonds' }">
          {{ RANK_LABEL[turnedUpCard.rank] ?? turnedUpCard.rank }}{{ SUIT_SYMBOLS[turnedUpCard.suit] ?? turnedUpCard.suit }}
        </span>
      </div>

      <div v-if="store.isMyTurn" class="order-prompt">
        <label class="alone-toggle">
          <input v-model="goAloneOrdering" type="checkbox" />
          Go Alone
        </label>
        <div class="actions">
          <button class="btn-order" @click="handleOrderUp">Order Up</button>
          <button class="btn-pass" @click="handlePassOrdering">Pass</button>
        </div>
      </div>
      <p v-else class="waiting-msg">
        Waiting for {{ playerName(state.current_player) }} to order or pass…
      </p>
    </template>

    <!-- ── "discarding" sub-phase: dealer discards one ───────────────── -->
    <template v-else-if="subPhase === 'discarding'">
      <div v-if="seat === state.dealer" class="discard-prompt">
        <p>
          Pick a card to discard
          <span class="count">({{ discardSelection.length }}/1 selected)</span>
        </p>
        <hand-component
          :cards="store.myHand"
          :selectable="true"
          :selected-cards="discardSelection"
          :sort-fn="discardSortFn"
          @select="toggleDiscard"
        />
        <button
          class="btn-discard"
          :disabled="discardSelection.length !== 1"
          @click="submitDiscard"
        >
          Discard selected card
        </button>
      </div>
      <p v-else class="waiting-msg">
        Waiting for {{ playerName(state.dealer) }} to discard…
      </p>
    </template>

    <!-- ── "calling" sub-phase: round 2 name a suit ──────────────────── -->
    <template v-else-if="subPhase === 'calling'">
      <div v-if="store.isMyTurn" class="call-prompt">
        <p>Choose a trump suit to call</p>
        <label class="alone-toggle">
          <input v-model="goAloneCalling" type="checkbox" />
          Go Alone
        </label>
        <div class="call-suits">
          <button
            v-for="suit in callableSuits"
            :key="suit"
            class="btn-call"
            @click="handleCallSuit(suit)"
          >
            {{ SUIT_LABELS[suit] ?? suit }}
          </button>
        </div>
        <div v-if="mustCall" class="must-call-notice">
          Must call — you are the dealer (stick the dealer)
        </div>
        <button
          v-else
          class="btn-pass"
          @click="handlePassCalling"
        >
          Pass
        </button>
      </div>
      <p v-else class="waiting-msg">
        Waiting for {{ playerName(state.current_player) }} to call…
      </p>
    </template>

    <!-- ── "done" sub-phase (bidding complete, playing starts) ───────── -->
    <template v-else-if="subPhase === 'done'">
      <p class="waiting-msg">Bidding complete — play begins…</p>
    </template>

  </div>
</template>

<style scoped>
.bidding-panel {
  background: rgba(0,0,0,0.25);
  border-radius: 8px;
  padding: 1rem 1.25rem;
  text-align: center;
}

.turned-up {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
  font-size: 1.1rem;
}
.turned-label { color: #9ca3af; font-size: 0.85rem; }
.turned-card { font-weight: 700; font-size: 1.4rem; }
.turned-card.red { color: #dc2626; }

.order-prompt, .discard-prompt, .call-prompt { display: flex; flex-direction: column; align-items: center; gap: 0.6rem; }
.order-prompt p, .discard-prompt p, .call-prompt p { margin: 0; font-size: 1rem; }

.alone-toggle {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.9rem;
  cursor: pointer;
  color: #d1d5db;
}

.count { color: #9ca3af; font-size: 0.85rem; }
.waiting-msg { color: #9ca3af; font-style: italic; margin: 0; }

.actions { display: flex; gap: 0.75rem; justify-content: center; }

.btn-order { background: #15803d; }
.btn-order:hover { background: #166534; }

.btn-pass { background: #6b7280; }
.btn-pass:hover { background: #4b5563; }

.btn-discard { margin-top: 0.5rem; background: #7c3aed; }
.btn-discard:hover:not(:disabled) { background: #6d28d9; }

.call-suits { display: flex; gap: 0.5rem; justify-content: center; flex-wrap: wrap; }
.btn-call { background: #0284c7; }
.btn-call:hover { background: #0369a1; }

.must-call-notice {
  color: #fbbf24;
  font-size: 0.85rem;
  font-style: italic;
}

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
