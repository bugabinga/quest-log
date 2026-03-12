//! Security integration tests
//! Tests for SQL injection, XSS, input validation, and security vulnerabilities

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
};
use chrono::{Datelike, Utc};
use quest_log::{database::Database, handlers, models::*, state::AppState};
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;
mod common;
use common::setup_test_app_state;

#[tokio::test]
async fn test_sql_injection_in_web_interface() {
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
        .with_state(setup_test_app_state(db.clone()));

    // Test SQL injection attempts in form data
    let sql_injection_attempts = vec![
        r#"{"quest_id":"1'; DROP TABLE quests; --"}"#,
        r#"{"quest_id":"1' OR '1'='1"}"#,
        r#"{"quest_id":"1'; SELECT * FROM settings; --"}"#,
        r#"{"quest_id":"admin'--"}"#,
        r#"{"quest_id":"1; DELETE FROM quests WHERE 1=1; --"}"#,
    ];

    for (i, injection) in sql_injection_attempts.iter().enumerate() {
        let request = Request::builder()
            .method("POST")
            .uri("/quests/toggle")
            .header("content-type", "application/json")
            .body(Body::from(injection.to_string()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        // Should handle gracefully without crashing or exposing data
        assert!(
            response.status().is_success() || response.status().is_client_error(),
            "SQL injection attempt {} should be handled safely",
            i
        );

        // Verify database integrity - all tables should still exist
        let settings_check = db.get_settings().await;
        assert!(
            settings_check.is_ok(),
            "Database should remain intact after SQL injection attempt {}",
            i
        );
    }

    println!("SQL injection in web interface test completed successfully");
}

#[tokio::test]
async fn test_input_validation_comprehensive() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Test comprehensive input validation
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    // Test various edge cases and potentially dangerous inputs
    let test_inputs = vec![
        ("", "Empty title"),
        ("   ", "Whitespace only title"),
        ("\t\n\r", "Control characters only"),
        ("🚀 Quest with emoji 🎯", "Unicode emoji"),
        ("Quest with <b>HTML</b>", "Basic HTML tags"),
        (
            "Very long title that exceeds normal length expectations and might cause issues with layout or database constraints if not properly handled by the application layer even though SQLite might handle it fine",
            "Extremely long title",
        ),
        ("Quest\nwith\nnewlines", "Multi-line title"),
        ("Quest\twith\ttabs", "Title with tabs"),
        ("Quest\x00with\x00null\x00bytes", "Title with null bytes"),
    ];

    for (title, description) in test_inputs {
        let quest_req = CreateQuestRequest {
            title: title.to_string(),
            description: Some(description.to_string()),
            exp_value: Some(10),
            day_of_week,
        };

        // Database layer should handle all these inputs safely
        let result = db.create_quest(quest_req).await;
        assert!(
            result.is_ok(),
            "Should handle input safely: '{}' - {}",
            title,
            description
        );

        if let Ok(quest) = result {
            // Verify data integrity
            assert_eq!(quest.title, title);
            assert_eq!(quest.exp_value, 10);

            // Verify we can retrieve safely
            let retrieved = db.get_quest_by_id(quest.id).await.unwrap().unwrap();
            assert_eq!(retrieved.title, title);
        }
    }

    println!("Comprehensive input validation test completed successfully");
}

#[tokio::test]
async fn test_path_traversal_prevention() {
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

    // Test path traversal attempts (though our routes are simple, test the principle)
    let path_traversal_attempts = vec![
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32",
        "/etc/passwd",
        "C:\\Windows\\System32",
        "../../../../root/.bashrc",
    ];

    for attempt in path_traversal_attempts {
        // Try to access as if it were a route parameter
        let request = Request::builder()
            .uri(&format!("/{}", attempt))
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();

        // Should return 404 Not Found, not expose file system
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Path traversal attempt should return 404: {}",
            attempt
        );
    }

    println!("Path traversal prevention test completed successfully");
}

