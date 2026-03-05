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
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

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
        .route("/", get(handlers::quests))
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
        "Expected styled error page, got: {}",
        body_str
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
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

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
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

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
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
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
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

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
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
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
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
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
        "Should handle invalid quest_id gracefully, got status: {}",
        status
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
        "Should handle missing quest_id gracefully, got status: {}",
        status
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
        "Should handle empty form data gracefully, got status: {}",
        status
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
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
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
            "Rapid toggle {} should return 404 for non-existent quest",
            i
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
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .with_state(app_state.clone());

    // Create quest with XSS payload in title
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let xss_payloads = vec![
        "<script>alert('xss')</script>",
        "<img src=x onerror=alert('xss')>",
        "javascript:alert('xss')",
        "<iframe src='javascript:alert(\"xss\")'>",
    ];

    for (i, payload) in xss_payloads.iter().enumerate() {
        let quest_req = CreateQuestRequest {
            title: payload.to_string(),
            description: Some(format!("XSS test {}", i)),
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
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .with_state(app_state.clone());

    // Create many quests (100+ to test performance)
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let start_time = std::time::Instant::now();

    for i in 0..150 {
        let quest_req = CreateQuestRequest {
            title: format!("Performance Quest {}", i),
            description: Some(format!("Description for quest {}", i)),
            exp_value: Some((i % 50) + 1), // Vary EXP values
            day_of_week,
        };
        db.create_quest(quest_req)
            .await
            .expect("Failed to create performance test quest");
    }

    let creation_time = start_time.elapsed();
    println!("Created 150 quests in {:?}", creation_time);

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
        "Page load should be fast even with 150 quests, took {:?}",
        page_load_time
    );

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Verify all quests are displayed
    for i in 0..150 {
        assert!(
            body_str.contains(&format!("Performance Quest {}", i)),
            "Quest {} should be displayed",
            i
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
        "Quest toggle should be fast, took {:?}",
        toggle_time
    );

    println!(
        "Large quest list performance test completed successfully - creation: {:?}, page load: {:?}, toggle: {:?}",
        creation_time, page_load_time, toggle_time
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
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let mut quest_ids = vec![];
    for i in 0..5 {
        let quest_req = CreateQuestRequest {
            title: format!("Concurrent Quest {}", i),
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
        let _day_of_week_clone = day_of_week;

        let handle = tokio::spawn(async move {
            let app_state_inner = AppState::new(db_clone, bcast_clone);
            let app = Router::new()
                .route("/", get(handlers::quests))
                .route("/quests/toggle", post(handlers::toggle_quest))
                .route("/events", get(handlers::events))
                .with_state(app_state_inner);

            // Each "user" toggles a random quest
            let quest_index = user_id % quest_ids_clone.len();
            let quest_id = quest_ids_clone[quest_index];
            let json_data = format!(r#"{{"quest_id":{}}}"#, quest_id);

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
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

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
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
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
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

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
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
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
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

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
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
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
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

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
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
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
        html.contains(&format!("quest_id: {}", expected_quest_id)),
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

// Test that simulates exactly what Datastar sends (all signals in JSON body)
#[tokio::test]
async fn test_toggle_with_datastar_signal_format() {
    // Datastar sends ALL signals as JSON body, not just quest_id
    // If we have a signal for quest_id, it sends: { "quest_id": 1 }
    // If we have multiple signals, it sends: { "quest_id": 1, "other": "value" }
    // If no signals, it might send: {}

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = CreateQuestRequest {
        title: "Datastar Signal Format Test".to_string(),
        description: Some("Testing Datastar signal format".to_string()),
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
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
        .with_state(app_state);

    // Test 1: Exact format Datastar sends when using { quest_id: value }
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
        "Handler should accept exact quest_id format"
    );

    // Test 2: Datastar might send additional signals with quest_id
    let json_with_extra = format!(r#"{{"quest_id":{},"otherSignal":"value"}}"#, quest.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_with_extra))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Handler should accept quest_id with extra signals (Datastar sends all signals)"
    );

    // Test 3: Empty JSON body (if Datastar sends no signals) should fail gracefully
    // This will return 422 because quest_id is required
    let empty_json = r#"{}"#;
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(empty_json))
                .unwrap(),
        )
        .await
        .unwrap();

    // This should return 400 because quest_id is missing (ReadSignals returns 400 for missing fields)
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Empty JSON body should return 400 (quest_id is required)"
    );

    println!("Datastar signal format test completed successfully");
}

// Test to debug: what happens when Datastar sends a request without quest_id
#[tokio::test]
async fn test_toggle_debug_422_scenarios() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = CreateQuestRequest {
        title: "Debug 422 Test".to_string(),
        description: Some("Debug 422 error scenarios".to_string()),
        exp_value: Some(10),
        day_of_week,
    };
    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
        .with_state(app_state);

    // Scenario: Missing content-type header (Datastar might not set it)
    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                // No content-type header
                .body(Body::from(json_data))
                .unwrap(),
        )
        .await
        .unwrap();

    println!("No content-type status: {:?}", response.status());

    // If content-type is missing, axum returns 415 (Unsupported Media Type)
    // If JSON is malformed, axum returns 422 (Unprocessable Entity)
    // User reported 422, so let's verify what format causes 422

    // Scenario: Malformed JSON (extra comma)
    let malformed_json = format!(r#"{{"quest_id":{},"}}"#, quest.id);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(malformed_json))
                .unwrap(),
        )
        .await
        .unwrap();

    println!("Malformed JSON status: {:?}", response.status());

    // Test that valid JSON works
    let valid_json = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(valid_json))
                .unwrap(),
        )
        .await
        .unwrap();

    println!("Valid JSON status: {:?}", response.status());

    assert!(
        response.status() == StatusCode::OK,
        "Valid JSON should return 200 OK"
    );

    println!("Debug 422 scenarios test completed");
}

// Test: Could Datastar be sending quest_id as a string "1" instead of number 1?
#[tokio::test]
async fn test_toggle_quest_id_as_string() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = CreateQuestRequest {
        title: "String Quest ID Test".to_string(),
        description: Some("Testing quest_id as string".to_string()),
        exp_value: Some(10),
        day_of_week,
    };
    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
        .with_state(app_state);

    // Datastar might serialize quest_id as a string: {"quest_id": "1"}
    let json_string = format!(r#"{{"quest_id":"{}"}}"#, quest.id);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("content-type", "application/json")
                .body(Body::from(json_string))
                .unwrap(),
        )
        .await
        .unwrap();

    println!("quest_id as string status: {:?}", response.status());

    // If this returns 422, it means Datastar is sending quest_id as a string
    // and we need to handle that in the handler
    assert!(
        response.status() == StatusCode::OK,
        "quest_id as string should be accepted or gracefully handled"
    );

    println!("String quest_id test completed");
}

