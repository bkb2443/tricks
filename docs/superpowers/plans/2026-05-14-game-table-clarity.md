# Game Table Clarity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the game table readable at a glance: show play order and the current winning card in the trick display, sort the player's hand by trump then suit, and give every seat a name (bots get "Bot 1", "Bot 2", etc.).

**Architecture:** Three independent improvements wired together: (1) server adds `names: Vec<String>` to `GameState` and populates it from bot seat info; (2) a new `sort.ts` module on the client handles `sortHand` and `trickWinnerIndex` using Sheepshead trump logic; (3) existing components (`TrickDisplay`, `HandComponent`, `GameTable`, `BiddingPanel`) consume names and sort/winner data. No new routes, no new WebSocket messages — names ride in the existing `Snapshot` payload.

**Tech Stack:** Rust (server), Vue 3 + TypeScript (client), Vitest (client unit tests)

---

## File Map

| File | Change |
|------|--------|
| `server/src/engine/state.rs` | Add `names: Vec<String>` to `GameState` |
| `server/src/lobby/room.rs` | Add `compute_names` helper; populate `state.names` in `start_next_hand` |
| `client/src/engine/types.ts` | Add `names: string[]` to `GameState` interface |
| `client/src/engine/sort.ts` | **New** — `sortHand`, `trickWinnerIndex`, shared trump helpers |
| `client/src/engine/sort.test.ts` | **New** — unit tests for both exports |
| `client/src/stores/game.ts` | Add `playerName(seat)` helper function |
| `client/src/components/TrickDisplay.vue` | Play-order badges, Led label, winner highlight, role badges |
| `client/src/components/HandComponent.vue` | Sort cards via `sortHand` computed |
| `client/src/games/sheepshead/GameTable.vue` | Use `playerName` in seat rail; pass names/roles to TrickDisplay |
| `client/src/games/sheepshead/BiddingPanel.vue` | Use `playerName` in waiting text |

---

### Task 1: Server — `names` field in `GameState`

**Files:**
- Modify: `server/src/engine/state.rs`
- Modify: `server/src/lobby/room.rs`

- [ ] **Step 1: Add `names` field to `GameState`**