#[tokio::test]
async fn test_http_header_injection() {
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

    // Test header injection attempts
    let header_injection_attempts = vec![
        ("x-test", "value\r\nSet-Cookie: malicious=value"),
        ("x-test", "value\nX-Custom-Header: injected"),
        (
            "content-type",
            "application/x-www-form-urlencoded\r\nX-Injected: header",
        ),
    ];

    for (header_name, header_value) in header_injection_attempts {
        // Try to create request with potentially malicious header
        let request_result = Request::builder()
            .uri("/")
            .header(header_name, header_value)
            .body(Body::empty());

        match request_result {
            Ok(request) => {
                // If request creation succeeded, test the response
                let response = app.clone().oneshot(request).await.unwrap();

                // Should handle safely - either reject or sanitize headers
                assert!(
                    response.status().is_success() || response.status().is_client_error(),
                    "Header injection attempt should be handled safely"
                );

                // Verify no malicious headers were injected into response
                let headers = response.headers();
                assert!(
                    !headers.contains_key("set-cookie"),
                    "Should not inject Set-Cookie headers"
                );
                assert!(
                    !headers.contains_key("x-custom-header"),
                    "Should not inject custom headers"
                );
                assert!(
                    !headers.contains_key("x-injected"),
                    "Should not inject arbitrary headers"
                );
            }
            Err(_) => {
                // If request creation failed due to invalid header, that's good security
                // Invalid headers should be rejected at the HTTP level
                continue;
            }
        }
    }

    println!("HTTP header injection test completed successfully");
}

#[tokio::test]
async fn test_dos_prevention_basic() {
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

    // Test rapid-fire requests (basic DoS simulation)
    let mut handles = vec![];

    for _i in 0..50 {
        let app_clone = app.clone();

        let handle = tokio::spawn(async move {
            let request = Request::builder().uri("/").body(Body::empty()).unwrap();

            let start = std::time::Instant::now();
            let response = app_clone.oneshot(request).await;
            let duration = start.elapsed();

            (response, duration)
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut total_duration = std::time::Duration::from_secs(0);
    let mut success_count = 0;

    for handle in handles {
        let (response_result, duration) = handle.await.unwrap();
        total_duration += duration;

        let response = response_result.unwrap();
        if response.status().is_success() {
            success_count += 1;
        }
    }

    let avg_duration = total_duration / 50;

    // All requests should succeed and be reasonably fast
    assert_eq!(success_count, 50, "All rapid requests should succeed");
    assert!(
        avg_duration < std::time::Duration::from_millis(100),
        "Average response time should be reasonable: {:?}",
        avg_duration
    );

    println!(
        "Basic DoS prevention test completed successfully - avg response time: {:?}",
        avg_duration
    );
}

#[tokio::test]
async fn test_data_exposure_prevention() {
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
        .with_state(setup_test_app_state(db.clone()));

    // Create some test data
    let _today = Utc::now().date_naive();
    let day_of_week = 1; // Monday for simplicity

    let quest_req = CreateQuestRequest {
        title: "Secret Quest".to_string(),
        description: Some("This should not be exposed".to_string()),
        exp_value: Some(100),
        day_of_week,
    };
    let quest = db.create_quest(quest_req).await.unwrap();

    // Test that error responses don't expose sensitive data
    let invalid_requests = vec![
        // Try to access non-existent quest
        Request::builder()
            .uri("/nonexistent")
            .body(Body::empty())
            .unwrap(),
        // Try invalid HTTP method
        Request::builder()
            .method("TRACE")
            .uri("/")
            .body(Body::empty())
            .unwrap(),
        // Try to access with invalid parameters
        Request::builder()
            .method("POST")
            .uri("/quests/toggle")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"quest_id":"invalid"}"#))
            .unwrap(),
    ];

    for request in invalid_requests {
        let response = app.clone().oneshot(request).await.unwrap();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Error responses should not contain sensitive data
        assert!(
            !body_str.contains("Secret Quest"),
            "Error responses should not expose quest data"
        );
        assert!(
            !body_str.contains("This should not be exposed"),
            "Error responses should not expose quest descriptions"
        );

        // Check for quest_id in the response (case-insensitive search for the actual field value)
        // We specifically look for patterns like "quest_id" followed by a number that matches quest.id
        // But exclude line/column number references like "at line 1 column 21"
        let quest_id_str = quest.id.to_string();
        let has_quest_id_exposed = body_str.contains(&format!(r#""{}""#, quest_id_str))
            || (body_str.contains(&format!("{}:", quest_id_str)) && !body_str.contains("line"))
            || (body_str.contains(&format!("{} ", quest_id_str)) && body_str.contains("error"));

        assert!(
            !has_quest_id_exposed,
            "Error responses should not expose internal IDs. Quest ID: {}, Response: {}",
            quest.id, body_str
        );
    }

    println!("Data exposure prevention test completed successfully");
}