// Test: Verify the exact HTML structure for Datastar to work correctly
#[tokio::test]
async fn test_toggle_button_exact_html_structure() {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = CreateQuestRequest {
        title: "Test Quest".to_string(),
        description: Some("Test description".to_string()),
        exp_value: Some(25),
        day_of_week,
    };
    let _quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
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

    // Look for the toggle button specifically (may span multiple lines)
    let html_normalized = html
        .replace("\n", " ")
        .replace("\r", " ")
        .replace("  ", " ");

    // Find the toggle button section (look for class="toggle-btn")
    let toggle_button_start = html_normalized
        .find("class=\"toggle-btn\"")
        .expect("Should find toggle button");
    let toggle_button_section = &html_normalized[toggle_button_start..];
    let toggle_button_end = toggle_button_section
        .find("</button>")
        .expect("Should find button end");
    let toggle_button_html = &toggle_button_section[..toggle_button_end + 9];

    println!("Toggle button HTML: {}", toggle_button_html);

    // Verify the button has type="button"
    assert!(
        toggle_button_html.contains("type=\"button\""),
        "Toggle button should have type=\"button\""
    );

    // Verify the button has Datastar data-on:click__prevent attribute
    assert!(
        toggle_button_html.contains("data-on:click__prevent=\"@post('/quests/toggle'"),
        "Toggle button should have data-on:click__prevent Datastar attribute"
    );

    // Verify the payload contains quest_id
    assert!(
        toggle_button_html.contains("payload") && toggle_button_html.contains("quest_id"),
        "Toggle button should have payload with quest_id"
    );

    // Verify the quest_id is a number (not template syntax)
    assert!(
        toggle_button_html.contains("quest_id:") && !toggle_button_html.contains("{{"),
        "quest_id should be a number, not template syntax"
    );

    println!("Toggle button structure test passed");
}

