<script setup lang="ts">
import { ref, computed } from 'vue'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'
import HandComponent from '@/components/HandComponent.vue'
import type { Card } from '@/engine/types'

const store = useGameStore()
// Safe to call in template (reads reactive refs); do not cache the return value outside a template
const { playerName } = store
const { pick, pass, bury, callAce, goAlone } = useGame()

// Two sub-phases: picking (anyone can pick/pass) and burying (picker discards 2)
const sm = computed(() => {
  const m = store.gameState?.meta
  return m?.kind === 'sheepshead' ? m : null
})

const isPickingPhase = computed(() => store.picker === null)
const isBuryPhase    = computed(() =>
  store.picker !== null &&
  store.phase === 'bidding' &&
  sm.value?.sub_phase === 'burying'
)

const isMyPickTurn = computed(() => isPickingPhase.value && store.isMyTurn)
const isMyBuryTurn = computed(() => isBuryPhase.value && store.isPicker)

const isCallingPhase = computed(() =>
  sm.value?.sub_phase === 'calling' && store.phase === 'bidding'
)
const callableSuits = computed<string[]>(() => {
  const cs = sm.value?.callable_suits
  return Array.isArray(cs) ? cs : []
})
const isMyCallTurn = computed(() => isCallingPhase.value && store.isPicker)

const SUIT_LABELS: Record<string, string> = {
  clubs:  '♣ Clubs',
  spades: '♠ Spades',
  hearts: '♥ Hearts',
}

// Bury selection — at most 2 cards
const burySelection = ref<Card[]>([])

function toggleCard(card: Card) {
  const idx = burySelection.value.findIndex(
    (c) => c.suit === card.suit && c.rank === card.rank,
  )
  if (idx >= 0) {
    burySelection.value.splice(idx, 1)
  } else if (burySelection.value.length < 2) {
    burySelection.value.push(card)
  }
}

function submitBury() {
  if (burySelection.value.length !== 2) return
  bury(burySelection.value as [Card, Card])
  burySelection.value = []
}
</script>

<template>
  <div class="bidding-panel" :class="{ 'your-turn-glow': store.isMyTurn }">
    <!-- ── Picking sub-phase ────────────────────────────────────── -->
    <template v-if="isPickingPhase">
      <div v-if="isMyPickTurn" class="pick-prompt">
        <p>Do you want to pick the blind?</p>
        <div class="actions">
          <button class="btn-pick" @click="pick">Pick</button>
          <button class="btn-pass" @click="pass">Pass</button>
        </div>
      </div>
      <p v-else class="waiting-msg">
        Waiting for {{ playerName(store.gameState?.current_player ?? 0) }} to pick or pass…
      </p>
    </template>

    <!-- ── Burying sub-phase ────────────────────────────────────── -->
    <template v-else-if="isBuryPhase">
      <div v-if="isMyBuryTurn" class="bury-prompt">
        <p>
          Select <strong>2 cards</strong> to bury
          <span class="count">({{ burySelection.length }}/2 selected)</span>
        </p>
        <hand-component
          :cards="store.myHand"
          :selectable="true"
          :selected-cards="burySelection"
          @select="toggleCard"
        />
        <button
          class="btn-bury"
          :disabled="burySelection.length !== 2"
          @click="submitBury"
        >
          Bury selected cards
        </button>
      </div>
      <p v-else class="waiting-msg">
        Waiting for {{ playerName(store.picker ?? 0) }} to bury…
      </p>
    </template>

    <!-- ── Calling sub-phase ───────────────────────────────────── -->
    <template v-else-if="isCallingPhase">
      <div v-if="isMyCallTurn" class="call-prompt">
        <p>Choose your partner card or go alone</p>
        <div class="call-suits">
          <button
            v-for="suit in callableSuits"
            :key="suit"
            class="btn-call"
            @click="callAce(suit)"
          >
            {{ SUIT_LABELS[suit] ?? suit }} Ace
          </button>
        </div>
        <button class="btn-alone" @click="goAlone">Go Alone (double stakes)</button>
      </div>
      <p v-else class="waiting-msg">
        Waiting for {{ playerName(store.picker ?? 0) }} to call their partner…
      </p>
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

.pick-prompt p, .bury-prompt p, .call-prompt p { margin-bottom: 0.75rem; font-size: 1rem; }
.count { color: #9ca3af; font-size: 0.85rem; }
.waiting-msg { color: #9ca3af; font-style: italic; margin: 0; }

.actions { display: flex; gap: 0.75rem; justify-content: center; }

.btn-pick { background: #15803d; }
.btn-pick:hover { background: #166534; }

.btn-pass { background: #6b7280; }
.btn-pass:hover { background: #4b5563; }

.btn-bury { margin-top: 0.75rem; background: #7c3aed; }
.btn-bury:hover:not(:disabled) { background: #6d28d9; }

.call-suits { display: flex; gap: 0.5rem; justify-content: center; flex-wrap: wrap; margin-bottom: 0.75rem; }
.btn-call { background: #0284c7; }
.btn-call:hover { background: #0369a1; }
.btn-alone { background: #9333ea; font-weight: 700; }
.btn-alone:hover { background: #7e22ce; }

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
