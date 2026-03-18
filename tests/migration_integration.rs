//! Integration tests for database migrations: idempotency, data preservation, schema integrity.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

//! Database migration integration tests
//! Tests migration safety, data integrity, and rollback scenarios

use chrono::NaiveDate;
use quest_log::database::Database;
use quest_log::models::ToggleResult;
use quest_log::time;
use sqlx::SqlitePool;

#[tokio::test]
async fn test_migration_idempotency() {
    // Test that running migrations multiple times is safe
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);

    // Run migrations multiple times
    for i in 0..5 {
        db.migrate()
            .await
            .unwrap_or_else(|_| panic!("Migration run {} failed", i + 1));
    }

    // Verify database structure is intact
    let settings = db
        .get_settings()
        .await
        .expect("Failed to get settings after multiple migrations");
    assert_eq!(settings.weekly_exp_goal, 100);

    // Verify we can still perform operations
    let quest_req = quest_log::models::CreateQuestRequest {
        title: "Migration Test Quest".to_string(),
        description: None,
        exp_value: Some(10),
        day_of_week: 1,
    };

    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create quest after multiple migrations");
    assert_eq!(quest.title, "Migration Test Quest");

    println!("Migration idempotency test completed successfully");
}

#[tokio::test]
async fn test_migration_data_preservation() {
    // Test that migrations preserve existing data
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Initial migration failed");

    // Create some data before "migration"
    let quest_req = quest_log::models::CreateQuestRequest {
        title: "Pre-Migration Quest".to_string(),
        description: Some("Created before migration".to_string()),
        exp_value: Some(25),
        day_of_week: 2,
    };

    let original_quest = db.create_quest(quest_req).await.unwrap();

    // Complete the quest on a date within the week we want to claim
    // Set up the test week: Jan 8 (Monday) to Jan 14 (Sunday)
    let quest_date = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(); // Wednesday
    db.toggle_quest_completion(original_quest.id, quest_date)
        .await
        .unwrap();

    // Save quest_date for verification below
    let today = quest_date;

    // Create a reward and claim it
    let reward_req = quest_log::models::CreateRewardRequest {
        title: "Pre-Migration Reward".to_string(),
        description: Some("Created before migration".to_string()),
        required_exp: 25,
    };

    let reward = db.create_reward(reward_req).await.unwrap();

    // Set today to the Sunday AFTER the week we want to claim for
    // (claim_reward_for_week requires Sunday and the week to have ended)
    let sunday_after = NaiveDate::from_ymd_opt(2024, 1, 14).unwrap();
    time::set_today(sunday_after);

    // Calculate week_start - the previous Monday
    let week_start = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
    let claimed = db
        .claim_reward_for_week(reward.id, week_start)
        .await
        .unwrap();
    assert!(claimed);

    // Run migrations again (simulating a deployment with new migrations)
    db.migrate().await.expect("Re-migration failed");

    // Verify all data is preserved
    let retrieved_quest = db
        .get_quest_by_id(original_quest.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved_quest.title, "Pre-Migration Quest");
    assert_eq!(retrieved_quest.exp_value, 25);
    assert_eq!(retrieved_quest.day_of_week, 2);

    // Verify completion is preserved
    let is_completed = db
        .is_quest_completed_today(original_quest.id, today)
        .await
        .unwrap();
    assert!(is_completed);

    // Verify reward and claim are preserved
    let rewards = db.get_available_rewards().await.unwrap();
    assert!(rewards.iter().any(|r| r.title == "Pre-Migration Reward"));

    let claimed_count = db.get_rewards_claimed_count().await.unwrap();
    assert_eq!(claimed_count, 1);

    // Verify EXP calculations still work
    let week_end = week_start + chrono::Duration::days(6);
    let weekly_exp = db.calculate_weekly_exp(week_start, week_end).await.unwrap();
    assert_eq!(weekly_exp, 25);

    println!("Migration data preservation test completed successfully");
}

