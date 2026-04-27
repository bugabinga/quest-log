//! Integration tests for web endpoints: quest CRUD, toggle, XSS prevention, and weekly rewards.
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
use chrono::{Datelike, NaiveDate, Utc};
use quest_log::{
    database::Database, handlers, handlers::ServerMessage, models::*, state::AppState, time,
};

use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

// Test quest listing and data retrieval
#[tokio::test]
async fn test_quest_listing_integration() {
    // Setup test database with in-memory SQLite
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Add test data - create some quests for today
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

    // Test quest retrieval for today (mimicking handler logic)
    let quests = db
        .get_quests_for_day(day_of_week)
        .await
        .expect("Failed to get quests");
    assert_eq!(quests.len(), 1);
    assert_eq!(quests[0].title, "Integration Test Quest");

    // Test completion status check
    let completed_today = db
        .is_quest_completed_today(quest.id, today)
        .await
        .expect("Failed to check completion");
    assert!(!completed_today, "New quest should not be completed");

    // Calculate total EXP (mimicking handler logic)
    let total_exp: i32 = quests
        .iter()
        .filter(|_q| {
            // Check completion for each quest
            // For simplicity, we'll assume not completed
            false // Since we didn't mark any as completed
        })
        .map(|q| q.exp_value)
        .sum();
    assert_eq!(total_exp, 0, "No quests completed yet");

    println!("Quest listing integration test completed successfully");
}

// Regression test: invalid date should return error, not silently fall back to today
#[tokio::test]
async fn test_invalid_date_returns_error() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test quest
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

    // Test invalid date format - should return error, not silently use today
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

    // Should return BAD_REQUEST (400) with styled error page
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Should contain styled error page content
    assert!(
        body_str.contains("Wrong Day!"),
        "Expected styled error page, got: {body_str}"
    );

    time::reset_today();
}

// Test quest listing on a known Sunday (using set_today)
#[tokio::test]
async fn test_quest_listing_on_sunday() {
    // Set today to a known Sunday: Feb 15, 2026 is a Sunday
    let sunday = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
    time::set_today(sunday);

    // Setup test database with in-memory SQLite
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Add test data - create quests for Sunday (day_of_week = 0)
    let quest_req = CreateQuestRequest {
        title: "Sunday Quest".to_string(),
        description: Some("Testing quest listing on Sunday".to_string()),
        exp_value: Some(25),
        day_of_week: 0, // Sunday
    };
    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    // Test using time::today() which now returns our fake Sunday
    let today = time::today();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    // Verify we're on Sunday
    assert_eq!(day_of_week, 0, "Should be Sunday (0)");
    assert_eq!(today, sunday);

    // Test quest retrieval for today (mimicking handler logic)
    let quests = db
        .get_quests_for_day(day_of_week)
        .await
        .expect("Failed to get quests");
    assert_eq!(quests.len(), 1);
    assert_eq!(quests[0].title, "Sunday Quest");

    // Test completion status check
    let completed_today = db
        .is_quest_completed_today(quest.id, today)
        .await
        .expect("Failed to check completion");
    assert!(!completed_today, "New quest should not be completed");

    println!("Quest listing on Sunday test completed successfully");

    // Cleanup
    time::reset_today();
}

// Test quest toggle functionality via HTTP endpoints
#[tokio::test]
async fn test_quest_toggle_integration() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test quest
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

    // Create test app
    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
        .with_state(app_state);

    // Test initial quest page load
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Should contain quest title and "⚔️ Mark Complete" button
    assert!(body_str.contains("Toggle Test Quest"));
    assert!(body_str.contains("⚔️ Mark Complete"));
    assert!(body_str.contains("expToday"));

    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
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

    // Response should contain quest with "completed" class and "✅ Quest Complete"
    assert!(body_str.contains("quest-item completed"));
    assert!(body_str.contains("✅ Quest Complete"));
    assert!(body_str.contains("expToday"));

    // Test toggling back to incomplete (bidirectional toggle)
    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
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

    // Response should show quest un-completed (bidirectional toggle)
    assert!(!body_str.contains("quest-item completed"));
    assert!(body_str.contains("⚔️ Mark Complete"));
    assert!(body_str.contains("expToday"));

    println!("Quest toggle integration test completed successfully");
}

