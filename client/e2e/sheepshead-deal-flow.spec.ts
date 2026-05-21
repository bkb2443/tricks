/**
 * E2E: full Sheepshead deal → pick → bury → call → begin play
 *
 * Uses page.routeWebSocket to intercept the WebSocket and drive the server
 * side with scripted responses, so no real Rust server is needed.
 *
 * FAILING BEFORE FIX: step 3 (calling phase) never appears because
 * BiddingPanel.isBuryPhase does not check meta.sub_phase, so the bury
 * template keeps winning the v-else-if chain even after the picker has buried.
 */
import { test, expect, type WebSocketRoute } from '@playwright/test'

const SEAT = 1

const INITIAL_HAND = [
  { suit: 'clubs',  rank: 'ace'  },
  { suit: 'clubs',  rank: 'king' },
  { suit: 'spades', rank: 'ace'  },
  { suit: 'spades', rank: 'king' },
  { suit: 'hearts', rank: 'ace'  },
  { suit: 'hearts', rank: 'king' },
]

const HAND_WITH_BLIND = [
  ...INITIAL_HAND,
  { suit: 'diamonds', rank: 'seven' },
  { suit: 'diamonds', rank: 'eight' },
]

const HAND_AFTER_BURY = INITIAL_HAND

const INITIAL_STATE = {
  game_id: '00000000-0000-0000-0000-000000000001',
  game_name: 'sheepshead',
  phase: 'bidding',
  player_count: 5,
  dealer: 0,
  current_player: SEAT,
  hands: [[], INITIAL_HAND, [], [], []],
  extra_piles: [],
  current_trick: null,
  completed_tricks: [],
  scores: [0, 0, 0, 0, 0],
  meta: {
    picker: null,
    sub_phase: 'picking',
    passed: 0,
    buried: [],
    leaster: false,
    callable_suits: [],
    called_suit: null,
    going_alone: false,
    partner: null,
  },
  names: ['Bot 1', 'You', 'Bot 2', 'Bot 3', 'Bot 4'],
}

function send(ws: WebSocketRoute, msg: object) {
  ws.send(JSON.stringify(msg))
}

test.describe('sheepshead deal flow', () => {
  test.beforeEach(async ({ page }) => {
    await page.routeWebSocket('**/ws', ws => {
      // Bootstrap immediately on connect: join + deal
      send(ws, { type: 'joined_room', room_id: 'test-room', seat: SEAT, room_code: 'TEST01' })
      send(ws, { type: 'snapshot', state: INITIAL_STATE })

      ws.onMessage(raw => {
        const msg = JSON.parse(raw as string)
        if (msg.type !== 'bid') return

        if (msg.value.action === 'pick') {
          send(ws, {
            type: 'bid_placed',
            player: SEAT,
            value: { picker: SEAT, sub_phase: 'burying' },
            current_player: SEAT,
          })
          send(ws, { type: 'hand_updated', hand: HAND_WITH_BLIND })
        } else if (msg.value.action === 'bury') {
          send(ws, {
            type: 'bid_placed',
            player: SEAT,
            value: { sub_phase: 'calling', callable_suits: ['clubs'] },
            current_player: SEAT,
          })
          send(ws, { type: 'hand_updated', hand: HAND_AFTER_BURY })
        } else if (msg.value.action === 'go_alone') {
          send(ws, {
            type: 'bid_placed',
            player: SEAT,
            value: { going_alone: true, called_suit: null },
            current_player: SEAT,
          })
          send(ws, { type: 'phase_changed', phase: 'playing' })
        }
      })
    })

    await page.goto('/game')
  })

  test('picking phase shows pick/pass prompt', async ({ page }) => {
    await expect(page.getByText('Do you want to pick the blind?')).toBeVisible()
    await expect(page.getByRole('button', { name: 'Pick' })).toBeEnabled()
    await expect(page.getByRole('button', { name: 'Pass' })).toBeEnabled()
  })

  test('picking → bury: bury UI appears after clicking Pick', async ({ page }) => {
    await page.getByRole('button', { name: 'Pick' }).click()

    await expect(page.getByText(/Select.*2 cards.*to bury/i)).toBeVisible()
    // Hand grew from 6 to 8 cards — blind cards are present
    await expect(page.getByRole('button', { name: '7 of diamonds' })).toBeVisible()
    await expect(page.getByRole('button', { name: '8 of diamonds' })).toBeVisible()
    // Bury button disabled until 2 cards selected
    await expect(page.getByRole('button', { name: 'Bury selected cards' })).toBeDisabled()
  })

  test('bury → calling: calling UI appears after burying 2 cards', async ({ page }) => {
    await page.getByRole('button', { name: 'Pick' }).click()
    await expect(page.getByText(/Select.*2 cards.*to bury/i)).toBeVisible()

    await page.getByRole('button', { name: '7 of diamonds' }).click()
    await page.getByRole('button', { name: '8 of diamonds' }).click()
    await page.getByRole('button', { name: 'Bury selected cards' }).click()

    // BUG: before fix this never renders — bury UI stays visible instead
    await expect(page.getByText('Choose your partner card or go alone')).toBeVisible()
    await expect(page.getByText(/Select.*2 cards.*to bury/i)).not.toBeVisible()
  })

  test('calling → playing: game starts after going alone', async ({ page }) => {
    await page.getByRole('button', { name: 'Pick' }).click()
    await expect(page.getByText(/Select.*2 cards.*to bury/i)).toBeVisible()
    await page.getByRole('button', { name: '7 of diamonds' }).click()
    await page.getByRole('button', { name: '8 of diamonds' }).click()
    await page.getByRole('button', { name: 'Bury selected cards' }).click()
    await expect(page.getByText('Choose your partner card or go alone')).toBeVisible()

    await page.getByRole('button', { name: /Go Alone/i }).click()

    // Bidding panel gone; phase badge (inside the table header) flips to Playing.
    // Use a scoped locator to avoid matching the brief phase-change toast that
    // also contains the text "Playing" during the transition animation.
    await expect(page.getByText('Do you want to pick the blind?')).not.toBeVisible()
    await expect(page.getByText(/Select.*2 cards.*to bury/i)).not.toBeVisible()
    await expect(page.getByText('Choose your partner card or go alone')).not.toBeVisible()
    await expect(page.locator('header').getByText('Playing')).toBeVisible()
  })
})
