import { describe, it, expect } from 'vitest'
import { sortHandEuchre } from './sort'
import type { Card } from '@/engine/types'

function card(suit: Card['suit'], rank: Card['rank']): Card {
  return { suit, rank }
}

const JS = card('spades',   'jack')  // Right Bower when spades is trump
const JC = card('clubs',    'jack')  // Left  Bower when spades is trump
const AS = card('spades',   'ace')
const KS = card('spades',   'king')
const QS = card('spades',   'queen')
const TS = card('spades',   'ten')
const NS = card('spades',   'nine')
const AH = card('hearts',   'ace')
const KH = card('hearts',   'king')
const JH = card('hearts',   'jack')
const AD = card('diamonds', 'ace')

describe('sortHandEuchre', () => {
  it('preserves order when no trump called', () => {
    const hand = [AH, JS, KS]
    const sorted = sortHandEuchre(hand, null)
    expect(sorted).toEqual([AH, JS, KS])
  })

  it('does not mutate the original array', () => {
    const hand = [AH, JS, KS]
    const original = [...hand]
    sortHandEuchre(hand, 'spades')
    expect(hand).toEqual(original)
  })

  it('Right Bower ranks highest', () => {
    const sorted = sortHandEuchre([AS, AH, JS], 'spades')
    expect(sorted[0]).toEqual(JS)
  })

  it('Left Bower ranks second (above Ace of trump)', () => {
    const sorted = sortHandEuchre([AS, JC, AH], 'spades')
    expect(sorted[0]).toEqual(JC)
    expect(sorted[1]).toEqual(AS)
  })

  it('sorts full trump suit high-to-low: RB > LB > A > K > Q > 10 > 9', () => {
    const hand = [NS, TS, QS, KS, AS, JC, JS]
    const sorted = sortHandEuchre(hand, 'spades')
    expect(sorted.map(c => `${c.rank}${c.suit}`)).toEqual([
      'jackspades', 'jackclubs', 'acespades', 'kingspades',
      'queenspades', 'tenspades', 'ninespades',
    ])
  })

  it('puts all trump before non-trump', () => {
    const sorted = sortHandEuchre([AH, NS, AD], 'spades')
    expect(sorted[0]).toEqual(NS) // 9♠ is trump
  })

  it('Left Bower treated as trump, not as its natural suit', () => {
    // J♥ is Left Bower when diamonds is trump — should appear with trump, not hearts
    const JH_as_LB = card('hearts', 'jack')
    const sorted = sortHandEuchre([AH, JH_as_LB, AD], 'diamonds')
    expect(sorted[0]).toEqual(JH_as_LB) // Left Bower leads
    expect(sorted[1].suit).toBe('diamonds') // then trump
  })

  it('non-trump cards grouped by suit and sorted A > K > J > 10 > 9', () => {
    const hand = [JH, KH, AH]
    const sorted = sortHandEuchre(hand, 'spades')
    expect(sorted.map(c => c.rank)).toEqual(['ace', 'king', 'jack'])
  })
})
