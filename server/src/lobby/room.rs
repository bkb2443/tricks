use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::engine::{Card, GamePhase, GameState, PlayResult, StateUpdate, deal_game};
use crate::engine::game::Game;

pub struct Room {
    pub id: Uuid,
    pub game_name: String,
    pub player_count: usize,
    pub victory_goal: i32,
    game: Box<dyn Game>,
    /// Per-player private send channels; index = seat. `None` = seat is empty.
    player_txs: Mutex<Vec<Option<mpsc::Sender<StateUpdate>>>>,
    /// Broadcast channel for public events (BidPlaced, CardPlayed, TrickComplete, …).
    broadcast_tx: broadcast::Sender<StateUpdate>,
    pub state: Mutex<Option<GameState>>,
    /// Cumulative per-player scores across all hands in this session.
    session_scores: Mutex<Vec<i32>>,
    /// Which seats are driven by the server-side bot (true = bot seat).
    bot_seats: Mutex<Vec<bool>>,
    bots_running: AtomicBool,
}

impl Room {
    pub fn new(
        id: Uuid,
        game_name: String,
        player_count: usize,
        game: Box<dyn Game>,
        victory_goal: i32,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(64);
        let player_txs = (0..player_count).map(|_| None).collect();
        let bot_seats = Mutex::new(vec![false; player_count]);
        let session_scores = Mutex::new(vec![0; player_count]);
        Self {
            id,
            game_name,
            player_count,
            victory_goal,
            game,
            player_txs: Mutex::new(player_txs),
            broadcast_tx,
            state: Mutex::new(None),
            session_scores,
            bot_seats,
            bots_running: AtomicBool::new(false),
        }
    }

    /// Assign the next empty seat. Returns `(seat, broadcast_rx)` or `None` if full.
    /// Dealing fires automatically once all seats are filled.
    pub fn join(
        &self,
        player_tx: mpsc::Sender<StateUpdate>,
    ) -> Option<(usize, broadcast::Receiver<StateUpdate>)> {
        let seat;
        let all_filled;
        {
            let mut txs = self.player_txs.lock().unwrap();
            seat = txs.iter().position(|p| p.is_none())?;
            txs[seat] = Some(player_tx);
            all_filled = txs.iter().all(|p| p.is_some());
        }

        if all_filled {
            self.start_game();
        }

        Some((seat, self.broadcast_tx.subscribe()))
    }

    /// Apply a bid action from `seat`. Broadcasts public events and sends private
    /// `HandUpdated` to the affected player.
    pub fn apply_bid(&self, seat: usize, value: serde_json::Value) -> Result<(), String> {
        let (result, current_player) = {
            let mut guard = self.state.lock().unwrap();
            let state = guard.as_mut().ok_or_else(|| "game not started".to_string())?;
            let result = self.game.apply_bid(state, seat, &value)?;
            let cp = state.current_player;
            (result, cp)
        };

        // Public: everyone sees the bid was placed; current_player tells clients whose turn is next.
        // Use broadcast_payload when the game provides a filtered view (e.g., bury → calling).
        let bid_value = result.broadcast_payload.unwrap_or(value);
        self.broadcast(StateUpdate::BidPlaced { player: seat, value: bid_value, current_player });

        // Private: send the affected player their updated hand
        if let Some(updated_seat) = result.hand_updated_seat {
            let hand = {
                let guard = self.state.lock().unwrap();
                guard.as_ref().map(|s| s.hands[updated_seat].clone()).unwrap_or_default()
            };
            self.send_private(updated_seat, StateUpdate::HandUpdated { hand });
        }

        // Public: broadcast phase transition so all clients can update their UI
        if result.phase_complete {
            self.broadcast(StateUpdate::PhaseChanged { phase: GamePhase::Playing });
        }

        Ok(())
    }

    /// Validate and apply a card play from `seat`. On the final trick, broadcasts
    /// `HandComplete` (and `SessionOver` if someone won the session).
    pub fn play_card(&self, seat: usize, card: Card) -> Result<(), String> {
        let (result, newly_revealed_partner) = {
            let mut guard = self.state.lock().unwrap();
            let state = guard.as_mut().ok_or_else(|| "game not started".to_string())?;
            let partner_was_null = state.meta["partner"].is_null();
            let result = self.game.apply_play(state, seat, card)?;
            let newly_revealed = if partner_was_null && !state.meta["partner"].is_null() {
                state.meta["partner"].as_u64().map(|p| p as usize)
            } else {
                None
            };
            (result, newly_revealed)
        };

        self.broadcast(StateUpdate::CardPlayed { player: seat, card });

        if let Some(partner_seat) = newly_revealed_partner {
            self.broadcast(StateUpdate::PartnerRevealed { seat: partner_seat });
        }

        match result {
            PlayResult::Continuing => {}
            PlayResult::TrickComplete { winner, points } => {
                self.broadcast(StateUpdate::TrickComplete { winner, points });
            }
            PlayResult::GameOver { last_trick_winner, last_trick_points, scores } => {
                self.broadcast(StateUpdate::TrickComplete {
                    winner: last_trick_winner,
                    points: last_trick_points,
                });

                let session_scores = {
                    let mut ss = self.session_scores.lock().unwrap();
                    for (i, &delta) in scores.iter().enumerate() {
                        ss[i] += delta;
                    }
                    ss.clone()
                };

                self.broadcast(StateUpdate::HandComplete {
                    hand_scores: scores,
                    session_scores: session_scores.clone(),
                });

                if let Some(winner) = self.session_winner(&session_scores) {
                    self.broadcast(StateUpdate::SessionOver {
                        winner,
                        final_scores: session_scores,
                    });
                }
                // Next hand is started by drive_bots() once it sees Scoring phase.
            }
        }

        Ok(())
    }

