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
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "tricks_server=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let lobby = Arc::new(lobby::Lobby::new());
    let mm = Arc::new(lobby::Matchmaker::new(Arc::clone(&lobby)));
    lobby.matchmaker.set(mm).ok();

    let app = Router::new()
        .route("/ws", get(ws::handler::upgrade))
        .route("/health", get(|| async { "ok" }))
        .route(
            "/api/training/tutorials/:game",
            get(training_tutorials_handler),
        )
        .layer(CorsLayer::permissive())
        .with_state(lobby);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(3000);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn training_tutorials_handler(
    axum::extract::Path(game_name): axum::extract::Path<String>,
) -> axum::response::Json<serde_json::Value> {
    let tutorials = games::get_game(&game_name)
        .map(|g| {
            g.tutorials()
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "id": t.id,
                        "title": t.title,
                        "description": t.description
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    axum::response::Json(serde_json::json!(tutorials))
}
