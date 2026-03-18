//! Integration tests for quest listing web endpoints.
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
use chrono::{Datelike, NaiveDate, Utc};
use quest_log::{database::Database, handlers, models::*, state::AppState, time};

use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_quest_listing_integration() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest_req = CreateQuestRequest {
        title: "Integration Test Quest".to_string(),
        description: Some("Testing quest listing integration".to_string()),
        exp_value: Some(25),
        day_of_week,
    };
    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let quests = db
        .get_quests_for_day(day_of_week)
        .await
        .expect("Failed to get quests");
    assert_eq!(quests.len(), 1);
    assert_eq!(quests[0].title, "Integration Test Quest");

    let completed_today = db
        .is_quest_completed_today(quest.id, today)
        .await
        .expect("Failed to check completion");
    assert!(!completed_today, "New quest should not be completed");
}

#[tokio::test]
async fn test_invalid_date_returns_error() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let quest_req = CreateQuestRequest {
        title: "Test Quest".to_string(),
        description: None,
        exp_value: Some(10),
        day_of_week: 1,
    };
    db.create_quest(quest_req)
        .await
        .expect("Failed to create quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .with_state(app_state);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/?date=invalid-date")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("Wrong Day!"),
        "Expected styled error page, got: {body_str}"
    );

    time::reset_today();
}

#[tokio::test]
async fn test_quest_listing_on_sunday() {
    let sunday = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
    time::set_today(sunday);

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let quest_req = CreateQuestRequest {
        title: "Sunday Quest".to_string(),
        description: Some("Testing quest listing on Sunday".to_string()),
        exp_value: Some(25),
        day_of_week: 0,
    };
    let _quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let today = time::today();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    assert_eq!(day_of_week, 0, "Should be Sunday (0)");
    assert_eq!(today, sunday);

    let quests = db
        .get_quests_for_day(day_of_week)
        .await
        .expect("Failed to get quests");
    assert_eq!(quests.len(), 1);
    assert_eq!(quests[0].title, "Sunday Quest");

    time::reset_today();
}

#[tokio::test]
async fn test_quest_page_returns_200() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest_req = CreateQuestRequest {
        title: "Test Quest".to_string(),
        description: Some("Test description".to_string()),
        exp_value: Some(25),
        day_of_week,
    };
    db.create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

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
    let html = String::from_utf8(body.to_vec()).unwrap();

    assert!(html.contains("Test Quest"));
    assert!(html.contains("25 EXP"));
}

#[tokio::test]
async fn test_no_quests_for_day() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

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
}
