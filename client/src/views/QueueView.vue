<script setup lang="ts">
import { watch } from 'vue'
import { useRouter } from 'vue-router'
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'

const router = useRouter()
const store  = useGameStore()
const { leaveQueue } = useGame()

// When matchmaker assigns a room, navigate to lobby
watch(() => store.roomId, (id) => {
  if (id) router.push('/lobby')
})

function handleCancel() {
  leaveQueue()
  router.push('/')
}
</script>

<template>
  <div class="queue">
    <h1>Finding a Game…</h1>
    <div v-if="store.queueStatus" class="status">
      <p>Position in queue: <strong>{{ store.queueStatus.position }}</strong></p>
    </div>
    <p class="hint">You'll be matched automatically. Bots fill any empty seats.</p>
    <button class="btn-cancel" @click="handleCancel">Cancel</button>
  </div>
</template>

<style scoped>
.queue { max-width: 400px; margin: 6rem auto; text-align: center; display: flex; flex-direction: column; align-items: center; gap: 1rem; }
h1 { font-size: 2rem; }
.status { background: rgba(0,0,0,0.2); border-radius: 8px; padding: 1rem 2rem; }
.hint { color: #6b7280; font-size: 0.85rem; }
.btn-cancel { background: #6b7280; }
.btn-cancel:hover { background: #4b5563; }
</style>
