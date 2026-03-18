//! Integration tests for navigation endpoint.
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
use chrono::{Datelike, Utc};
use quest_log::{database::Database, handlers, models::*, state::AppState};

use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_navigate_to_today() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest_req = CreateQuestRequest {
        title: "Navigate Test Quest".to_string(),
        description: None,
        exp_value: Some(15),
        day_of_week,
    };
    db.create_quest(quest_req)
        .await
        .expect("Failed to create quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/navigate/{date}", get(handlers::navigate::navigate))
        .with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/navigate/today")
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

    assert!(body_str.contains("datastar-patch-elements") || body_str.contains("Navigate"));
}

#[tokio::test]
async fn test_navigate_to_specific_date() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let monday = 1i32;
    let quest_req = CreateQuestRequest {
        title: "Monday Quest".to_string(),
        description: None,
        exp_value: Some(20),
        day_of_week: monday,
    };
    db.create_quest(quest_req)
        .await
        .expect("Failed to create quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/navigate/{date}", get(handlers::navigate::navigate))
        .with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/navigate/2024-01-01")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_navigate_invalid_date() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/navigate/{date}", get(handlers::navigate::navigate))
        .with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/navigate/invalid-date")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_navigate_does_not_broadcast() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest_req = CreateQuestRequest {
        title: "Navigate Broadcast Test".to_string(),
        description: None,
        exp_value: Some(15),
        day_of_week,
    };
    db.create_quest(quest_req)
        .await
        .expect("Failed to create quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/navigate/{date}", get(handlers::navigate::navigate))
        .with_state(app_state.clone());

    let mut receiver = app_state.bcast.subscribe();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/navigate/today")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let result =
        tokio::time::timeout(tokio::time::Duration::from_millis(100), receiver.recv()).await;

    assert!(
        result.is_err(),
        "Navigate should NOT broadcast to other clients"
    );
}
