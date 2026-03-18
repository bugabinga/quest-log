//! Integration tests for the bounty/weekly rewards page endpoints.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

//! Tests for the Bounty Board page route.
//!
//! This test verifies that the `/bounty` route exists and returns
//! the weekly rewards (Bounty Board) content.
//!
//! Run these tests to verify the feature is implemented:
//! - `cargo test bounty`

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use quest_log::{database::Database, handlers, state::AppState};
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

/// Test that GET /bounty returns a successful response (200 OK)
///
/// This test verifies the Bounty Board page route exists and works:
/// 1. Creates a test app with the /bounty route
/// 2. Makes a GET request to /bounty
/// 3. Verifies it returns 200 OK
/// 4. Verifies the response contains weekly rewards content
#[tokio::test]
async fn test_bounty_page_returns_200_ok() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app with the /bounty route
    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/bounty", get(handlers::bounty::bounty))
        .with_state(app_state);

    // Make GET request to /bounty
    let response = app
        .oneshot(
            Request::builder()
                .uri("/bounty")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Verify response is 200 OK
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "GET /bounty should return 200 OK, got {}",
        response.status()
    );

    println!("Bounty page returned 200 OK successfully");
}

/// Test that GET /bounty returns weekly rewards content
///
/// This test verifies the Bounty Board page contains the expected content:
/// - Should contain references to "bounty" or "weekly rewards"
/// - Should contain reward-related elements
#[tokio::test]
async fn test_bounty_page_contains_weekly_rewards_content() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app with the /bounty route
    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/bounty", get(handlers::bounty::bounty))
        .with_state(app_state);

    // Make GET request to /bounty
    let response = app
        .oneshot(
            Request::builder()
                .uri("/bounty")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Get response body
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Verify the response contains weekly rewards / bounty content
    // The exact content will depend on the implementation, but should contain
    // relevant terms like "bounty", "reward", "weekly", etc.
    let has_bounty_content = body_str.to_lowercase().contains("bounty")
        || body_str.to_lowercase().contains("reward")
        || body_str.to_lowercase().contains("weekly");

    assert!(
        has_bounty_content,
        "Bounty page should contain bounty/reward/weekly content. Got body: {}",
        &body_str[..body_str.len().min(500)]
    );

    println!("Bounty page contains weekly rewards content successfully");
}

/// Test that the /bounty route is accessible alongside other routes
///
/// This test ensures the bounty route works in the full app context
/// alongside the existing quest routes.
#[tokio::test]
async fn test_bounty_route_works_alongside_quest_routes() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app with both / (quests) and /bounty routes
    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .route("/bounty", get(handlers::bounty::bounty))
        .with_state(app_state);

    // Test that / (quests) still works
    let quests_response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(
        quests_response.status(),
        StatusCode::OK,
        "Quests route should still work"
    );

    // Test that /bounty works
    let bounty_response = app
        .oneshot(
            Request::builder()
                .uri("/bounty")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        bounty_response.status(),
        StatusCode::OK,
        "Bounty route should work alongside quest routes"
    );

    println!("Bounty route works correctly alongside quest routes");
}