    /// Broadcast a public event to every player in the room.
    pub fn broadcast(&self, update: StateUpdate) {
        let _ = self.broadcast_tx.send(update);
    }

    /// Send a private event to one player's channel.
    pub fn send_private(&self, seat: usize, update: StateUpdate) {
        let txs = self.player_txs.lock().unwrap();
        if let Some(Some(tx)) = txs.get(seat) {
            let _ = tx.try_send(update);
        }
    }

    /// Fill all empty seats with server-driven bots, then start the game.
    pub fn fill_bots(&self) {
        let empty_seats: Vec<usize> = {
            let txs = self.player_txs.lock().unwrap();
            txs.iter().enumerate().filter(|(_, t)| t.is_none()).map(|(i, _)| i).collect()
        };
        {
            let mut bots = self.bot_seats.lock().unwrap();
            for &seat in &empty_seats {
                bots[seat] = true;
            }
        }
        for _ in 0..empty_seats.len() {
            let (bot_tx, _bot_rx) = mpsc::channel(16);
            let _ = self.join(bot_tx);
        }
    }

    const BOT_ACTION_DELAY_MS: u64 = 1200;

    /// Apply consecutive bot actions until it's a human player's turn or the session ends.
    /// Sleeps `BOT_ACTION_DELAY_MS` before each action so bots feel like real players.
    pub async fn drive_bots(&self) {
        // Only one drive_bots task per room at a time. If another is running, return immediately.
        if self.bots_running.swap(true, Ordering::SeqCst) {
            return;
        }
        // Reset the flag when this task exits (for any reason).
        struct Guard<'a>(&'a AtomicBool);
        impl Drop for Guard<'_> {
            fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); }
        }
        let _guard = Guard(&self.bots_running);

        loop {
            let (seat, phase) = {
                let guard = self.state.lock().unwrap();
                let Some(state) = guard.as_ref() else { break };
                (state.current_player, state.phase.clone())
            };

            if phase == GamePhase::Scoring {
                let session_scores = self.session_scores.lock().unwrap().clone();
                if self.session_winner(&session_scores).is_some() {
                    break; // Session is over; stop driving.
                }
                // Pause before starting the next hand.
                tokio::time::sleep(std::time::Duration::from_millis(Self::BOT_ACTION_DELAY_MS)).await;
                let next_dealer = {
                    let guard = self.state.lock().unwrap();
                    guard.as_ref().map(|s| (s.dealer + 1) % self.player_count).unwrap_or(0)
                };
                self.start_next_hand(next_dealer);
                continue;
            }

            // Human player's turn — stop driving immediately (no delay).
            if !self.bot_seats.lock().unwrap().get(seat).copied().unwrap_or(false) {
                break;
            }

            // Pause before the bot acts.
            tokio::time::sleep(std::time::Duration::from_millis(Self::BOT_ACTION_DELAY_MS)).await;

            if phase == GamePhase::Bidding {
                let value = {
                    let guard = self.state.lock().unwrap();
                    let Some(state) = guard.as_ref() else { break };
                    crate::bot::bid_action(state, seat)
                };
                if self.apply_bid(seat, value).is_err() { break; }
            } else {
                let card = {
                    let guard = self.state.lock().unwrap();
                    let Some(state) = guard.as_ref() else { break };
                    match crate::bot::play_card(state, seat, self.game.as_ref()) {
                        Some(c) => c,
                        None => break,
                    }
                };
                if self.play_card(seat, card).is_err() { break; }
            }
        }
    }

    fn start_game(&self) {
        let mut rng = rand::thread_rng();
        let dealer = rand::Rng::gen_range(&mut rng, 0..self.player_count);
        self.start_next_hand(dealer);
        tracing::info!(room_id = %self.id, "session started");
    }

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

    /// Returns the session winner if anyone has reached `victory_goal`.
    /// Ties (multiple players at goal on the same hand) broken by highest score.
    fn session_winner(&self, session_scores: &[i32]) -> Option<usize> {
        let goal = self.victory_goal;
        let mut reached: Vec<usize> = session_scores
            .iter()
            .enumerate()
            .filter(|&(_, &s)| s >= goal)
            .map(|(i, _)| i)
            .collect();
        if reached.is_empty() {
            return None;
        }
        reached.sort_by(|&a, &b| session_scores[b].cmp(&session_scores[a]));
        Some(reached[0])
    }
}

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
