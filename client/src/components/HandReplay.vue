<script setup lang="ts">
import { computed, ref, onUnmounted, watch } from 'vue'
import type { Card, Trick } from '@/engine/types'
import CardComponent from './CardComponent.vue'

const props = defineProps<{
  tricks: Trick[]
  names: string[]
  mySeat: number
  bury?: { picker: number; cards: Card[] } | null
}>()

const emit = defineEmits<{ close: [] }>()

const AUTO_ADVANCE_MS = 3000

const ORDER_BADGES = ['①', '②', '③', '④', '⑤', '⑥']
const ORDINALS = ['1st', '2nd', '3rd', '4th', '5th', '6th', '7th', '8th']

function ordinal(n: number): string {
  return ORDINALS[n] ?? `${n + 1}th`
}

function playerLabel(seat: number): string {
  if (seat === props.mySeat) return 'You'
  return props.names[seat] || `P${seat}`
}

const hasBuryStep = computed(
  () => !!props.bury && props.bury.cards.length > 0,
)

const totalSteps = computed(
  () => props.tricks.length + (hasBuryStep.value ? 1 : 0),
)

const step = ref(0)

const isBuryStep = computed(
  () => hasBuryStep.value && step.value === props.tricks.length,
)

const currentTrick = computed<Trick | null>(() => {
  if (isBuryStep.value) return null
  return props.tricks[step.value] ?? null
})

const autoAdvance = ref(false)
let timer: ReturnType<typeof setInterval> | null = null

function clearTimer() {
  if (timer !== null) { clearInterval(timer); timer = null }
}

function stopAuto() {
  autoAdvance.value = false
  clearTimer()
}

function startAuto() {
  if (totalSteps.value === 0) return
  autoAdvance.value = true
  clearTimer()
  timer = setInterval(() => {
    if (step.value + 1 >= totalSteps.value) {
      stopAuto()
      return
    }
    step.value += 1
  }, AUTO_ADVANCE_MS)
}

function toggleAuto() {
  if (autoAdvance.value) stopAuto()
  else startAuto()
}

function next() {
  if (step.value + 1 < totalSteps.value) step.value += 1
  stopAuto()
}

function prev() {
  if (step.value > 0) step.value -= 1
  stopAuto()
}

function close() {
  stopAuto()
  emit('close')
}

watch(
  () => props.tricks.length,
  () => {
    step.value = 0
    stopAuto()
  },
)

onUnmounted(() => { clearTimer() })

const canPrev = computed(() => step.value > 0)
const canNext = computed(() => step.value + 1 < totalSteps.value)
</script>

<template>
  <div class="replay-overlay" role="dialog" aria-label="Hand replay">
    <div class="replay-panel">
      <header class="replay-header">
        <h3>Hand Replay</h3>
        <button class="close-btn" aria-label="Close replay" @click="close">×</button>
      </header>

      <div class="replay-progress">
        <span class="step-label" v-if="!isBuryStep">
          {{ ordinal(step) }} trick ({{ step + 1 }} / {{ totalSteps }})
        </span>
        <span class="step-label" v-else>
          Bury ({{ totalSteps }} / {{ totalSteps }})
        </span>
      </div>

      <!-- Trick view -->
      <div v-if="currentTrick" class="trick-area">
        <div v-if="currentTrick.winner !== null" class="winner-banner">
          {{ playerLabel(currentTrick.winner) }} wins
        </div>
        <div class="trick-plays">
          <div
            v-for="([player, card], i) in currentTrick.plays"
            :key="`${player}-${card.suit}-${card.rank}`"
            class="play"
          >
            <div class="play-header">
              <span class="order-badge">{{ ORDER_BADGES[i] ?? String(i + 1) }}</span>
              <span class="player-label">{{ playerLabel(player) }}</span>
            </div>
            <card-component :card="card" />
          </div>
        </div>
      </div>

      <!-- Bury view (Sheepshead only) -->
      <div v-else-if="isBuryStep && bury" class="trick-area bury-area">
        <div class="winner-banner">
          {{ playerLabel(bury.picker) }} buried
        </div>
        <div class="trick-plays">
          <div
            v-for="(card, i) in bury.cards"
            :key="`bury-${i}-${card.suit}-${card.rank}`"
            class="play"
          >
            <card-component :card="card" />
          </div>
        </div>
      </div>

      <div v-else class="trick-area empty">
        <p class="waiting">No tricks to replay.</p>
      </div>

      <div class="replay-controls">
        <button class="ctrl-btn" :disabled="!canPrev" @click="prev">◀ Prev</button>
        <button class="ctrl-btn play-btn" :disabled="totalSteps === 0" @click="toggleAuto">
          {{ autoAdvance ? '⏸ Pause' : '▶ Auto' }}
        </button>
        <button class="ctrl-btn" :disabled="!canNext" @click="next">Next ▶</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.replay-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.78);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 300;
}

.replay-panel {
  background: #1f2937;
  border-radius: 12px;
  padding: 1.25rem 1.5rem;
  width: 90%;
  max-width: 640px;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  border: 1px solid rgba(99, 102, 241, 0.4);
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
}

.replay-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.replay-header h3 { margin: 0; font-size: 1.15rem; }
.close-btn {
  background: transparent;
  color: #d1d5db;
  border: none;
  font-size: 1.6rem;
  line-height: 1;
  cursor: pointer;
  padding: 0 0.4rem;
}
.close-btn:hover { color: #fff; }

.replay-progress {
  text-align: center;
  font-size: 0.85rem;
  color: #9ca3af;
}

.trick-area {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 160px;
  background: rgba(0, 0, 0, 0.25);
  border-radius: 10px;
  padding: 0.85rem;
}
.trick-area.empty { color: #6b7280; }
.bury-area { border: 1px dashed rgba(245, 158, 11, 0.4); }

.winner-banner {
  font-weight: 700;
  color: #fbbf24;
  font-size: 0.9rem;
  margin-bottom: 0.6rem;
}

.trick-plays {
  display: flex;
  gap: 14px;
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

.replay-controls {
  display: flex;
  gap: 0.5rem;
  justify-content: center;
}
.ctrl-btn {
  padding: 0.45rem 1rem;
  background: #374151;
  color: #fff;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 0.9rem;
}
.ctrl-btn:hover:not(:disabled) { background: #4b5563; }
.ctrl-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.ctrl-btn.play-btn { background: #6366f1; }
.ctrl-btn.play-btn:hover:not(:disabled) { background: #4f46e5; }

.waiting { color: #6b7280; font-style: italic; margin: 0; }
</style>
