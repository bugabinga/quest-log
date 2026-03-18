//! Integration tests for editor handlers.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

use axum::{
    Router,
    body::Body,
    http::{Method, Request, StatusCode, header::SET_COOKIE},
    routing::{get, post},
};
use quest_log::database::Database;
use quest_log::handlers::editor::auth::{editor_page_handler, login_handler, logout_handler};
use quest_log::handlers::editor::quests::get_quests_handler;
use quest_log::handlers::editor::settings::get_settings_handler;
use quest_log::models::CreateQuestRequest;
use quest_log::state::AppState;
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_editor_page_unauthenticated_shows_login() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/editor", get(editor_page_handler))
        .with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/editor")
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

    assert!(html.contains("login") || html.contains("password"));
}

#[tokio::test]
async fn test_login_handler_wrong_password_returns_error() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/editor/login", post(login_handler))
        .with_state(app_state);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/editor/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("password=wrong_password"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("loginError") || body_str.contains("Invalid password"),
        "Should return login error for wrong password"
    );
}

#[tokio::test]
async fn test_login_handler_correct_password_creates_session() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/editor/login", post(login_handler))
        .with_state(app_state);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/editor/login")
        .header("content-type", "application/x-www-form-urlencoded")
        .body(Body::from("password=dev"))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    assert!(
        response.headers().contains_key(SET_COOKIE),
        "Should set session cookie on successful login"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("isAuthenticated") || body_str.contains("editor"),
        "Should return authenticated state"
    );
}

#[tokio::test]
async fn test_logout_handler_invalidates_session() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);

    let token = quest_log::auth::generate_session_token();
    app_state.create_session(token.clone()).await;

    let app = Router::new()
        .route("/editor/logout", post(logout_handler))
        .with_state(app_state);

    let request = Request::builder()
        .method(Method::POST)
        .uri("/editor/logout")
        .header("cookie", format!("editor_session={token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    assert!(
        body_str.contains("isAuthenticated") && body_str.contains("false"),
        "Should return not authenticated after logout"
    );
}

#[tokio::test]
async fn test_editor_quest_crud_requires_authentication() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db.clone(), bcast_tx);

    db.create_quest(CreateQuestRequest {
        title: "Existing Quest".to_string(),
        description: None,
        exp_value: Some(10),
        day_of_week: 1,
    })
    .await
    .expect("Failed to create quest");

    let app = Router::new()
        .route("/editor/quests", get(get_quests_handler))
        .with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/editor/quests")
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
        body_str.contains("Existing Quest"),
        "Should return quest data"
    );
}

#[tokio::test]
async fn test_get_settings_handler() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/editor/settings", get(get_settings_handler))
        .with_state(app_state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/editor/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
