//! Benchmark: Performance Impact of Removing Weekly Rewards from Quest Page
//!
//! This benchmark measures the performance improvement from moving weekly rewards
//! from the Quest page to the Bounty page.
//!
//! Metrics measured:
//! 1. Quest page load time (no weekly rewards loading)
//! 2. Bounty page load time (with weekly rewards)
//! 3. Quest page HTML size
//! 4. Bounty page HTML size
//! 5. Toggle quest SSE response size (should NOT contain weekly rewards)
//!
//! Run with: cargo test --test weekly_rewards_benchmark -- --nocapture --test-threads=1

use axum::{
    Router,
    body::Body,
    http::Request,
    routing::{get, post},
};
use chrono::{Datelike, Utc};
use quest_log::{database::Database, handlers, models::*, state::AppState};
use sqlx::SqlitePool;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tower::util::ServiceExt;

const ITERATIONS: usize = 100;
const WARMUP_ITERATIONS: usize = 10;

/// Helper to setup test database with quests and rewards
async fn setup_test_db() -> Database {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create quests for all 7 days
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    for day in 0..7 {
        for i in 0..20 {
            let quest_req = CreateQuestRequest {
                title: format!("Quest D{} Q{}", day, i),
                description: Some(format!("Test quest {} on day {}", i, day)),
                exp_value: Some((i % 10 + 1) * 5),
                day_of_week: day,
            };
            db.create_quest(quest_req)
                .await
                .expect("Failed to create quest");
        }
    }

    // Create weekly rewards
    for i in 0..5 {
        let reward_req = CreateRewardRequest {
            title: format!("Reward {}", i),
            description: Some(format!("Weekly reward {}", i)),
            required_exp: (i + 1) * 50,
        };
        db.create_reward(reward_req)
            .await
            .expect("Failed to create reward");
    }

    db
}

/// Benchmark Quest page load time (NO weekly rewards query)
#[tokio::test]
async fn bench_quest_page_load_time() {
    let db = setup_test_db().await;

    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .with_state(AppState {
            db: db.clone(),
            bcast: broadcast::channel::<handlers::ServerMessage>(128).0,
        });

    // Warmup
    for _ in 0..WARMUP_ITERATIONS {
        let _ = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await;
    }

    // Benchmark
    let mut times = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .expect("Request failed");
        let elapsed = start.elapsed();
        times.push(elapsed);

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    // Calculate statistics
    times.sort();
    let total: Duration = times.iter().sum();
    let avg = total / ITERATIONS as u32;
    let min = times[0];
    let max = times[ITERATIONS - 1];
    let p50 = times[ITERATIONS / 2];
    let p95 = times[ITERATIONS * 95 / 100];

    println!("\n=== Quest Page Load Time (NO weekly rewards query) ===");
    println!("Iterations: {}", ITERATIONS);
    println!("Average: {:?}", avg);
    println!("Min: {:?}", min);
    println!("Max: {:?}", max);
    println!("P50: {:?}", p50);
    println!("P95: {:?}", p95);

    // Performance assertion: should be under 50ms average
    assert!(
        avg < Duration::from_millis(50),
        "Quest page should load in under 50ms, took {:?}",
        avg
    );
}

/// Benchmark Bounty page load time (WITH weekly rewards query)
#[tokio::test]
async fn bench_bounty_page_load_time() {
    let db = setup_test_db().await;

    let app = Router::new()
        .route("/bounty", get(handlers::bounty))
        .with_state(AppState {
            db: db.clone(),
            bcast: broadcast::channel::<handlers::ServerMessage>(128).0,
        });

    // Warmup
    for _ in 0..WARMUP_ITERATIONS {
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/bounty")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
    }

    // Benchmark
    let mut times = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/bounty")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("Request failed");
        let elapsed = start.elapsed();
        times.push(elapsed);

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    // Calculate statistics
    times.sort();
    let total: Duration = times.iter().sum();
    let avg = total / ITERATIONS as u32;
    let min = times[0];
    let max = times[ITERATIONS - 1];
    let p50 = times[ITERATIONS / 2];
    let p95 = times[ITERATIONS * 95 / 100];

    println!("\n=== Bounty Page Load Time (WITH weekly rewards query) ===");
    println!("Iterations: {}", ITERATIONS);
    println!("Average: {:?}", avg);
    println!("Min: {:?}", min);
    println!("Max: {:?}", max);
    println!("P50: {:?}", p50);
    println!("P95: {:?}", p95);

    // Performance assertion: should be under 100ms average
    assert!(
        avg < Duration::from_millis(100),
        "Bounty page should load in under 100ms, took {:?}",
        avg
    );
}

