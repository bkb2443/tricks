// New lobby methods (join_lobby, handle_lobby_chat, start_game, on_disconnect, etc.) will be
// wired to ws/handler.rs in Task 6. Suppress dead-code until then.
#![allow(dead_code)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::engine::{Card, GamePhase, GameState, PlayResult, SeatInfo, StateUpdate, deal_game};
use crate::engine::game::Game;

// ── Seat model ───────────────────────────────────────────────────────────────

enum SeatState {
    Empty,
    Human {
        name: String,
        ws_id: Uuid,
        tx: mpsc::Sender<StateUpdate>,
    },
    Bot,
    Disconnected {
        name: String,
        rejoin_deadline: std::time::Instant,
        /// Whether host has already used their one extend for this seat.
        extend_used: bool,
    },
}

impl SeatState {
    fn is_empty(&self) -> bool { matches!(self, SeatState::Empty) }
    fn is_human(&self) -> bool { matches!(self, SeatState::Human { .. }) }
    fn is_bot(&self) -> bool { matches!(self, SeatState::Bot) }

    fn name(&self) -> Option<&str> {
        match self {
            SeatState::Human { name, .. } => Some(name),
            SeatState::Disconnected { name, .. } => Some(name),
            _ => None,
        }
    }

    fn ws_id(&self) -> Option<Uuid> {
        match self {
            SeatState::Human { ws_id, .. } => Some(*ws_id),
            _ => None,
        }
    }

    fn tx(&self) -> Option<&mpsc::Sender<StateUpdate>> {
        match self {
            SeatState::Human { tx, .. } => Some(tx),
            _ => None,
        }
    }

    fn to_seat_info(&self, seat: usize) -> SeatInfo {
        let (state_str, name) = match self {
            SeatState::Empty => ("empty", None),
            SeatState::Human { name, .. } => ("human", Some(name.clone())),
            SeatState::Bot => ("bot", None),
            SeatState::Disconnected { name, .. } => ("disconnected", Some(name.clone())),
        };
        SeatInfo { seat, state: state_str.into(), name }
    }
}

// ── Room ─────────────────────────────────────────────────────────────────────

pub struct Room {
    pub id: Uuid,
    pub room_code: String,
    pub game_name: String,
    pub player_count: usize,
    pub victory_goal: i32,
    pub room_type: String, // "private" | "public"
    game: Box<dyn Game>,
    seats: Mutex<Vec<SeatState>>,
    broadcast_tx: broadcast::Sender<StateUpdate>,
    pub state: Mutex<Option<GameState>>,
    session_scores: Mutex<Vec<i32>>,
    bots_running: AtomicBool,
    chat_history: Mutex<VecDeque<(String, String, u64)>>, // (from, text, timestamp_ms)
    max_hands: Mutex<Option<u32>>,
    hands_played: Mutex<u32>,
}

impl Room {
    pub fn new(
        id: Uuid,
        game_name: String,
        player_count: usize,
        game: Box<dyn Game>,
        victory_goal: i32,
        room_code: String,
        room_type: String,
    ) -> Self {
        let (broadcast_tx, _) = broadcast::channel(64);
        let seats = (0..player_count).map(|_| SeatState::Empty).collect();
        Self {
            id,
            room_code: room_code.clone(),
            game_name: game_name.clone(),
            player_count,
            victory_goal,
            room_type: room_type.clone(),
            game,
            seats: Mutex::new(seats),
            broadcast_tx,
            state: Mutex::new(Some(GameState::new_lobby(
                id,
                game_name,
                player_count,
                &room_type,
                None,
            ))),
            session_scores: Mutex::new(vec![0; player_count]),
            bots_running: AtomicBool::new(false),
            chat_history: Mutex::new(VecDeque::new()),
            max_hands: Mutex::new(None),
            hands_played: Mutex::new(0),
        }
    }

    pub fn set_max_hands(&self, max: u32) {
        *self.max_hands.lock().unwrap() = Some(max);
    }

    // ── Seat info ─────────────────────────────────────────────────────────────

    fn seat_infos(&self) -> Vec<SeatInfo> {
        let seats = self.seats.lock().unwrap();
        seats.iter().enumerate().map(|(i, s)| s.to_seat_info(i)).collect()
    }

