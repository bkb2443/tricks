<script setup lang="ts">
import { useGameStore } from '@/stores/game'
import { useGame } from '@/composables/useGame'
import { ref } from 'vue'

const store = useGameStore()
const { toggleHint } = useGame()

const rulesOpen = ref(false)
</script>

<template>
  <div v-if="store.isTraining" class="training-overlay">
    <!-- Narration banner -->
    <div v-if="store.trainingNarration" class="narration-banner">
      <span class="narration-icon">📖</span>
      <p class="narration-text">{{ store.trainingNarration }}</p>
    </div>

    <!-- Hint text (when hint is active) -->
    <div v-if="store.gameState?.hint_enabled && store.gameState?.hint" class="hint-banner">
      <span class="hint-label">Hint:</span>
      {{ store.gameState.hint.reason }}
    </div>

    <!-- Training controls bar -->
    <div class="training-controls">
      <button
        class="training-btn"
        :class="{ active: store.gameState?.hint_enabled }"
        @click="toggleHint"
      >
        💡 {{ store.gameState?.hint_enabled ? 'Hint On' : 'Hint Off' }}
      </button>
      <button class="training-btn" @click="rulesOpen = !rulesOpen">
        📋 Rules
      </button>
    </div>

    <!-- Rules panel (inline expandable) -->
    <div v-if="rulesOpen" class="rules-panel">
      <div class="rules-header">
        <span>Rules Reference — {{ store.gameState?.game_name === 'sheepshead' ? 'Sheepshead' : 'Euchre' }}</span>
        <button class="close-btn" @click="rulesOpen = false">✕</button>
      </div>
      <div class="rules-body">
        <!-- Sheepshead rules -->
        <template v-if="store.gameState?.game_name === 'sheepshead'">
          <h3>The Deck</h3>
          <p>32 cards: 7 through Ace in all four suits. All Queens and Jacks leave their suits to become permanent trump.</p>
          <h3>Trump (14 cards, strongest → weakest)</h3>
          <p>♣Q · ♠Q · ♥Q · ♦Q · ♣J · ♠J · ♥J · ♦J · A♦ · 10♦ · K♦ · 9♦ · 8♦ · 7♦</p>
          <h3>Plain Suits</h3>
          <p>Clubs, Spades, Hearts — but only their non-Queen, non-Jack cards. Within each plain suit: A &gt; 10 &gt; K &gt; 9 &gt; 8 &gt; 7.</p>
          <h3>Card Points (total 120 per hand)</h3>
          <p>A=11 · 10=10 · K=4 · Q=3 · J=2 · 9/8/7=0</p>
          <h3>Picking</h3>
          <p>Starting left of the dealer, each player may pick the 2-card blind or pass. The first to pick becomes the <strong>picker</strong>, takes both blind cards (hand grows to 8), then buries 2 cards face-down. Buried cards count toward the picker's score at the end.</p>
          <h3>Calling a Partner</h3>
          <p>After burying, the picker names a non-trump ace they don't hold (and still have at least one other card of that suit). Whoever holds that ace is the secret partner — revealed only when they play the ace.</p>
          <h3>Going Alone</h3>
          <p>If the picker cannot or chooses not to call a partner, they go alone (1 vs 4). Points scored are doubled.</p>
          <h3>Leaster</h3>
          <p>If all 5 players pass, the hand is a Leaster. No picker, no teams — everyone plays for themselves. The player who takes the most card points <strong>loses</strong> 4 points; everyone else gains 1.</p>
          <h3>Winning (normal hand)</h3>
          <p>The picker + partner need <strong>more than 60 points</strong> to win. Exactly 60 is a loss for the picker.</p>
        </template>

        <!-- Euchre rules -->
        <template v-if="store.gameState?.game_name === 'euchre'">
          <h3>The Deck</h3>
          <p>24 cards: 9, 10, J, Q, K, A in all four suits.</p>
          <h3>Trump</h3>
          <p>One suit is trump per hand — set during bidding. Trump ranking (strongest → weakest): Right Bower (Jack of trump suit) · Left Bower (Jack of same-color suit) · A · K · Q · 10 · 9. The Left Bower plays as trump, not as its printed suit.</p>
          <p>Same-color pairs: ♣ ↔ ♠ and ♥ ↔ ♦</p>
          <h3>Bidding — Round 1 (ordering)</h3>
          <p>The dealer turns up the top kitty card. Starting left of the dealer, each player may order the turned-up suit as trump or pass. If ordered, the dealer picks up that card and discards one.</p>
          <h3>Bidding — Round 2 (calling)</h3>
          <p>If all four pass round 1, players may call any other suit as trump or pass. The dealer cannot pass if everyone else has — <strong>stick the dealer</strong>.</p>
          <h3>Going Alone</h3>
          <p>When ordering or calling, declare "alone." Your partner sits out. Win all 5 tricks alone → 4 points; win 3–4 tricks alone → 1 point.</p>
          <h3>Legal Plays</h3>
          <p>You must follow the led suit if you can. The Left Bower must follow trump, not its printed suit.</p>
          <h3>Scoring</h3>
          <p>3–4 tricks: +1 each maker, 0 defenders · 5 tricks (march): +2 each · 5 tricks alone: caller +4 · Euchred (≤2 tricks): defenders +2</p>
          <p><strong>Match</strong>: first team to reach 10 points wins.</p>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.training-overlay {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin: 0.5rem 0;
}

