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
use chrono::{Datelike, Utc};
use quest_log::{
    database::Database, handlers, handlers::ServerMessage, models::*, state::AppState,
};

use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

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
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

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
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
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
    let empty_json = r"{}";
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
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

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
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
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
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

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
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
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
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

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
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
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
    let html_normalized = html.replace(['\n', '\r'], " ").replace("  ", " ");

    // Find the toggle button section (look for class="toggle-btn")
    let toggle_button_start = html_normalized
        .find("class=\"toggle-btn\"")
        .expect("Should find toggle button");
    let toggle_button_section = &html_normalized[toggle_button_start..];
    let toggle_button_end = toggle_button_section
        .find("</button>")
        .expect("Should find button end");
    let toggle_button_html = &toggle_button_section[..toggle_button_end + 9];

    println!("Toggle button HTML: {toggle_button_html}");

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
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

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
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
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
            "HTML should not contain {description}: {pattern}"
        );
    }

    // Verify all script tags are properly closed
    let script_opens = html.matches("<script").count();
    let script_closes = html.matches("</script>").count();
    assert_eq!(
        script_opens, script_closes,
        "Script tags should be properly closed ({script_opens} opens, {script_closes} closes)"
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

    let _unused = tx.send("hello".to_string());

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

    let (bcast_tx, _) = broadcast::channel::<ServerMessage>(128);
    let app_state = AppState::new(db, bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
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

    // Try to receive multiple messages (we now send 2: quest, signals - no rewards on Quest page)
    for _ in 0..3 {
        let result =
            tokio::time::timeout(tokio::time::Duration::from_millis(500), receiver.recv()).await;

        match result {
            Ok(Ok(msg)) => match msg {
                ServerMessage::Elements(html, _) => {
                    eprintln!("Received elements: {}", &html[..html.len().min(100)]);
                    if html.contains("quest-item") {
                        quest_received = true;
                    }
                    // Note: weekly rewards are no longer broadcast from Quest page (moved to Bounty page)
                }
                ServerMessage::Signals(json, _) => {
                    eprintln!("Received signals: {json}");
                    assert!(json.contains("expToday"), "Signals should contain expToday");
                }
            },
            Ok(Err(e)) => panic!("Broadcast error: {e}"),
            Err(_) => break, // Timeout - no more messages
        }
    }

    assert!(
        quest_received,
        "Should receive quest elements (quest={quest_received})"
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
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
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
        "Content-type should be text/event-stream, got: {content_type:?}"
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
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

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
        .route("/", get(handlers::quests::quests))
        .route("/day/{date}", get(handlers::quests::quests))
        .route("/navigate/{date}", get(handlers::navigate::navigate))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
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
// After fix: Test verifies weekly rewards are NOT sent for Quest page toggle (they're on Bounty page now)
#[tokio::test]
async fn test_toggle_quest_does_not_include_weekly_rewards() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Get today's day of week
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

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
        .route("/", get(handlers::quests::quests))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/events", get(handlers::events::events))
        .with_state(app_state.clone());

    // Subscribe to the broadcast channel BEFORE making the request
    let _receiver = app_state.bcast.subscribe();

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

    // Check the SSE response body
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // The response should NOT contain weekly-rewards element for Quest page
    // Weekly rewards are now only on the Bounty page
    assert!(
        !body_str.contains("weekly-rewards") && !body_str.contains("id=\"weekly-rewards\""),
        "Response should NOT contain weekly-rewards element for Quest page. \
         Weekly rewards are now only on the Bounty page. Got response: {body_str}"
    );

    // But it SHOULD contain the quest element
    assert!(
        body_str.contains("quest-item") || body_str.contains("datastar-patch-elements"),
        "Response should contain quest element for UI update. Got response: {body_str}"
    );

    println!("Toggle does not include weekly rewards test passed - they are on Bounty page now");
}

// Regression test: Verify weekly rewards are NOT on Quest page (they were moved to Bounty page)
#[tokio::test]
async fn test_weekly_rewards_not_on_quest_page() {
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
        .route("/events", get(handlers::events::events))
        .with_state(app_state.clone());

    // Create test quest to have something on the page
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

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

    // Verify weekly rewards are NOT on the Quest page (they were moved to Bounty page)
    assert!(
        !html.contains("weekly-rewards") && !html.contains("id=\"weekly-rewards\""),
        "Quest page should NOT contain weekly-rewards element. \
         Weekly rewards were moved to the Bounty page."
    );

    // Verify $weeklyRewardsOpen signal is NOT on Quest page
    assert!(
        !html.contains("$weeklyRewardsOpen"),
        "Quest page should NOT contain $weeklyRewardsOpen signal. \
         Weekly rewards were moved to the Bounty page."
    );

    println!("Verified weekly rewards are not on Quest page - they are on Bounty page now");
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
        .route("/", get(handlers::quests::quests))
        .with_state(app_state.clone());

    // Create test quest
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

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
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

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
        .route("/", get(handlers::quests::quests))
        .route("/bounty", get(handlers::bounty::bounty))
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
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

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
        .route("/", get(handlers::quests::quests))
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