    fn host_seat(&self) -> Option<usize> {
        let seats = self.seats.lock().unwrap();
        seats.iter().position(|s| s.is_human())
    }

    // ── Joining ───────────────────────────────────────────────────────────────

    /// Join the room in lobby phase. Returns `(seat, broadcast_rx)` or `None` if
    /// the room is full or the name is already taken.
    pub fn join_lobby(
        &self,
        name: String,
        ws_id: Uuid,
        tx: mpsc::Sender<StateUpdate>,
    ) -> Option<(usize, broadcast::Receiver<StateUpdate>)> {
        let seat;
        {
            let mut seats = self.seats.lock().unwrap();
            // Name uniqueness check (Human + Disconnected seats reserve their name)
            let name_taken = seats.iter().any(|s| s.name() == Some(name.as_str()));
            if name_taken { return None; }

            seat = seats.iter().position(|s| s.is_empty())?;
            seats[seat] = SeatState::Human { name: name.clone(), ws_id, tx: tx.clone() };
        }

        // Update lobby GameState names and host
        {
            let mut guard = self.state.lock().unwrap();
            if let Some(ref mut state) = *guard {
                state.names = self.compute_names();
                if state.meta["host_seat"].is_null() {
                    state.meta["host_seat"] = serde_json::json!(seat);
                }
            }
        }

        // Send the joiner a lobby snapshot
        let snapshot = {
            let guard = self.state.lock().unwrap();
            guard.as_ref().cloned()
        };
        if let Some(state) = snapshot {
            let _ = tx.try_send(StateUpdate::Snapshot { state });
        }
        // Replay chat history
        {
            let history = self.chat_history.lock().unwrap();
            for (from, text, timestamp) in history.iter() {
                let _ = tx.try_send(StateUpdate::LobbyChat {
                    from: from.clone(),
                    text: text.clone(),
                    timestamp: *timestamp,
                });
            }
        }

        // Send seat update privately to the joiner (they subscribe to broadcast after this call,
        // so they'd miss the broadcast version), then broadcast to other players already in the room.
        let seat_infos = self.seat_infos();
        let _ = tx.try_send(StateUpdate::SeatUpdate { seats: seat_infos.clone() });
        self.broadcast(StateUpdate::SeatUpdate { seats: seat_infos });
        tracing::info!(room_code = %self.room_code, seat, name, "player joined lobby");

        Some((seat, self.broadcast_tx.subscribe()))
    }

    /// Legacy path for solo mode (fill_bots). Kept for backward compat.
    pub fn join(&self, tx: mpsc::Sender<StateUpdate>) -> Option<(usize, broadcast::Receiver<StateUpdate>)> {
        let seat;
        let all_filled;
        {
            let mut seats = self.seats.lock().unwrap();
            seat = seats.iter().position(|s| s.is_empty())?;
            seats[seat] = SeatState::Human {
                name: format!("Player{seat}"),
                ws_id: Uuid::new_v4(),
                tx,
            };
            all_filled = seats.iter().all(|s| !s.is_empty());
        }
        if all_filled { self.start_game_inner(); }
        Some((seat, self.broadcast_tx.subscribe()))
    }

    // ── Lobby chat ────────────────────────────────────────────────────────────

    pub fn handle_lobby_chat(&self, seat: usize, text: String) -> Result<(), String> {
        if text.is_empty() { return Err("message cannot be empty".into()); }
        if text.len() > 200 { return Err("message too long (max 200 chars)".into()); }

        let from = {
            let seats = self.seats.lock().unwrap();
            seats.get(seat).and_then(|s| s.name().map(|n| n.to_string()))
                .unwrap_or_else(|| format!("Seat {seat}"))
        };
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        {
            let mut history = self.chat_history.lock().unwrap();
            history.push_back((from.clone(), text.clone(), timestamp));
            if history.len() > 50 { history.pop_front(); }
        }

        self.broadcast(StateUpdate::LobbyChat { from, text, timestamp });
        Ok(())
    }

