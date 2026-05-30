<script setup lang="ts">
import { onMounted } from 'vue'
import { useGameStore } from '@/stores/game'
import { initSocket, connectSocket, connected } from '@/engine/socket'

const store = useGameStore()

// Wire the socket to the store before the first frame
initSocket(store.handleUpdate)
onMounted(connectSocket)
</script>

<template>
  <div id="app">
    <header>
      <nav>
        <router-link to="/">Lobby</router-link>
        <router-link v-if="store.gameStarted" to="/game">Table</router-link>
      </nav>
      <span class="connection-status" :class="connected ? 'online' : 'offline'">
        {{ connected ? 'Connected' : 'Disconnected' }}
      </span>
    </header>

    <main>
      <p v-if="store.error" class="error-banner" @click="store.error = null">
        ⚠ {{ store.error }}
      </p>
      <router-view />
    </main>
  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; }

:root {
  --card-w: 62px;
  --card-h: 92px;
}

@media (max-width: 640px) {
  :root {
    --card-w: 46px;
    --card-h: 68px;
  }

  main { padding: 0.5rem; }

  header {
    padding: 0.35rem 0.5rem;
  }
}

body {
  margin: 0;
  font-family: system-ui, sans-serif;
  background: #1a4731;
  color: #f0f0f0;
  min-height: 100vh;
}

#app {
  display: flex;
  flex-direction: column;
  min-height: 100vh;
}

header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.5rem 1rem;
  background: rgba(0,0,0,0.3);
}

nav a {
  color: #f0f0f0;
  text-decoration: none;
  margin-right: 1rem;
  font-weight: 500;
}
nav a.router-link-active { text-decoration: underline; }

main { flex: 1; padding: 1rem; }

.connection-status { font-size: 0.8rem; }
.connection-status.online  { color: #6ee7b7; }
.connection-status.offline { color: #f87171; }

.error-banner {
  background: #7f1d1d;
  color: #fca5a5;
  padding: 0.5rem 1rem;
  border-radius: 6px;
  cursor: pointer;
  margin-bottom: 1rem;
}

button {
  padding: 0.4rem 1rem;
  border: none;
  border-radius: 6px;
  background: #2563eb;
  color: white;
  cursor: pointer;
  font-size: 0.95rem;
}
button:hover:not(:disabled) { background: #1d4ed8; }
button:disabled { opacity: 0.4; cursor: not-allowed; }

input, select {
  padding: 0.35rem 0.5rem;
  border: 1px solid #4b5563;
  border-radius: 4px;
  background: #374151;
  color: #f0f0f0;
  font-size: 0.95rem;
}
</style>
