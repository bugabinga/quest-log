//! Miscellaneous web integration tests: error handling, security, performance.
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
use std::time::Duration;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_invalid_form_data() {
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

    let invalid_json = r#"{"quest_id":"abc"}"#;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(invalid_json))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    assert!(status.is_client_error() || status.is_success());
}

#[tokio::test]
async fn test_missing_quest_id() {
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

    let missing_json = r#"{"other_param":"value"}"#;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(missing_json))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    assert!(status.is_client_error() || status.is_success());
}

#[tokio::test]
async fn test_xss_prevention() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let xss_payload = "<script>alert('xss')</script>";
    let quest_req = CreateQuestRequest {
        title: xss_payload.to_string(),
        description: None,
        exp_value: Some(10),
        day_of_week,
    };
    db.create_quest(quest_req)
        .await
        .expect("Failed to create XSS test quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .with_state(app_state);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("&#60;script&#62;") || body_str.contains("&lt;script&gt;"),
        "XSS payloads should be HTML escaped"
    );
    assert!(!body_str.contains("<script>"));
}

#[tokio::test]
async fn test_large_quest_list_performance() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    for i in 0..100 {
        let quest_req = CreateQuestRequest {
            title: format!("Performance Quest {i}"),
            description: Some(format!("Description {i}")),
            exp_value: Some((i % 50) + 1),
            day_of_week,
        };
        db.create_quest(quest_req)
            .await
            .expect("Failed to create quest");
    }

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .with_state(app_state);

    let start = std::time::Instant::now();
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        elapsed < Duration::from_secs(2),
        "Page load should be fast with 100 quests"
    );
}

#[tokio::test]
async fn test_concurrent_toggles() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    for i in 0..5 {
        let quest_req = CreateQuestRequest {
            title: format!("Concurrent Quest {i}"),
            description: None,
            exp_value: Some(10),
            day_of_week,
        };
        db.create_quest(quest_req)
            .await
            .expect("Failed to create quest");
    }

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .with_state(app_state);

    let mut handles = vec![];
    for user_id in 0..10 {
        let app_clone = app.clone();

        let handle = tokio::spawn(async move {
            let quest_id = user_id % 5 + 1;
            let json_data = format!(r#"{{"quest_id":{quest_id}}}"#);

            app_clone
                .oneshot(
                    Request::builder()
                        .method(Method::POST)
                        .uri("/quests/toggle")
                        .header("content-type", "application/json")
                        .body(Body::from(json_data))
                        .unwrap(),
                )
                .await
        });

        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        if let Ok(Ok(response)) = handle.await
            && response.status().is_success()
        {
            success_count += 1;
        }
    }

    assert!(success_count > 0, "At least some requests should succeed");
}