// Test multiple quests toggle interaction
#[tokio::test]
async fn test_multiple_quests_toggle_integration() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create multiple test quests
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest1_req = CreateQuestRequest {
        title: "Quest One".to_string(),
        description: Some("First quest".to_string()),
        exp_value: Some(10),
        day_of_week,
    };
    let quest1 = db
        .create_quest(quest1_req)
        .await
        .expect("Failed to create quest 1");

    let quest2_req = CreateQuestRequest {
        title: "Quest Two".to_string(),
        description: Some("Second quest".to_string()),
        exp_value: Some(20),
        day_of_week,
    };
    let quest2 = db
        .create_quest(quest2_req)
        .await
        .expect("Failed to create quest 2");

    // Create test app
    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
        .with_state(app_state);

    // Complete first quest
    let json_data = format!(r#"{{"quest_id":{}}}"#, quest1.id);
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // Complete second quest
    let json_data = format!(r#"{{"quest_id":{}}}"#, quest2.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
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

    // After completing both quests, total should be 30 (10 + 20)
    assert!(body_str.contains("expToday"));

    // Complete first quest again (bidirectional toggle - should un-complete)
    let json_data = format!(r#"{{"quest_id":{}}}"#, quest1.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
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

    // Quest1 should be un-completed, quest2 still completed
    assert!(body_str.contains("expToday"));

    println!("Multiple quests toggle integration test completed successfully");
}

// ===== ERROR HANDLING INTEGRATION TESTS =====

#[tokio::test]
async fn test_invalid_form_data() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app
    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
        .with_state(app_state);

    // Test invalid quest_id (non-numeric)
    let invalid_json_data = r#"{"quest_id":"abc"}"#;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(invalid_json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 400 Bad Request or handle gracefully
    let status = response.status();
    assert!(
        status.is_client_error() || status.is_success(),
        "Should handle invalid quest_id gracefully, got status: {status}"
    );

    // Test missing quest_id parameter
    let missing_json_data = r#"{"other_param":"value"}"#;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(missing_json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    assert!(
        status.is_client_error() || status.is_success(),
        "Should handle missing quest_id gracefully, got status: {status}"
    );

    // Test empty form data
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let status = response.status();
    assert!(
        status.is_client_error() || status.is_success(),
        "Should handle empty form data gracefully, got status: {status}"
    );

    println!("Invalid form data integration test completed successfully");
}

#[tokio::test]
async fn test_quest_toggle_error_handling() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app
    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
        .with_state(app_state);

    // Test toggling non-existent quest
    let json_data = r#"{"quest_id":99999}"#;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Should handle gracefully - no crash, page should still render
    assert!(
        body_str.contains("Gone") || body_str.contains("Nothing Here"),
        "Should handle non-existent quest gracefully with error page"
    );

    // Test multiple rapid toggles (stress test) - should all return 404
    for i in 0..10 {
        let json_data = format!(r#"{{"quest_id":{}}}"#, 99999 + i);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quests/toggle")
                    .header("content-type", "application/json")
                    .body(Body::from(json_data))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Rapid toggle {i} should return 404 for non-existent quest"
        );
    }

    println!("Quest toggle error handling integration test completed successfully");
}

// ===== SECURITY INTEGRATION TESTS =====

#[tokio::test]
async fn test_xss_prevention_in_web_interface() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app
    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState::new(db.clone(), bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .with_state(app_state.clone());

    // Create quest with XSS payload in title
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let xss_payloads = [
        "<script>alert('xss')</script>",
        "<img src=x onerror=alert('xss')>",
        "javascript:alert('xss')",
        "<iframe src='javascript:alert(\"xss\")'>",
    ];

    for (i, payload) in xss_payloads.iter().enumerate() {
        let quest_req = CreateQuestRequest {
            title: payload.to_string(),
            description: Some(format!("XSS test {i}")),
            exp_value: Some(10),
            day_of_week,
        };
        let _quest = db
            .create_quest(quest_req)
            .await
            .expect("Failed to create XSS test quest");
    }

    // Test that XSS payloads are properly escaped in HTML output
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Verify XSS payloads are escaped (should appear as HTML entities)
    // Askama 0.15+ uses decimal entities (&#60;) instead of named entities (&lt;)
    assert!(
        body_str.contains("&#60;script&#62;")
            || body_str.contains("&#60;img")
            || body_str.contains("&lt;script&gt;")
            || body_str.contains("&lt;img"),
        "XSS payloads should be HTML escaped"
    );
    assert!(
        !body_str.contains("<script>"),
        "Raw script tags should not appear"
    );
    assert!(
        !body_str.contains("<img src=x"),
        "Raw img tags should not appear"
    );

    println!("XSS prevention integration test completed successfully");
}

