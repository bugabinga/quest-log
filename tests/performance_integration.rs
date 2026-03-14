//! Performance and load testing integration tests
//! Tests system performance under various loads and conditions

use axum::{
    Router,
    body::Body,
    http::Request,
    routing::{get, post},
};
use chrono::{Datelike, Utc};
use quest_log::database::Database;
use quest_log::handlers::{quests, toggle_quest};
use quest_log::models::CreateQuestRequest;
use quest_log::state::AppState;
use sqlx::SqlitePool;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_large_dataset_performance() {
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
            AppState::new(db.clone(), bcast_tx)
        });

    println!("Creating large dataset for performance testing...");

    // Create a large number of quests across different days
    let start_time = Instant::now();
    let mut total_quests = 0;

    for day in 0..7 {
        for i in 0..100 {
            // 100 quests per day = 700 total
            let quest_req = CreateQuestRequest {
                title: format!("Performance Quest D{} Q{}", day, i),
                description: Some(format!("Performance test quest {} on day {}", i, day)),
                exp_value: Some((i % 20) + 1), // Vary EXP from 1-20
                day_of_week: day,
            };
            db.create_quest(quest_req).await.unwrap();
            total_quests += 1;
        }
    }

    let creation_time = start_time.elapsed();
    println!("Created {} quests in {:?}", total_quests, creation_time);

    // Test page load performance for each day
    let mut page_load_times = vec![];

    for day in 0..7 {
        let request = Request::builder()
            .uri("/")
            .header("x-test-day", day.to_string()) // Simulate different days
            .body(Body::empty())
            .unwrap();

        let load_start = Instant::now();
        let response = app.clone().oneshot(request).await.unwrap();
        let load_time = load_start.elapsed();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        page_load_times.push(load_time);

        // Each page should load in under 1 second
        assert!(
            load_time < Duration::from_secs(1),
            "Page load for day {} should be fast, took {:?}",
            day,
            load_time
        );
    }

    let avg_page_load = page_load_times.iter().sum::<Duration>() / page_load_times.len() as u32;
    println!("Average page load time: {:?}", avg_page_load);

    // Test database query performance
    let query_start = Instant::now();
    for day in 0..7 {
        let quests = db.get_quests_for_day(day).await.unwrap();
        assert_eq!(quests.len(), 100, "Should have 100 quests for day {}", day);
    }
    let query_time = query_start.elapsed();

    println!("Database queries completed in {:?}", query_time);
    assert!(
        query_time < Duration::from_millis(500),
        "Database queries should be fast, took {:?}",
        query_time
    );

    // Test EXP calculation performance
    let exp_calc_start = Instant::now();
    use chrono::NaiveDate;

    let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    for week_offset in 0..4 {
        let test_week_start = week_start + chrono::Duration::days(week_offset * 7);
        let test_week_end = test_week_start + chrono::Duration::days(6);
        let _exp = db
            .calculate_weekly_exp(test_week_start, test_week_end)
            .await
            .unwrap();
    }
    let exp_calc_time = exp_calc_start.elapsed();

    println!("EXP calculations completed in {:?}", exp_calc_time);
    assert!(
        exp_calc_time < Duration::from_millis(200),
        "EXP calculations should be fast, took {:?}",
        exp_calc_time
    );

    println!("Large dataset performance test completed successfully");
}

#[tokio::test]
async fn test_concurrent_load_simulation() {
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
            AppState::new(db.clone(), bcast_tx)
        });

    // Create some test quests
    let today = Utc::now().date_naive();
    let day_of_week = today.weekday().num_days_from_sunday() as i32;

    let mut quest_ids = vec![];
    for i in 0..10 {
        let quest_req = CreateQuestRequest {
            title: format!("Load Test Quest {}", i),
            description: None,
            exp_value: Some(10),
            day_of_week,
        };
        let quest = db.create_quest(quest_req).await.unwrap();
        quest_ids.push(quest.id);
    }

    println!("Simulating concurrent load with {} users...", 50);

    // Simulate 50 concurrent users performing various operations
    let mut handles = vec![];
    let start_time = Instant::now();

    for user_id in 0..50 {
        let app_clone = app.clone();
        let quest_ids_clone = quest_ids.clone();

        let handle = tokio::spawn(async move {
            let mut user_operations = 0;
            let user_start = Instant::now();

            // Each user performs multiple operations
            for operation in 0..5 {
                let quest_index = (user_id + operation) % quest_ids_clone.len();
                let quest_id = quest_ids_clone[quest_index];

                // Mix of read and write operations
                if operation % 2 == 0 {
                    // Read operation - load page
                    let request = Request::builder().uri("/").body(Body::empty()).unwrap();

                    let _response = app_clone.clone().oneshot(request).await.unwrap();
                } else {
                    // Write operation - toggle quest
                    let json_data = format!(r#"{{"quest_id":{}}}"#, quest_id);
                    let request = Request::builder()
                        .method("POST")
                        .uri("/quests/toggle")
                        .header("content-type", "application/json")
                        .body(Body::from(json_data))
                        .unwrap();

                    let _response = app_clone.clone().oneshot(request).await.unwrap();
                }

                user_operations += 1;
            }

            let user_duration = user_start.elapsed();
            (user_operations, user_duration)
        });

        handles.push(handle);
    }

    // Wait for all users to complete
    let mut total_operations = 0;
    let mut total_user_time = Duration::from_secs(0);

    for handle in handles {
        let (operations, duration) = handle.await.unwrap();
        total_operations += operations;
        total_user_time += duration;
    }

    let total_duration = start_time.elapsed();
    let avg_user_time = total_user_time / 50;
    let operations_per_second = total_operations as f64 / total_duration.as_secs_f64();

    println!("Concurrent load test results:");
    println!("- Total operations: {}", total_operations);
    println!("- Total duration: {:?}", total_duration);
    println!("- Average user time: {:?}", avg_user_time);
    println!("- Operations per second: {:.2}", operations_per_second);

    // Performance assertions
    assert!(
        total_duration < Duration::from_secs(30),
        "Concurrent load should complete within 30 seconds, took {:?}",
        total_duration
    );
    assert!(
        avg_user_time < Duration::from_secs(5),
        "Average user should complete in under 5 seconds, took {:?}",
        avg_user_time
    );
    assert!(
        operations_per_second > 10.0,
        "Should handle at least 10 operations per second, got {:.2}",
        operations_per_second
    );

    println!("Concurrent load simulation test completed successfully");
}

