//! Integration tests for bounty page web endpoints.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::get,
};
use quest_log::{database::Database, handlers, models::*, state::AppState};

use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_bounty_page_returns_200() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    db.create_reward(CreateRewardRequest {
        title: "Test Reward".to_string(),
        description: None,
        required_exp: 50,
    })
    .await
    .expect("Failed to create reward");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/bounty", get(handlers::bounty::bounty))
        .with_state(app_state);

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

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.to_lowercase().contains("bounty")
            || body_str.to_lowercase().contains("reward")
            || body_str.to_lowercase().contains("weekly"),
        "Bounty page should contain bounty/reward/weekly content"
    );
}

#[tokio::test]
async fn test_bounty_page_contains_rewards() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    db.create_reward(CreateRewardRequest {
        title: "Bronze Reward".to_string(),
        description: Some("50 EXP reward".to_string()),
        required_exp: 50,
    })
    .await
    .expect("Failed to create reward");

    db.create_reward(CreateRewardRequest {
        title: "Silver Reward".to_string(),
        description: Some("100 EXP reward".to_string()),
        required_exp: 100,
    })
    .await
    .expect("Failed to create reward");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/bounty", get(handlers::bounty::bounty))
        .with_state(app_state);

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

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("Bronze Reward"));
    assert!(body_str.contains("Silver Reward"));
}

#[tokio::test]
async fn test_bounty_route_works_alongside_quest_routes() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .route("/bounty", get(handlers::bounty::bounty))
        .with_state(app_state);

    let quests_response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(quests_response.status(), StatusCode::OK);

    let bounty_response = app
        .oneshot(
            Request::builder()
                .uri("/bounty")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(bounty_response.status(), StatusCode::OK);
}