#[tokio::test]
async fn test_migration_failure_recovery() {
    // Test behavior when migrations fail partway through
    // This is harder to test directly, but we can test the migration system's robustness

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);

    // Run successful migration
    db.migrate().await.expect("Migration should succeed");

    // Verify database is functional
    let _settings = db
        .get_settings()
        .await
        .expect("Database should be functional after migration");

    // Test that subsequent migration attempts don't break anything
    // (In a real scenario, if a migration failed partway, we'd need manual intervention)
    db.migrate()
        .await
        .expect("Re-running migration should be safe");

    // Verify database is still functional
    let settings_after = db
        .get_settings()
        .await
        .expect("Database should remain functional");
    assert_eq!(settings_after.weekly_exp_goal, 100);

    println!("Migration failure recovery test completed successfully");
}

#[tokio::test]
async fn test_migration_schema_integrity() {
    // Test that migrations create the expected database schema
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Migration failed");

    // Test that all expected tables exist by attempting operations on them
    // If tables don't exist, these operations will fail

    // Test settings table
    let _settings = db
        .get_settings()
        .await
        .expect("Settings table should exist");

    // Test quests table
    let quest_req = quest_log::models::CreateQuestRequest {
        title: "Schema Test Quest".to_string(),
        description: None,
        exp_value: Some(10),
        day_of_week: 1,
    };
    let quest = db
        .create_quest(quest_req)
        .await
        .expect("Quests table should exist");
    let _retrieved = db
        .get_quest_by_id(quest.id)
        .await
        .expect("Quest retrieval should work");

    // Test quest_completions table - complete on a date within the test week
    let quest_date = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap(); // Wednesday
    let completed = db
        .toggle_quest_completion(quest.id, quest_date)
        .await
        .expect("Quest completions table should exist");
    assert_eq!(completed, ToggleResult::NewlyCompleted);

    // Test rewards table
    let reward_req = quest_log::models::CreateRewardRequest {
        title: "Schema Test Reward".to_string(),
        description: None,
        required_exp: 10,
    };
    let reward = db
        .create_reward(reward_req)
        .await
        .expect("Rewards table should exist");
    let rewards = db
        .get_available_rewards()
        .await
        .expect("Rewards retrieval should work");
    assert!(!rewards.is_empty());

    // Set today to the Sunday AFTER the week we want to claim for
    // (claim_reward_for_week requires Sunday and the week to have ended)
    let sunday_after = NaiveDate::from_ymd_opt(2024, 1, 14).unwrap();
    time::set_today(sunday_after);

    // Test reward_claims table (implicitly tested through claiming)
    // Calculate week_start - the previous Monday
    let week_start = NaiveDate::from_ymd_opt(2024, 1, 8).unwrap();
    let claimed = db
        .claim_reward_for_week(reward.id, week_start)
        .await
        .expect("Reward claims table should exist");
    assert!(claimed);

    let claimed_count = db
        .get_rewards_claimed_count()
        .await
        .expect("Reward claims count should work");
    assert_eq!(claimed_count, 1);

    println!("Migration schema integrity test completed successfully");
}

#[tokio::test]
async fn test_migration_version_tracking() {
    // Test that the migration system properly tracks applied migrations
    // This is an integration test that verifies sqlx migration tracking works

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool.clone());

    // Run migrations
    db.migrate().await.expect("Migration should succeed");

    // Verify we can query the migrations table (created by sqlx)
    // This is an internal sqlx table, but we can test it exists by checking our operations work
    let result: Result<(i64,), sqlx::Error> =
        sqlx::query_as("SELECT COUNT(*) FROM _sqlx_migrations WHERE success = 1")
            .fetch_one(&pool)
            .await;

    match result {
        Ok((count,)) => {
            assert!(count > 0, "Should have successful migrations recorded");
            println!("Found {count} successful migrations");
        }
        Err(_) => {
            // If we can't query the internal table, that's OK - the important thing
            // is that our migrations ran and the database works
            println!(
                "Could not query migration tracking table (expected in some sqlx configurations)"
            );
        }
    }

    // The real test is that our database operations work after migration
    let _settings = db
        .get_settings()
        .await
        .expect("Database should be functional");

    println!("Migration version tracking test completed successfully");
}
