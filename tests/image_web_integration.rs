//! Integration tests for quest and reward image endpoints.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::get,
};
use quest_log::{database::Database, handlers, models::CreateQuestRequest, state::AppState};
use sqlx::SqlitePool;
use tower::util::ServiceExt;

async fn test_db() -> Database {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");
    db
}

fn test_router(db: Database) -> Router {
    let (bcast_tx, _) = tokio::sync::broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    Router::new()
        .route("/quests/{id}/image", get(handlers::quests::quest_image))
        .route("/rewards/{id}/image", get(handlers::bounty::reward_image))
        .with_state(app_state)
}

#[tokio::test]
async fn quest_image_returns_stored_bytes() {
    let db = test_db().await;
    let quest = db
        .create_quest_with_image(
            "Dragon".to_string(),
            None,
            Some(5),
            1,
            Some(vec![1, 2, 3]),
            Some("image/png".to_string()),
        )
        .await
        .expect("Failed to create quest with image");

    let response = test_router(db)
        .oneshot(
            Request::builder()
                .uri(format!("/quests/{id}/image", id = quest.id))
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Failed to fetch quest image");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "image/png"
    );
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .expect("Failed to read body");
    assert_eq!(&body[..], &[1, 2, 3]);
}

#[tokio::test]
async fn quest_image_without_image_is_not_found() {
    let db = test_db().await;
    let quest = db
        .create_quest(CreateQuestRequest {
            title: "No image".to_string(),
            description: None,
            exp_value: Some(5),
            day_of_week: 1,
        })
        .await
        .expect("Failed to create quest");

    let response = test_router(db)
        .oneshot(
            Request::builder()
                .uri(format!("/quests/{id}/image", id = quest.id))
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Failed to fetch quest image");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn quest_image_for_unknown_quest_is_not_found() {
    let db = test_db().await;

    let response = test_router(db)
        .oneshot(
            Request::builder()
                .uri("/quests/999/image")
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Failed to fetch quest image");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn reward_image_returns_stored_bytes() {
    let db = test_db().await;
    let reward = db
        .create_reward_with_image(
            "Treasure".to_string(),
            None,
            50,
            Some(vec![4, 5]),
            Some("image/jpeg".to_string()),
        )
        .await
        .expect("Failed to create reward with image");

    let response = test_router(db)
        .oneshot(
            Request::builder()
                .uri(format!("/rewards/{id}/image", id = reward.id))
                .body(Body::empty())
                .expect("Failed to build request"),
        )
        .await
        .expect("Failed to fetch reward image");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        "image/jpeg"
    );
}
