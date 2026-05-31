import { test, expect, type WebSocketRoute } from '@playwright/test'

const SEAT = 0

// GameState with training_mode=true, hint_enabled=false, in Playing phase
// Player's turn to lead (no current_trick), with legal_cards populated
const TRAINING_STATE = {
  game_id: '00000000-0000-0000-0000-000000000002',
  game_name: 'sheepshead',
  phase: 'playing',
  player_count: 5,
  dealer: 4,
  current_player: SEAT,
  hands: [
    [
      { suit: 'clubs', rank: 'queen' },
      { suit: 'spades', rank: 'queen' },
      { suit: 'hearts', rank: 'jack' },
      { suit: 'clubs', rank: 'nine' },
    ],
    [], [], [], [],
  ],
  extra_piles: [],
  current_trick: null,
  completed_tricks: [],
  scores: [0, 0, 0, 0, 0],
  session_scores: [0, 0, 0, 0, 0],
  meta: {
    kind: 'sheepshead',
    picker: SEAT,
    sub_phase: 'done',
    passed: 0,
    leaster: false,
    buried: [],
    callable_suits: [],
    called_suit: null,
    going_alone: false,
    partner: null,
  },
  names: ['You', 'Bot 1', 'Bot 2', 'Bot 3', 'Bot 4'],
  training_mode: true,
  hint_enabled: false,
  legal_cards: [
    { suit: 'clubs', rank: 'queen' },
    { suit: 'spades', rank: 'queen' },
    { suit: 'hearts', rank: 'jack' },
    { suit: 'clubs', rank: 'nine' },
  ],
  hint: null,
}

// Same state but with a hint card
const TRAINING_STATE_WITH_HINT = {
  ...TRAINING_STATE,
  hint_enabled: true,
  hint: { card: { suit: 'clubs', rank: 'queen' }, reason: 'Lead trump to draw defenders.' },
}

function send(ws: WebSocketRoute, msg: object) {
  ws.send(JSON.stringify(msg))
}

test.describe('training mode', () => {
  test('training toggle is visible on home page and off by default', async ({ page }) => {
    await page.goto('/')
    // Training mode toggle should be present in the solo section
    const toggle = page.getByLabel(/training mode/i)
    await expect(toggle).toBeVisible()
    await expect(toggle).not.toBeChecked()
  })

  test('tutorial list appears when training toggle is enabled', async ({ page }) => {
    // Mock the /api/training/tutorials/sheepshead endpoint
    await page.route('**/api/training/tutorials/sheepshead', async route => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          { id: 'sheepshead-picking', title: 'Picking Hand', description: 'Learn to pick.' },
          { id: 'sheepshead-partner', title: 'Defender Hand', description: 'Play as partner.' },
        ]),
      })
    })

    await page.goto('/')
    await page.getByLabel(/training mode/i).check()

    // Tutorial list should appear
    await expect(page.getByText('Picking Hand')).toBeVisible()
    await expect(page.getByText('Defender Hand')).toBeVisible()
    await expect(page.getByRole('button', { name: /free training/i })).toBeVisible()
  })

  test('narration banner visible when server sends tutorial_narration', async ({ page }) => {
    await page.routeWebSocket('**/ws', ws => {
      send(ws, { type: 'joined_room', room_id: 'test', seat: SEAT, room_code: 'TRN01' })
      send(ws, { type: 'snapshot', state: TRAINING_STATE })
      // Send narration right away
      send(ws, { type: 'tutorial_narration', text: 'Lead trump to draw out defenders.' })
    })

    await page.goto('/game')
    await expect(page.getByText('Lead trump to draw out defenders.')).toBeVisible()
  })

  test('hint text visible when hint is active', async ({ page }) => {
    await page.routeWebSocket('**/ws', ws => {
      send(ws, { type: 'joined_room', room_id: 'test', seat: SEAT, room_code: 'TRN02' })
      send(ws, { type: 'snapshot', state: TRAINING_STATE_WITH_HINT })
    })

    await page.goto('/game')
    await expect(page.getByText('Lead trump to draw defenders.')).toBeVisible()
  })

  test('rules panel opens and closes', async ({ page }) => {
    await page.routeWebSocket('**/ws', ws => {
      send(ws, { type: 'joined_room', room_id: 'test', seat: SEAT, room_code: 'TRN03' })
      send(ws, { type: 'snapshot', state: TRAINING_STATE })
    })

    await page.goto('/game')

    // Rules button should be visible in training mode
    const rulesBtn = page.getByRole('button', { name: /rules/i })
    await expect(rulesBtn).toBeVisible()
    await rulesBtn.click()

    // Rules panel content should appear
    await expect(page.getByText('Trump (14 cards')).toBeVisible()

    // Close it
    await page.getByRole('button', { name: '✕' }).click()
    await expect(page.getByText('Trump (14 cards')).not.toBeVisible()
  })

  test('training UI absent when training_mode is false', async ({ page }) => {
    const normalState = { ...TRAINING_STATE, training_mode: false, legal_cards: [], hint: null }
    await page.routeWebSocket('**/ws', ws => {
      send(ws, { type: 'joined_room', room_id: 'test', seat: SEAT, room_code: 'TRN04' })
      send(ws, { type: 'snapshot', state: normalState })
    })

    await page.goto('/game')

    // No training UI elements
    await expect(page.getByRole('button', { name: /rules/i })).not.toBeVisible()
    await expect(page.getByRole('button', { name: /hint/i })).not.toBeVisible()
  })
})
