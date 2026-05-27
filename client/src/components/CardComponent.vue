<script setup lang="ts">
import { computed, ref } from 'vue'
import type { Card } from '@/engine/types'

const props = defineProps<{
  card: Card
  selectable?: boolean
  selected?: boolean
  faceDown?: boolean
}>()

const emit = defineEmits<{ select: [card: Card] }>()

const RANK_LABEL: Record<string, string> = {
  ace: 'A', two: '2', three: '3', four: '4', five: '5', six: '6',
  seven: '7', eight: '8', nine: '9', ten: '10', jack: 'J', queen: 'Q', king: 'K',
}
const SUIT_SYMBOL: Record<string, string> = {
  clubs: '♣', spades: '♠', hearts: '♥', diamonds: '♦',
}

const rank   = computed(() => RANK_LABEL[props.card.rank])
const symbol = computed(() => SUIT_SYMBOL[props.card.suit])
const isRed  = computed(() => props.card.suit === 'hearts' || props.card.suit === 'diamonds')

function handleClick() {
  if (props.selectable) emit('select', props.card)
}

// Touch drag-to-play
const dragOffsetY = ref(0)
const isDragging = ref(false)
const isCancelling = ref(false)
let touchStartY = 0

const PLAY_THRESHOLD = 40

function onTouchStart(e: TouchEvent) {
  if (!props.selectable) return
  touchStartY = e.touches[0].clientY
  isDragging.value = true
  dragOffsetY.value = 0
}

function onTouchMove(e: TouchEvent) {
  if (!isDragging.value) return
  e.preventDefault()
  const deltaY = touchStartY - e.touches[0].clientY
  // Only move upward (positive deltaY means upward); clamp at 0 so card doesn't go down
  dragOffsetY.value = Math.max(0, deltaY)
}

function onTouchEnd() {
  if (!isDragging.value) return
  isDragging.value = false
  if (dragOffsetY.value >= PLAY_THRESHOLD) {
    dragOffsetY.value = 0
    emit('select', props.card)
  } else {
    // Snap back with shake animation
    dragOffsetY.value = 0
    isCancelling.value = true
    setTimeout(() => { isCancelling.value = false }, 250)
  }
}

const dragStyle = computed(() => {
  if (dragOffsetY.value > 0) {
    return { transform: `translateY(-${dragOffsetY.value}px)`, transition: 'none' }
  }
  return {}
})
</script>

<template>
  <div
    class="card"
    :class="{
      selectable,
      selected,
      red: isRed && !faceDown,
      black: !isRed && !faceDown,
      'face-down': faceDown,
      'drag-cancel': isCancelling,
    }"
    :style="dragStyle"
    :aria-label="faceDown ? 'face-down card' : `${rank} of ${card.suit}`"
    :role="selectable ? 'button' : undefined"
    @click="handleClick"
    @touchstart.passive="onTouchStart"
    @touchmove="onTouchMove"
    @touchend.passive="onTouchEnd"
  >
    <template v-if="!faceDown">
      <span class="corner top-left">{{ rank }}<br />{{ symbol }}</span>
      <span class="center-symbol">{{ symbol }}</span>
      <span class="corner bottom-right">{{ rank }}<br />{{ symbol }}</span>
    </template>
    <template v-else>
      <span class="card-back">🂠</span>
    </template>
  </div>
</template>

<style scoped>
.card {
  position: relative;
  width: var(--card-w);
  height: var(--card-h);
  border: 1px solid #9ca3af;
  border-radius: 7px;
  background: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  user-select: none;
  flex-shrink: 0;
  transition: transform 0.1s, box-shadow 0.1s;
  touch-action: manipulation;
}

.card.red   { color: #dc2626; }
.card.black { color: #111827; }

.card.face-down {
  background: #1e3a8a;
  color: transparent;
  cursor: default;
}
.card-back { font-size: 2.5rem; color: #1e3a8a; }

.card.selectable { cursor: pointer; border-color: #6b7280; }
.card.selectable:hover { transform: translateY(-6px); box-shadow: 0 6px 16px rgba(0,0,0,0.4); }
.card.selected { transform: translateY(-12px); border-color: #2563eb; box-shadow: 0 8px 20px rgba(37,99,235,0.5); }

.corner {
  position: absolute;
  font-size: 0.7rem;
  font-weight: 700;
  line-height: 1.1;
  text-align: center;
}
.top-left     { top: 4px; left: 5px; }
.bottom-right { bottom: 4px; right: 5px; transform: rotate(180deg); }

.center-symbol { font-size: 1.6rem; }

@keyframes drag-cancel {
  0%, 100% { transform: translateX(0); }
  25% { transform: translateX(-4px); }
  75% { transform: translateX(4px); }
}
.drag-cancel { animation: drag-cancel 0.25s ease; }

@media (max-width: 640px) {
  .corner { font-size: 0.55rem; }
  .center-symbol { font-size: 1.15rem; }
  .card.selectable:hover { transform: translateY(-4px); }
  .card.selected { transform: translateY(-8px); }
}
</style>