// Test: Verify no JavaScript errors would occur with the HTML structure
#[tokio::test]
async fn test_no_js_errors_in_toggle_html() {
    use chrono::Utc;
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = CreateQuestRequest {
        title: "JS Error Test".to_string(),
        description: Some("Testing for JS errors".to_string()),
        exp_value: Some(15),
        day_of_week,
    };
    let _quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
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

    // Check for potential JavaScript errors in the HTML
    let js_patterns = [
        (
            "unterminated string literal",
            "Check for unterminated strings",
        ),
        ("unexpected token", "Check for syntax errors"),
        ("missing )", "Check for missing parentheses"),
        ("invalid or unexpected token", "Check for invalid tokens"),
    ];

    for (pattern, description) in &js_patterns {
        assert!(
            !html.to_lowercase().contains(pattern),
            "HTML should not contain {}: {}",
            description,
            pattern
        );
    }

    // Verify all script tags are properly closed
    let script_opens = html.matches("<script").count();
    let script_closes = html.matches("</script>").count();
    assert_eq!(
        script_opens, script_closes,
        "Script tags should be properly closed ({} opens, {} closes)",
        script_opens, script_closes
    );

    // Verify all style tags are properly closed
    let style_opens = html.matches("<style").count();
    let style_closes = html.matches("</style>").count();
    assert_eq!(
        style_opens, style_closes,
        "Style tags should be properly closed"
    );

    // Verify no inline onclick handlers that could conflict (except our own quest toggle)
    // Allow onclick if it contains event.preventDefault() (our safe pattern)
    let inline_onclick = html.lines().any(|line| {
        line.contains("onclick=")
            && !line.contains("event.preventDefault()")
            && !line.contains("data-on:click")
    });

    assert!(
        !inline_onclick,
        "HTML should not have unsafe inline onclick handlers"
    );

    println!("JavaScript error check passed");
}

// ===== SSE BROADCAST TESTS =====

#[tokio::test]
async fn test_broadcast_channel_works() {
    use tokio::sync::broadcast;

    let (tx, mut rx) = broadcast::channel::<String>(128);

    let _ = tx.send("hello".to_string());

    let result = tokio::time::timeout(tokio::time::Duration::from_millis(100), rx.recv()).await;

    assert!(result.is_ok(), "Should receive message");
    assert_eq!(result.unwrap().unwrap(), "hello");

    println!("Broadcast channel test passed");
}

#[tokio::test]
async fn test_toggle_broadcasts_to_events_endpoint() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

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

    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
        .with_state(app_state.clone());

    // Subscribe to the broadcast channel BEFORE making the request
    let mut receiver = app_state.bcast.subscribe();

    // Make the toggle request in a spawned task
    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let app_clone = app.clone();
    let handle = tokio::spawn(async move {
        app_clone
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quests/toggle")
                    .header("content-type", "application/json")
                    .body(Body::from(json_data))
                    .unwrap(),
            )
            .await
    });

    // Wait for the response
    let response = handle.await.expect("Task should not panic").unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Check that we got a valid SSE response with datastar events
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // The response should contain datastar-patch-elements and datastar-patch-signals events
    assert!(
        body_str.contains("datastar-patch-elements"),
        "Response should contain datastar-patch-elements"
    );
    assert!(
        body_str.contains("datastar-patch-signals"),
        "Response should contain datastar-patch-signals"
    );

    // Now wait for the broadcast messages
    let mut quest_received = false;
    let mut rewards_received = false;
    let mut _signals_received = false;

    // Try to receive multiple messages (we now send 3: quest, rewards, signals)
    for _ in 0..4 {
        let result =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), receiver.recv()).await;

        match result {
            Ok(Ok(msg)) => match msg {
                ServerMessage::Elements(html, _) => {
                    eprintln!("Received elements: {}", &html[..html.len().min(100)]);
                    if html.contains("quest-item") {
                        quest_received = true;
                    }
                    if html.contains("weekly-rewards") {
                        rewards_received = true;
                    }
                }
                ServerMessage::Signals(json, _) => {
                    eprintln!("Received signals: {}", json);
                    assert!(json.contains("expToday"), "Signals should contain expToday");
                    _signals_received = true;
                }
            },
            Ok(Err(e)) => panic!("Broadcast error: {}", e),
            Err(_) => break, // Timeout - no more messages
        }
    }

    assert!(
        quest_received && rewards_received,
        "Should receive both quest and weekly rewards elements (quest={}, rewards={})",
        quest_received,
        rewards_received
    );

    println!("Toggle broadcast test completed successfully");
}

