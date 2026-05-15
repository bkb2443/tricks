/**
 * Module-level WebSocket singleton.
 *
 * Call `initSocket(handler)` once (in App.vue) to register the update handler,
 * then `connectSocket()` to open the connection. Use `sendMessage()` anywhere.
 */
import { ref } from 'vue'
import type { ClientMessage, StateUpdate } from './types'

const WS_URL = import.meta.env.VITE_WS_URL ?? `ws://${location.host}/ws`

export const connected = ref(false)

let ws: WebSocket | null = null
let messageHandler: ((update: StateUpdate) => void) | null = null

export function initSocket(handler: (update: StateUpdate) => void): void {
  messageHandler = handler
}

export function connectSocket(): void {
  if (ws && ws.readyState <= WebSocket.OPEN) return

  ws = new WebSocket(WS_URL)

  ws.onopen = () => {
    connected.value = true
    console.info('[socket] connected')
  }

  ws.onclose = (e) => {
    connected.value = false
    console.warn('[socket] closed', e.code, e.reason)
  }

  ws.onerror = (e) => {
    console.error('[socket] error', e)
  }

  ws.onmessage = (e: MessageEvent<string>) => {
    try {
      const update = JSON.parse(e.data) as StateUpdate
      messageHandler?.(update)
    } catch (err) {
      console.error('[socket] failed to parse message', e.data, err)
    }
  }
}

export function sendMessage(msg: ClientMessage): void {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(msg))
  } else {
    console.warn('[socket] send called while not connected', msg)
  }
}

export function disconnectSocket(): void {
  ws?.close()
  ws = null
}
