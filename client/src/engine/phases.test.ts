import { describe, it, expect } from 'vitest'
import { phaseLabel } from './phases'

describe('phaseLabel', () => {
  it('returns "Picking" for Sheepshead bidding phase', () => {
    expect(phaseLabel('sheepshead', 'bidding')).toBe('Picking')
  })

  it('returns "Calling" for Euchre bidding phase', () => {
    expect(phaseLabel('euchre', 'bidding')).toBe('Calling')
  })

  it('returns "Bidding" for Spades bidding phase', () => {
    expect(phaseLabel('spades', 'bidding')).toBe('Bidding')
  })

  it('returns "Bidding" as fallback for unknown game', () => {
    expect(phaseLabel('unknown_game', 'bidding')).toBe('Bidding')
  })

  it('returns "Playing" for any game in playing phase', () => {
    expect(phaseLabel('sheepshead', 'playing')).toBe('Playing')
    expect(phaseLabel('euchre', 'playing')).toBe('Playing')
  })

  it('returns "Scoring" for any game in scoring phase', () => {
    expect(phaseLabel('sheepshead', 'scoring')).toBe('Scoring')
  })
})
