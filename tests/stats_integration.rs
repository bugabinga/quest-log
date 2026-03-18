//! Integration tests for highscore/stats handlers.
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
use quest_log::database::Database;
use quest_log::handlers::stats::highscore;
use quest_log::models::{CreateQuestRequest, CreateRewardRequest};
use quest_log::state::AppState;
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_highscore_handler_returns_200_ok() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/stats", get(highscore))
        .with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_highscore_handler_empty_database() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/stats", get(highscore))
        .with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    assert!(html.contains("Highscore") || html.contains("Stats") || html.contains('0'));
}

#[tokio::test]
async fn test_highscore_handler_with_data() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    db.create_quest(CreateQuestRequest {
        title: "Test Quest".to_string(),
        description: None,
        exp_value: Some(50),
        day_of_week: 1,
    })
    .await
    .expect("Failed to create quest");

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
        .route("/stats", get(highscore))
        .with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    assert!(html.contains("Total EXP") || html.contains("Quests Completed"));
}

#[tokio::test]
async fn test_highscore_handler_weekly_champions() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let week_start = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    db.create_weekly_champion(week_start)
        .await
        .expect("Failed to create weekly champion");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/stats", get(highscore))
        .with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/stats")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    assert!(html.contains("Champion") || html.contains('1'));
}
