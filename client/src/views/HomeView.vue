<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useGame } from '@/composables/useGame'
import { connected } from '@/engine/socket'

const router = useRouter()
const { store, createRoom, createSoloRoom, joinRoom } = useGame()

const game     = ref('sheepshead')
const players  = ref(5)
const joinId   = ref('')

function handleCreate() {
  createRoom(game.value, players.value)
  router.push('/game')
}

function handleSolo() {
  createSoloRoom(game.value, players.value)
  router.push('/game')
}

function handleJoin() {
  if (!joinId.value.trim()) return
  joinRoom(joinId.value.trim())
  router.push('/game')
}
</script>

<template>
  <div class="home">
    <h1>Tricks</h1>

    <p v-if="!connected" class="warn">Not connected to server — waiting…</p>

    <!-- Solo mode — prominent single CTA -->
    <section class="solo">
      <div class="solo-text">
        <h2>Play Solo</h2>
        <p>You vs 4 bots — starts immediately.</p>
      </div>
      <div class="solo-controls">
        <label>
          Game
          <select v-model="game">
            <option value="sheepshead">Sheepshead</option>
          </select>
        </label>
        <button class="btn-solo" :disabled="!connected" @click="handleSolo">Play Solo →</button>
      </div>
    </section>

    <div class="panels">
      <!-- Create a multiplayer room -->
      <section>
        <h2>New Room</h2>
        <form @submit.prevent="handleCreate">
          <label>
            Game
            <select v-model="game">
              <option value="sheepshead">Sheepshead</option>
            </select>
          </label>
          <label>
            Players
            <input type="number" v-model.number="players" min="5" max="5" />
          </label>
          <button type="submit" :disabled="!connected">Create Room</button>
        </form>
      </section>

      <!-- Join an existing room -->
      <section>
        <h2>Join Game</h2>
        <form @submit.prevent="handleJoin">
          <label>
            Room ID
            <input v-model="joinId" placeholder="paste room ID" />
          </label>
          <button type="submit" :disabled="!connected || !joinId.trim()">Join</button>
        </form>
      </section>
    </div>

    <!-- Already in a room -->
    <div v-if="store.roomId" class="in-room">
      <p>You're in room <code>{{ store.roomId }}</code>, seat {{ store.seat }}.</p>
      <router-link to="/game">
        <button>Go to Table →</button>
      </router-link>
    </div>
  </div>
</template>

<style scoped>
.home { max-width: 600px; margin: 2rem auto; }
h1 { font-size: 2.5rem; margin-bottom: 1.5rem; }
.warn { color: #fbbf24; }

.solo {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  background: rgba(99, 102, 241, 0.15);
  border: 1px solid rgba(99, 102, 241, 0.4);
  border-radius: 8px;
  padding: 1rem 1.25rem;
  margin-bottom: 1.5rem;
}
.solo-text h2 { margin: 0 0 0.25rem; font-size: 1.1rem; }
.solo-text p  { margin: 0; font-size: 0.85rem; color: #9ca3af; }
.solo-controls { display: flex; align-items: flex-end; gap: 0.75rem; }
.btn-solo {
  background: #6366f1;
  white-space: nowrap;
}
.btn-solo:hover:not(:disabled) { background: #4f46e5; }

.panels {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 1.5rem;
  margin-bottom: 2rem;
}

section {
  background: rgba(0,0,0,0.25);
  border-radius: 8px;
  padding: 1rem 1.25rem;
}
h2 { margin: 0 0 0.75rem; font-size: 1.1rem; }

form { display: flex; flex-direction: column; gap: 0.5rem; }
label { display: flex; flex-direction: column; gap: 0.25rem; font-size: 0.85rem; color: #9ca3af; }

.in-room {
  background: rgba(0,0,0,0.25);
  border-radius: 8px;
  padding: 1rem 1.25rem;
}
code { font-size: 0.8rem; background: rgba(255,255,255,0.1); padding: 0.1rem 0.3rem; border-radius: 3px; }
</style>