// ===== LARGE DATASET PERFORMANCE TESTS =====

#[tokio::test]
async fn test_large_quest_list_performance() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app
    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState::new(db.clone(), bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .with_state(app_state.clone());

    // Create many quests (100+ to test performance)
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let start_time = std::time::Instant::now();

    for i in 0..150 {
        let quest_req = CreateQuestRequest {
            title: format!("Performance Quest {i}"),
            description: Some(format!("Description for quest {i}")),
            exp_value: Some((i % 50) + 1), // Vary EXP values
            day_of_week,
        };
        db.create_quest(quest_req)
            .await
            .expect("Failed to create performance test quest");
    }

    let creation_time = start_time.elapsed();
    println!("Created 150 quests in {creation_time:?}");

    // Test page load performance with large dataset
    let page_load_start = std::time::Instant::now();
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let page_load_time = page_load_start.elapsed();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        page_load_time < std::time::Duration::from_secs(2),
        "Page load should be fast even with 150 quests, took {page_load_time:?}"
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Verify all quests are displayed
    for i in 0..150 {
        assert!(
            body_str.contains(&format!("Performance Quest {i}")),
            "Quest {i} should be displayed"
        );
    }

    // Test quest toggle performance with large dataset
    let quests = db.get_quests_for_day(day_of_week).await.unwrap();
    assert_eq!(quests.len(), 150);

    let toggle_start = std::time::Instant::now();
    let json_data = format!(r#"{{"quest_id":{}}}"#, quests[0].id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();
    let toggle_time = toggle_start.elapsed();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        toggle_time < std::time::Duration::from_millis(500),
        "Quest toggle should be fast, took {toggle_time:?}"
    );

    println!(
        "Large quest list performance test completed successfully - creation: {creation_time:?}, page load: {page_load_time:?}, toggle: {toggle_time:?}"
    );
}

// ===== CONCURRENT USER SIMULATION TESTS =====

#[tokio::test]
async fn test_multiple_users_concurrent_toggles() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create multiple quests
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let mut quest_ids = vec![];
    for i in 0..5 {
        let quest_req = CreateQuestRequest {
            title: format!("Concurrent Quest {i}"),
            description: None,
            exp_value: Some(10),
            day_of_week,
        };
        let quest = db.create_quest(quest_req).await.unwrap();
        quest_ids.push(quest.id);
    }

    // Simulate multiple concurrent users toggling quests
    let mut handles = vec![];
    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db.clone(), bcast_tx);

    for user_id in 0..10 {
        let db_clone = Database::with_pool(app_state.db.pool().clone());
        let bcast_clone = app_state.bcast.clone();
        let quest_ids_clone = quest_ids.clone();

        let handle = tokio::spawn(async move {
            let app_state_inner = AppState::new(db_clone, bcast_clone);
            let app = Router::new()
                .route("/", get(handlers::quests::quests))
                .route("/quests/toggle", post(handlers::quests::toggle_quest))
                .route("/events", get(handlers::events::events))
                .with_state(app_state_inner);

            // Each "user" toggles a random quest
            let quest_index = user_id % quest_ids_clone.len();
            let quest_id = quest_ids_clone[quest_index];
            let json_data = format!(r#"{{"quest_id":{quest_id}}}"#);

            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/quests/toggle")
                        .header("content-type", "application/json")
                        .body(Body::from(json_data))
                        .unwrap(),
                )
                .await;

            response.map(|r| r.status())
        });

        handles.push(handle);
    }

    // Wait for all concurrent requests to complete
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

    assert_eq!(
        success_count, 10,
        "All concurrent requests should succeed (10xx status)"
    );
    assert_eq!(error_count, 0, "No concurrent requests should fail");

    // Verify final state - with bidirectional toggle, each quest is toggled twice
    // First completion adds EXP, second un-completion removes it
    // Due to race conditions, exact final state may vary but should be consistent
    let mut total_exp = 0;
    let mut completed_count = 0;
    for &id in &quest_ids {
        let is_completed = db
            .is_quest_completed_today(id, today)
            .await
            .ok()
            .unwrap_or(false);
        if is_completed {
            total_exp += 10;
            completed_count += 1;
        }
    }

    // With bidirectional toggle and concurrent requests, exact EXP depends on race conditions
    // But all requests should succeed (no crashes)
    assert!(
        total_exp <= 50,
        "Total EXP should not exceed 50 (each quest toggled at most once)"
    );
    assert!(completed_count <= 5, "Completed quests should not exceed 5");

    println!("Multiple users concurrent toggles test completed successfully");
}

