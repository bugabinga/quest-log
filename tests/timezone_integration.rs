//! Regression tests for timezone handling bug.
//!
//! This test proves that the server ignores the `X-Timezone` header and always
//! uses UTC for calculating "today", violating spec 0002-quest-page.md.
//!
//! ## Bug Summary
//!
//! The client sends its timezone via:
//! - `X-Timezone` HTTP header (IANA timezone, e.g., `America/New_York`)
//! - `QuestLog-TZ` cookie (fallback)
//!
//! However, the server ignores this timezone and always uses `Utc::now().date_naive()`
//! for calculating "today". This violates spec 0002-quest-page.md which states:
//!
//! > "Server stores all times in UTC internally. The client sends its timezone via
//! > X-Timezone header... Server uses this to determine 'today' for quest filtering"
//!
//! ## Expected Behavior
//!
//! - **Current (BUG)**: Tests FAIL because server ignores timezone and uses UTC
//! - **After fix**: Tests PASS because server respects X-Timezone header
//!
//! ## Test Scenarios
//!
//! 1. Request with `X-Timezone: America/New_York` when UTC is ahead → should get NY's "today"
//! 2. Request with `X-Timezone: Asia/Tokyo` when UTC is behind → should get Tokyo's "today"
//! 3. Edge case: Near midnight when client is ~12 hours offset from UTC
//! 4. Fallback: Request without timezone header → should default to UTC
#![allow(
    clippy::tests_outside_test_module,
    clippy::unwrap_used,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    routing::{get, post},
};
use chrono::{NaiveDate, TimeZone, Utc};
use quest_log::{database::Database, handlers, models::*, state::AppState, time};

use sqlx::SqlitePool;
use tokio::sync::broadcast;
use tower::util::ServiceExt;

/// Helper to create a test app with in-memory database
async fn create_test_app() -> (Router, Database) {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let db: Database = Database::with_pool(pool);
    db.migrate().await.unwrap();

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db.clone(), bcast_tx);
    let app = Router::new()
        .route("/", get(handlers::quests::quests))
        .with_state(app_state);

    (app, db)
}

/// Helper to create a quest for a specific day of week
async fn create_quest_for_day(db: &Database, title: &str, day_of_week: i32) -> Quest {
    let quest_req = CreateQuestRequest {
        title: title.to_string(),
        description: Some("Timezone test quest".to_string()),
        exp_value: Some(25),
        day_of_week,
    };
    db.create_quest(quest_req).await.unwrap()
}

// =============================================================================
// REGRESSION TESTS - These tests FAIL with current code (proving the bug exists)
// =============================================================================

/// Test: X-Timezone header with `America/New_York` when UTC is ahead
///
/// Scenario: When it's "tomorrow" in UTC but still "today" in New York,
/// the server should return NY's today, not UTC's today.
///
/// Example: If UTC is 02:00 Jan 2nd, NY is 21:00 Jan 1st (previous day).
/// A proper implementation would show Jan 1st quests to NY users.
///
/// CURRENT BUG: Server ignores X-Timezone and shows Jan 2nd (UTC) to everyone.
#[tokio::test]
async fn test_timezone_header_america_new_york() {
    // Set "fake now" to Jan 2nd, 2026 at 02:00 UTC
    // This is Jan 1st, 2026 at 21:00 NY time (previous day in NY)
    let utc_dt = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2026, 1, 2)
            .unwrap()
            .and_hms_opt(2, 0, 0)
            .unwrap(),
    );
    time::set_fake_datetime(utc_dt);

    // Create quests for both Jan 1st (NY yesterday) and Jan 2nd (UTC today)
    let (app, db) = create_test_app().await;

    // Quest for Thursday (Jan 1st, 2026 was a Thursday)
    // num_days_from_sunday: 0=Sunday, 1=Monday, 2=Tuesday, 3=Wednesday, 4=Thursday
    let _thursday_quest = create_quest_for_day(&db, "Thursday Quest", 4).await;

    // Quest for Monday (Jan 5th, 2026)
    let _monday_quest = create_quest_for_day(&db, "Monday Quest", 1).await; // Monday

    // Request with X-Timezone: America/New_York
    // Since UTC is Jan 2nd 02:00 but NY is Jan 1st 21:00, NY's "today" is still Jan 1st (Thursday)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("X-Timezone", "America/New_York")
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

    // EXPECTED: With timezone fix, NY user should see Thursday quests (Jan 1st)
    // BUG: Without fix, server shows Monday quests (Jan 5th UTC)
    assert!(
        html.contains("Thursday Quest") && !html.contains("Monday Quest"),
        "With timezone fix, server should return NY's 'today' (Thursday Jan 1st). \
         Without fix, returns UTC date (Monday Jan 5th)."
    );

    time::reset_today();
}

