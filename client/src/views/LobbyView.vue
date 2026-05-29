<script setup lang="ts">
import { computed, ref, nextTick, watch, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'

const router = useRouter()
const gs = useGameStore()
const { startGame, sendLobbyChat, forceBot, extendRejoin } = useGame()

const chatInput = ref('')
const chatEl    = ref<HTMLElement | null>(null)

const isHost = computed(() => {
  const meta = gs.gameState?.meta
  if (meta?.kind !== 'lobby') return false
  return typeof meta.host_seat === 'number' && meta.host_seat === gs.seat
})

const roomCode = computed(() => gs.roomCode ?? '—')

const countdownEndsAt = computed<number | null>(() => {
  const meta = gs.gameState?.meta
  if (meta?.kind !== 'lobby') return null
  return typeof meta.countdown_ends_at === 'number' ? meta.countdown_ends_at : null
})

const secondsLeft = ref<number | null>(null)
let countdownInterval: ReturnType<typeof setInterval> | null = null

watch(countdownEndsAt, (val) => {
  if (countdownInterval) clearInterval(countdownInterval)
  if (val === null) { secondsLeft.value = null; return }
  const update = () => {
    const diff = Math.ceil((val - Date.now()) / 1000)
    secondsLeft.value = diff > 0 ? diff : 0
    if (diff <= 0 && countdownInterval) clearInterval(countdownInterval)
  }
  update()
  countdownInterval = setInterval(update, 500)
})

onUnmounted(() => {
  if (countdownInterval) clearInterval(countdownInterval)
})

// Navigate to game when phase changes out of lobby
watch(() => gs.gameState?.phase, (phase) => {
  if (phase && phase !== 'lobby') router.push('/game')
})

function sendChat() {
  if (!chatInput.value.trim()) return
  sendLobbyChat(chatInput.value.trim())
  chatInput.value = ''
}

watch(() => gs.lobbyChat.length, async () => {
  await nextTick()
  if (chatEl.value) chatEl.value.scrollTop = chatEl.value.scrollHeight
})

async function copyCode() {
  await navigator.clipboard.writeText(roomCode.value)
}

const seatStateLabel = (state: string) =>
  state === 'empty' ? 'Empty' : state === 'bot' ? 'Bot' : state === 'disconnected' ? 'Disconnected' : ''
</script>

<template>
  <div class="lobby">
    <header class="lobby-header">
      <h1>Lobby</h1>
      <div v-if="roomCode !== '—'" class="room-code">
        <span>{{ roomCode }}</span>
        <button class="btn-copy" @click="copyCode" title="Copy room code">📋</button>
      </div>
    </header>

    <!-- Spectator count -->
    <p v-if="gs.spectatorCount > 0" class="spectator-count">
      {{ gs.spectatorCount }} {{ gs.spectatorCount === 1 ? 'person' : 'people' }} watching
    </p>

    <!-- Seat rail -->
    <section class="seats">
      <div
        v-for="info in gs.seats"
        :key="info.seat"
        class="seat-card"
        :class="[info.state, { me: info.seat === gs.seat }]"
      >
        <div class="seat-name">
          {{ info.name ?? seatStateLabel(info.state) }}
          <span v-if="info.seat === gs.seat" class="you-badge">You</span>
        </div>
        <div class="seat-state">{{ info.state }}</div>
        <div v-if="info.state === 'disconnected' && isHost" class="host-controls">
          <button @click="forceBot(info.seat)" class="btn-sm">Bot now</button>
          <button @click="extendRejoin(info.seat)" class="btn-sm">+30s</button>
        </div>
      </div>
    </section>

    <!-- Chat -->
    <section class="chat">
      <div ref="chatEl" class="chat-messages">
        <div
          v-for="(msg, i) in gs.lobbyChat"
          :key="i"
          class="chat-msg"
          :class="{ system: msg.from === 'System' }"
        >
          <span class="chat-from">{{ msg.from }}:</span>
          <span class="chat-text">{{ msg.text }}</span>
        </div>
        <div v-if="gs.lobbyChat.length === 0" class="chat-empty">No messages yet…</div>
      </div>
      <div class="chat-input-row">
        <input
          v-model="chatInput"
          placeholder="Say something…"
          maxlength="200"
          @keydown.enter="sendChat"
        />
        <button @click="sendChat" :disabled="!chatInput.trim()">Send</button>
      </div>
    </section>

    <!-- Host controls -->
    <div class="start-area">
      <template v-if="isHost">
        <p class="hint">You are the host.</p>
        <button class="btn-start" @click="startGame">Start Game →</button>
      </template>
      <p v-else class="waiting">Waiting for host to start the game…</p>
      <div v-if="secondsLeft !== null" class="countdown">Game starting in {{ secondsLeft }}s</div>
    </div>
  </div>
</template>

<style scoped>
.lobby { max-width: 700px; margin: 2rem auto; display: flex; flex-direction: column; gap: 1.25rem; }
.lobby-header { display: flex; align-items: center; justify-content: space-between; }
h1 { margin: 0; font-size: 2rem; }
.room-code { display: flex; align-items: center; gap: 0.5rem; background: rgba(255,255,255,0.08); border-radius: 6px; padding: 0.3rem 0.75rem; font-family: monospace; font-size: 1.2rem; }
.btn-copy { background: none; border: none; cursor: pointer; font-size: 1rem; padding: 0; }

.spectator-count { margin: 0 0 0.5rem; font-size: 0.8rem; color: #9ca3af; }
.seats { display: grid; grid-template-columns: repeat(5, 1fr); gap: 0.75rem; }
.seat-card { background: rgba(0,0,0,0.25); border-radius: 8px; padding: 0.75rem; text-align: center; border: 1px solid transparent; }
.seat-card.me { border-color: rgba(34,197,94,0.6); }
.seat-card.empty .seat-name { color: #4b5563; }
.seat-card.disconnected { opacity: 0.6; }
.seat-name { font-weight: 600; font-size: 0.9rem; }
.you-badge { background: #16a34a; color: #fff; font-size: 0.6rem; padding: 0 4px; border-radius: 3px; margin-left: 4px; vertical-align: middle; }
.seat-state { font-size: 0.7rem; color: #6b7280; margin-top: 0.2rem; }
.host-controls { display: flex; gap: 0.25rem; justify-content: center; margin-top: 0.4rem; }
.btn-sm { font-size: 0.7rem; padding: 0.15rem 0.4rem; background: #374151; }

.chat { background: rgba(0,0,0,0.2); border-radius: 8px; overflow: hidden; }
.chat-messages { height: 180px; overflow-y: auto; padding: 0.75rem; display: flex; flex-direction: column; gap: 0.35rem; }
.chat-msg { font-size: 0.85rem; }
.chat-from { color: #9ca3af; margin-right: 0.4rem; }
.chat-msg.system .chat-from { color: #f59e0b; }
.chat-empty { color: #4b5563; font-style: italic; font-size: 0.85rem; }
.chat-input-row { display: flex; gap: 0.5rem; padding: 0.5rem 0.75rem; border-top: 1px solid rgba(255,255,255,0.08); }
.chat-input-row input { flex: 1; }

.start-area { text-align: center; padding: 0.5rem; }
.btn-start { background: #15803d; font-size: 1rem; padding: 0.6rem 2rem; }
.btn-start:hover { background: #166534; }
.hint { color: #9ca3af; font-size: 0.85rem; margin: 0 0 0.5rem; }
.waiting { color: #6b7280; font-style: italic; }
.countdown { color: #fbbf24; font-size: 1.1rem; font-weight: 700; margin-top: 0.5rem; }
</style>
