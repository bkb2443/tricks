<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useGame } from '@/composables/useGame'
import { connected } from '@/engine/socket'
import { GAMES, getGameInfo } from '@/engine/games'

const router = useRouter()
const { createSoloRoom, joinWithCode, spectateRoom, joinQueue, createPrivateRoom } = useGame()

const guestName = ref(localStorage.getItem('guestName') ?? '')
const joinCode  = ref('')
const nameError = ref('')
const selectedGame = ref<string>('sheepshead')

const selectedGameInfo = computed(() => getGameInfo(selectedGame.value) ?? GAMES[0])

function saveName() {
  const n = guestName.value.trim()
  if (!n) { nameError.value = 'Enter a name to continue.'; return false }
  localStorage.setItem('guestName', n)
  nameError.value = ''
  return true
}

function handleSolo() {
  if (!saveName()) return
  createSoloRoom(selectedGame.value, selectedGameInfo.value.playerCount)
  router.push('/game')
}

function handleCreatePrivate() {
  if (!saveName()) return
  createPrivateRoom(selectedGame.value, null, guestName.value.trim())
  router.push('/lobby')
}

function handleJoinCode() {
  if (!saveName()) return
  if (!joinCode.value.trim()) return
  joinWithCode(guestName.value.trim(), joinCode.value.trim().toUpperCase())
  router.push('/lobby')
}

function handleSpectate() {
  if (!saveName()) return
  if (!joinCode.value.trim()) return
  spectateRoom(guestName.value.trim(), joinCode.value.trim().toUpperCase())
  router.push('/game')
}

function handleFindGame() {
  if (!saveName()) return
  joinQueue()
  router.push('/queue')
}

onMounted(() => {
  guestName.value = localStorage.getItem('guestName') ?? ''
})
</script>

<template>
  <div class="home">
    <h1>Tricks</h1>
    <p v-if="!connected" class="warn">Not connected to server — waiting…</p>

    <!-- Name prompt -->
    <section class="name-section">
      <label>
        Your name
        <input v-model="guestName" placeholder="Enter display name" maxlength="20" @input="nameError = ''" />
      </label>
      <p v-if="nameError" class="name-error">{{ nameError }}</p>
    </section>

    <!-- Game selector -->
    <section class="game-selector">
      <h2>Choose Game</h2>
      <div class="game-options">
        <button
          v-for="game in GAMES"
          :key="game.name"
          class="game-option"
          :class="{ selected: selectedGame === game.name }"
          @click="selectedGame = game.name"
        >
          <span class="game-name">{{ game.label }}</span>
          <span class="game-detail">{{ game.description }}</span>
        </button>
      </div>
    </section>

    <!-- Solo -->
    <section class="solo">
      <div class="solo-text">
        <h2>Play Solo</h2>
        <p>You vs {{ selectedGameInfo.playerCount - 1 }} bots — starts immediately.</p>
      </div>
      <button class="btn-solo" :disabled="!connected" @click="handleSolo">Play Solo →</button>
    </section>

    <!-- Multiplayer actions -->
    <div class="panels">
      <section>
        <h2>Create Private Room</h2>
        <p class="hint">Share the room code with friends.</p>
        <button :disabled="!connected" @click="handleCreatePrivate">Create Room →</button>
      </section>

      <section>
        <h2>Find a Game</h2>
        <p class="hint">Match with others online.</p>
        <button :disabled="!connected" @click="handleFindGame">Find Game →</button>
      </section>

      <section class="join-code-section">
        <h2>Join with Code</h2>
        <input v-model="joinCode" placeholder="WOLF-42" maxlength="10" @keydown.enter="handleJoinCode" />
        <div class="join-actions">
          <button :disabled="!connected || !joinCode.trim()" @click="handleJoinCode">Join →</button>
          <button class="btn-watch" :disabled="!connected || !joinCode.trim()" @click="handleSpectate">Watch →</button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
.home { max-width: 600px; margin: 2rem auto; }
h1 { font-size: 2.5rem; margin-bottom: 1.5rem; }
.warn { color: #fbbf24; }
.name-section { margin-bottom: 1.25rem; }
.name-section label { display: flex; flex-direction: column; gap: 0.4rem; font-size: 0.9rem; color: #9ca3af; }
.name-section input { font-size: 1rem; }
.name-error { color: #f87171; font-size: 0.85rem; margin: 0.25rem 0 0; }
.solo { display: flex; align-items: center; justify-content: space-between; gap: 1rem;
  background: rgba(99,102,241,0.15); border: 1px solid rgba(99,102,241,0.4);
  border-radius: 8px; padding: 1rem 1.25rem; margin-bottom: 1.5rem; }
.solo-text h2 { margin: 0 0 0.25rem; font-size: 1.1rem; }
.solo-text p { margin: 0; font-size: 0.85rem; color: #9ca3af; }
.btn-solo { background: #6366f1; white-space: nowrap; }
.btn-solo:hover:not(:disabled) { background: #4f46e5; }
.panels { display: grid; grid-template-columns: 1fr 1fr; gap: 1rem; margin-bottom: 2rem; }
.join-code-section { grid-column: 1 / -1; }
.join-actions { display: flex; gap: 0.5rem; }
.join-actions button { flex: 1; }
.btn-watch { background: rgba(255,255,255,0.08); border: 1px solid rgba(255,255,255,0.2); }
.btn-watch:hover:not(:disabled) { background: rgba(255,255,255,0.15); }
section { background: rgba(0,0,0,0.25); border-radius: 8px; padding: 1rem 1.25rem; display: flex; flex-direction: column; gap: 0.5rem; }
h2 { margin: 0; font-size: 1rem; }
.hint { margin: 0; font-size: 0.8rem; color: #6b7280; }
input { background: rgba(255,255,255,0.08); border: 1px solid rgba(255,255,255,0.15); border-radius: 5px; padding: 0.4rem 0.6rem; color: #fff; font-size: 0.9rem; }

/* Game selector */
.game-selector { margin-bottom: 0.5rem; }
.game-selector h2 { margin: 0 0 0.5rem; font-size: 1rem; }
.game-options { display: flex; gap: 0.75rem; }
.game-option {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.2rem;
  background: rgba(0,0,0,0.25);
  border: 2px solid transparent;
  border-radius: 8px;
  padding: 0.75rem 1rem;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
  color: #d1d5db;
}
.game-option:hover { border-color: #6366f1; background: rgba(99,102,241,0.1); }
.game-option.selected { border-color: #6366f1; background: rgba(99,102,241,0.2); color: #fff; }
.game-name { font-size: 1rem; font-weight: 600; }
.game-detail { font-size: 0.75rem; color: #9ca3af; }
.game-option.selected .game-detail { color: #c7d2fe; }

@media (max-width: 640px) {
  .home {
    margin: 1rem auto;
  }

  h1 {
    font-size: 2rem;
    margin-bottom: 1rem;
  }

  .panels {
    grid-template-columns: 1fr;
  }

  .join-code-section {
    grid-column: auto;
  }

  .solo {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .game-options {
    flex-wrap: wrap;
  }

  .game-option {
    min-width: 120px;
  }
}
</style>