/// Test: X-Timezone header with Asia/Tokyo when UTC is behind
///
/// Scenario: When UTC is "yesterday" but Tokyo is already "tomorrow",
/// the server should return Tokyo's today, not UTC's today.
///
/// Example: If UTC is 20:00 Jan 1st, Tokyo is 05:00 Jan 2nd (next day).
///
/// CURRENT BUG: Server ignores X-Timezone and shows Jan 1st (UTC) to everyone.
#[tokio::test]
async fn test_timezone_header_asia_tokyo() {
    // Jan 1st, 2026 at 20:00 UTC = Jan 2nd, 2026 at 05:00 Tokyo time (next day)
    let utc_dt = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(20, 0, 0)
            .unwrap(),
    );
    time::set_fake_datetime(utc_dt);

    let (app, db) = create_test_app().await;

    // Jan 1st 2026 UTC = Thursday (4), Jan 2nd Tokyo = Friday (5)
    // num_days_from_sunday: 0=Sunday, 1=Monday, 2=Tuesday, 3=Wednesday, 4=Thursday, 5=Friday, 6=Saturday
    let _thursday_quest = create_quest_for_day(&db, "Thursday Quest", 4).await;
    let _friday_quest = create_quest_for_day(&db, "Friday Quest", 5).await;
    let _saturday_quest = create_quest_for_day(&db, "Saturday Quest", 6).await;

    // Request with X-Timezone: Asia/Tokyo
    // Tokyo time is Jan 2nd 05:00, which is already "tomorrow" in Tokyo
    // So Tokyo user should see Friday quests (Jan 2nd), not Thursday quests (Jan 1st UTC)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("X-Timezone", "Asia/Tokyo")
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

    // EXPECTED: With timezone fix, Tokyo user should see Friday quests (Jan 2nd)
    // BUG: Without fix, server shows Thursday quests (Jan 1st UTC)

    assert!(
        html.contains("Friday Quest") && !html.contains("Thursday Quest"),
        "With timezone fix, server should return Tokyo's 'today' (Friday Jan 2nd). \
         Without fix, returns UTC date (Thursday Jan 1st)."
    );

    time::reset_today();
}

/// Test: Edge case at 12-hour offset boundary (Pacific/Honolulu vs Japan)
///
/// Scenario: When the timezone offset is approximately 12 hours,
/// test the boundary condition.
///
/// CURRENT BUG: Server ignores X-Timezone entirely.
#[tokio::test]
async fn test_timezone_header_12_hour_offset_edge_case() {
    // UTC Jan 14th 15:00 = Jan 15th 00:00 Tokyo (midnight boundary)
    let utc_dt = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2026, 1, 14)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap(),
    );
    time::set_fake_datetime(utc_dt);

    let (app, db) = create_test_app().await;

    // Jan 14th 2026 UTC = Wednesday, Jan 15th Tokyo = Thursday (next day at midnight)
    // num_days_from_sunday: 0=Sunday, 1=Monday, 2=Tuesday, 3=Wednesday, 4=Thursday, 5=Friday, 6=Saturday
    let _wednesday_quest = create_quest_for_day(&db, "Wednesday Quest", 3).await;
    let _thursday_quest = create_quest_for_day(&db, "Thursday Quest", 4).await;
    let _friday_quest = create_quest_for_day(&db, "Friday Quest", 5).await;

    // Request with X-Timezone: Asia/Tokyo
    // UTC Jan 14th 15:00 = Tokyo Jan 15th 00:00 (midnight - start of next day)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("X-Timezone", "Asia/Tokyo")
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

    // EXPECTED: With timezone fix, Tokyo user at UTC Jan 14th 15:00 = Tokyo Jan 15th 00:00 should see Thursday
    assert!(
        html.contains("Thursday Quest") && !html.contains("Wednesday Quest"),
        "With timezone fix, server should return Tokyo's 'today' (Thursday Jan 15th). \
         UTC date was Wednesday Jan 14th, but Tokyo was already Thursday Jan 15th."
    );

    time::reset_today();
}

