import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import HandReplay from './HandReplay.vue'
import type { Card, Trick } from '@/engine/types'

function card(suit: Card['suit'], rank: Card['rank']): Card {
  return { suit, rank }
}

function trick(plays: [number, Card][], winner: number): Trick {
  return { led_by: plays[0]![0], plays, winner }
}

const NAMES = ['Alice', 'Bob', 'Carol', 'Dave', 'Eve']

function sampleTricks(): Trick[] {
  return [
    trick(
      [
        [0, card('clubs', 'ace')],
        [1, card('clubs', 'king')],
        [2, card('clubs', 'queen')],
      ],
      0,
    ),
    trick(
      [
        [0, card('spades', 'ten')],
        [1, card('spades', 'jack')],
        [2, card('spades', 'nine')],
      ],
      1,
    ),
  ]
}

describe('HandReplay', () => {
  beforeEach(() => { vi.useFakeTimers() })
  afterEach(() => {
    vi.runOnlyPendingTimers()
    vi.useRealTimers()
  })

  it('shows the first trick on open with player names and step indicator', () => {
    const w = mount(HandReplay, {
      props: { tricks: sampleTricks(), names: NAMES, mySeat: 0, bury: null },
    })
    expect(w.text()).toContain('1st trick')
    expect(w.text()).toContain('1 / 2')
    expect(w.text()).toContain('You') // mySeat=0 is "You"
    expect(w.text()).toContain('Bob')
    expect(w.text()).toContain('Carol')
    expect(w.text()).toContain('You wins') // winner banner for seat 0
  })

  it('step forward advances to the next trick', async () => {
    const w = mount(HandReplay, {
      props: { tricks: sampleTricks(), names: NAMES, mySeat: 0, bury: null },
    })
    await w.find('button.ctrl-btn:nth-of-type(3)').trigger('click') // Next
    expect(w.text()).toContain('2nd trick')
    expect(w.text()).toContain('2 / 2')
    expect(w.text()).toContain('Bob wins')
  })

  it('step backward returns to the previous trick', async () => {
    const w = mount(HandReplay, {
      props: { tricks: sampleTricks(), names: NAMES, mySeat: 0, bury: null },
    })
    const buttons = w.findAll('button.ctrl-btn')
    await buttons[2]!.trigger('click') // Next
    await buttons[0]!.trigger('click') // Prev
    expect(w.text()).toContain('1st trick')
  })

  it('Prev is disabled on the first step; Next is disabled on the last step', async () => {
    const w = mount(HandReplay, {
      props: { tricks: sampleTricks(), names: NAMES, mySeat: 0, bury: null },
    })
    const buttons = w.findAll('button.ctrl-btn')
    expect(buttons[0]!.attributes('disabled')).toBeDefined()
    await buttons[2]!.trigger('click') // Next to last
    expect(buttons[2]!.attributes('disabled')).toBeDefined()
    expect(buttons[0]!.attributes('disabled')).toBeUndefined()
  })

  it('auto-advance moves through steps on a fixed timer and stops at end', async () => {
    const w = mount(HandReplay, {
      props: { tricks: sampleTricks(), names: NAMES, mySeat: 0, bury: null },
    })
    const playBtn = w.find('button.play-btn')
    await playBtn.trigger('click') // start auto
    expect(playBtn.text()).toContain('Pause')

    await vi.advanceTimersByTimeAsync(3000)
    expect(w.text()).toContain('2nd trick')

    // last step, one more tick stops auto
    await vi.advanceTimersByTimeAsync(3000)
    expect(playBtn.text()).toContain('Auto')
  })

  it('manual nav stops auto-advance', async () => {
    const w = mount(HandReplay, {
      props: { tricks: sampleTricks(), names: NAMES, mySeat: 0, bury: null },
    })
    const buttons = w.findAll('button.ctrl-btn')
    await w.find('button.play-btn').trigger('click') // start auto
    expect(w.find('button.play-btn').text()).toContain('Pause')
    await buttons[2]!.trigger('click') // Next (manual)
    expect(w.find('button.play-btn').text()).toContain('Auto')
  })

  it('shows the bury step after the last trick when bury prop is provided', async () => {
    const bury = { picker: 1, cards: [card('hearts', 'king'), card('diamonds', 'seven')] }
    const w = mount(HandReplay, {
      props: { tricks: sampleTricks(), names: NAMES, mySeat: 0, bury },
    })
    const buttons = w.findAll('button.ctrl-btn')
    await buttons[2]!.trigger('click') // -> 2nd trick
    await buttons[2]!.trigger('click') // -> bury
    expect(w.text()).toContain('Bury')
    expect(w.text()).toContain('3 / 3')
    expect(w.text()).toContain('Bob buried')
  })

  it('does not add a bury step when bury is null', async () => {
    const w = mount(HandReplay, {
      props: { tricks: sampleTricks(), names: NAMES, mySeat: 0, bury: null },
    })
    expect(w.text()).toContain('1 / 2') // 2 tricks, no bury
    expect(w.text()).not.toContain('Bury')
  })

  it('does not add a bury step when bury.cards is empty', async () => {
    const w = mount(HandReplay, {
      props: {
        tricks: sampleTricks(),
        names: NAMES,
        mySeat: 0,
        bury: { picker: 1, cards: [] },
      },
    })
    expect(w.text()).toContain('1 / 2')
    expect(w.text()).not.toContain('Bury')
  })

  it('close button emits close event', async () => {
    const w = mount(HandReplay, {
      props: { tricks: sampleTricks(), names: NAMES, mySeat: 0, bury: null },
    })
    await w.find('button.close-btn').trigger('click')
    expect(w.emitted('close')).toBeTruthy()
  })

  it('handles empty trick list gracefully', () => {
    const w = mount(HandReplay, {
      props: { tricks: [], names: NAMES, mySeat: 0, bury: null },
    })
    expect(w.text()).toContain('No tricks to replay')
    const buttons = w.findAll('button.ctrl-btn')
    expect(buttons[0]!.attributes('disabled')).toBeDefined()
    expect(buttons[1]!.attributes('disabled')).toBeDefined()
    expect(buttons[2]!.attributes('disabled')).toBeDefined()
  })
})
