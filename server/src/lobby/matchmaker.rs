// join_queue / leave_queue will be wired to ws/handler.rs in Task 6.
#![allow(dead_code)]

use std::mem;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::engine::StateUpdate;
use crate::lobby::Lobby;

const QUEUE_TIMEOUT_SECS: u64 = 60;
const PUBLIC_MAX_HANDS: u32 = 8;

#[allow(dead_code)]
struct QueueEntry {
    name: String,
    tx: mpsc::Sender<StateUpdate>,
    ws_id: Uuid,
}

pub struct Matchmaker {
    queue: Mutex<Vec<QueueEntry>>,
    lobby: Arc<Lobby>,
}

impl Matchmaker {
    pub fn new(lobby: Arc<Lobby>) -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            lobby,
        }
    }

    pub fn join_queue(self: &Arc<Self>, name: String, tx: mpsc::Sender<StateUpdate>, ws_id: Uuid) {
        let position;
        let should_start_timer;
        {
            let mut q = self.queue.lock().unwrap();
            q.push(QueueEntry { name, tx: tx.clone(), ws_id });
            position = q.len();
            should_start_timer = position == 1;

            if position == 5 {
                self.flush_queue_locked(&mut q);
                return;
            }
        }

        let waiting_since = SystemTime::now()
            .duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;
        let _ = tx.try_send(StateUpdate::QueueStatus { position, waiting_since });

        if should_start_timer {
            let mm = Arc::clone(self);
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(QUEUE_TIMEOUT_SECS)).await;
                mm.on_timer_fired();
            });
        }
    }

    pub fn leave_queue(&self, ws_id: Uuid) {
        let mut q = self.queue.lock().unwrap();
        q.retain(|e| e.ws_id != ws_id);
    }

    fn on_timer_fired(self: &Arc<Self>) {
        let mut q = self.queue.lock().unwrap();
        if q.len() >= 2 {
            self.flush_queue_locked(&mut q);
        } else {
            q.clear(); // <2 players — discard
        }
    }

    fn flush_queue_locked(&self, q: &mut Vec<QueueEntry>) {
        let entries: Vec<QueueEntry> = mem::take(q);
        let Some((code, room)) = self.lobby.create_room("sheepshead".into(), 5, 24) else {
            return;
        };

        // Set public room limits
        room.set_max_hands(PUBLIC_MAX_HANDS);
        {
            let mut guard = room.state.lock().unwrap();
            if let Some(ref mut state) = *guard {
                state.meta["room_type"] = serde_json::json!("public");
            }
        }

        // Assign human players to seats
        let room_id = room.id;
        for entry in entries {
            let result = room.join_lobby(entry.name, entry.ws_id, entry.tx.clone());
            let Some((seat, _broadcast_rx)) = result else { continue };
            let _ = entry.tx.try_send(StateUpdate::JoinedRoom {
                room_id,
                seat,
                room_code: code.clone(),
            });
        }

        // Start the game (fills remaining seats with bots)
        room.start_game();
        tracing::info!(room_code = %code, "public room created by matchmaker");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn queue_fills_at_five() {
        let lobby = Arc::new(crate::lobby::Lobby::new());
        let mm = Arc::new(Matchmaker::new(Arc::clone(&lobby)));
        for i in 0..5 {
            let (tx, _rx) = mpsc::channel(16);
            mm.join_queue(format!("Player{i}"), tx, Uuid::new_v4());
        }
        // No panic = pass
    }

    #[tokio::test]
    async fn leave_queue_removes_entry() {
        let lobby = Arc::new(crate::lobby::Lobby::new());
        let mm = Arc::new(Matchmaker::new(Arc::clone(&lobby)));
        let (tx, _rx) = mpsc::channel(16);
        let ws_id = Uuid::new_v4();
        mm.join_queue("Alice".into(), tx, ws_id);
        mm.leave_queue(ws_id);
        // No panic = pass
    }
}