/// Test: Fallback behavior when no timezone header is provided
///
/// When no X-Timezone header is sent, server should default to UTC.
///
/// CURRENT BUG: This actually works correctly (falls back to UTC),
/// but the problem is it ALWAYS falls back to UTC instead of using the header.
#[tokio::test]
async fn test_no_timezone_header_defaults_to_utc() {
    // Set "fake now" to known UTC date/time - noon on March 15th, 2026 (Sunday)
    let utc_dt = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2026, 3, 15)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap(),
    );
    time::set_fake_datetime(utc_dt);

    let (app, db) = create_test_app().await;

    // March 15th, 2026 is a Sunday
    let _sunday_quest = create_quest_for_day(&db, "Sunday Quest", 0).await;

    // Request WITHOUT X-Timezone header
    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let html = String::from_utf8(body.to_vec()).unwrap();

    // This works correctly (falls back to UTC)
    assert!(
        html.contains("Sunday Quest"),
        "Without timezone header, server correctly defaults to UTC"
    );

    time::reset_today();
}

/// Test: Verify that different timezones get different "today" dates
///
/// This is the key test that demonstrates the bug by showing that
/// two requests with different X-Timezone headers get the SAME response,
/// when they should get DIFFERENT responses.
#[tokio::test]
async fn test_timezone_header_produces_different_results() {
    // UTC Jan 1st 02:00 = Dec 31st 21:00 NY (Wednesday), Jan 1st 11:00 Tokyo (Thursday)
    let utc_dt = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(2, 0, 0)
            .unwrap(),
    );
    time::set_fake_datetime(utc_dt);

    let (app, db) = create_test_app().await;

    // Jan 1st 2026 UTC = Thursday, Dec 31st 2025 NY = Wednesday, Jan 2nd 2026 = Friday
    // num_days_from_sunday: 0=Sunday, 1=Monday, 2=Tuesday, 3=Wednesday, 4=Thursday, 5=Friday, 6=Saturday
    let _wednesday_quest = create_quest_for_day(&db, "Wednesday Quest", 3).await;
    let _thursday_quest = create_quest_for_day(&db, "Thursday Quest", 4).await;
    let _friday_quest = create_quest_for_day(&db, "Friday Quest", 5).await;

    // Request from America/New_York
    // UTC: Jan 1st 02:00 = Dec 31st 21:00 NY (Wednesday)
    let ny_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("X-Timezone", "America/New_York")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Request from Asia/Tokyo
    // UTC: Jan 1st 02:00 = Jan 1st 11:00 Tokyo (Thursday)
    let tokyo_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("X-Timezone", "Asia/Tokyo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let ny_body = axum::body::to_bytes(ny_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let ny_html = String::from_utf8(ny_body.to_vec()).unwrap();

    let tokyo_body = axum::body::to_bytes(tokyo_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let tokyo_html = String::from_utf8(tokyo_body.to_vec()).unwrap();

    // BUG CONFIRMATION:
    // Both responses should be DIFFERENT because NY and Tokyo have different "today"
    // NY: Dec 31st (Wednesday) | Tokyo: Jan 1st (Thursday)
    //
    // But with the bug, BOTH responses are the SAME (UTC Jan 1st = Thursday)
    // So both show Thursday quests

    let ny_has_wednesday = ny_html.contains("Wednesday Quest");
    let ny_has_thursday = ny_html.contains("Thursday Quest");
    let tokyo_has_thursday = tokyo_html.contains("Thursday Quest");
    let tokyo_has_friday = tokyo_html.contains("Friday Quest");

    // EXPECTED: With timezone fix, NY should see Wednesday (Dec 31) and Tokyo should see Thursday (Jan 1)
    // This PROVES the fix works: different timezones get different "today" dates
    assert!(
        ny_has_wednesday && !ny_has_thursday && tokyo_has_thursday && !tokyo_has_friday,
        "With timezone fix, NY should see Wednesday (Dec 31) and Tokyo should see Thursday (Jan 1). \
         Different timezones should return different 'today' dates based on local time."
    );

    time::reset_today();
}

/// Test: Cookie-based timezone fallback (QuestLog-TZ)
///
/// The spec mentions QuestLog-TZ cookie as a fallback for timezone.
/// This test verifies that when the cookie is present but header is missing,
/// the cookie should be used.
///
/// CURRENT BUG: Cookie is also ignored.
#[tokio::test]
async fn test_timezone_cookie_fallback() {
    // UTC Jan 1st 02:00 = Dec 31st 21:00 NY (Wednesday)
    let utc_dt = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(2, 0, 0)
            .unwrap(),
    );
    time::set_fake_datetime(utc_dt);

    let (app, db) = create_test_app().await;

    // Jan 1st 2026 UTC = Thursday (4), Dec 31st 2025 NY = Wednesday (3)
    // num_days_from_sunday: 0=Sunday, 1=Monday, 2=Tuesday, 3=Wednesday, 4=Thursday, 5=Friday, 6=Saturday
    let _thursday_quest = create_quest_for_day(&db, "Thursday Quest", 4).await;
    let _wednesday_quest = create_quest_for_day(&db, "Wednesday Quest", 3).await;

    // Request with QuestLog-TZ cookie but no X-Timezone header
    // UTC Jan 1st 02:00 = Dec 31st 21:00 NY (Wednesday)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/")
                .header("Cookie", "QuestLog-TZ=America/New_York")
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

    // EXPECTED: With timezone fix, cookie should be used, so NY user should see Wednesday (Dec 31)
    assert!(
        html.contains("Wednesday Quest") && !html.contains("Thursday Quest"),
        "With timezone fix, QuestLog-TZ cookie should be used for timezone. \
         UTC was Jan 1st (Thursday) but NY time was Dec 31st (Wednesday)."
    );

    time::reset_today();
}

