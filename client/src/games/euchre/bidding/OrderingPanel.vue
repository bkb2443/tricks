<script setup lang="ts">
import { ref, computed } from 'vue'
import { useGameStore } from '@/stores/game'
import { useEuchreBidding } from '@/games/euchre/useEuchreBidding'
import { useEuchreState } from '@/games/euchre/state'

const store = useGameStore()
const { playerName } = store
const { orderUp, euchrePass } = useEuchreBidding()
const { turnedUpCard } = useEuchreState()

const state = computed(() => store.gameState!)

const RANK_LABEL: Record<string, string> = {
  ace: 'A', two: '2', three: '3', four: '4', five: '5', six: '6',
  seven: '7', eight: '8', nine: '9', ten: '10', jack: 'J', queen: 'Q', king: 'K',
}

const SUIT_SYMBOLS: Record<string, string> = { clubs: '♣', spades: '♠', hearts: '♥', diamonds: '♦' }

const goAloneOrdering = ref(false)

function handleOrderUp() {
  orderUp(goAloneOrdering.value)
  goAloneOrdering.value = false
}

function handlePassOrdering() {
  euchrePass()
}
</script>

<template>
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

<style scoped>
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

.order-prompt { display: flex; flex-direction: column; align-items: center; gap: 0.6rem; }
.order-prompt p { margin: 0; font-size: 1rem; }

.alone-toggle {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.9rem;
  cursor: pointer;
  color: #d1d5db;
}

.waiting-msg { color: #9ca3af; font-style: italic; margin: 0; }

.actions { display: flex; gap: 0.75rem; justify-content: center; }

.btn-order { background: #15803d; }
.btn-order:hover { background: #166534; }

.btn-pass { background: #6b7280; }
.btn-pass:hover { background: #4b5563; }
</style>