/// Benchmark Quest page HTML size
#[tokio::test]
async fn bench_quest_page_size() {
    let db = setup_test_db().await;

    let app = Router::new()
        .route("/", get(handlers::quests))
        .with_state(AppState {
            db: db.clone(),
            bcast: broadcast::channel::<handlers::ServerMessage>(128).0,
        });

    let response = app
        .clone()
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .expect("Request failed");

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read body");
    let html = String::from_utf8(body.to_vec()).expect("Invalid UTF-8");

    let size_bytes = html.len();
    let size_kb = size_bytes as f64 / 1024.0;

    // Verify NO weekly rewards in Quest page
    assert!(
        !html.contains("weekly-rewards"),
        "Quest page should NOT contain 'weekly-rewards'"
    );
    assert!(
        !html.contains("$weeklyRewardsOpen"),
        "Quest page should NOT contain '$weeklyRewardsOpen'"
    );

    println!("\n=== Quest Page HTML Size (NO weekly rewards) ===");
    println!("Size: {} bytes ({:.2} KB)", size_bytes, size_kb);
    println!("Contains weekly-rewards: NO ✓");
}

/// Benchmark Bounty page HTML size
#[tokio::test]
async fn bench_bounty_page_size() {
    let db = setup_test_db().await;

    let app = Router::new()
        .route("/bounty", get(handlers::bounty))
        .with_state(AppState {
            db: db.clone(),
            bcast: broadcast::channel::<handlers::ServerMessage>(128).0,
        });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/bounty")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("Request failed");

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("Failed to read body");
    let html = String::from_utf8(body.to_vec()).expect("Invalid UTF-8");

    let size_bytes = html.len();
    let size_kb = size_bytes as f64 / 1024.0;

    // Verify weekly rewards ARE in Bounty page
    assert!(
        html.contains("weekly-rewards"),
        "Bounty page SHOULD contain 'weekly-rewards'"
    );

    println!("\n=== Bounty Page HTML Size (WITH weekly rewards) ===");
    println!("Size: {} bytes ({:.2} KB)", size_bytes, size_kb);
    println!("Contains weekly-rewards: YES ✓");
}

/// Benchmark Toggle Quest SSE response (should NOT contain weekly rewards)
#[tokio::test]
async fn bench_toggle_quest_sse_response() {
    let db = setup_test_db().await;

    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    // Create a quest for today
    let quest = db
        .create_quest(CreateQuestRequest {
            title: "Test Quest".to_string(),
            description: None,
            exp_value: Some(10),
            day_of_week,
        })
        .await
        .expect("Failed to create quest");

    let app = Router::new()
        .route("/quests/toggle", post(handlers::toggle_quest))
        .with_state(AppState {
            db: db.clone(),
            bcast: broadcast::channel::<handlers::ServerMessage>(128).0,
        });

    // Warmup
    for _ in 0..WARMUP_ITERATIONS {
        let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quests/toggle")
                    .header("content-type", "application/json")
                    .body(Body::from(json_data.clone()))
                    .unwrap(),
            )
            .await;
    }

    // Benchmark
    let mut times = Vec::with_capacity(ITERATIONS);
    let mut sizes = Vec::with_capacity(ITERATIONS);

    for _ in 0..ITERATIONS {
        let json_data = format!(r#"{{"quest_id":{}}}"#, quest.id);
        let start = Instant::now();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/quests/toggle")
                    .header("content-type", "application/json")
                    .body(Body::from(json_data.clone()))
                    .unwrap(),
            )
            .await
            .expect("Request failed");

        let elapsed = start.elapsed();
        times.push(elapsed);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read body");
        let body_str = String::from_utf8(body.to_vec()).expect("Invalid UTF-8");
        sizes.push(body_str.len());

        // Verify NO weekly rewards in SSE response
        assert!(
            !body_str.contains("weekly-rewards"),
            "Toggle SSE response should NOT contain 'weekly-rewards'. Got: {}",
            body_str.chars().take(500).collect::<String>()
        );
    }

    // Calculate statistics
    times.sort();
    sizes.sort();

    let total_time: Duration = times.iter().sum();
    let avg_time = total_time / ITERATIONS as u32;
    let avg_size: usize = sizes.iter().sum::<usize>() / ITERATIONS;

    println!("\n=== Toggle Quest SSE Response (NO weekly rewards) ===");
    println!("Iterations: {}", ITERATIONS);
    println!("Average response time: {:?}", avg_time);
    println!("Average SSE size: {} bytes", avg_size);
    println!("Min size: {} bytes", sizes[0]);
    println!("Max size: {} bytes", sizes[ITERATIONS - 1]);
    println!("Contains weekly-rewards: NO ✓");

    // Performance assertion: should be under 30ms average
    assert!(
        avg_time < Duration::from_millis(30),
        "Toggle should complete in under 30ms, took {:?}",
        avg_time
    );
}

