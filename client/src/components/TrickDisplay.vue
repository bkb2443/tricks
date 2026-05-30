<script setup lang="ts">
import { computed } from 'vue'
import type { Trick } from '@/engine/types'
import CardComponent from './CardComponent.vue'

const props = defineProps<{
  trick: Trick | null
  completedTrick: Trick | null
  mySeat: number
  names: string[]
  pickerSeat: number | null
  partnerSeat: number | null
  currentWinnerSeat?: number
}>()

const ORDER_BADGES = ['①', '②', '③', '④', '⑤']

function playerLabel(seat: number): string {
  if (seat === props.mySeat) return 'You'
  return props.names[seat] || `P${seat}`
}

// Show live trick if present, fall back to the just-completed trick
const activeTrick = computed(() => props.trick ?? props.completedTrick)
const isCompleted = computed(() => props.trick === null && props.completedTrick !== null)

const winnerIdx = computed(() => {
  const s = props.currentWinnerSeat ?? -1
  if (!props.trick || s < 0) return -1
  return props.trick.plays.findIndex(([seat]) => seat === s)
})
</script>

<template>
  <div class="trick-area">
    <template v-if="activeTrick && activeTrick.plays.length">
      <!-- Winner banner: shown only when displaying a just-completed trick -->
      <Transition name="winner-fade">
        <div v-if="isCompleted && activeTrick.winner !== null" class="winner-banner">
          {{ playerLabel(activeTrick.winner!) }} wins the trick
        </div>
      </Transition>

      <!-- Card plays: animated entry for live trick, static for completed -->
      <TransitionGroup
        :name="isCompleted ? '' : 'card-play'"
        tag="div"
        class="trick-plays"
      >
        <div
          v-for="([player, card], i) in activeTrick.plays"
          :key="`${player}-${card.suit}-${card.rank}`"
          class="play"
        >
          <div class="play-header">
            <span class="order-badge">{{ ORDER_BADGES[i] ?? String(i + 1) }}</span>
            <span class="player-label">{{ playerLabel(player) }}</span>
          </div>
          <div class="play-meta">
            <span v-if="i === 0" class="meta-label">Led</span>
            <span v-if="player === pickerSeat" class="role-badge picker">Picker</span>
            <span v-if="partnerSeat !== null && player === partnerSeat" class="role-badge partner">Partner</span>
          </div>
          <div class="card-wrapper" :class="{ winning: !isCompleted && i === winnerIdx && winnerIdx !== -1 }">
            <card-component :card="card" />
          </div>
        </div>
      </TransitionGroup>
    </template>
    <p v-else class="waiting">Waiting for first card…</p>
  </div>
</template>

<style scoped>
/* Card play enter animation */
.card-play-enter-active {
  transition: opacity 0.25s ease-out, transform 0.25s ease-out;
}
.card-play-enter-from {
  opacity: 0;
  transform: translateY(12px);
}
.card-play-enter-to {
  opacity: 1;
  transform: translateY(0);
}

/* Winner banner fade */
.winner-fade-enter-active, .winner-fade-leave-active {
  transition: opacity 0.2s ease;
}
.winner-fade-enter-from, .winner-fade-leave-to { opacity: 0; }

.winner-banner {
  text-align: center;
  font-weight: 700;
  color: #fbbf24;
  font-size: 0.9rem;
  margin-bottom: 0.5rem;
  width: 100%;
}

.trick-area {
  display: flex;
  flex-direction: column;
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
.play-header { display: flex; align-items: center; gap: 4px; }
.order-badge { font-size: 0.85rem; color: #d1d5db; }
.player-label { font-size: 0.75rem; color: #9ca3af; }
.play-meta { display: flex; gap: 4px; align-items: center; min-height: 16px; }
.meta-label { font-size: 0.65rem; color: #6b7280; font-style: italic; }
.role-badge {
  font-size: 0.6rem;
  padding: 1px 5px;
  border-radius: 999px;
  font-weight: 600;
  letter-spacing: 0.02em;
}
.role-badge.picker  { background: #7c3aed; color: #fff; }
.role-badge.partner { background: #0d9488; color: #fff; }
.card-wrapper { border-radius: 6px; }
.card-wrapper.winning {
  outline: 2px solid #f59e0b;
  box-shadow: 0 0 8px rgba(245, 158, 11, 0.45);
}
.waiting { color: #6b7280; font-style: italic; }

@media (max-width: 640px) {
  .trick-area {
    padding: 0.5rem;
    min-height: 90px;
    margin: 0.5rem 0;
  }

  .trick-plays {
    gap: 8px;
  }

  .play-header { gap: 2px; }
  .player-label { font-size: 0.65rem; }
  .order-badge { font-size: 0.75rem; }
}
</style>
