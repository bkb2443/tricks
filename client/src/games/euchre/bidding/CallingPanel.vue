<script setup lang="ts">
import { ref, computed } from 'vue'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'
import { useEuchreState } from '@/games/euchre/state'

const store = useGameStore()
const { playerName } = store
const { callSuit, euchrePass } = useGame()
const { turnedUpCard } = useEuchreState()

const state = computed(() => store.gameState!)
const seat  = computed(() => store.seat ?? 0)

const SUIT_LABELS: Record<string, string> = {
  clubs:    '♣ Clubs',
  spades:   '♠ Spades',
  hearts:   '♥ Hearts',
  diamonds: '♦ Diamonds',
}

const goAloneCalling = ref(false)

// Suits available for round 2 — all suits except the turned-up card's suit
const callableSuits = computed<string[]>(() => {
  const exclude = turnedUpCard.value?.suit
  return ['clubs', 'spades', 'hearts', 'diamonds'].filter(s => s !== exclude)
})

// "Stick the dealer" rule: dealer cannot pass in round 2 once 3 others have passed
const mustCall = computed<boolean>(() => {
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

<style scoped>
.call-prompt { display: flex; flex-direction: column; align-items: center; gap: 0.6rem; }
.call-prompt p { margin: 0; font-size: 1rem; }

.alone-toggle {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.9rem;
  cursor: pointer;
  color: #d1d5db;
}

.waiting-msg { color: #9ca3af; font-style: italic; margin: 0; }

.call-suits { display: flex; gap: 0.5rem; justify-content: center; flex-wrap: wrap; }
.btn-call { background: #0284c7; }
.btn-call:hover { background: #0369a1; }

.btn-pass { background: #6b7280; }
.btn-pass:hover { background: #4b5563; }

.must-call-notice {
  color: #fbbf24;
  font-size: 0.85rem;
  font-style: italic;
}
</style>
