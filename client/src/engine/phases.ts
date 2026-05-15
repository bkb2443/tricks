import type { GamePhase } from './types'

const BIDDING_LABELS: Record<string, string> = {
  sheepshead: 'Picking',
  euchre:     'Calling',
  spades:     'Bidding',
  hearts:     'Playing',
}

/**
 * Returns a game-specific display label for a phase.
 * The raw phase names ('bidding', 'playing', 'scoring') are engine terms;
 * this maps them to player-facing language per game.
 */
export function phaseLabel(gameName: string, phase: GamePhase): string {
  if (phase === 'bidding') {
    return BIDDING_LABELS[gameName] ?? 'Bidding'
  }
  return phase.charAt(0).toUpperCase() + phase.slice(1)
}