In `server/src/engine/state.rs`, add `names` to the struct and initialise it in `new()`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    pub game_id: Uuid,
    pub game_name: String,
    pub phase: GamePhase,
    pub player_count: usize,
    pub dealer: usize,
    pub current_player: usize,
    pub hands: Vec<Vec<Card>>,
    pub extra_piles: Vec<(String, Vec<Card>)>,
    pub current_trick: Option<Trick>,
    pub completed_tricks: Vec<Trick>,
    pub scores: Vec<i32>,
    pub meta: serde_json::Value,
    /// Display name for each seat. Populated by the room before the first Snapshot.
    #[serde(default)]
    pub names: Vec<String>,
}
```

Update `GameState::new()` to initialise `names` as an empty vec:

```rust
pub fn new(game_id: Uuid, game_name: String, player_count: usize, dealer: usize) -> Self {
    Self {
        game_id,
        game_name,
        phase: GamePhase::Bidding,
        player_count,
        dealer,
        current_player: (dealer + 1) % player_count,
        hands: vec![Vec::new(); player_count],
        extra_piles: Vec::new(),
        current_trick: None,
        completed_tricks: Vec::new(),
        scores: vec![0; player_count],
        meta: serde_json::Value::Null,
        names: Vec::new(),
    }
}
```

- [ ] **Step 2: Add `compute_names` helper to `room.rs`**

Add this private function at the bottom of `room.rs` (before the closing `}`):

```rust
/// Compute display names for all seats.
/// Bot seats are named "Bot 1", "Bot 2", … in ascending seat-index order.
/// Human seats are named "Player".
fn compute_names(player_count: usize, bot_seats: &[bool]) -> Vec<String> {
    let mut bot_counter = 0usize;
    (0..player_count)
        .map(|i| {
            if bot_seats.get(i).copied().unwrap_or(false) {
                bot_counter += 1;
                format!("Bot {bot_counter}")
            } else {
                "Player".to_string()
            }
        })
        .collect()
}
```

- [ ] **Step 3: Populate `names` in `start_next_hand`**

In `room.rs`, find `fn start_next_hand(&self, dealer: usize)`. After the `deal_game(...)` call and before the snapshot loop, add one line to populate names:

```rust
fn start_next_hand(&self, dealer: usize) {
    let mut rng = rand::thread_rng();
    let mut state = GameState::new(self.id, self.game_name.clone(), self.player_count, dealer);
    deal_game(self.game.as_ref(), &mut state, &mut rng);

    // Populate seat names so clients can display "Bot 1" instead of "P0".
    state.names = compute_names(self.player_count, &self.bot_seats.lock().unwrap());

    {
        let txs = self.player_txs.lock().unwrap();
        for (seat, tx_opt) in txs.iter().enumerate() {
            let Some(tx) = tx_opt else { continue };
            let mut view = state.clone();
            for (i, hand) in view.hands.iter_mut().enumerate() {
                if i != seat { hand.clear(); }
            }
            view.extra_piles.clear();
            let _ = tx.try_send(StateUpdate::Snapshot { state: view });
        }
    }

    *self.state.lock().unwrap() = Some(state);
    tracing::info!(room_id = %self.id, dealer, "hand started");
}
```

- [ ] **Step 4: Run server tests**

```bash
cd server && cargo test -- --nocapture 2>&1 | tail -10
```
Expected: all 39 tests pass, no compile errors.

- [ ] **Step 5: Verify `names` appears in JSON**

```bash
cd server && cargo test -- --nocapture 2>&1 | grep -i "names" | head -5
```
(No specific test yet — compilation passing confirms the field serialises. The integration smoke test in Task 8 will verify the value.)

- [ ] **Step 6: Commit**

```bash
git add server/src/engine/state.rs server/src/lobby/room.rs
git commit -m "feat(server): add names field to GameState, populated from bot seat info"
```

---

### Task 2: Client Types + Store Helper

**Files:**
- Modify: `client/src/engine/types.ts`
- Modify: `client/src/stores/game.ts`

- [ ] **Step 1: Add `names` to the TypeScript `GameState` interface**

In `client/src/engine/types.ts`, add `names` to `GameState`:

```typescript
export interface GameState {
  game_id: string
  game_name: string
  phase: GamePhase
  player_count: number
  dealer: number
  current_player: number
  /** Only `hands[mySeat]` is populated; the rest are empty arrays. */
  hands: Card[][]
  /** Hidden from clients (e.g. blind) — always empty in received snapshots. */
  extra_piles: [string, Card[]][]
  current_trick: Trick | null
  completed_tricks: Trick[]
  scores: number[]
  /** Game-specific metadata (opaque; typed per-game where needed). */
  meta: Record<string, unknown>
  /** Display name for each seat index. May be empty array on old server versions. */
  names: string[]
}
```

- [ ] **Step 2: Add `playerName` helper to the Pinia store**

In `client/src/stores/game.ts`, add `playerName` as a function (not a computed — it takes a seat argument). Add it in the "Derived" section and export it from the return:

```typescript
  /** Returns "You" for the local player's seat, the server-assigned name otherwise,
   *  falling back to "P{seat}" if names haven't loaded yet. */
  function playerName(s: number): string {
    if (s === seat.value) return 'You'
    return gameState.value?.names?.[s] || `P${s}`
  }
```

Add `playerName` to the return object:

```typescript
  return {
    // state
    roomId, seat, gameState, myHand, error, isSolo, sessionScores, sessionWinner,
    // derived
    phase, isMyTurn, picker, isPicker, gameStarted, playerName,
    // actions
    handleUpdate, reset,
  }
