# Game Table Clarity Design

**Date:** 2026-05-14  
**Status:** Approved  
**Scope:** Trick display overhaul, card sorting, and player naming

## Context

The game table is where players spend 95% of their time, but three clarity issues make it hard to read during play: the trick display doesn't show play order or who led; cards in hand appear in deal order rather than a useful sort; and players are identified only as "P0", "P1", making it hard to track roles and opponents. This spec addresses all three.

---

## 1. Trick Display

### What Changes

`TrickDisplay.vue` is updated to show:

- **Play-order badges** — each card gets a circled number (①②③…) indicating when it was played in the trick
- **Led label** — the first player's name gets a small "Led" label beneath it (muted text, no extra color)
- **Winning card highlight** — a gold/amber border ring on whichever card is currently winning the trick; updates live as plays come in
- **Role badges** — player names show inline role context: `Bot 2 · Picker` or `Bot 1 · Partner`; no badge if the player has no special role

Layout stays horizontal flex with slightly more spacing between cards so badges don't crowd.

### Winning Card Computation

The current trick winner is computed client-side from `current_trick.plays` using trump rank values already available in `engine/types.ts`. A `currentTrickWinner` computed property is added to the Pinia store (or as a local computed in `TrickDisplay.vue`). It replicates the same trump > led-suit-rank priority used server-side.

### Role Badges

- `picker` seat is read from `GameState.meta["picker"]` (already present)
- `partner` seat will be read from `GameState.meta["partner"]` when partner mechanics are added — the component already handles both; defaults to no badge if the field is absent

### Seat Rail Consistency

The same role badges (`Picker`, `Partner`) are applied to player cards in the seat rail at the top of `GameTable.vue`, using the same colors and inline format, so role information is consistent across the entire table view.

---

## 2. Card Sorting

### Sort Order

Cards in `HandComponent.vue` are sorted before rendering using a `sortHand(cards: Card[], state: GameState): Card[]` helper:

1. **Trump first, high to low** — ♣Q(14), ♠Q(13), ♥Q(12), ♦Q(11), ♣J(10), ♠J(9), ♥J(8), ♦J(7), A♦(6), 10♦(5), K♦(4), 9♦(3), 8♦(2), 7♦(1)
2. **Fail suits after trump, grouped by suit** — Clubs, then Spades, then Hearts (Diamonds is entirely trump in Sheepshead)
3. **Within each fail suit, high to low** — A, 10, K, 9, 8, 7

### Future-Proofing

`sortHand` is a standalone function in `client/src/engine/sort.ts`, not inlined in the component. This makes it trivial to add alternative sort strategies (e.g., trump low-to-high with fail on the left) and a user preference setting later — the component just calls whichever strategy is active.

Sorting is purely presentational and reactive: it runs as a computed property whenever `hand` changes.

---

## 3. Player Naming

### Server Changes

`GameState` gains a `names: Vec<String>` field:

```rust
pub struct GameState {
    // ... existing fields ...
    pub names: Vec<String>,  // names[i] = display name for seat i
}
```

The server populates names when the room is created and players join:
- Bot seats: `"Bot 1"`, `"Bot 2"`, … assigned sequentially in ascending seat-index order (the lowest-indexed bot seat is Bot 1, the next is Bot 2, etc., regardless of where human seats fall)
- Human seats: `"Player"` for now; extensible to socket-provided names when auth/profiles are added

### Client Changes

`engine/types.ts` adds `names: string[]` to `GameState`. All components that currently display seat numbers switch to `store.gameState.names[seat]`:

- Seat rail player labels in `GameTable.vue`
- Trick display player labels in `TrickDisplay.vue`
- Bidding panel waiting text in `BiddingPanel.vue` — "Waiting for Bot 2 to pick or pass…" instead of "Waiting for player 3…"
- Score table rows
- "Led" label in trick display

The human player's own seat still shows as **"You"** (not their name) everywhere — this is a local override in the client, not a name change.

---

## 4. Files Modified

| File | Change |
|------|--------|
| `server/src/engine/state.rs` | Add `names: Vec<String>` to `GameState` |
| `server/src/lobby/room.rs` | Populate `names` on room creation / player join |
| `client/src/engine/types.ts` | Add `names: string[]` to `GameState` type |
| `client/src/engine/sort.ts` | New file — `sortHand()` function |
| `client/src/components/TrickDisplay.vue` | Play-order badges, Led label, winning card highlight, role badges |
| `client/src/components/HandComponent.vue` | Sort cards via `sortHand()` computed |
| `client/src/games/sheepshead/GameTable.vue` | Role badges in seat rail; pass names to child components |
| `client/src/games/sheepshead/BiddingPanel.vue` | Use names in waiting text |
| `client/src/stores/game.ts` | Add `currentTrickWinner` computed |

---

## 5. Verification

- **Unit tests:** `sortHand` tested for trump ordering, suit grouping, and within-suit rank order in `client/src/engine/sort.test.ts`
- **Unit tests:** `currentTrickWinner` tested for trump-beats-fail, higher-trump-wins, led-suit-rank scenarios in `client/src/stores/game.test.ts`
- **Manual:** Start a solo Sheepshead game; confirm hand is sorted trump-first on deal; confirm trick display shows ① badge on first played card, "Led" under first player, gold ring on winning card, "Bot N · Picker" badge on picker's plays
- **Manual:** Confirm seat rail shows Picker badge on correct seat during bidding and playing phases
- **Manual:** Confirm BiddingPanel waiting text reads "Waiting for Bot 2 to pick or pass…"
- **Regression:** `cargo test` passes; `npm run test:unit` passes