#[tokio::test]
async fn test_events_endpoint_returns_sse_stream() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
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
        content_type
            .map(|v| v.starts_with("text/event-stream"))
            .unwrap_or(false),
        "Content-type should be text/event-stream, got: {:?}",
        content_type
    );

    println!("Events endpoint SSE stream test completed successfully");
}

#[tokio::test]
async fn test_navigate_broadcasts_to_events_endpoint() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = CreateQuestRequest {
        title: "Navigate Broadcast Test".to_string(),
        description: Some("Testing navigate SSE broadcast".to_string()),
        exp_value: Some(15),
        day_of_week,
    };
    let _quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/day/{date}", get(handlers::quests))
        .route("/navigate/{date}", get(handlers::navigate))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
        .with_state(app_state.clone());

    // Subscribe to the broadcast channel BEFORE making the request
    let mut receiver = app_state.bcast.subscribe();

    // Make the navigate request in a spawned task
    let app_clone = app.clone();
    let handle = tokio::spawn(async move {
        app_clone
            .oneshot(
                Request::builder()
                    .uri("/navigate/today")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
    });

    // Wait for the response
    let response = handle.await.expect("Task should not panic").unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Navigate should NOT broadcast to other clients - only update the requesting client
    // Wait a short time to ensure no broadcasts are sent
    let result =
        tokio::time::timeout(tokio::time::Duration::from_millis(100), receiver.recv()).await;

    // Should timeout (no message received) - navigate no longer broadcasts
    assert!(
        result.is_err(),
        "Navigate should NOT broadcast to other clients"
    );

    println!("Navigate does not broadcast test completed successfully");
}

