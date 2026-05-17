<script setup lang="ts">
import { ref, computed } from 'vue'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'
import { useEuchreState } from '@/games/euchre/state'
import HandComponent from '@/components/HandComponent.vue'
import type { Card } from '@/engine/types'
import { sortHandEuchre } from '@/games/euchre/sort'

const store = useGameStore()
const { playerName } = store
const { discard } = useGame()
const { calledSuit } = useEuchreState()

const state = computed(() => store.gameState!)

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
</script>

<template>
  <div v-if="store.seat === state.dealer" class="discard-prompt">
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

<style scoped>
.discard-prompt { display: flex; flex-direction: column; align-items: center; gap: 0.6rem; }
.discard-prompt p { margin: 0; font-size: 1rem; }

.count { color: #9ca3af; font-size: 0.85rem; }
.waiting-msg { color: #9ca3af; font-style: italic; margin: 0; }

.btn-discard { margin-top: 0.5rem; background: #7c3aed; }
.btn-discard:hover:not(:disabled) { background: #6d28d9; }
</style>
