<script setup lang="ts">
import { computed, defineAsyncComponent, ref, watch } from 'vue'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'
import ChatPanel from '@/components/ChatPanel.vue'
import type { Component } from 'vue'

const store = useGameStore()
const { sendLobbyChat } = useGame()

const GAME_TABLES: Record<string, Component> = {
  sheepshead: defineAsyncComponent(() => import('@/games/sheepshead/GameTable.vue')),
  euchre:     defineAsyncComponent(() => import('@/games/euchre/GameTable.vue')),
}

const notInRoom = computed(() => store.roomId === null)
const gameTable = computed(() => store.gameState ? GAME_TABLES[store.gameState.game_name] : null)

const showChatToggle = computed(() => {
  const phase = store.gameState?.phase
  return phase === 'playing' || phase === 'intermission'
})

const chatOpen = ref(false)
const unreadCount = ref(0)

watch(
  () => store.lobbyChat.length,
  () => {
    if (!chatOpen.value) unreadCount.value++
  },
)

watch(chatOpen, (open) => {
  if (open) unreadCount.value = 0
})

function toggleChat() {
  chatOpen.value = !chatOpen.value
}
</script>

<template>
  <div v-if="notInRoom" class="center">
    <p>You haven't joined a room yet.</p>
    <router-link to="/"><button>Back to Home</button></router-link>
  </div>
  <template v-else-if="gameTable">
    <component :is="gameTable" />

    <template v-if="showChatToggle">
      <!-- Collapsible chat drawer -->
      <div v-if="chatOpen" class="chat-drawer">
        <div class="chat-drawer-header">
          <span>Chat</span>
          <button class="chat-close-btn" @click="toggleChat" aria-label="Close chat">✕</button>
        </div>
        <div class="chat-drawer-body">
          <ChatPanel :messages="store.lobbyChat" @send="sendLobbyChat" />
        </div>
      </div>

      <!-- Toggle button (fixed, bottom-right) -->
      <button class="chat-toggle-btn" @click="toggleChat" aria-label="Toggle chat">
        💬
        <span v-if="!chatOpen && unreadCount > 0" class="unread-badge">{{ unreadCount }}</span>
      </button>
    </template>
  </template>
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

/* Fixed toggle button */
.chat-toggle-btn {
  position: fixed;
  bottom: 1.25rem;
  right: 1.25rem;
  z-index: 200;
  width: 3rem;
  height: 3rem;
  border-radius: 50%;
  background: rgba(30, 30, 40, 0.92);
  border: 1px solid rgba(255, 255, 255, 0.15);
  font-size: 1.25rem;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
  padding: 0;
}

.unread-badge {
  position: absolute;
  top: -4px;
  right: -4px;
  min-width: 1.1rem;
  height: 1.1rem;
  border-radius: 999px;
  background: var(--color-error, #ef4444);
  color: #fff;
  font-size: 0.65rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 3px;
  pointer-events: none;
}

/* Collapsible drawer */
.chat-drawer {
  position: fixed;
  bottom: 5rem;
  right: 1.25rem;
  z-index: 199;
  width: 280px;
  height: min(360px, calc(100vh - 120px));
  background: rgba(20, 20, 30, 0.96);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.6);
  overflow: hidden;
}

.chat-drawer-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
  font-size: 0.85rem;
  font-weight: 600;
  flex-shrink: 0;
}

.chat-close-btn {
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
  font-size: 0.9rem;
  padding: 0 0.1rem;
  line-height: 1;
  opacity: 0.7;
}
.chat-close-btn:hover { opacity: 1; }

.chat-drawer-body {
  flex: 1;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
</style>
