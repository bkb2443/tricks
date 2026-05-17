<script setup lang="ts">
import { useEuchreState } from '@/games/euchre/state'
import { useGameStore } from '@/stores/game'
import OrderingPanel from './bidding/OrderingPanel.vue'
import DiscardingPanel from './bidding/DiscardingPanel.vue'
import CallingPanel from './bidding/CallingPanel.vue'

const store = useGameStore()
const { subPhase } = useEuchreState()
</script>

<template>
  <div class="bidding-panel" :class="{ 'your-turn-glow': store.isMyTurn }">
    <ordering-panel  v-if="subPhase === 'ordering'" />
    <discarding-panel v-else-if="subPhase === 'discarding'" />
    <calling-panel    v-else-if="subPhase === 'calling'" />
    <p v-else class="waiting-msg">Bidding complete — play begins…</p>
  </div>
</template>

<style scoped>
.bidding-panel {
  background: rgba(0,0,0,0.25);
  border-radius: 8px;
  padding: 1rem 1.25rem;
  text-align: center;
}

.waiting-msg { color: #9ca3af; font-style: italic; margin: 0; }

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
