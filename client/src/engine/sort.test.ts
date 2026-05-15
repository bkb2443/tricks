import { describe, it, expect } from 'vitest'
import { sortHand, trickWinnerIndex } from './sort'
import type { Card, Trick } from './types'

function card(suit: Card['suit'], rank: Card['rank']): Card {
  return { suit, rank }
}

describe('sortHand', () => {
  it('puts trump before fail cards', () => {
    const hand = [
      card('clubs', 'ace'),    // fail
      card('clubs', 'queen'),  // trump (rank 14)
      card('hearts', 'seven'), // fail
    ]
    const sorted = sortHand(hand)
    expect(sorted[0]).toEqual(card('clubs', 'queen'))
  })

  it('sorts trump high-to-low by rank', () => {
    const hand = [
      card('diamonds', 'jack'), // trump rank 7
      card('clubs', 'queen'),   // trump rank 14
      card('spades', 'jack'),   // trump rank 9
    ]
    const sorted = sortHand(hand)
    expect(sorted[0]).toEqual(card('clubs', 'queen'))   // 14
    expect(sorted[1]).toEqual(card('spades', 'jack'))   // 9
    expect(sorted[2]).toEqual(card('diamonds', 'jack')) // 7
  })

  it('groups fail cards by suit: clubs, spades, hearts', () => {
    const hand = [
      card('hearts', 'ace'),
      card('spades', 'ace'),
      card('clubs', 'ace'),
    ]
    const sorted = sortHand(hand)
    expect(sorted[0].suit).toBe('clubs')
    expect(sorted[1].suit).toBe('spades')
    expect(sorted[2].suit).toBe('hearts')
  })

  it('sorts within a fail suit high-to-low: A > 10 > K > 9 > 8 > 7', () => {
    const hand = [
      card('clubs', 'seven'),
      card('clubs', 'ace'),
      card('clubs', 'king'),
      card('clubs', 'ten'),
    ]
    const sorted = sortHand(hand)
    expect(sorted.map(c => c.rank)).toEqual(['ace', 'ten', 'king', 'seven'])
  })

  it('does not mutate the original array', () => {
    const hand = [card('clubs', 'ace'), card('clubs', 'queen')]
    const original = [...hand]
    sortHand(hand)
    expect(hand).toEqual(original)
  })
})

describe('trickWinnerIndex', () => {
  it('returns 0 for a single-card trick', () => {
    const trick: Trick = {
      led_by: 0,
      plays: [[0, card('clubs', 'ace')]],
      winner: null,
    }
    expect(trickWinnerIndex(trick)).toBe(0)
  })

  it('trump beats fail regardless of rank', () => {
    const trick: Trick = {
      led_by: 0,
      plays: [
        [0, card('clubs', 'ace')],      // led: fail ace
        [1, card('diamonds', 'seven')], // trump (rank 1)
      ],
      winner: null,
    }
    expect(trickWinnerIndex(trick)).toBe(1) // trump wins
  })

  it('higher trump beats lower trump', () => {
    const trick: Trick = {
      led_by: 0,
      plays: [
        [0, card('diamonds', 'seven')], // trump rank 1
        [1, card('spades', 'queen')],   // trump rank 13
        [2, card('clubs', 'jack')],     // trump rank 10
      ],
      winner: null,
    }
    expect(trickWinnerIndex(trick)).toBe(1) // Q♠ wins
  })

  it('within fail suit, led suit beats off suit, higher rank wins', () => {
    const trick: Trick = {
      led_by: 0,
      plays: [
        [0, card('clubs', 'seven')], // led clubs
        [1, card('clubs', 'ace')],   // clubs ace — wins
        [2, card('hearts', 'ace')],  // off suit — doesn't count
      ],
      winner: null,
    }
    expect(trickWinnerIndex(trick)).toBe(1) // A♣ wins
  })

  it('returns -1 for empty trick', () => {
    const trick: Trick = { led_by: 0, plays: [], winner: null }
    expect(trickWinnerIndex(trick)).toBe(-1)
  })
})
