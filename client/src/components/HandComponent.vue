<script setup lang="ts">
import { computed } from 'vue'
import type { Card } from '@/engine/types'
import { sortHand } from '@/engine/sort'
import CardComponent from './CardComponent.vue'

const props = defineProps<{
  cards: Card[]
  selectable?: boolean
  selectedCards?: Card[]
}>()

const emit = defineEmits<{ select: [card: Card] }>()

const sortedCards = computed(() => sortHand(props.cards))

function isSelected(card: Card): boolean {
  return (
    props.selectedCards?.some(
      (c) => c.suit === card.suit && c.rank === card.rank,
    ) ?? false
  )
}
</script>

<template>
  <div class="hand">
    <card-component
      v-for="(card, i) in sortedCards"
      :key="`${card.suit}-${card.rank}-${i}`"
      :card="card"
      :selectable="selectable"
      :selected="isSelected(card)"
      @select="emit('select', $event)"
    />
    <p v-if="cards.length === 0" class="empty">No cards</p>
  </div>
</template>

<style scoped>
.hand {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 0.5rem;
  justify-content: center;
}
.empty { color: #6b7280; font-style: italic; }
</style>
