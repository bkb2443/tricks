<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import type { ChatMessage } from '@/engine/types'

const props = defineProps<{
  messages: ChatMessage[]
}>()

const emit = defineEmits<{
  send: [text: string]
}>()

const chatInput = ref('')
const chatEl = ref<HTMLElement | null>(null)

function sendChat() {
  const text = chatInput.value.trim()
  if (!text) return
  emit('send', text)
  chatInput.value = ''
}

watch(
  () => props.messages.length,
  async () => {
    await nextTick()
    if (chatEl.value) chatEl.value.scrollTop = chatEl.value.scrollHeight
  },
)
</script>

<template>
  <div ref="chatEl" class="chat-messages">
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
      v-model="chatInput"
      placeholder="Say something…"
      maxlength="200"
      @keydown.enter="sendChat"
    />
    <button @click="sendChat" :disabled="!chatInput.trim()">Send</button>
  </div>
</template>

<style scoped>
.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
}
.chat-msg { font-size: 0.85rem; }
.chat-from { color: var(--color-text-muted, #9ca3af); margin-right: 0.4rem; }
.chat-msg.system .chat-from { color: var(--color-warning, #f59e0b); }
.chat-empty { color: var(--color-text-dim, #4b5563); font-style: italic; font-size: 0.85rem; }
.chat-input-row {
  display: flex;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
  flex-shrink: 0;
}
.chat-input-row input { flex: 1; }
</style>