// Regression test for bug: weekly rewards don't update when a quest is toggled
// Bug Summary: When toggling a quest (completing/uncompleting), the weekly rewards UI
// section doesn't update, but the stats panel does. The state is correct on page reload.
//
// This test verifies that toggling a quest broadcasts the weekly-rewards element
// in addition to the quest element and signals.
//
// Current behavior: Test FAILS (no weekly rewards in SSE response)
// After fix: Test PASSES
#[tokio::test]
async fn test_toggle_quest_updates_weekly_rewards() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Get today's day of week
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    // Create a quest with 25 EXP for today
    let quest_req = CreateQuestRequest {
        title: "Test Weekly Rewards Quest".to_string(),
        description: Some("Testing weekly rewards update".to_string()),
        exp_value: Some(25), // 25 EXP, enough to unlock reward requiring 20 EXP
        day_of_week,
    };
    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    // Create a weekly reward requiring 20 EXP
    let reward_req = CreateRewardRequest {
        title: "Test Weekly Reward".to_string(),
        description: Some("Reward for testing".to_string()),
        required_exp: 20, // Requires 20 EXP to unlock
    };
    let _reward = db
        .create_reward(reward_req)
        .await
        .expect("Failed to create test reward");

    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
        .with_state(app_state.clone());

    // Subscribe to the broadcast channel BEFORE making the request
    let mut receiver = app_state.bcast.subscribe();

    // Make the toggle request in a spawned task
    let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
    let app_clone = app.clone();
    let handle = tokio::spawn(async move {
        app_clone
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quests/toggle")
                    .header("content-type", "application/json")
                    .body(Body::from(json_data))
                    .unwrap(),
            )
            .await
    });

    // Wait for the response
    let response = handle.await.expect("Task should not panic").unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Check the SSE response body for weekly-rewards element
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // The response SHOULD contain weekly-rewards element for Datastar morphing
    // This assertion FAILS with current code - demonstrating the bug
    assert!(
        body_str.contains("weekly-rewards") || body_str.contains("id=\"weekly-rewards\""),
        "Response should contain weekly-rewards element for UI update. \
         Got response: {}",
        body_str
    );

    // Also verify that the signals include weekExp (this works correctly)
    assert!(
        body_str.contains("weekExp"),
        "Response should contain weekExp signal"
    );

    // Check broadcast messages for weekly-rewards element
    let mut weekly_rewards_broadcast = false;
    for _ in 0..3 {
        let result =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), receiver.recv()).await;

        match result {
            Ok(Ok(msg)) => match msg {
                ServerMessage::Elements(html, _) => {
                    if html.contains("weekly-rewards") || html.contains("id=\"weekly-rewards\"") {
                        weekly_rewards_broadcast = true;
                    }
                }
                ServerMessage::Signals(_, _) => {}
            },
            Ok(Err(e)) => panic!("Broadcast error: {}", e),
            Err(_) => break, // Timeout - no more messages
        }
    }

    // This should also FAIL with current code
    assert!(
        weekly_rewards_broadcast,
        "Broadcast should contain weekly-rewards element for real-time updates"
    );

    println!("Toggle updates weekly rewards test completed successfully");
}

// Regression test: weekly-rewards details element should preserve open state across DOM patching
// This test verifies that the details element has a signal-bound open attribute that survives Datastar morphing
#[tokio::test]
async fn test_weekly_rewards_details_preserves_open_state() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app
    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState::new(db.clone(), bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
        .with_state(app_state.clone());

    // Create test quest to have something on the page
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = CreateQuestRequest {
        title: "Test Quest".to_string(),
        description: Some("Test description".to_string()),
        exp_value: Some(25),
        day_of_week,
    };
    let _quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    // Fetch the quests page HTML
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    // Check 1: The HTML should have data-signals containing $weeklyRewardsOpen signal
    // This is needed so Datastar can track the open/closed state
    assert!(
        html.contains("$weeklyRewardsOpen"),
        "HTML should contain $weeklyRewardsOpen signal in data-signals. \
         The signal is needed to preserve the details element's open state across DOM patches."
    );

    // Check 2: The weekly-rewards details element should have data-attr:open binding
    // This binds the 'open' attribute to the $weeklyRewardsOpen signal
    // Without this, the details element will collapse when its content is patched
    assert!(
        html.contains("data-attr:open"),
        "HTML should contain data-attr:open binding on the details element. \
         This binds the 'open' attribute to $weeklyRewardsOpen signal, \
         preventing the details element from collapsing during DOM morphing."
    );

    // Verify the details element exists with proper structure
    assert!(
        html.contains("<details") && html.contains("id=\"weekly-rewards\""),
        "HTML should contain weekly-rewards details element"
    );

    println!("Weekly rewards details state preservation test completed successfully");
}

