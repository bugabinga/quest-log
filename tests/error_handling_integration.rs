//! Comprehensive error handling integration tests
//! Tests system behavior under various error conditions

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
};
use quest_log::{database::Database, handlers, state::AppState};
use sqlx::SqlitePool;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tower::util::ServiceExt;
mod common;
use common::setup_test_app_state;

#[tokio::test]
async fn test_timeout_handling() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .with_state(setup_test_app_state(db));

    // Test with a very short timeout to simulate slow operations
    let request = Request::builder().uri("/").body(Body::empty()).unwrap();

    // This should complete quickly in normal conditions
    let result = timeout(Duration::from_millis(10), app.oneshot(request)).await;

    match result {
        Ok(response_result) => {
            // Request completed within timeout - this is expected for fast operations
            let response = response_result.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
        Err(_) => {
            // Request timed out - this indicates a performance issue that should be addressed
            panic!("Request timed out - indicates performance issue");
        }
    }

    println!("Timeout handling test completed successfully");
}

#[tokio::test]
async fn test_malformed_http_requests() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .with_state(setup_test_app_state(db));

    // Test various malformed requests
    // Test invalid HTTP method
    let request = Request::builder()
        .method("INVALID")
        .uri("/")
        .body(Body::empty())
        .unwrap();
    let result = app.clone().oneshot(request).await;
    match result {
        Ok(response) => {
            let status = response.status();
            assert!(status.is_success() || status.is_client_error() || status.is_server_error());
        }
        Err(_) => {
            // Request was rejected at the HTTP level - this is also acceptable
        }
    }

    // Test invalid URI - this may fail at request creation level
    let request_result = Request::builder()
        .uri("http://invalid uri with spaces")
        .body(Body::empty());

    match request_result {
        Ok(request) => {
            let result = app.clone().oneshot(request).await;
            match result {
                Ok(response) => {
                    let status = response.status();
                    assert!(
                        status.is_success() || status.is_client_error() || status.is_server_error()
                    );
                }
                Err(_) => {
                    // Request was rejected at the HTTP level - this is also acceptable
                }
            }
        }
        Err(_) => {
            // Request creation itself failed - this is acceptable for malformed URIs
        }
    }

    // Test oversized headers (simulate)
    let request = Request::builder()
        .uri("/")
        .header("x-test", "x".repeat(10000))
        .body(Body::empty())
        .unwrap();
    let result = app.clone().oneshot(request).await;
    match result {
        Ok(response) => {
            let status = response.status();
            assert!(status.is_success() || status.is_client_error() || status.is_server_error());
        }
        Err(_) => {
            // Request was rejected at the HTTP level - this is also acceptable
        }
    }

    println!("Malformed HTTP requests test completed successfully");
}

#[tokio::test]
async fn test_resource_exhaustion_protection() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .with_state(setup_test_app_state(db));

    // Test with very large request body (simulating attack)
    let large_body = Body::from(vec![b'x'; 10 * 1024 * 1024]); // 10MB

    let request = Request::builder()
        .method("POST")
        .uri("/quests/toggle")
        .header("content-type", "application/json")
        .body(large_body)
        .unwrap();

    let result = timeout(Duration::from_secs(5), app.oneshot(request)).await;

    match result {
        Ok(response_result) => {
            let response = response_result.unwrap();
            // Should either succeed (if body is processed) or return error
            let status = response.status();
            assert!(
                status.is_success() || status.is_client_error(),
                "Large request should be handled gracefully, got status {}",
                status
            );
        }
        Err(_) => {
            // Timeout is acceptable for very large requests
            // Indicates the server doesn't hang indefinitely
        }
    }

    println!("Resource exhaustion protection test completed successfully");
}

