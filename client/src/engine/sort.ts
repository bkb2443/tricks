import type { Card, Suit, Trick } from './types'

// ---------------------------------------------------------------------------
// Sheepshead trump logic (mirrors server-side rules)
// ---------------------------------------------------------------------------

/** Returns trump rank (higher = stronger) or null if the card is not trump. */
function trumpRank(card: Card): number | null {
  if (card.rank === 'queen') {
    const r: Partial<Record<Suit, number>> = { clubs: 14, spades: 13, hearts: 12, diamonds: 11 }
    return r[card.suit] ?? null
  }
  if (card.rank === 'jack') {
    const r: Partial<Record<Suit, number>> = { clubs: 10, spades: 9, hearts: 8, diamonds: 7 }
    return r[card.suit] ?? null
  }
  if (card.suit === 'diamonds') {
    const r: Partial<Record<Card['rank'], number>> = {
      ace: 6, ten: 5, king: 4, nine: 3, eight: 2, seven: 1,
    }
    return r[card.rank] ?? null
  }
  return null
}

/** Strength of a card within its plain (non-trump) suit. Higher = stronger. */
function plainRank(card: Card): number {
  const r: Partial<Record<Card['rank'], number>> = {
    ace: 6, ten: 5, king: 4, nine: 3, eight: 2, seven: 1,
  }
  return r[card.rank] ?? 0
}

/** Effective suit for trick-following purposes: trump cards all share 'trump'. */
function effectiveSuit(card: Card): string {
  return trumpRank(card) !== null ? 'trump' : card.suit
}

// ---------------------------------------------------------------------------
// Exports
// ---------------------------------------------------------------------------

/** Fail suit display order (higher = displayed first, after trump). */
const SUIT_ORDER: Partial<Record<Suit, number>> = { clubs: 3, spades: 2, hearts: 1, diamonds: 0 }

/**
 * Sort a hand for display: trump high→low, then fail suits (clubs, spades, hearts)
 * each sorted high→low by plain rank. Does not mutate the input array.
 *
 * This is the default Sheepshead sort strategy. The function is intentionally
 * standalone so alternative strategies can be swapped in later.
 */
export function sortHand(cards: Card[]): Card[] {
  return [...cards].sort((a, b) => {
    const ta = trumpRank(a)
    const tb = trumpRank(b)

    if (ta !== null && tb === null) return -1  // trump before fail
    if (ta === null && tb !== null) return 1

    if (ta !== null && tb !== null) return tb - ta  // higher trump first

    // Both fail: sort by suit order first, then by plain rank
    const suitDiff = (SUIT_ORDER[b.suit] ?? 0) - (SUIT_ORDER[a.suit] ?? 0)
    return suitDiff !== 0 ? suitDiff : plainRank(b) - plainRank(a)
  })
}

/**
 * Returns the index within `trick.plays` of the currently winning play.
 * Returns -1 if the trick has no plays.
 * Works on partial (in-progress) tricks.
 */
export function trickWinnerIndex(trick: Trick): number {
  if (trick.plays.length === 0) return -1

  let bestIdx = 0
  let bestTrump = trumpRank(trick.plays[0][1])
  const ledSuit = effectiveSuit(trick.plays[0][1])

  for (let i = 1; i < trick.plays.length; i++) {
    const card = trick.plays[i][1]
    const t = trumpRank(card)
    let beats = false

    if (bestTrump === null && t !== null) {
      beats = true
    } else if (bestTrump !== null && t !== null) {
      beats = t > bestTrump
    } else if (bestTrump === null && t === null) {
      beats =
        effectiveSuit(card) === ledSuit &&
        plainRank(card) > plainRank(trick.plays[bestIdx][1])
    }

    if (beats) {
      bestIdx = i
      bestTrump = t
    }
  }

  return bestIdx
}

// ---------------------------------------------------------------------------
// Euchre trump logic
// ---------------------------------------------------------------------------

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

function euchreEffectiveSuit(card: Card, calledSuit: Suit): string {
  return euchreTrumpRank(card, calledSuit) !== null ? 'trump' : card.suit
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

/**
 * Returns the index within `trick.plays` of the currently winning play for Euchre.
 * Returns -1 if the trick has no plays or no trump has been called.
 * Works on partial (in-progress) tricks.
 */
export function trickWinnerIndexEuchre(trick: Trick, calledSuit: Suit | null): number {
  if (trick.plays.length === 0 || !calledSuit) return -1
  let bestIdx = 0
  let bestTrump = euchreTrumpRank(trick.plays[0][1], calledSuit)
  const ledSuit = euchreEffectiveSuit(trick.plays[0][1], calledSuit)
  for (let i = 1; i < trick.plays.length; i++) {
    const card = trick.plays[i][1]
    const t = euchreTrumpRank(card, calledSuit)
    let beats = false
    if (bestTrump === null && t !== null) beats = true
    else if (bestTrump !== null && t !== null) beats = t > bestTrump
    else if (bestTrump === null && t === null) {
      beats = euchreEffectiveSuit(card, calledSuit) === ledSuit && euchrePlainRank(card) > euchrePlainRank(trick.plays[bestIdx][1])
    }
    if (beats) { bestIdx = i; bestTrump = t }
  }
  return bestIdx
}