```

- [ ] **Step 3: Run type-check**

```bash
cd client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vue-tsc --noEmit 2>&1 | tail -10
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add client/src/engine/types.ts client/src/stores/game.ts
git commit -m "feat(client): add names to GameState type and playerName store helper"
```

---

### Task 3: `sort.ts` — `sortHand` and `trickWinnerIndex`

**Files:**
- Create: `client/src/engine/sort.ts`
- Create: `client/src/engine/sort.test.ts`

- [ ] **Step 1: Write failing tests**

Create `client/src/engine/sort.test.ts`:

```typescript
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
        [0, card('clubs', 'ace')],   // led: fail ace
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
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cd client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vitest run src/engine/sort.test.ts 2>&1 | tail -15
```
Expected: error — `sort.ts` not found.

- [ ] **Step 3: Implement `sort.ts`**

Create `client/src/engine/sort.ts`:

```typescript
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
```

- [ ] **Step 4: Run tests**

```bash
cd client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vitest run src/engine/sort.test.ts 2>&1 | tail -15
```
Expected: 10 tests pass.

- [ ] **Step 5: Commit**

```bash
git add client/src/engine/sort.ts client/src/engine/sort.test.ts
git commit -m "feat(client): sortHand and trickWinnerIndex for Sheepshead"
```

---

### Task 4: `HandComponent.vue` — Sorted Cards

**Files:**
- Modify: `client/src/components/HandComponent.vue`

- [ ] **Step 1: Update `HandComponent` to sort cards before rendering**

Replace the full contents of `client/src/components/HandComponent.vue`:

```vue
<script setup lang="ts">
import { computed } from 'vue'
import type { Card } from '@/engine/types'
import { sortHand } from '@/engine/sort'
import CardComponent from './CardComponent.vue'

const props = defineProps<{
  cards: Card[]
  selectable?: boolean
  selectedCards?: Card[]
}>()

const emit = defineEmits<{ select: [card: Card] }>()

const sortedCards = computed(() => sortHand(props.cards))

function isSelected(card: Card): boolean {
  return (
    props.selectedCards?.some(
      (c) => c.suit === card.suit && c.rank === card.rank,
    ) ?? false
  )
}
</script>

<template>
  <div class="hand">
    <card-component
      v-for="(card, i) in sortedCards"
      :key="`${card.suit}-${card.rank}-${i}`"
      :card="card"
      :selectable="selectable"
      :selected="isSelected(card)"
      @select="emit('select', $event)"
    />
    <p v-if="cards.length === 0" class="empty">No cards</p>
  </div>
</template>

