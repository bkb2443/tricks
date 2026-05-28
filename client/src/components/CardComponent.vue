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

// ── Touch drag-to-play ──────────────────────────────────────────────────────
const dragY     = ref(0)     // current upward drag distance in px (≥0)
const cancelled = ref(false) // true during the snap-back animation
let dragStartY  = -1

function onTouchStart(e: TouchEvent) {
  if (!props.selectable) return
  dragStartY = e.touches[0]!.clientY
  dragY.value = 0
  cancelled.value = false
}

function onTouchMove(e: TouchEvent) {
  if (dragStartY < 0 || !props.selectable) return
  const delta = dragStartY - e.touches[0]!.clientY  // positive = upward
  dragY.value = Math.max(0, delta)
  if (dragY.value > 0) e.preventDefault()           // block page scroll during drag
}

function onTouchEnd() {
  if (dragStartY < 0) return
  if (dragY.value >= 40) {
    emit('select', props.card)
  } else if (dragY.value > 0) {
    cancelled.value = true
    setTimeout(() => { cancelled.value = false }, 350)
  }
  dragStartY = -1
  dragY.value = 0
}

const dragStyle = computed(() =>
  dragY.value > 0 ? { transform: `translateY(-${dragY.value}px)`, transition: 'none' } : {},
)

function handleClick() {
  if (props.selectable) emit('select', props.card)
}
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
      'cancel-snap': cancelled,
    }"
    :style="dragStyle"
    :aria-label="faceDown ? 'face-down card' : `${rank} of ${card.suit}`"
    :role="selectable ? 'button' : undefined"
    @click="handleClick"
    @touchstart="onTouchStart"
    @touchmove="onTouchMove"
    @touchend="onTouchEnd"
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
  width: var(--card-w, 62px);
  height: var(--card-h, 92px);
  border: 1px solid #9ca3af;
  border-radius: 7px;
  background: #fff;
  display: flex;
  align-items: center;
  justify-content: center;
  user-select: none;
  flex-shrink: 0;
  transition: transform 0.1s, box-shadow 0.1s;
  touch-action: none;
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

/* Hover lift only on pointer-capable (non-touch) devices */
@media (hover: hover) and (pointer: fine) {
  .card.selectable:hover { transform: translateY(-6px); box-shadow: 0 6px 16px rgba(0,0,0,0.4); }
}

.card.selected { transform: translateY(-12px); border-color: #2563eb; box-shadow: 0 8px 20px rgba(37,99,235,0.5); }

.corner {
  position: absolute;
  font-size: var(--card-corner-font, 0.7rem);
  font-weight: 700;
  line-height: 1.1;
  text-align: center;
}
.top-left     { top: 4px; left: 5px; }
.bottom-right { bottom: 4px; right: 5px; transform: rotate(180deg); }

.center-symbol { font-size: var(--card-center-font, 1.6rem); }

/* Snap-back cancel animation */
@keyframes cancel-snap {
  0%   { transform: translateX(0); }
  25%  { transform: translateX(-5px); }
  60%  { transform: translateX(4px); }
  85%  { transform: translateX(-2px); }
  100% { transform: translateX(0); }
}
.card.cancel-snap {
  animation: cancel-snap 0.35s ease;
}
</style>
