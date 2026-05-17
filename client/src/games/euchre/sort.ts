import type { Card, Suit } from '@/engine/types'
import { SUIT_ORDER } from '@/engine/sort'

function sameColorSuit(suit: Suit): Suit {
  const map: Record<Suit, Suit> = { clubs: 'spades', spades: 'clubs', hearts: 'diamonds', diamonds: 'hearts' }
  return map[suit]
}

function euchreTrumpRank(card: Card, calledSuit: Suit): number | null {
  if (card.rank === 'jack' && card.suit === calledSuit) return 8          // Right Bower
  if (card.rank === 'jack' && card.suit === sameColorSuit(calledSuit)) return 7  // Left Bower
  if (card.suit === calledSuit) {
    const r: Partial<Record<Card['rank'], number>> = { ace: 6, king: 5, queen: 4, ten: 3, nine: 2 }
    return r[card.rank] ?? null
  }
  return null
}

function euchrePlainRank(card: Card): number {
  const r: Partial<Record<Card['rank'], number>> = { ace: 6, king: 5, queen: 4, jack: 3, ten: 2, nine: 1 }
  return r[card.rank] ?? 0
}

/**
 * Sort a Euchre hand for display: trump high→low, then fail suits each sorted high→low.
 * If no trump has been called yet, preserves original order.
 * Does not mutate the input array.
 */
export function sortHandEuchre(cards: Card[], calledSuit: Suit | null): Card[] {
  if (!calledSuit) return [...cards]  // no trump called yet — preserve order
  return [...cards].sort((a, b) => {
    const ta = euchreTrumpRank(a, calledSuit)
    const tb = euchreTrumpRank(b, calledSuit)
    if (ta !== null && tb === null) return -1
    if (ta === null && tb !== null) return 1
    if (ta !== null && tb !== null) return tb - ta
    const suitDiff = (SUIT_ORDER[b.suit] ?? 0) - (SUIT_ORDER[a.suit] ?? 0)
    return suitDiff !== 0 ? suitDiff : euchrePlainRank(b) - euchrePlainRank(a)
  })
}