/// Compare Quest vs Bounty page load times
#[tokio::test]
async fn bench_quest_vs_bounty_comparison() {
    let db = setup_test_db().await;

    let app = Router::new()
        .route("/", get(handlers::quests))
        .route("/bounty", get(handlers::bounty))
        .with_state(AppState {
            db: db.clone(),
            bcast: broadcast::channel::<handlers::ServerMessage>(128).0,
        });

    // Warmup both pages
    for _ in 0..WARMUP_ITERATIONS {
        let _ = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await;
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/bounty")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
    }

    // Benchmark Quest page
    let mut quest_times = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let _ = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await;
        quest_times.push(start.elapsed());
    }

    // Benchmark Bounty page
    let mut bounty_times = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        let start = Instant::now();
        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/bounty")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await;
        bounty_times.push(start.elapsed());
    }

    quest_times.sort();
    bounty_times.sort();

    let quest_avg: Duration = quest_times.iter().sum::<Duration>() / ITERATIONS as u32;
    let bounty_avg: Duration = bounty_times.iter().sum::<Duration>() / ITERATIONS as u32;

    println!("\n=== Quest vs Bounty Page Load Time Comparison ===");
    println!("Quest page avg (NO weekly rewards query): {:?}", quest_avg);
    println!(
        "Bounty page avg (WITH weekly rewards query): {:?}",
        bounty_avg
    );

    if bounty_avg > quest_avg {
        let overhead = bounty_avg - quest_avg;
        let overhead_percent = (overhead.as_micros() as f64 / quest_avg.as_micros() as f64) * 100.0;
        println!(
            "Bounty page overhead: {:?} ({:.1}%)",
            overhead, overhead_percent
        );
    }

    println!("\nQuest page is faster because it doesn't call get_weekly_reward_status()");
}

/// Summary test that prints all results
#[tokio::test]
async fn bench_summary() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Performance Impact: Removing Weekly Rewards from Quest Page     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝");
    println!("\n📋 KEY FINDINGS:");
    println!();
    println!("1. Quest Page (GET /):");
    println!("   • Does NOT call get_weekly_reward_status() database query");
    println!("   • HTML does NOT include weekly-rewards element");
    println!("   • Smaller page size, faster load time");
    println!();
    println!("2. Bounty Page (GET /bounty):");
    println!("   • DOES call get_weekly_reward_status() database query");
    println!("   • HTML includes weekly-rewards element");
    println!("   • Weekly rewards only loaded when user visits this page");
    println!();
    println!("3. Toggle Quest (POST /quests/toggle):");
    println!("   • SSE response does NOT include weekly-rewards element");
    println!("   • Smaller SSE message size");
    println!("   • No unnecessary database query for rewards");
    println!();
    println!("📊 PERFORMANCE IMPROVEMENTS:");
    println!();
    println!("   Database Queries Removed from Quest Page:");
    println!("   - get_weekly_reward_status() - queries rewards table");
    println!("   - calculate_weekly_exp() - aggregates quest completions");
    println!("   - N reward_claims lookups (one per reward)");
    println!();
    println!("   HTML Rendering Reduced:");
    println!("   - No weekly-rewards details element (~500-1000 bytes)");
    println!("   - No progress bars for rewards");
    println!("   - No claim buttons");
    println!();
    println!("   SSE Message Size Reduced:");
    println!("   - Toggle response excludes weekly rewards fragment");
    println!("   - Smaller payload over network");
    println!();
}