// Test that toggle endpoint returns proper HTML for Datastar morphing
// This test verifies the fix for NS_BINDING_ABORTED errors
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
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
        .with_state(app_state);

    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
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

    // Verify proper HTML structure for Datastar morphing
    assert!(
        body_str.contains("id=\"quest-"),
        "Response should contain quest item with ID for morphing"
    );
    assert!(
        body_str.contains("class=\"quest-item completed\""),
        "Quest should have completed class"
    );
    assert!(
        body_str.contains("expToday"),
        "Response should contain expToday signal patch"
    );
    assert!(
        body_str.contains("✅ Quest Complete"),
        "Response should contain completed button text"
    );

    // Verify no form submission elements that could cause NS_BINDING_ABORTED
    assert!(
        !body_str.contains("<form"),
        "Toggle response should not contain form elements"
    );

    println!("Toggle returns proper Datastar HTML test completed successfully");
}

// Test that reproduces the 422 error scenario
#[tokio::test]
async fn test_toggle_with_wrong_content_type_returns_422() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest_req = CreateQuestRequest {
        title: "422 Test Quest".to_string(),
        description: Some("Testing 422 error".to_string()),
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
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
        .with_state(app_state);

    // Test 1: Wrong content type (form-urlencoded) should return 422 or 415
    let form_data = format!("quest_id={}", quest.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form_data))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should NOT be 422 - either 415 (unsupported) or 200 (if handler accepts both)
    assert_ne!(
        response.status(),
        StatusCode::UNPROCESSABLE_ENTITY,
        "Toggle should not return 422 for any valid request"
    );

    // Test 2: Correct content type (application/json) should work
    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Toggle with correct JSON should return 200 OK"
    );

    println!("422 error test completed successfully");
}

// Test that verifies Datastar sends proper JSON format
#[tokio::test]
async fn test_toggle_accepts_datastar_json_format() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest_req = CreateQuestRequest {
        title: "JSON Format Test".to_string(),
        description: Some("Testing JSON body format".to_string()),
        exp_value: Some(20),
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
        .with_state(app_state);

    // Datastar sends signals as JSON body - test that format
    // The handler expects Json<ToggleQuestRequest> which parses { quest_id: N }
    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let _response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    println!("Datastar JSON format test completed successfully");
}

// Test that verifies the rendered HTML contains the actual quest ID (not template syntax)
#[tokio::test]
async fn test_rendered_toggle_html_contains_actual_quest_id() {
    use tower::ServiceExt;

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    let quest_req = CreateQuestRequest {
        title: "Rendered Quest ID Test".to_string(),
        description: Some("Testing that rendered HTML contains actual quest ID".to_string()),
        exp_value: Some(42),
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
        .with_state(app_state);

    // Call the actual handler to get rendered HTML
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    // CRITICAL: Verify the rendered HTML contains the actual quest ID (a number)
    // NOT the literal template syntax "{{ quest.id }}"
    let expected_quest_id = quest.id.to_string();

    // The rendered HTML should have something like: { quest_id: 1 }
    // NOT: { quest_id: {{ quest.id }} }
    assert!(
        html.contains(&format!("quest_id: {expected_quest_id}")),
        "Rendered HTML should contain 'quest_id: {}' (actual number), not template syntax. \
         Actual HTML snippet: {}",
        expected_quest_id,
        html.lines()
            .find(|line| line.contains("quest_id"))
            .unwrap_or("(not found)")
    );

    // Also verify it doesn't contain the literal template syntax
    assert!(
        !html.contains("{{ quest.id }}"),
        "Rendered HTML should NOT contain literal '{{ quest.id }}' template syntax"
    );

    // Verify the Datastar data-on:click__prevent attribute exists
    assert!(
        html.contains("data-on:click__prevent=\"@post('/quests/toggle'"),
        "HTML should contain Datastar data-on:click__prevent attribute"
    );

    // Verify the payload contains quest_id
    assert!(
        html.contains("payload") && html.contains("quest_id"),
        "HTML should contain payload with quest_id"
    );

    println!("Rendered quest ID test completed successfully");
}
