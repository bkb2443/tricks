# Sheepshead Partner Mechanics Design

**Date:** 2026-05-15  
**Status:** Approved  
**Scope:** Called-ace partner mechanic, going alone, 2v3 scoring, bot partner/defender awareness, calling UI

## Context

Sheepshead currently plays 1v4 (picker alone against four defenders). This spec adds the standard called-ace partner mechanic: after picking and burying, the picker either calls a fail ace to designate a secret partner (2v3) or declares they're going alone (1v4 with double stakes). The partner's identity is hidden until the called ace is played during tricks.

---

## 1. Bidding Extension — The Calling Sub-Phase

### Sub-phase tracking

The bidding phase tracks its current sub-phase in `meta["sub_phase"]` (string). The progression is:

```
"picking" → "burying" → "calling" → (phase transition to Playing)
```

When the picker buries, the server transitions `sub_phase` to `"calling"`, sets `current_player` back to the picker seat, and broadcasts a `BidPlaced` event so clients know the phase advanced. The `BidPlaced.value` payload for the calling-sub-phase transition has the shape `{"sub_phase": "calling", "callable_suits": ["clubs", "spades"]}` — the server computes valid callable suits and sends them so the picker's client can render valid options without duplicating validation logic. Other clients ignore this payload (they show a waiting message).

### Bid actions

**Calling sub-phase bid:**
- `{"action": "call", "suit": "clubs"}` — calls the Ace of Clubs as the partner card (suit may be `"clubs"`, `"spades"`, or `"hearts"`)
- `{"action": "go_alone"}` — picker plays 1v4 for double stakes

### Validation — which aces are callable?

After burying, an ace of suit S is callable if both conditions hold:
1. The Ace of S is **not** in the picker's post-bury hand
2. The picker holds **at least one other non-trump card** of suit S (ensures the suit can be led, allowing the partner to play the called ace)

Diamonds are entirely trump in Sheepshead, so only A♣, A♠, and A♥ are ever callable.

If no ace is callable (edge case: picker holds all non-trump cards of every fail suit), the picker is forced to go alone.

### Meta state after calling

```json
{
  "picker": 2,
  "sub_phase": "calling",
  "called_suit": "clubs",   // null if going alone
  "going_alone": false,     // true if going alone
  "partner": null           // populated when called ace is played
}
```

---

## 2. Partner Revelation During Play

When `apply_play` processes a card during the Playing phase, it checks:

```rust
if card.rank == Rank::Ace && Some(card.suit) == called_suit && !going_alone {
    meta["partner"] = seat;
    // broadcast PartnerRevealed
}
```

A new `StateUpdate::PartnerRevealed { seat: usize }` variant is added. The server broadcasts it immediately when the called ace is played. All clients receive it; the client shows a reveal toast and lights up the Partner badge.

**Future hook:** `PartnerRevealed` is designed so it can later be sent only to the picker and partner (omitting the broadcast to defenders) to replicate the "did anyone notice?" aspect of human play. For now it is broadcast to all.

**Edge case — called ace never played:** If the hand ends without the called ace appearing (e.g., it was never led), the picker is treated as having gone alone for scoring purposes. The partner slot in meta remains `null`.

---

## 3. Scoring

`score_game` in `sheepshead/rules.rs` gains three scoring paths, selected by `meta["going_alone"]` and `meta["partner"]`.

### Going alone (1v4, double stakes)

Picker + buried cards > 60 points → picker wins. Multiplier starts at 4 (one point per defender, doubled from called-partner base).

| Result | Picker | Each defender |
|--------|--------|---------------|
| Win | +4 | -1 |
| Win + schneider (defenders ≤30 pts) | +8 | -2 |
| Loss | -4 | +1 |
| Loss + schneider (picker ≤30 pts) | -8 | +2 |

### Called partner (2v3)

Picker + partner's combined tricks > 60 points → they win.

| Result | Picker | Partner | Each defender |
|--------|--------|---------|---------------|
| Win | +2 | +1 | -1 |
| Win + schneider (defenders ≤30 pts) | +4 | +2 | -2 |
| Loss | -2 | -1 | +1 |
| Loss + schneider (picker+partner ≤30 pts) | -4 | -2 | +2 |

If `meta["partner"]` is `null` at scoring time (ace never revealed), treat as going alone with the 1v4 multiplier.

