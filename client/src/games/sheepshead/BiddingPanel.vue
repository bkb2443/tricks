<script setup lang="ts">
import { ref, computed } from 'vue'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'
import HandComponent from '@/components/HandComponent.vue'
import type { Card } from '@/engine/types'

const store = useGameStore()
const { playerName } = store
const { pick, pass, bury } = useGame()

// Two sub-phases: picking (anyone can pick/pass) and burying (picker discards 2)
const isPickingPhase = computed(() => store.picker === null)
const isBuryPhase    = computed(() => store.picker !== null && store.phase === 'bidding')

const isMyPickTurn = computed(() => isPickingPhase.value && store.isMyTurn)
const isMyBuryTurn = computed(() => isBuryPhase.value && store.isPicker)

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
  <div class="bidding-panel">
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
  </div>
</template>

<style scoped>
.bidding-panel {
  background: rgba(0,0,0,0.25);
  border-radius: 8px;
  padding: 1rem 1.25rem;
  text-align: center;
}

.pick-prompt p, .bury-prompt p { margin-bottom: 0.75rem; font-size: 1rem; }
.count { color: #9ca3af; font-size: 0.85rem; }
.waiting-msg { color: #9ca3af; font-style: italic; margin: 0; }

.actions { display: flex; gap: 0.75rem; justify-content: center; }

.btn-pick { background: #15803d; }
.btn-pick:hover { background: #166534; }

.btn-pass { background: #6b7280; }
.btn-pass:hover { background: #4b5563; }

.btn-bury { margin-top: 0.75rem; background: #7c3aed; }
.btn-bury:hover:not(:disabled) { background: #6d28d9; }
</style>