/// Test: Navigate handler should respect timezone header
///
/// REGRESSION TEST for bug where navigate used `time::today()` (UTC)
/// while toggle used `time::today_with_timezone(tz)`.
///
/// Scenario: It's Saturday in Europe/Berlin (UTC+1) but still Friday in UTC.
/// Navigate should show Saturday's quests (Berlin's today), not Friday's quests (UTC's today).
#[tokio::test]
async fn test_navigate_respects_timezone_header() {
    // March 20, 2026 at 23:30 UTC = March 21, 2026 at 00:30 Berlin time (Saturday)
    // In UTC, it's still Friday March 20
    let utc_dt = Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(2026, 3, 20)
            .unwrap()
            .and_hms_opt(23, 30, 0)
            .unwrap(),
    );
    time::set_fake_datetime(utc_dt);

    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let db: Database = Database::with_pool(pool);
    db.migrate().await.unwrap();

    let (bcast_tx, _) = broadcast::channel(128);
    let app_state = AppState::new(db.clone(), bcast_tx);
    let app = Router::new()
        .route("/navigate/{date}", get(handlers::navigate::navigate))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .with_state(app_state);

    // Create quests for Saturday (day 6) and Friday (day 5)
    // March 20, 2026 was a Friday (day 5)
    // March 21, 2026 was a Saturday (day 6)
    let saturday_quest = create_quest_for_day(&db, "Saturday Quest", 6).await;
    let _friday_quest = create_quest_for_day(&db, "Friday Quest", 5).await;

    // With X-Timezone: Europe/Berlin, it should show Saturday (00:30 March 21 in Berlin)
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/navigate/today")
                .header("X-Timezone", "Europe/Berlin")
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

    // With timezone fix, Berlin user should see Saturday (March 21)
    // Before fix, navigate used UTC so it would show Friday (March 20)
    assert!(
        html.contains("Saturday Quest") && !html.contains("Friday Quest"),
        "Navigate with Europe/Berlin timezone should show Saturday (UTC was Friday). \
         Got HTML containing: {}",
        if html.contains("Friday Quest") {
            "Friday Quest (BUG: using UTC instead of timezone)"
        } else {
            "neither Friday nor Saturday"
        }
    );

    // Also verify toggle with same timezone would accept Saturday quests
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/quests/toggle")
                .header("Content-Type", "application/json")
                .header("X-Timezone", "Europe/Berlin")
                .body(Body::from(r#"{"quest_id":"#.chars().chain(saturday_quest.id.to_string().chars()).chain("}".chars()).collect::<String>()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Toggle should succeed for Saturday quest (not rejected as "wrong day")
    // It might fail for other reasons (already completed, etc.) but NOT "wrong day"
    // If it's "wrong day" error, that means toggle is using a different "today" than navigate
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        !body_str.contains("wrong day"),
        "Toggle with Europe/Berlin should NOT reject Saturday quest as 'wrong day'. \
         Navigate and toggle must agree on 'today'. Body: {body_str}"
    );

    time::reset_today();
}