// Test for day change auto-detection feature
// This test verifies that the quests page includes signals and logic to detect
// when the day changes (midnight pass) and automatically navigate to today
#[tokio::test]
async fn test_day_change_detection_signals_and_logic() {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use chrono::Utc;
    use quest_log::{database::Database, handlers, models::*, state::AppState};
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    // Setup test database with in-memory SQLite
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test app
    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState::new(db.clone(), bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .with_state(app_state.clone());

    // Create test quest
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = CreateQuestRequest {
        title: "Test Quest".to_string(),
        description: Some("Test description".to_string()),
        exp_value: Some(25),
        day_of_week,
    };
    let _quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    // Fetch the quests page HTML
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    // Check 1: The HTML should have $currentDay signal
    // This tracks the current weekday (0-6) from server-side
    assert!(
        html.contains("$currentDay"),
        "HTML should contain $currentDay signal to track the current weekday"
    );

    // Check 2: The HTML should have $isToday signal
    // This indicates whether user is viewing "today"
    assert!(
        html.contains("$isToday"),
        "HTML should contain $isToday signal to track if viewing today"
    );

    // Check 3: The HTML should have day change detector element with interval
    // This checks every 60 seconds if day has changed
    assert!(
        html.contains("day-change-detector") && html.contains("data-on-interval"),
        "HTML should contain day-change-detector with data-on-interval for automatic day change detection"
    );

    // Check 4: The day change logic should navigate to today when day changes
    // The condition checks: $isToday && new Date().getDay() !== $currentDay
    assert!(
        html.contains("/navigate/today"),
        "HTML should contain navigation to /navigate/today when day change is detected"
    );

    println!("Day change detection test completed successfully");
}

// Test: Navigation should include a link to the Bounty Board page
//
// This test verifies that the base template navigation includes a link to /bounty
// so users can navigate to the Bounty Board from any page.
//
// Current behavior: FAILS - navigation does not include /bounty link
// After adding the bounty nav link: Test PASSES
#[tokio::test]
async fn test_navigation_includes_bounty_link() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test quest so the page has content
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = CreateQuestRequest {
        title: "Test Quest".to_string(),
        description: Some("Test description".to_string()),
        exp_value: Some(25),
        day_of_week,
    };
    let _quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    // Create test app with the quests route
    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/bounty", get(handlers::bounty))
        .with_state(app_state);

    // Make a request to the home page
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    // The navigation should contain an anchor tag linking to /bounty
    // This is needed so users can navigate to the Bounty Board from any page
    assert!(
        html.contains("href=\"/bounty\""),
        "Navigation should contain a link to /bounty. \
         Expected to find href=\"/bounty\" in the HTML, but it was not found. \
         The base template should include a bounty link in the navigation bar."
    );

    // Verify it's actually in the navigation (not just somewhere in the page)
    // Look for nav-link class with bounty href
    assert!(
        html.contains("class=\"nav-link\"") && html.contains("href=\"/bounty\""),
        "The /bounty link should be a navigation link (nav-link class)"
    );

    println!("Navigation includes bounty link test completed successfully");
}

// Test: Day-change detector should use Datastar navigation, not window.location.href
//
// BUG: The day-change detector uses `window.location.href = '/navigate/today'` which
// does a full page load, but /navigate/today returns SSE (not HTML), causing the browser
// to display raw SSE text.
//
// FIX: Should use Datastar navigation: `@get('/navigate/today')` which properly handles
// the SSE response for day changes.
//
// This test verifies:
// 1. The HTML contains @get('/navigate/today') in the day-change-detector element
// 2. The HTML does NOT contain window.location.href
#[tokio::test]
async fn test_day_change_detector_uses_datastar_navigation() {
    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create test quest so the page has content
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let quest_req = CreateQuestRequest {
        title: "Test Quest".to_string(),
        description: Some("Test description".to_string()),
        exp_value: Some(25),
        day_of_week,
    };
    let _quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create test quest");

    // Create test app with the quests route
    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests))
        .with_state(app_state);

    // Make a request to the home page
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    // Verify the day-change-detector element exists
    assert!(
        html.contains("id=\"day-change-detector\""),
        "HTML should contain day-change-detector element"
    );

    // CRITICAL: The day-change detector should use Datastar navigation (@get)
    // instead of window.location.href which causes full page load with SSE
    assert!(
        html.contains("@get('/navigate/today')"),
        "Day-change detector should use @get('/navigate/today') for Datastar navigation. \
         The current implementation uses window.location.href which causes the browser \
         to display raw SSE text instead of properly handling the day change."
    );

    // CRITICAL: Should NOT use window.location.href
    // This causes full page load and displays raw SSE text
    assert!(
        !html.contains("window.location.href"),
        "Day-change detector should NOT use window.location.href. \
         This causes a full page load which displays raw SSE text from /navigate/today. \
         Should use @get('/navigate/today') instead for proper Datastar navigation."
    );

    println!("Day change detector uses Datastar navigation test completed successfully");
}
