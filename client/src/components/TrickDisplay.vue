<script setup lang="ts">
import type { Trick } from '@/engine/types'
import CardComponent from './CardComponent.vue'

defineProps<{
  trick: Trick | null
  mySeat: number
  playerCount: number
}>()
</script>

<template>
  <div class="trick-area">
    <div v-if="trick && trick.plays.length" class="trick-plays">
      <div v-for="([player, card], i) in trick.plays" :key="i" class="play">
        <span class="player-label">{{ player === mySeat ? 'You' : `P${player}` }}</span>
        <card-component :card="card" />
      </div>
    </div>
    <p v-else class="waiting">Waiting for first card…</p>
  </div>
</template>

<style scoped>
.trick-area {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 120px;
  background: rgba(0,0,0,0.2);
  border-radius: 12px;
  padding: 1rem;
  margin: 1rem 0;
}
.trick-plays {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  justify-content: center;
}
.play {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}
.player-label { font-size: 0.75rem; color: #9ca3af; }
.waiting { color: #6b7280; font-style: italic; }
</style>
