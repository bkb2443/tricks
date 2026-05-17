<script setup lang="ts">
import { computed, defineAsyncComponent } from 'vue'
import { useGameStore } from '@/stores/game'
import type { Component } from 'vue'

const store = useGameStore()

const GAME_TABLES: Record<string, Component> = {
  sheepshead: defineAsyncComponent(() => import('@/games/sheepshead/GameTable.vue')),
  euchre:     defineAsyncComponent(() => import('@/games/euchre/GameTable.vue')),
}

const notInRoom = computed(() => store.roomId === null)
const gameTable = computed(() => store.gameState ? GAME_TABLES[store.gameState.game_name] : null)
</script>

<template>
  <div v-if="notInRoom" class="center">
    <p>You haven't joined a room yet.</p>
    <router-link to="/"><button>Back to Home</button></router-link>
  </div>
  <component :is="gameTable" v-else-if="gameTable" />
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
</style>