    fn system_chat(&self, text: String) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let mut history = self.chat_history.lock().unwrap();
        history.push_back(("System".into(), text.clone(), timestamp));
        if history.len() > 50 { history.pop_front(); }
        drop(history);
        self.broadcast(StateUpdate::LobbyChat { from: "System".into(), text, timestamp });
    }

    // ── Game start ────────────────────────────────────────────────────────────

    /// Fill all Empty seats with bots, then start the first hand.
    pub fn start_game(self: &Arc<Self>) {
        {
            let mut seats = self.seats.lock().unwrap();
            for s in seats.iter_mut() {
                if s.is_empty() { *s = SeatState::Bot; }
            }
        }
        self.start_game_inner();
        let room_arc = Arc::clone(self);
        tokio::spawn(async move { room_arc.drive_bots().await });
        tracing::info!(room_code = %self.room_code, "game started");
    }

    /// For solo / legacy fill_bots path.
    pub fn fill_bots(&self) {
        {
            let mut seats = self.seats.lock().unwrap();
            for s in seats.iter_mut() {
                if s.is_empty() { *s = SeatState::Bot; }
            }
        }
    }

    fn start_game_inner(&self) {
        let dealer = {
            let mut rng = rand::thread_rng();
            rand::Rng::gen_range(&mut rng, 0..self.player_count)
        };
        self.start_next_hand(dealer);
        tracing::info!(room_code = %self.room_code, "session started");
    }

    // ── Disconnection & rejoin ────────────────────────────────────────────────

    pub fn on_disconnect(self: &Arc<Self>, seat: usize, ws_id: Uuid) {
        let is_lobby = {
            let guard = self.state.lock().unwrap();
            guard.as_ref().map(|s| s.phase == GamePhase::Lobby).unwrap_or(true)
        };

        let name = {
            let seats = self.seats.lock().unwrap();
            seats.get(seat).and_then(|s| {
                if s.ws_id() == Some(ws_id) { s.name().map(|n| n.to_string()) } else { None }
            })
        };
        let Some(name) = name else { return };

        if is_lobby {
            let mut seats = self.seats.lock().unwrap();
            if let Some(s) = seats.get_mut(seat) {
                *s = SeatState::Empty;
            }
            drop(seats);
            self.broadcast(StateUpdate::SeatUpdate { seats: self.seat_infos() });
        } else {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
            {
                let mut seats = self.seats.lock().unwrap();
                if let Some(s) = seats.get_mut(seat) {
                    *s = SeatState::Disconnected { name: name.clone(), rejoin_deadline: deadline, extend_used: false };
                }
            }
            self.broadcast(StateUpdate::SeatUpdate { seats: self.seat_infos() });
            self.system_chat(format!("{name} disconnected — 30 seconds to rejoin."));

            let room = Arc::clone(self);
            let name_clone = name.clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                room.on_rejoin_expired(seat, &name_clone);
            });
        }
    }

    pub fn on_rejoin_expired(&self, seat: usize, expected_name: &str) {
        let should_bot = {
            let seats = self.seats.lock().unwrap();
            matches!(seats.get(seat), Some(SeatState::Disconnected { name, .. }) if name == expected_name)
        };
        if should_bot {
            {
                let mut seats = self.seats.lock().unwrap();
                if let Some(s) = seats.get_mut(seat) {
                    *s = SeatState::Bot;
                }
            }
            self.broadcast(StateUpdate::SeatUpdate { seats: self.seat_infos() });
            self.system_chat(format!("{expected_name}'s hand has been taken over by a bot."));
        }
    }

    pub fn on_rejoin(
        &self,
        seat: usize,
        name: &str,
        ws_id: Uuid,
        tx: mpsc::Sender<StateUpdate>,
    ) -> bool {
        let can_rejoin = {
            let seats = self.seats.lock().unwrap();
            matches!(seats.get(seat), Some(SeatState::Disconnected { name: n, rejoin_deadline, .. })
                if n == name && *rejoin_deadline > std::time::Instant::now())
        };
        if !can_rejoin { return false; }

        {
            let mut seats = self.seats.lock().unwrap();
            if let Some(s) = seats.get_mut(seat) {
                *s = SeatState::Human { name: name.to_string(), ws_id, tx: tx.clone() };
            }
        }

        let snapshot = {
            let guard = self.state.lock().unwrap();
            guard.as_ref().map(|s| {
                let mut view = s.clone();
                for (i, hand) in view.hands.iter_mut().enumerate() {
                    if i != seat { hand.clear(); }
                }
                view.extra_piles.clear();
                view
            })
        };
        if let Some(state) = snapshot {
            let _ = tx.try_send(StateUpdate::Snapshot { state });
        }

        self.broadcast(StateUpdate::SeatUpdate { seats: self.seat_infos() });
        self.system_chat(format!("{name} rejoined."));
        true
    }

    pub fn force_bot(&self, seat: usize, requester_seat: usize) -> Result<(), String> {
        if self.host_seat() != Some(requester_seat) {
            return Err("only the host can force a bot takeover".into());
        }
        let name = {
            let seats = self.seats.lock().unwrap();
            match seats.get(seat) {
                Some(SeatState::Disconnected { name, .. }) => name.clone(),
                _ => return Err("seat is not disconnected".into()),
            }
        };
        self.on_rejoin_expired(seat, &name);
        Ok(())
    }

    pub fn extend_rejoin(&self, seat: usize, requester_seat: usize) -> Result<(), String> {
        if self.host_seat() != Some(requester_seat) {
            return Err("only the host can extend rejoin window".into());
        }
        let mut seats = self.seats.lock().unwrap();
        match seats.get_mut(seat) {
            Some(SeatState::Disconnected { rejoin_deadline, extend_used, .. }) => {
                if *extend_used { return Err("extend already used for this seat".into()); }
                *rejoin_deadline += std::time::Duration::from_secs(30);
                *extend_used = true;
                Ok(())
            }
            _ => Err("seat is not disconnected".into()),
        }
    }

    // ── Existing game methods (unchanged signatures) ───────────────────────────

    pub fn apply_bid(&self, seat: usize, value: serde_json::Value) -> Result<(), String> {
        let (result, current_player) = {
            let mut guard = self.state.lock().unwrap();
            let state = guard.as_mut().ok_or_else(|| "game not started".to_string())?;
            let result = self.game.apply_bid(state, seat, &value)?;
            let cp = state.current_player;
            (result, cp)
        };
        let bid_value = result.broadcast_payload.unwrap_or(value);
        self.broadcast(StateUpdate::BidPlaced { player: seat, value: bid_value, current_player });
        if let Some(updated_seat) = result.hand_updated_seat {
            let hand = {
                let guard = self.state.lock().unwrap();
                guard.as_ref().map(|s| s.hands[updated_seat].clone()).unwrap_or_default()
            };
            self.send_private(updated_seat, StateUpdate::HandUpdated { hand });
        }
        if result.phase_complete {
            self.broadcast(StateUpdate::PhaseChanged { phase: GamePhase::Playing });
        }
        Ok(())
    }

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
                    for (i, &delta) in scores.iter().enumerate() { ss[i] += delta; }
                    ss.clone()
                };
                let mut hp = self.hands_played.lock().unwrap();
                *hp += 1;
                let hands_done = *hp;
                drop(hp);

                self.broadcast(StateUpdate::HandComplete {
                    hand_scores: scores,
                    session_scores: session_scores.clone(),
                });

                if (*self.max_hands.lock().unwrap()).is_some_and(|max| hands_done >= max) {
                    let winner = self.session_winner(&session_scores).unwrap_or(0);
                    self.broadcast(StateUpdate::SessionOver {
                        winner,
                        final_scores: session_scores,
                    });
                    return Ok(());
                }

                if let Some(winner) = self.session_winner(&session_scores) {
                    self.broadcast(StateUpdate::SessionOver { winner, final_scores: session_scores });
                }
            }
        }
        Ok(())
    }

    pub fn broadcast(&self, update: StateUpdate) {
        let _ = self.broadcast_tx.send(update);
    }

    pub fn send_private(&self, seat: usize, update: StateUpdate) {
        let seats = self.seats.lock().unwrap();
        if let Some(tx) = seats.get(seat).and_then(|s| s.tx()) {
            let _ = tx.try_send(update);
        }
    }

    const BOT_ACTION_DELAY_MS: u64 = 1200;

    pub async fn drive_bots(self: &Arc<Self>) {
        if self.bots_running.swap(true, Ordering::SeqCst) { return; }
        struct Guard<'a>(&'a AtomicBool);
        impl Drop for Guard<'_> { fn drop(&mut self) { self.0.store(false, Ordering::SeqCst); } }
        let _guard = Guard(&self.bots_running);

        loop {
            let (seat, phase) = {
                let guard = self.state.lock().unwrap();
                let Some(state) = guard.as_ref() else { break };
                (state.current_player, state.phase.clone())
            };

            if phase == GamePhase::Scoring {
                let session_scores = self.session_scores.lock().unwrap().clone();
                let hands_done = *self.hands_played.lock().unwrap();
                let session_over = (*self.max_hands.lock().unwrap()).is_some_and(|max| hands_done >= max)
                    || self.session_winner(&session_scores).is_some();
                if session_over { break; }
                tokio::time::sleep(std::time::Duration::from_millis(Self::BOT_ACTION_DELAY_MS)).await;
                let next_dealer = {
                    let guard = self.state.lock().unwrap();
                    guard.as_ref().map(|s| (s.dealer + 1) % self.player_count).unwrap_or(0)
                };
                self.start_next_hand(next_dealer);
                continue;
            }

            if phase == GamePhase::Lobby { break; }

            let is_bot = {
                let seats = self.seats.lock().unwrap();
                seats.get(seat).map(|s| s.is_bot()).unwrap_or(false)
            };
            if !is_bot { break; }

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

    fn start_next_hand(&self, dealer: usize) {
        let mut rng = rand::thread_rng();
        let mut state = GameState::new(self.id, self.game_name.clone(), self.player_count, dealer);
        deal_game(self.game.as_ref(), &mut state, &mut rng);
        state.names = self.compute_names();
        {
            let seats = self.seats.lock().unwrap();
            for (seat, seat_state) in seats.iter().enumerate() {
                let Some(tx) = seat_state.tx() else { continue };
                let mut view = state.clone();
                for (i, hand) in view.hands.iter_mut().enumerate() {
                    if i != seat { hand.clear(); }
                }
                view.extra_piles.clear();
                let _ = tx.try_send(StateUpdate::Snapshot { state: view });
            }
        }
        *self.state.lock().unwrap() = Some(state);
        tracing::info!(room_code = %self.room_code, dealer, "hand started");
    }

    fn session_winner(&self, session_scores: &[i32]) -> Option<usize> {
        let goal = self.victory_goal;
        let mut reached: Vec<usize> = session_scores.iter().enumerate()
            .filter(|&(_, &s)| s >= goal).map(|(i, _)| i).collect();
        if reached.is_empty() { return None; }
        reached.sort_by(|&a, &b| session_scores[b].cmp(&session_scores[a]));
        Some(reached[0])
    }

    fn compute_names(&self) -> Vec<String> {
        let seats = self.seats.lock().unwrap();
        let mut bot_counter = 0usize;
        seats.iter().map(|s| match s {
            SeatState::Human { name, .. } => name.clone(),
            SeatState::Disconnected { name, .. } => name.clone(),
            SeatState::Bot => { bot_counter += 1; format!("Bot {bot_counter}") }
            SeatState::Empty => "Empty".into(),
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games;

    fn make_room() -> Arc<Room> {
        let game = games::get_game("sheepshead").unwrap();
        Arc::new(Room::new(
            Uuid::new_v4(),
            "sheepshead".into(),
            5,
            game,
            24,
            "TEST-42".into(),
            "private".into(),
        ))
    }

    #[test]
    fn join_lobby_claims_seat() {
        let room = make_room();
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        let result = room.join_lobby("Alice".into(), Uuid::new_v4(), tx);
        assert!(result.is_some());
        let (seat, _) = result.unwrap();
        assert_eq!(seat, 0);
    }

    #[test]
    fn duplicate_name_rejected() {
        let room = make_room();
        let (tx1, _) = tokio::sync::mpsc::channel(16);
        let (tx2, _) = tokio::sync::mpsc::channel(16);
        room.join_lobby("Alice".into(), Uuid::new_v4(), tx1).unwrap();
        assert!(room.join_lobby("Alice".into(), Uuid::new_v4(), tx2).is_none());
    }

    #[test]
    fn lobby_chat_validates_length() {
        let room = make_room();
        let (tx, _) = tokio::sync::mpsc::channel(16);
        room.join_lobby("Alice".into(), Uuid::new_v4(), tx).unwrap();
        assert!(room.handle_lobby_chat(0, "hello".into()).is_ok());
        assert!(room.handle_lobby_chat(0, "".into()).is_err());
        assert!(room.handle_lobby_chat(0, "x".repeat(201)).is_err());
    }
}
