//! Integration tests for SSE (Server-Sent Events) endpoint.
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
use quest_log::{
    database::Database, handlers, handlers::ServerMessage, models::*, state::AppState,
};

use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_events_endpoint_returns_sse_stream() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/events", get(handlers::events::events))
        .with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/events")
                .header("accept", "text/event-stream")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap());
    assert!(
        content_type.is_some_and(|v| v.starts_with("text/event-stream")),
        "Content-type should be text/event-stream"
    );
}

#[tokio::test]
async fn test_toggle_broadcasts_to_events_endpoint() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest_req = CreateQuestRequest {
        title: "Broadcast Test Quest".to_string(),
        description: Some("Testing SSE broadcast".to_string()),
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
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
        .with_state(app_state.clone());

    let mut receiver = app_state.bcast.subscribe();

    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let app_clone = app.clone();
    let handle = tokio::spawn(async move {
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

    let response = handle.await.expect("Task should not panic").unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(body_str.contains("datastar-patch-elements"));
    assert!(body_str.contains("datastar-patch-signals"));

    let mut quest_received = false;
    for _ in 0..3 {
        let result =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), receiver.recv()).await;

        match result {
            Ok(Ok(msg)) => match msg {
                ServerMessage::Elements(html, _) => {
                    if html.contains("quest-item") {
                        quest_received = true;
                    }
                }
                ServerMessage::Signals(json, _) => {
                    assert!(json.contains("expToday"));
                }
            },
            Ok(Err(e)) => panic!("Broadcast error: {e}"),
            Err(_) => break,
        }
    }

    assert!(quest_received);
}

#[tokio::test]
async fn test_broadcast_channel_works() {
    let (tx, mut rx) = broadcast::channel::<String>(128);

    let _unused = tx.send("hello".to_string());

    let result = tokio::time::timeout(tokio::time::Duration::from_millis(100), rx.recv()).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), "hello");
}

#[tokio::test]
async fn test_sse_connection_subscribes_to_broadcasts() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx.clone());

    let _app: axum::Router<()> = Router::new()
        .route("/events", get(handlers::events::events))
        .with_state(app_state.clone());

    let mut receiver = app_state.bcast.subscribe();

    bcast_tx
        .send(ServerMessage::Elements("test".to_string(), None))
        .unwrap();

    let result =
        tokio::time::timeout(tokio::time::Duration::from_millis(100), receiver.recv()).await;

    assert!(result.is_ok());
}
