# Game Feel & Animations Design

**Date:** 2026-05-15  
**Status:** Approved  
**Scope:** Card play animations, trick completion pause, bot play delay, your-turn indicator, phase change toast, game-specific phase labels

## Context

The game is fully playable but everything happens instantly — cards appear, tricks vanish, phases change, and bots act at machine speed. This makes it hard to follow what just happened and removes the sense of rhythm that card games rely on. This spec adds four complementary improvements that together make the game feel alive and legible.

---

## 1. Card Play Animation

When a card is played by any player, it appears in `TrickDisplay`. Currently it renders instantly. With `<TransitionGroup>`, each new card slides up from slightly below and fades in over 250ms.

**Implementation:**
- In `TrickDisplay.vue`, change the `v-for` loop key from `i` to `${player}-${card.suit}-${card.rank}` so Vue can track individual entries
- Wrap the `.play` divs in `<TransitionGroup name="card-play" tag="div" class="trick-plays">`
- CSS classes: `.card-play-enter-active` (250ms ease-out), `.card-play-enter-from` (opacity 0, translateY 12px), `.card-play-enter-to` (opacity 1, translateY 0)

No server changes.

---

## 2. Trick Completion Pause

The store currently clears `current_trick` immediately on `trick_complete`, causing the trick to vanish before the player can read it.

**Store changes (`stores/game.ts`):**
- Add `completedTrick: ref<Trick | null>(null)` to store state
- Export `completedTrick` from the store
- On `trick_complete`: mark the trick with its winner, assign it to `completedTrick`, and set `current_trick = null`
- Start a `setTimeout` (1500ms) that clears `completedTrick`; store the timer ID in a module-level `let pauseTimer: ReturnType<typeof setTimeout> | null`
- If a new `CardPlayed` arrives while the timer is running, call `clearTimeout(pauseTimer)` and set `completedTrick.value = null` immediately before processing the new play — a new trick has started

**TrickDisplay changes:**
- Accept a new prop `completedTrick: Trick | null`
- If `trick` is null but `completedTrick` is set, render `completedTrick` instead with a winner banner: "**[Name] wins the trick**" displayed above the cards
- Wrap the entire trick area content in `<Transition name="trick-fade">` (200ms fade) so it fades out gracefully when `completedTrick` is cleared

The winner name uses the existing `playerName` helper (already in the store) — "You win the trick" for the local player, "Bot 1 wins the trick" for others.

---

## 3. Bot Play Delay

Bots currently act at machine speed. A 1200ms delay before each bot action makes the game feel like real players.

**Server changes (`server/src/lobby/room.rs`):**
- Change `fn drive_bots(&self)` to `async fn drive_bots(&self)`
- Add `tokio::time::sleep(std::time::Duration::from_millis(1200)).await` at the top of each loop iteration, before the bot action is computed
- Update the `tokio::spawn` call site to `tokio::spawn(async move { room.drive_bots().await })`
- The 1200ms value is a named constant: `const BOT_ACTION_DELAY_MS: u64 = 1200`

The bot delay works in concert with the trick completion pause: after a trick ends, the 1.2s bot delay gives the client time to show the winner before the next lead arrives.

---

## 4. Your Turn Indicator

The current "↑ Your turn" label is easy to miss. A pulsing glow border provides an unmistakable peripheral signal.

**CSS animation** (added to global or scoped styles):
```css
@keyframes your-turn-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(34, 197, 94, 0); }
  50%       { box-shadow: 0 0 0 6px rgba(34, 197, 94, 0.4); }
}
.your-turn-glow {
  border: 2px solid #22c55e;
  animation: your-turn-pulse 1.2s ease-in-out infinite;
}
```

**Applied in two places:**
- `GameTable.vue` — the hand section container gets `:class="{ 'your-turn-glow': store.isMyTurn && state.phase === 'playing' }"`
- `BiddingPanel.vue` — the `.bidding-panel` root div gets `:class="{ 'your-turn-glow': store.isMyTurn }"`

When it's not your turn, the class is removed immediately (no transition — it's an attention signal).

---

## 5. Phase Change Toast

When the game transitions between phases (Bidding → Playing, Playing → Scoring), a brief centered overlay shows the new phase's display name.

**Implementation in `GameTable.vue`:**
- Add `phaseToast: ref<string | null>(null)` local state
- `watch(() => state.value?.phase, (newPhase, oldPhase) => { if (newPhase && oldPhase && newPhase !== oldPhase) { phaseToast.value = phaseLabel(state.value.game_name, newPhase); setTimeout(() => { phaseToast.value = null }, 1500) } })`
- Add a fixed-position overlay div wrapped in `<Transition name="toast">` (200ms fade in/out)
- The overlay is centered, semi-transparent dark background, large white text, pointer-events none

---

## 6. Game-Specific Phase Labels

The raw phase names ("bidding", "playing", "scoring") are engine terms. Display names are game-specific.

**New file: `client/src/engine/phases.ts`:**
```typescript
import type { GamePhase } from './types'

const BIDDING_LABELS: Record<string, string> = {
  sheepshead: 'Picking',
  euchre:     'Calling',
  spades:     'Bidding',
  hearts:     'Playing',  // hearts has no bidding phase
}

export function phaseLabel(gameName: string, phase: GamePhase): string {
  if (phase === 'bidding') {
    return BIDDING_LABELS[gameName] ?? 'Bidding'
  }
  return phase.charAt(0).toUpperCase() + phase.slice(1)
}
```

**Used in two places:**
- Phase change toast (Section 5) — passes the game-specific label to the overlay
- Phase badge in `GameTable.vue` header — replace `{{ state.phase }}` with `{{ phaseLabel(state.game_name, state.phase) }}`

---

## 7. Files Modified

| File | Change |
|------|--------|
| `server/src/lobby/room.rs` | Bot delay — `drive_bots` becomes async, adds 1200ms sleep per action |
| `client/src/engine/phases.ts` | **New** — `phaseLabel` helper |
| `client/src/stores/game.ts` | Add `completedTrick` state; update `trick_complete` handler |
| `client/src/components/TrickDisplay.vue` | `<TransitionGroup>` for card plays; winner banner; accept `completedTrick` prop |
| `client/src/games/sheepshead/GameTable.vue` | Your-turn pulse on hand; phase toast; phase badge uses `phaseLabel` |
| `client/src/games/sheepshead/BiddingPanel.vue` | Your-turn pulse on bidding panel |

---

## 8. Verification

- **Server:** `cargo test` (39 tests), `cargo clippy -- -D warnings`
- **Client:** `npx vue-tsc --noEmit`, `npx vitest run`
- **Manual smoke:**
  - Start a solo game; confirm each bot action takes ~1.2s before appearing
  - Play a card; confirm it slides up into the trick area
  - Complete a trick; confirm it stays visible ~1.5s with a winner banner before clearing
  - Confirm your hand container glows green when it's your turn to play
  - Confirm bidding panel glows when it's your turn to pick/bury
  - Confirm "Playing" toast appears when bidding ends
  - Confirm "Scoring" toast appears when all tricks are played
  - Confirm phase badge shows "Picking" (not "Bidding") for Sheepshead
