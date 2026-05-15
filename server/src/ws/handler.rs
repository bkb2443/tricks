use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

use crate::{
    engine::{ClientMessage, StateUpdate},
    lobby::{Lobby, Room},
};

type Sink = futures_util::stream::SplitSink<WebSocket, Message>;

/// State for a connected player once they've joined a room.
struct PlayerCtx {
    seat: usize,
    room: Arc<Room>,
    broadcast_rx: broadcast::Receiver<StateUpdate>,
}

pub async fn upgrade(ws: WebSocketUpgrade, State(lobby): State<Arc<Lobby>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, lobby))
}

async fn handle_socket(socket: WebSocket, lobby: Arc<Lobby>) {
    let (mut sink, mut stream) = socket.split();

    // Private channel: room → this player only (e.g. Snapshot on deal, HandUpdated on pick).
    let (player_tx, mut player_rx) = mpsc::channel::<StateUpdate>(16);

    let mut ctx: Option<PlayerCtx> = None;

    loop {
        tokio::select! {
            // ── Incoming message from client ──────────────────────────────
            msg = stream.next() => {
                let Some(Ok(msg)) = msg else { break };
                match msg {
                    Message::Text(text) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(client_msg) => {
                                let reply = route(client_msg, &lobby, &player_tx, &mut ctx);
                                if let Some(upd) = reply {
                                    if send(&mut sink, &upd).await.is_err() { break; }
                                }
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

            // ── Private message from room (Snapshot, HandUpdated) ─────────
            Some(upd) = player_rx.recv() => {
                if send(&mut sink, &upd).await.is_err() { break; }
            }

            // ── Broadcast from room (BidPlaced, PhaseChanged, CardPlayed…) ─
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
                if let Some(u) = upd {
                    if send(&mut sink, &u).await.is_err() { break; }
                }
            }
        }
    }
}

fn route(
    msg: ClientMessage,
    lobby: &Lobby,
    player_tx: &mpsc::Sender<StateUpdate>,
    ctx: &mut Option<PlayerCtx>,
) -> Option<StateUpdate> {
    match msg {
        ClientMessage::JoinRoom { room_id, game, players, fill_bots } => {
            let room = match room_id {
                Some(id) => lobby.get_room(id).or_else(|| lobby.create_room(game, players, 24))?,
                None => lobby.create_room(game, players, 24)?,
            };
            match room.join(player_tx.clone()) {
                Some((seat, broadcast_rx)) => {
                    tracing::info!(room_id = %room.id, seat, "player joined");
                    let reply = StateUpdate::JoinedRoom { room_id: room.id, seat };
                    if fill_bots {
                        room.fill_bots();
                        room.drive_bots();
                    }
                    *ctx = Some(PlayerCtx { seat, room, broadcast_rx });
                    Some(reply)
                }
                None => Some(StateUpdate::Error { message: "room is full".into() }),
            }
        }

        ClientMessage::Bid { value } => {
            let Some(c) = ctx.as_ref() else {
                return Some(StateUpdate::Error { message: "not in a room".into() });
            };
            match c.room.apply_bid(c.seat, value) {
                Ok(()) => { c.room.drive_bots(); None }
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }

        ClientMessage::PlayCard { card } => {
            let Some(c) = ctx.as_ref() else {
                return Some(StateUpdate::Error { message: "not in a room".into() });
            };
            match c.room.play_card(c.seat, card) {
                Ok(()) => { c.room.drive_bots(); None }
                Err(msg) => Some(StateUpdate::Error { message: msg }),
            }
        }
    }
}

async fn send(sink: &mut Sink, update: &StateUpdate) -> Result<(), axum::Error> {
    let json = serde_json::to_string(update).expect("StateUpdate serialization failed");
    sink.send(Message::Text(json.into())).await
}
