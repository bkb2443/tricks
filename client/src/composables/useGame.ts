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

  function joinWithCode(name: string, roomCode: string): void {
    store.isSolo = false
    sendMessage({ type: 'join', name, room_code: roomCode })
  }

  function createPrivateRoom(game: string, maxHands: number | null, name: string): void {
    store.isSolo = false
    sendMessage({ type: 'create_room', name, game, max_hands: maxHands })
  }

  function joinQueue(): void {
    sendMessage({ type: 'join_queue' })
  }

  function leaveQueue(): void {
    sendMessage({ type: 'leave_queue' })
  }

  function startGame(): void {
    sendMessage({ type: 'start_game' })
  }

  function sendLobbyChat(text: string): void {
    sendMessage({ type: 'lobby_chat', text })
  }

  function forceBot(seat: number): void {
    sendMessage({ type: 'force_bot', seat })
  }

  function extendRejoin(seat: number): void {
    sendMessage({ type: 'extend_rejoin', seat })
  }

  // ── Euchre bidding actions ────────────────────────────────────────────────

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

  return {
    store, connected,
    createRoom, createSoloRoom, joinRoom, playCard, pick, pass, bury, callAce, goAlone,
    joinWithCode, createPrivateRoom, joinQueue, leaveQueue, startGame, sendLobbyChat, forceBot, extendRejoin,
    orderUp, euchrePass, discard, callSuit,
  }
}
