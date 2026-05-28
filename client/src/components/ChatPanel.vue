<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'

export interface ChatMessage {
  from: string
  text: string
  timestamp: number
}

const props = defineProps<{
  messages: ChatMessage[]
  onSend: (text: string) => void
}>()

const input = ref('')
const listEl = ref<HTMLElement | null>(null)

watch(() => props.messages.length, async () => {
  await nextTick()
  if (listEl.value) listEl.value.scrollTop = listEl.value.scrollHeight
})

function send() {
  const t = input.value.trim()
  if (!t) return
  props.onSend(t)
  input.value = ''
}
</script>

<template>
  <div class="chat-panel">
    <div ref="listEl" class="chat-messages">
      <div
        v-for="(msg, i) in messages"
        :key="i"
        class="chat-msg"
        :class="{ system: msg.from === 'System' }"
      >
        <span class="chat-from">{{ msg.from }}:</span>
        <span class="chat-text">{{ msg.text }}</span>
      </div>
      <div v-if="messages.length === 0" class="chat-empty">No messages yet…</div>
    </div>
    <div class="chat-input-row">
      <input
        v-model="input"
        placeholder="Say something…"
        maxlength="200"
        @keydown.enter="send"
      />
      <button class="send-btn" @click="send" :disabled="!input.trim()">Send</button>
    </div>
  </div>
</template>

<style scoped>
.chat-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}
.chat-msg { font-size: 0.85rem; }
.chat-from { color: #9ca3af; margin-right: 0.4rem; }
.chat-msg.system .chat-from { color: #f59e0b; }
.chat-empty { color: #4b5563; font-style: italic; font-size: 0.85rem; }
.chat-input-row {
  display: flex;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  border-top: 1px solid rgba(255,255,255,0.08);
}
.chat-input-row input { flex: 1; }
.send-btn { white-space: nowrap; }
</style>