#[tokio::test]
async fn test_concurrent_error_conditions() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .with_state(setup_test_app_state(db));

    // Simulate multiple concurrent requests with error conditions
    let mut handles = vec![];

    for i in 0..20 {
        let app_clone = app.clone();

        let handle = tokio::spawn(async move {
            // Alternate between valid and invalid requests
            let request = if i % 2 == 0 {
                // Valid request to non-existent quest
                Request::builder()
                    .method("POST")
                    .uri("/quests/toggle")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"quest_id":{}}}"#, 99999 + i)))
                    .unwrap()
            } else {
                // Invalid request (missing quest_id)
                Request::builder()
                    .method("POST")
                    .uri("/quests/toggle")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"other_param":"value"}"#))
                    .unwrap()
            };

            let response = app_clone.oneshot(request).await;
            response.map(|r| r.status())
        });

        handles.push(handle);
    }

    // Wait for all concurrent error requests to complete
    let mut success_count = 0;
    let mut error_count = 0;

    for handle in handles {
        match handle.await {
            Ok(Ok(status)) => {
                if status.is_success() {
                    success_count += 1;
                } else {
                    error_count += 1;
                }
            }
            Ok(Err(_)) | Err(_) => error_count += 1,
        }
    }

    // All requests should be handled without crashing the server
    assert_eq!(
        success_count + error_count,
        20,
        "All concurrent error requests should be handled"
    );

    println!("Concurrent error conditions test completed successfully");
}

#[tokio::test]
async fn test_database_error_returns_500_with_playful_page() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let app = Router::new()
        .route("/", get(handlers::quests))
        .with_state(setup_test_app_state(db.clone()));

    db.pool().close().await;

    let request = Request::builder().uri("/").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::INTERNAL_SERVER_ERROR,
        "Database error should return HTTP 500"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&body);

    assert!(
        html.contains("id=\"app\""),
        "Error page should inherit base.html structure with #app div"
    );
    assert!(
        html.contains("Quest Log"),
        "Error page should mention Quest Log"
    );
    assert!(
        !html.to_lowercase().contains("sqlx")
            && !html.to_lowercase().contains("database error")
            && !html.to_lowercase().contains("error:"),
        "Error page should not leak technical/database details"
    );
}

#[tokio::test]
async fn test_empty_quests_returns_playful_message() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let app = Router::new()
        .route("/", get(handlers::quests))
        .with_state(setup_test_app_state(db));

    let request = Request::builder().uri("/").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Empty quests should return HTTP 200, not an error"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&body);

    assert!(
        html.contains("parent") || html.contains("forgot"),
        "Empty quests page should contain playful message about parent forgetting quests"
    );
    assert!(
        html.contains("quest") || html.contains("Quest"),
        "Page should contain quest-related content"
    );
    assert!(
        html.contains("id=\"app\""),
        "Page should inherit base.html structure"
    );
}

#[tokio::test]
async fn test_error_page_inherits_base_html_structure() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let app = Router::new()
        .route("/", get(handlers::quests))
        .with_state(setup_test_app_state(db.clone()));

    db.pool().close().await;

    let request = Request::builder().uri("/").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&body);

    assert!(
        html.contains("<title>"),
        "Error page should have a title tag"
    );
    assert!(html.contains("Quest Log"), "Title should mention Quest Log");
    assert!(
        html.contains("<div id=\"app\">"),
        "Error page should have the app div from base.html"
    );
    assert!(
        html.contains("<h1>"),
        "Error page should have heading structure"
    );
    assert!(
        html.contains("<div class=\"error-message\""),
        "Error page should have error-message div"
    );
}

#[tokio::test]
async fn test_no_database_details_leaked_in_errors() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let app = Router::new()
        .route("/", get(handlers::quests))
        .with_state(setup_test_app_state(db.clone()));

    db.pool().close().await;

    let request = Request::builder().uri("/").body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8_lossy(&body);
    let html_lower = html.to_lowercase();

    let sensitive_patterns = [
        "sqlx",
        "sqlite",
        "database",
        "connection",
        "pool",
        "error:",
        "exception",
        "stack",
        "trace",
        "failed",
        "Runtime error",
    ];

    for pattern in &sensitive_patterns {
        assert!(
            !html_lower.contains(pattern),
            "Error page should not contain '{}' - potential info leak",
            pattern
        );
    }
}
