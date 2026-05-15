<script setup lang="ts">
import { computed } from 'vue'
import { useGameStore } from '@/stores/game'
import SheepsheadTable from '@/games/sheepshead/GameTable.vue'

const store = useGameStore()

const notInRoom = computed(() => store.roomId === null)
</script>

<template>
  <!-- No room → redirect back to lobby -->
  <div v-if="notInRoom" class="center">
    <p>You haven't joined a room yet.</p>
    <router-link to="/"><button>Back to Lobby</button></router-link>
  </div>

  <!-- In room, waiting for all players to connect -->
  <div v-else-if="!store.gameStarted" class="center waiting">
    <p>{{ store.isSolo ? 'Starting solo game…' : 'Waiting for players to join…' }}</p>
    <p class="sub">Room ID: <code>{{ store.roomId }}</code></p>
    <p class="sub">Your seat: {{ store.seat }}</p>
    <p v-if="!store.isSolo" class="hint">Share the room ID with other players.</p>
  </div>

  <!-- Game in progress -->
  <sheepshead-table v-else-if="store.gameState?.game_name === 'sheepshead'" />

  <!-- Fallback for unknown games -->
  <div v-else class="center">
    <p>Unknown game: {{ store.gameState?.game_name }}</p>
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
