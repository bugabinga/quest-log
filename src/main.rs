mod database;
mod handlers;
mod models;
mod state;

use crate::database::Database;
use crate::handlers::ServerMessage;
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use static_serve::embed_assets;
use std::net::SocketAddr;
use tokio::sync::broadcast;

#[cfg(debug_assertions)]
embed_assets!("static", compress = true);

#[cfg(not(debug_assertions))]
embed_assets!("static", compress = true, ignore_paths = ["*.map"]);

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    println!("Initializing database...");
    let db = Database::new()
        .await
        .expect("Failed to connect to database");

    println!("Database ready!");

    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState {
        db,
        bcast: bcast_tx,
    };

    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/day/{date}", get(handlers::quests))
        .route("/navigate/{date}", get(handlers::navigate))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
        .merge(static_router())
        .with_state(app_state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