.narration-banner {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  background: rgba(99, 102, 241, 0.12);
  border: 1px solid rgba(99, 102, 241, 0.3);
  border-radius: 8px;
  padding: 0.6rem 0.75rem;
}

.narration-icon { flex-shrink: 0; }

.narration-text {
  margin: 0;
  font-size: 0.85rem;
  line-height: 1.5;
  color: #c7d2fe;
}

.hint-banner {
  background: rgba(245, 158, 11, 0.1);
  border: 1px solid rgba(245, 158, 11, 0.3);
  border-radius: 8px;
  padding: 0.4rem 0.75rem;
  font-size: 0.82rem;
  color: #fcd34d;
}

.hint-label { font-weight: 600; margin-right: 0.3rem; }

.training-controls {
  display: flex;
  gap: 0.5rem;
}

.training-btn {
  flex: 1;
  padding: 0.35rem 0.75rem;
  font-size: 0.82rem;
  background: rgba(255,255,255,0.07);
  border: 1px solid rgba(255,255,255,0.15);
  border-radius: 6px;
  cursor: pointer;
  color: #d1d5db;
  transition: background 0.15s;
}

.training-btn:hover { background: rgba(255,255,255,0.14); }
.training-btn.active {
  background: rgba(245,158,11,0.2);
  border-color: rgba(245,158,11,0.5);
  color: #fcd34d;
}

.rules-panel {
  background: rgba(0,0,0,0.4);
  border: 1px solid rgba(255,255,255,0.12);
  border-radius: 8px;
  overflow: hidden;
  max-height: 300px;
  overflow-y: auto;
}

.rules-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid rgba(255,255,255,0.1);
  font-size: 0.85rem;
  font-weight: 600;
  background: rgba(0,0,0,0.2);
  position: sticky;
  top: 0;
}

.close-btn {
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
  font-size: 0.85rem;
  opacity: 0.7;
  padding: 0;
}
.close-btn:hover { opacity: 1; }

.rules-body {
  padding: 0.75rem;
  font-size: 0.8rem;
  line-height: 1.6;
}

.rules-body h3 {
  margin: 0.75rem 0 0.25rem;
  font-size: 0.85rem;
  color: #a5b4fc;
  border-bottom: 1px solid rgba(255,255,255,0.06);
  padding-bottom: 0.2rem;
}

.rules-body h3:first-child { margin-top: 0; }

.rules-body p {
  margin: 0 0 0.3rem;
  color: #d1d5db;
}
</style>
