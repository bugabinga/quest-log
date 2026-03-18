//! Integration tests for quest toggle endpoint.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode},
    routing::{get, post},
};
use chrono::{Datelike, Utc};
use quest_log::{database::Database, handlers, models::*, state::AppState};

use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_quest_toggle_integration() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest_req = CreateQuestRequest {
        title: "Toggle Test Quest".to_string(),
        description: Some("Testing quest toggle functionality".to_string()),
        exp_value: Some(30),
        day_of_week,
    };
    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .with_state(app_state);

    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("quest-item completed"));
    assert!(body_str.contains("✅ Quest Complete"));
    assert!(body_str.contains("expToday"));
}

#[tokio::test]
async fn test_quest_toggle_bidirectional() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest_req = CreateQuestRequest {
        title: "Bidirectional Toggle Test".to_string(),
        description: None,
        exp_value: Some(15),
        day_of_week,
    };
    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .with_state(app_state);

    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data.clone()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(!body_str.contains("quest-item completed"));
    assert!(body_str.contains("⚔️ Mark Complete"));
}

#[tokio::test]
async fn test_toggle_invalid_quest_id() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .with_state(app_state);

    let json_data = r#"{"quest_id":99999}"#;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_toggle_wrong_day() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let monday = 1i32;
    let quest_req = CreateQuestRequest {
        title: "Monday Quest".to_string(),
        description: None,
        exp_value: Some(10),
        day_of_week: monday,
    };
    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .with_state(app_state);

    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_toggle_returns_proper_datastar_html() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest_req = CreateQuestRequest {
        title: "Datastar Toggle Test".to_string(),
        description: Some("Testing Datastar HTML structure".to_string()),
        exp_value: Some(25),
        day_of_week,
    };
    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .with_state(app_state);

    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("id=\"quest-"));
    assert!(body_str.contains("class=\"quest-item completed\""));
    assert!(body_str.contains("expToday"));
    assert!(!body_str.contains("<form"));
}
