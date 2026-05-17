import { describe, it, expect } from 'vitest'
import { sortHand } from './sort'
import type { Card } from './types'

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
