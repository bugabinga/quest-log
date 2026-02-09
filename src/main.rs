mod database;
mod handlers;
mod models;

use axum::{Router, routing::get};
use database::Database;
use std::net::SocketAddr;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize database
    println!("Initializing database...");
    let db = Database::new()
        .await
        .expect("Failed to connect to database");

    // Run migrations
    println!("Running database migrations...");
    db.migrate().await.expect("Failed to run migrations");

    println!("Database ready!");

    // Build our application with a route
    let app = Router::new()
        .route("/", get(handlers::quests))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(db);

    // Get port from environment or default to 3000
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()
        .expect("PORT must be a number");

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
