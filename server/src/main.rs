mod bot;
mod engine;
mod games;
mod lobby;
mod ws;

use axum::{Router, routing::get};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::new(
                std::env::var("RUST_LOG").unwrap_or_else(|_| "tricks_server=debug".into()),
            ),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let lobby = Arc::new(lobby::Lobby::new());
    let mm = Arc::new(lobby::Matchmaker::new(Arc::clone(&lobby)));
    lobby.matchmaker.set(mm).ok();

    let app = Router::new()
        .route("/ws", get(ws::handler::upgrade))
        .route("/health", get(|| async { "ok" }))
        .layer(CorsLayer::permissive())
        .with_state(lobby);

    let port = std::env::var("PORT").ok().and_then(|p| p.parse::<u16>().ok()).unwrap_or(3000);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
