use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::{
    engine::{ClientMessage, GamePhase, StateUpdate},
    lobby::{Lobby, Room},
};

type Sink = futures_util::stream::SplitSink<WebSocket, Message>;

struct PlayerCtx {
    seat: usize,
    name: Option<String>,
    ws_id: Uuid,
    room: Arc<Room>,
    broadcast_rx: broadcast::Receiver<StateUpdate>,
}

pub async fn upgrade(ws: WebSocketUpgrade, State(lobby): State<Arc<Lobby>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, lobby))
}

async fn handle_socket(socket: WebSocket, lobby: Arc<Lobby>) {
    let (mut sink, mut stream) = socket.split();
    let (player_tx, mut player_rx) = mpsc::channel::<StateUpdate>(16);
    let ws_id = Uuid::new_v4();

    let mut ctx: Option<PlayerCtx> = None;

    loop {
        tokio::select! {
            msg = stream.next() => {
                let Some(Ok(msg)) = msg else { break };
                match msg {
                    Message::Text(text) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(client_msg) => {
                                let reply = route(client_msg, &lobby, &player_tx, ws_id, &mut ctx);
                                if let Some(upd) = reply
                                    && send(&mut sink, &upd).await.is_err() { break; }
                            }
                            Err(e) => {
                                let err = StateUpdate::Error { message: e.to_string() };
                                if send(&mut sink, &err).await.is_err() { break; }
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }

            Some(upd) = player_rx.recv() => {
                if send(&mut sink, &upd).await.is_err() { break; }
            }

            upd = async {
                match ctx.as_mut() {
                    Some(c) => match c.broadcast_rx.recv().await {
                        Ok(u) => Some(u),
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::warn!("broadcast lagged by {n} messages");
                            None
                        }
                        Err(broadcast::error::RecvError::Closed) => None,
                    },
                    None => std::future::pending::<Option<StateUpdate>>().await,
                }
            } => {
                if let Some(u) = upd
                    && send(&mut sink, &u).await.is_err() { break; }
            }
        }
    }

    // Cleanup on disconnect
    if let Some(c) = ctx {
        let phase = {
            let guard = c.room.state.lock().unwrap();
            guard.as_ref().map(|s| s.phase.clone())
        };
        if phase == Some(GamePhase::Lobby) {
            c.room.on_disconnect(c.seat, c.ws_id);
        } else {
            let room = Arc::clone(&c.room);
            tokio::spawn(async move { room.on_disconnect(c.seat, c.ws_id); });
        }
        if let Some(mm) = lobby.matchmaker.get() {
            mm.leave_queue(c.ws_id);
        }
    } else {
        if let Some(mm) = lobby.matchmaker.get() {
            mm.leave_queue(ws_id);
        }
    }
}

fn route(
    msg: ClientMessage,
    lobby: &Lobby,
    player_tx: &mpsc::Sender<StateUpdate>,
    ws_id: Uuid,
    ctx: &mut Option<PlayerCtx>,
) -> Option<StateUpdate> {
    match msg {
        // ── Legacy solo path ─────────────────────────────────────────────────
        ClientMessage::JoinRoom { room_id, game, players, fill_bots } => {
            if ctx.is_some() {
                return Some(StateUpdate::Error { message: "already in a room".into() });
            }
            let room = match room_id {
                Some(ref id) => lobby.get_room(&id.to_string()).or_else(|| {
                    lobby.create_room(game, players, 24).map(|(_, r)| r)
                })?,
                None => lobby.create_room(game, players, 24).map(|(_, r)| r)?,
            };
            match room.join(player_tx.clone()) {
                Some((seat, broadcast_rx)) => {
                    let code = room.room_code.clone();
                    let reply = StateUpdate::JoinedRoom { room_id: room.id, seat, room_code: code };
                    if fill_bots {
                        let room_arc = Arc::clone(&room);
                        room_arc.fill_bots();
                        room_arc.start_game();
                    }
                    *ctx = Some(PlayerCtx { seat, name: None, ws_id, room, broadcast_rx });
                    Some(reply)
                }
                None => Some(StateUpdate::Error { message: "room is full".into() }),
            }
        }

        // ── Multiplayer: create a new private room ────────────────────────────
        ClientMessage::CreateRoom { name, game, max_hands } => {
            if ctx.is_some() {
                return Some(StateUpdate::Error { message: "already in a room".into() });
            }
            let (code, room) = match lobby.create_room(game, 5, 24) {
                Some(r) => r,
                None => return Some(StateUpdate::Error { message: "unknown game".into() }),
            };
            if let Some(mh) = max_hands {
                room.set_max_hands(mh);
            }
            match room.join_lobby(name.clone(), ws_id, player_tx.clone()) {
                Some((seat, broadcast_rx)) => {
                    let reply = StateUpdate::JoinedRoom { room_id: room.id, seat, room_code: code };
                    *ctx = Some(PlayerCtx { seat, name: Some(name), ws_id, room, broadcast_rx });
                    Some(reply)
                }
                None => Some(StateUpdate::Error { message: "failed to join room".into() }),
            }
        }

        // ── Multiplayer: join existing room by short code ─────────────────────
        ClientMessage::Join { name, room_code } => {
            if ctx.is_some() {
                return Some(StateUpdate::Error { message: "already in a room".into() });
            }
            let room = match lobby.get_room(&room_code) {
                Some(r) => r,
                None => return Some(StateUpdate::Error {
                    message: format!("room '{room_code}' not found"),
                }),
            };
            match room.join_lobby(name.clone(), ws_id, player_tx.clone()) {
                Some((seat, broadcast_rx)) => {
                    let reply = StateUpdate::JoinedRoom { room_id: room.id, seat, room_code };
                    *ctx = Some(PlayerCtx { seat, name: Some(name), ws_id, room, broadcast_rx });
                    Some(reply)
                }
                None => Some(StateUpdate::Error {
                    message: "room is full or name already taken".into(),
                }),
            }
        }

        // ── Game actions ──────────────────────────────────────────────────────
        ClientMessage::Bid { value } => {
            let c = ctx.as_ref()?;
            match c.room.apply_bid(c.seat, value) {
                Ok(()) => {
                    let room_arc = Arc::clone(&c.room);
                    tokio::spawn(async move { room_arc.drive_bots().await });
                    None
                }
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        ClientMessage::PlayCard { card } => {
            let c = ctx.as_ref()?;
            match c.room.play_card(c.seat, card) {
                Ok(()) => {
                    let room_arc = Arc::clone(&c.room);
                    tokio::spawn(async move { room_arc.drive_bots().await });
                    None
                }
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        // ── Lobby actions ─────────────────────────────────────────────────────
        ClientMessage::LobbyChat { text } => {
            let c = ctx.as_ref()?;
            match c.room.handle_lobby_chat(c.seat, text) {
                Ok(()) => None,
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        ClientMessage::StartGame => {
            let c = ctx.as_ref()?;
            let room = Arc::clone(&c.room);
            room.start_game();
            None
        }

        ClientMessage::ForceBot { seat } => {
            let c = ctx.as_ref()?;
            match c.room.force_bot(seat, c.seat) {
                Ok(()) => None,
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        ClientMessage::ExtendRejoin { seat } => {
            let c = ctx.as_ref()?;
            match c.room.extend_rejoin(seat, c.seat) {
                Ok(()) => None,
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        // ── Match-play ────────────────────────────────────────────────────────
        ClientMessage::StartNextHand => {
            let c = ctx.as_ref()?;
            let seat = c.seat;
            match c.room.start_next_hand_dealer(seat) {
                Ok(()) => {
                    let room_arc = Arc::clone(&c.room);
                    tokio::spawn(async move { room_arc.drive_bots().await });
                    None
                }
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        // ── Matchmaking ───────────────────────────────────────────────────────
        ClientMessage::JoinQueue => {
            let name = ctx.as_ref()
                .and_then(|c| c.name.clone())
                .unwrap_or_else(|| format!("Player-{}", &ws_id.to_string()[..4]));
            if let Some(mm) = lobby.matchmaker.get() {
                let mm_arc = Arc::clone(mm);
                mm_arc.join_queue(name, player_tx.clone(), ws_id);
            }
            None
        }

        ClientMessage::LeaveQueue => {
            if let Some(mm) = lobby.matchmaker.get() {
                mm.leave_queue(ws_id);
            }
            None
        }
    }
}

async fn send(sink: &mut Sink, update: &StateUpdate) -> Result<(), axum::Error> {
    let json = serde_json::to_string(update).expect("StateUpdate serialization failed");
    sink.send(Message::Text(json)).await
}
