<script setup lang="ts">
import { computed } from 'vue'
import type { Trick } from '@/engine/types'
import { trickWinnerIndex } from '@/engine/sort'
import CardComponent from './CardComponent.vue'

const props = defineProps<{
  trick: Trick | null
  mySeat: number
  names: string[]
  pickerSeat: number | null
  partnerSeat: number | null
}>()

// Circled number badges ①②③④⑤ for play order
const ORDER_BADGES = ['①', '②', '③', '④', '⑤']

function playerLabel(seat: number): string {
  if (seat === props.mySeat) return 'You'
  return props.names[seat] || `P${seat}`
}

const winnerIdx = computed(() =>
  props.trick ? trickWinnerIndex(props.trick) : -1,
)
</script>

<template>
  <div class="trick-area">
    <div v-if="trick && trick.plays.length" class="trick-plays">
      <div v-for="([player, card], i) in trick.plays" :key="i" class="play">
        <!-- Play-order badge and player name row -->
        <div class="play-header">
          <span class="order-badge">{{ ORDER_BADGES[i] ?? String(i + 1) }}</span>
          <span class="player-label">
            {{ playerLabel(player) }}
          </span>
        </div>
        <!-- Role badges + Led label -->
        <div class="play-meta">
          <span v-if="i === 0" class="meta-label">Led</span>
          <span v-if="player === pickerSeat" class="role-badge picker">Picker</span>
          <span v-if="partnerSeat !== null && player === partnerSeat" class="role-badge partner">Partner</span>
        </div>
        <!-- Card with winner highlight -->
        <div class="card-wrapper" :class="{ winning: i === winnerIdx && winnerIdx !== -1 }">
          <card-component :card="card" />
        </div>
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
  gap: 16px;
  flex-wrap: wrap;
  justify-content: center;
}
.play {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 3px;
}
.play-header {
  display: flex;
  align-items: center;
  gap: 4px;
}
.order-badge {
  font-size: 0.85rem;
  color: #d1d5db;
}
.player-label {
  font-size: 0.75rem;
  color: #9ca3af;
}
.play-meta {
  display: flex;
  gap: 4px;
  align-items: center;
  min-height: 16px;
}
.meta-label {
  font-size: 0.65rem;
  color: #6b7280;
  font-style: italic;
}
.role-badge {
  font-size: 0.6rem;
  padding: 1px 5px;
  border-radius: 999px;
  font-weight: 600;
  letter-spacing: 0.02em;
}
.role-badge.picker  { background: #7c3aed; color: #fff; }
.role-badge.partner { background: #0d9488; color: #fff; }
.card-wrapper {
  border-radius: 6px;
}
.card-wrapper.winning {
  outline: 2px solid #f59e0b;
  box-shadow: 0 0 8px rgba(245, 158, 11, 0.45);
}
.waiting { color: #6b7280; font-style: italic; }
</style>