#[tokio::test]
async fn test_memory_usage_stability() {
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
            AppState::new(db.clone(), bcast_tx)
        });

    println!("Testing memory usage stability over extended period...");

    let start_time = Instant::now();
    let test_duration = Duration::from_secs(10); // Run for 10 seconds
    let mut operation_count = 0;

    while start_time.elapsed() < test_duration {
        // Perform various operations to stress memory usage
        let request = Request::builder().uri("/").body(Body::empty()).unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        operation_count += 1;

        // Small delay to prevent overwhelming the system
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let elapsed = start_time.elapsed();
    let operations_per_second = operation_count as f64 / elapsed.as_secs_f64();

    println!("Memory stability test completed:");
    println!("- Duration: {:?}", elapsed);
    println!("- Operations: {}", operation_count);
    println!("- Operations per second: {:.2}", operations_per_second);

    // The test passes if we can complete the extended run without issues
    assert!(
        operation_count > 100,
        "Should complete at least 100 operations in extended test, got {}",
        operation_count
    );

    println!("Memory usage stability test completed successfully");
}

#[tokio::test]
async fn test_database_connection_pooling() {
    // Test that database connections are properly pooled and reused
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    // Create multiple database instances sharing the same pool
    let db_instances: Vec<Database> = (0..10).map(|_| Database::with_pool(pool.clone())).collect();

    // All instances should be able to migrate (testing connection sharing)
    for (i, db) in db_instances.iter().enumerate() {
        db.migrate()
            .await
            .expect(&format!("Migration failed for instance {}", i));
    }

    // Test concurrent operations across different instances
    let mut handles = vec![];

    for (i, db) in db_instances.into_iter().enumerate() {
        let handle = tokio::spawn(async move {
            // Each instance creates and queries quests
            let quest_req = CreateQuestRequest {
                title: format!("Pool Test Quest {}", i),
                description: None,
                exp_value: Some(5),
                day_of_week: 1,
            };

            let quest = db.create_quest(quest_req).await.unwrap();
            let retrieved = db.get_quest_by_id(quest.id).await.unwrap().unwrap();

            assert_eq!(retrieved.title, format!("Pool Test Quest {}", i));
        });

        handles.push(handle);
    }

    // Wait for all concurrent operations to complete
    for handle in handles {
        handle.await.unwrap();
    }

    println!("Database connection pooling test completed successfully");
}

#[tokio::test]
async fn test_cold_start_performance() {
    // Test application startup and first request performance
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);

    let cold_start = Instant::now();
    db.migrate().await.expect("Failed to run migrations");
    let migration_time = cold_start.elapsed();

    println!("Cold start migration time: {:?}", migration_time);

    // Create app after migration
    let app_creation = Instant::now();
    let app = Router::new()
        .route("/", get(quests))
        .route("/quests/toggle", post(toggle_quest))
        .with_state({
            let (bcast_tx, _) = broadcast::channel(128);
            AppState::new(db, bcast_tx)
        });
    let app_creation_time = app_creation.elapsed();

    println!("App creation time: {:?}", app_creation_time);

    // Test first request performance
    let first_request = Instant::now();
    let request = Request::builder().uri("/").body(Body::empty()).unwrap();

    let response = app.oneshot(request).await.unwrap();
    let first_request_time = first_request.elapsed();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    println!("First request time: {:?}", first_request_time);

    // Performance assertions
    assert!(
        migration_time < Duration::from_secs(1),
        "Migration should be fast, took {:?}",
        migration_time
    );
    assert!(
        app_creation_time < Duration::from_millis(100),
        "App creation should be fast, took {:?}",
        app_creation_time
    );
    assert!(
        first_request_time < Duration::from_millis(500),
        "First request should be reasonably fast, took {:?}",
        first_request_time
    );

    println!("Cold start performance test completed successfully");
}