<style scoped>
.hand {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 0.5rem;
  justify-content: center;
}
.empty { color: #6b7280; font-style: italic; }
</style>
```

- [ ] **Step 2: Type-check**

```bash
cd client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vue-tsc --noEmit 2>&1 | tail -10
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add client/src/components/HandComponent.vue
git commit -m "feat(client): sort hand cards trump-first before rendering"
```

---

### Task 5: `TrickDisplay.vue` — Overhaul

**Files:**
- Modify: `client/src/components/TrickDisplay.vue`

- [ ] **Step 1: Replace `TrickDisplay.vue`**

Replace the full contents of `client/src/components/TrickDisplay.vue`:

```vue
<script setup lang="ts">
import { computed } from 'vue'
import type { Trick } from '@/engine/types'
import { trickWinnerIndex } from '@/engine/sort'
import CardComponent from './CardComponent.vue'

const props = defineProps<{
  trick: Trick | null
  mySeat: number
  names: string[]          // names[seat] = display name for that seat
  pickerSeat: number | null
  partnerSeat: number | null  // always null until partner mechanic is added
}>()

// Circled number badges ①②③④⑤ for play order
const ORDER_BADGES = ['①', '②', '③', '④', '⑤']

function playerLabel(seat: number): string {
  if (seat === props.mySeat) return 'You'
  return props.names[seat] || `P${seat}`
}

const winnerIdx = computed(() =>
  props.trick ? trickWinnerIndex(props.trick) : -1,
)
</script>

<template>
  <div class="trick-area">
    <div v-if="trick && trick.plays.length" class="trick-plays">
      <div v-for="([player, card], i) in trick.plays" :key="i" class="play">
        <!-- Play-order badge and player name row -->
        <div class="play-header">
          <span class="order-badge">{{ ORDER_BADGES[i] ?? i + 1 }}</span>
          <span class="player-label">
            {{ playerLabel(player) }}
          </span>
        </div>
        <!-- Role badges + Led label -->
        <div class="play-meta">
          <span v-if="i === 0" class="meta-label">Led</span>
          <span v-if="player === pickerSeat" class="role-badge picker">Picker</span>
          <span v-if="partnerSeat !== null && player === partnerSeat" class="role-badge partner">Partner</span>
        </div>
        <!-- Card with winner highlight -->
        <div class="card-wrapper" :class="{ winning: i === winnerIdx && winnerIdx !== -1 }">
          <card-component :card="card" />
        </div>
      </div>
    </div>
    <p v-else class="waiting">Waiting for first card…</p>
  </div>
</template>

<style scoped>
.trick-area {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 120px;
  background: rgba(0,0,0,0.2);
  border-radius: 12px;
  padding: 1rem;
  margin: 1rem 0;
}
.trick-plays {
  display: flex;
  gap: 16px;
  flex-wrap: wrap;
  justify-content: center;
}
.play {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 3px;
}
.play-header {
  display: flex;
  align-items: center;
  gap: 4px;
}
.order-badge {
  font-size: 0.85rem;
  color: #d1d5db;
}
.player-label {
  font-size: 0.75rem;
  color: #9ca3af;
}
.play-meta {
  display: flex;
  gap: 4px;
  align-items: center;
  min-height: 16px;
}
.meta-label {
  font-size: 0.65rem;
  color: #6b7280;
  font-style: italic;
}
.role-badge {
  font-size: 0.6rem;
  padding: 1px 5px;
  border-radius: 999px;
  font-weight: 600;
  letter-spacing: 0.02em;
}
.role-badge.picker  { background: #7c3aed; color: #fff; }
.role-badge.partner { background: #0d9488; color: #fff; }
.card-wrapper {
  border-radius: 6px;
}
.card-wrapper.winning {
  outline: 2px solid #f59e0b;
  box-shadow: 0 0 8px rgba(245, 158, 11, 0.45);
}
.waiting { color: #6b7280; font-style: italic; }
</style>
```

- [ ] **Step 2: Type-check**

```bash
cd client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vue-tsc --noEmit 2>&1 | tail -10
```
Expected: no errors. (GameTable.vue will error until Task 6 passes the new props — fix that next.)

- [ ] **Step 3: Commit**

```bash
git add client/src/components/TrickDisplay.vue
git commit -m "feat(client): trick display — play-order badges, Led label, winner highlight, role badges"
```

---

### Task 6: `GameTable.vue` — Seat Rail Names + TrickDisplay Props

**Files:**
- Modify: `client/src/games/sheepshead/GameTable.vue`

- [ ] **Step 1: Read the full current GameTable**

Read `client/src/games/sheepshead/GameTable.vue` to see what comes after line 80 (the template continues beyond what was initially read). You need the full file to make targeted edits.

- [ ] **Step 2: Update GameTable to use `playerName` and pass new TrickDisplay props**

In `GameTable.vue`:

**Script section** — destructure `playerName` from the store. `playerName` is a plain function exported from the Pinia store (not a ref), so it's safe to destructure directly. The existing `store` reference already exists in the file:

```typescript
const store = useGameStore()
const { playerName } = store  // add this line after the existing store declaration
```

Add a `partnerSeat` computed (always `null` until partner mechanic exists):

```typescript
const partnerSeat = computed<number | null>(() => null)
```

**Template — seat rail** — replace `P{{ p }}` with `playerName(p)` and update dealer badge:

Change:
```html
<span class="seat-label">
  P{{ p }}
  <span v-if="p === state.dealer" class="badge">D</span>
  <span v-if="p === store.picker" class="badge picker">P</span>
</span>
```
To:
```html
<span class="seat-label">
  {{ playerName(p) }}
  <span v-if="p === state.dealer" class="badge">D</span>
  <span v-if="p === store.picker" class="role-badge picker">Picker</span>
</span>
```

**Template — dealer badge in header** — change `Dealer: P{{ state.dealer }}` to:
```html
<span class="dealer-badge">Dealer: {{ playerName(state.dealer) }}</span>
```

**Template — TrickDisplay call** — add the three new required props:

Change:
```html
<trick-display
  :trick="state.current_trick"
  :my-seat="seat"
  :player-count="state.player_count"
/>
```
To:
```html
<trick-display
  :trick="state.current_trick"
  :my-seat="seat"
  :names="state.names ?? []"
  :picker-seat="store.picker"
  :partner-seat="partnerSeat"
/>
```

Note: `:player-count` is no longer needed by TrickDisplay — remove it.

- [ ] **Step 3: Type-check**

```bash
cd client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vue-tsc --noEmit 2>&1 | tail -10
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add client/src/games/sheepshead/GameTable.vue
git commit -m "feat(client): seat rail uses player names, TrickDisplay receives names and roles"
```

---

### Task 7: `BiddingPanel.vue` — Named Players in Waiting Text

**Files:**
- Modify: `client/src/games/sheepshead/BiddingPanel.vue`

- [ ] **Step 1: Import `playerName` from the store and update waiting text**

In `BiddingPanel.vue`, destructure `playerName` from the store:

```typescript
const store = useGameStore()
const { pick, pass, bury } = useGame()
const { playerName } = store
```

Change the picking-phase waiting message from:
```html
<p v-else class="waiting-msg">
  Waiting for player {{ store.gameState?.current_player }} to pick or pass…
</p>
```
To:
```html
<p v-else class="waiting-msg">
  Waiting for {{ playerName(store.gameState?.current_player ?? 0) }} to pick or pass…
</p>
```

Change the bury-phase waiting message from:
```html
<p v-else class="waiting-msg">
  Waiting for player {{ store.picker }} to bury…
</p>
```
To:
```html
<p v-else class="waiting-msg">
  Waiting for {{ playerName(store.picker ?? 0) }} to bury…
</p>
```

- [ ] **Step 2: Type-check**

```bash
cd client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vue-tsc --noEmit 2>&1 | tail -10
```
Expected: no errors.

- [ ] **Step 3: Run all client unit tests**

```bash
cd client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vitest run 2>&1 | tail -10
```
Expected: all tests pass (sort.test.ts + game.test.ts).

- [ ] **Step 4: Commit**

```bash
git add client/src/games/sheepshead/BiddingPanel.vue
git commit -m "feat(client): bidding panel waiting text uses player names"
```

---

### Task 8: End-to-End Verification

- [ ] **Step 1: Run server tests**

```bash
cd server && cargo test 2>&1 | tail -5
```
Expected: 39 passed, 0 failed.

- [ ] **Step 2: Run server clippy**

```bash
cd server && cargo clippy -- -D warnings 2>&1 | tail -5
```
Expected: no errors.

- [ ] **Step 3: Run client unit tests**

```bash
cd client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npx vitest run 2>&1 | tail -5
```
Expected: all pass.

- [ ] **Step 4: Start dev server and smoke test**

In one terminal:
```bash
cd server && cargo run
```
In another:
```bash
cd client && export PATH="/opt/homebrew/opt/node@20/bin:$PATH" && npm run dev
```
Open `http://localhost:5173`, create a solo Sheepshead game, and verify:
- Hand is sorted trump-first (Queens and Jacks appear on the left of your hand)
- Seat rail shows "Bot 1", "Bot 2", "Bot 3", "Bot 4" instead of "P1", "P2", "P3", "P4"
- After the first card is played, the trick display shows ① badge and "Led" label
- As more cards are played, ②③ badges appear
- The currently winning card has a gold/amber outline ring
- The picker's play in the trick shows a purple "Picker" badge
- Bidding panel waiting text reads "Waiting for Bot 1 to pick or pass…"

- [ ] **Step 5: Final commit if any cleanup needed**

If any minor issues were found and fixed during smoke test:
```bash
git add -u && git commit -m "fix(client): game table clarity smoke test fixes"
```

---

## Verification Summary

| Check | Command |
|-------|---------|
| Server tests | `cd server && cargo test` |
| Server lint | `cd server && cargo clippy -- -D warnings` |
| Client types | `cd client && npx vue-tsc --noEmit` |
| Client unit tests | `cd client && npx vitest run` |
| Manual smoke | Solo game: hand sorted, Bot names, trick badges, winner ring, Picker badge |