### Leaster

Unchanged — no partner mechanic applies.

---

## 4. Bot Calling + Partner/Defender Awareness

### Calling decision

When a bot is the picker and reaches the calling sub-phase:

1. **Go alone** if trump strength is very high: holds ≥5 trump including ≥2 Jacks (trump rank ≥ 8) — double stakes are worth the risk
2. **Call an ace** otherwise: among callable aces, call the suit where the bot holds the most non-trump cards (maximises chances to lead the suit and bring out the partner)
3. **Forced go alone** if no ace is callable

### Partner/defender awareness in play

`BotState` already has `predicted_partner: Option<usize>`. It is now populated in two ways:

**Definitive (post-revelation):** Once `meta["partner"]` is set, all bots set `predicted_partner` to that seat value.

**Inferred (pre-revelation):** A player who fails to follow the called suit (and is not the picker) is **not** the partner. Bots track this via `known_voids` — if a player is void in the called suit, they're eliminated as the partner candidate. If only one seat remains as a possible partner, `predicted_partner` is set to that seat.

### Team-aware play adjustments

With `predicted_partner` set, the existing following/leading heuristics are extended:

- **Bot is picker:** treat `predicted_partner` the same as a teammate — dump points on their winning tricks
- **Bot is partner:** treat picker as teammate — dump points on picker's winning tricks, hold trump to support picker
- **Bot is defender:** treat both picker and partner as opponents — don't dump points on either's winning tricks, play to win points away from them

The `follow_as_defender` function already uses "is the picker winning?" logic; it is extended to also check "is the partner winning?" using the same path.

---

## 5. Client — Calling UI

### BiddingPanel.vue — new calling block

A third conditional block alongside existing picking and burying blocks:

**Picker's calling turn (`isMyCallTurn`):**
- Header: "Choose your partner card"
- Three suit buttons for callable aces (♣ ♠ ♥), disabled if that suit is not callable
- A separate "Go Alone" button (higher visual weight — it's a meaningful commitment)
- Callable suits come from `store.gameState.meta.callable_suits` (server sends this in the `BidPlaced` payload when transitioning to calling sub-phase)

**Waiting for bot to call:**
- "Waiting for [playerName] to call their partner…"

### GameTable.vue — called ace indicator

After the picker calls (before the partner is revealed), show a small indicator in the picker's seat card in the seat rail: "Called: A♣" (using the suit symbol). This is visible to all players.

### Partner reveal toast

When `PartnerRevealed` arrives, show a phase-toast-style overlay: "[Name] is the partner!" for 2 seconds. The existing toast pattern (phase toast) can be reused.

### Partner badge

`partnerSeat` in `GameTable.vue` already reads `meta["partner"]` — no change needed. The badge appears automatically once `meta["partner"]` is set.

---

## 6. Files Modified

| File | Change |
|------|--------|
| `server/src/engine/state.rs` | Add `PartnerRevealed { seat: usize }` to `StateUpdate` |
| `server/src/games/sheepshead/rules.rs` | Calling sub-phase in `apply_bid`; partner revelation in `apply_play`; 2v3 + going-alone scoring in `score_game` |
| `server/src/bot.rs` | Bot calling decision; `predicted_partner` population; team-aware play adjustments |
| `client/src/engine/types.ts` | Add `partner_revealed` to `StateUpdate` union |
| `client/src/stores/game.ts` | Handle `partner_revealed` update; expose `callableSuits` from meta |
| `client/src/games/sheepshead/BiddingPanel.vue` | Calling sub-phase UI (call ace / go alone) |
| `client/src/games/sheepshead/GameTable.vue` | Called ace indicator in seat rail; partner reveal toast |

---

## 7. Verification

- **Server unit tests:** calling validation (callable vs. non-callable aces), partner revelation on ace play, 2v3 scoring (all four cases), going-alone scoring (all four cases), edge case (partner never revealed → goes alone at scoring)
- **Client unit tests:** store handles `partner_revealed` update; `callableSuits` computed correctly
- **Manual smoke:** Start solo game → pick → bury → call an ace → play until called ace appears → confirm partner badge lights up and reveal toast fires; play until end → confirm correct scores; test go-alone path; test schneider in both 2v3 and 1v4 modes
