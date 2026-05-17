<script setup lang="ts">
import { computed } from 'vue'
import { useGameStore } from '@/stores/game'
import SheepsheadTable from '@/games/sheepshead/GameTable.vue'
import EuchreTable from '@/games/euchre/GameTable.vue'

const store = useGameStore()

const notInRoom = computed(() => store.roomId === null)
</script>

<template>
  <div v-if="notInRoom" class="center">
    <p>You haven't joined a room yet.</p>
    <router-link to="/"><button>Back to Home</button></router-link>
  </div>
  <sheepshead-table v-else-if="store.gameState?.game_name === 'sheepshead'" />
  <euchre-table v-else-if="store.gameState?.game_name === 'euchre'" />
  <div v-else class="center">
    <p>Waiting for game to start…</p>
  </div>
</template>

<style scoped>
.center {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.75rem;
  padding: 4rem 1rem;
  text-align: center;
}
.waiting p { font-size: 1.2rem; }
.sub  { color: #9ca3af; font-size: 0.9rem; }
.hint { font-size: 0.85rem; color: #6b7280; font-style: italic; }
code { background: rgba(255,255,255,0.1); padding: 0.1rem 0.35rem; border-radius: 3px; }
</style>
