export interface GameInfo {
  name: string
  label: string
  playerCount: number
  description: string
}

export const GAMES: GameInfo[] = [
  { name: 'sheepshead', label: 'Sheepshead', playerCount: 5, description: '5 players' },
  { name: 'euchre',     label: 'Euchre',     playerCount: 4, description: '4 players' },
]

export function getGameInfo(name: string): GameInfo | undefined {
  return GAMES.find(g => g.name === name)
}
