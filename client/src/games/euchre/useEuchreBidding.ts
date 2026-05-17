import { sendMessage } from '@/engine/socket'
import type { Card } from '@/engine/types'

export function useEuchreBidding() {
  function orderUp(alone = false): void {
    sendMessage({ type: 'bid', value: { action: 'order_up', alone } })
  }

  function euchrePass(): void {
    sendMessage({ type: 'bid', value: { action: 'pass' } })
  }

  function discard(card: Card): void {
    sendMessage({ type: 'bid', value: { action: 'discard', card } })
  }

  function callSuit(suit: string, alone = false): void {
    sendMessage({ type: 'bid', value: { action: 'call', suit, alone } })
  }

  return { orderUp, euchrePass, discard, callSuit }
}
