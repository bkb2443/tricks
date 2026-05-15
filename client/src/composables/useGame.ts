/**
 * Thin action layer over the socket + store.
 * Import this in components instead of calling sendMessage directly.
 */
import { useGameStore } from '@/stores/game'
import { sendMessage, connected } from '@/engine/socket'
import type { Card } from '@/engine/types'

export function useGame() {
  const store = useGameStore()

  function createRoom(game: string, players: number): void {
    store.isSolo = false
    sendMessage({ type: 'join_room', game, players })
  }

  function createSoloRoom(game: string, players: number): void {
    store.isSolo = true
    sendMessage({ type: 'join_room', game, players, fill_bots: true })
  }

  function joinRoom(roomId: string, game = 'sheepshead', players = 5): void {
    store.isSolo = false
    sendMessage({ type: 'join_room', room_id: roomId, game, players })
  }

  function playCard(card: Card): void {
    sendMessage({ type: 'play_card', card })
  }

  // ── Sheepshead bidding actions ─────────────────────────────────────────────

  function pick(): void {
    sendMessage({ type: 'bid', value: { action: 'pick' } })
  }

  function pass(): void {
    sendMessage({ type: 'bid', value: { action: 'pass' } })
  }

  function bury(cards: [Card, Card]): void {
    sendMessage({ type: 'bid', value: { action: 'bury', cards } })
  }

  function callAce(suit: string): void {
    sendMessage({ type: 'bid', value: { action: 'call', suit } })
  }

  function goAlone(): void {
    sendMessage({ type: 'bid', value: { action: 'go_alone' } })
  }

  return { store, connected, createRoom, createSoloRoom, joinRoom, playCard, pick, pass, bury, callAce, goAlone }
}
