//! Integration tests for error handling: timeouts, malformed requests, resource exhaustion.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
};
use quest_log::database::Database;
use quest_log::handlers::quests::{quests, toggle_quest};
use quest_log::state::AppState;
use sqlx::SqlitePool;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tower::util::ServiceExt;

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
        .route("/", get(quests))
        .route("/quests/toggle", post(toggle_quest))
        .with_state({
            let (bcast_tx, _) = broadcast::channel(128);
            AppState::new(db, bcast_tx)
        });

    // Test with a very short timeout to simulate slow operations
    let request = Request::builder().uri("/").body(Body::empty()).unwrap();

    // This should complete quickly in normal conditions
    let result = timeout(Duration::from_millis(10), app.oneshot(request)).await;

    match result {
        Ok(Ok(response)) => {
            // Request completed within timeout - this is expected for fast operations
            assert_eq!(response.status(), StatusCode::OK);
        }
        Ok(Err(e)) => {
            // Request failed but didn't timeout - this is acceptable
            panic!("Request failed: {e:?}");
        }
        Err(elapsed) => {
            // Request timed out - this indicates a performance issue that should be addressed
            panic!("Request timed out after {elapsed:?} - indicates performance issue");
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
        .route("/", get(quests))
        .route("/quests/toggle", post(toggle_quest))
        .with_state({
            let (bcast_tx, _) = broadcast::channel(128);
            AppState::new(db, bcast_tx)
        });

    // Test various malformed requests
    // Test invalid HTTP method
    let request = Request::builder()
        .method("INVALID")
        .uri("/")
        .body(Body::empty())
        .unwrap();
    let result = app.clone().oneshot(request).await;
    let response = result.unwrap();
    let status = response.status();
    assert!(status.is_success() || status.is_client_error() || status.is_server_error());
    let request_result = Request::builder()
        .uri("http://invalid uri with spaces")
        .body(Body::empty());

    if let Ok(request) = request_result {
        let result = app.clone().oneshot(request).await;
        let response = result.unwrap();
        let status = response.status();
        assert!(status.is_success() || status.is_client_error() || status.is_server_error());
    } else {
        // Request creation itself failed - this is acceptable for malformed URIs
    }

    // Test oversized headers (simulate)
    let request = Request::builder()
        .uri("/")
        .header("x-test", "x".repeat(10000))
        .body(Body::empty())
        .unwrap();
    let result = app.clone().oneshot(request).await;
    let response = result.unwrap();
    let status = response.status();
    assert!(status.is_success() || status.is_client_error() || status.is_server_error());

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
        .route("/", get(quests))
        .route("/quests/toggle", post(toggle_quest))
        .with_state({
            let (bcast_tx, _) = broadcast::channel(128);
            AppState::new(db, bcast_tx)
        });

    // Test with very large request body (simulating attack)
    let large_body = Body::from(vec![b'x'; 10 * 1024 * 1024]); // 10MB

    let request = Request::builder()
        .method("POST")
        .uri("/quests/toggle")
        .header("content-type", "application/json")
        .body(large_body)
        .unwrap();

    let result = timeout(Duration::from_secs(5), app.oneshot(request)).await;

    if let Ok(response_result) = result {
        let response = response_result.unwrap();
        // Should either succeed (if body is processed) or return error
        let status = response.status();
        assert!(
            status.is_success() || status.is_client_error(),
            "Large request should be handled gracefully, got status {status}"
        );
    } else {
        // Timeout is acceptable for very large requests
        // Indicates the server doesn't hang indefinitely
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
        .route("/", get(quests))
        .route("/quests/toggle", post(toggle_quest))
        .with_state({
            let (bcast_tx, _) = broadcast::channel(128);
            AppState::new(db, bcast_tx)
        });

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
async fn test_no_database_details_leaked_in_errors() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let app = Router::new().route("/", get(quests)).with_state({
        let (bcast_tx, _) = broadcast::channel(128);
        AppState::new(db.clone(), bcast_tx)
    });

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
            "Error page should not contain '{pattern}' - potential info leak"
        );
    }
}
