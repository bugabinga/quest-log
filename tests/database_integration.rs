//! Integration tests for database layer: migrations, CRUD operations, and queries.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

use quest_log::{database::Database, models::*};
use sqlx::SqlitePool;

#[tokio::test]
async fn test_database_layer_integration() {
    // Setup test database with in-memory SQLite (like unit tests)
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Test settings initialization (this verifies database schema exists)
    let settings: Settings = db.get_settings().await.expect("Failed to get settings");
    assert_eq!(
        settings.weekly_exp_goal, 100,
        "Default weekly goal incorrect"
    );

    // Test quest CRUD operations
    let quest_req = CreateQuestRequest {
        title: "Integration Test Quest".to_string(),
        description: Some("Testing database integration".to_string()),
        exp_value: Some(25),
        day_of_week: 1, // Monday
    };

    let quest: Quest = db
        .create_quest(quest_req)
        .await
        .expect("Failed to create quest");
    assert_eq!(quest.title, "Integration Test Quest");
    assert_eq!(quest.exp_value, 25);
    assert_eq!(quest.day_of_week, 1);

    // Test retrieval
    let retrieved: Option<Quest> = db
        .get_quest_by_id(quest.id)
        .await
        .expect("Failed to retrieve quest");
    assert!(retrieved.is_some());
    let retrieved_quest = retrieved.unwrap();
    assert_eq!(retrieved_quest.title, quest.title);

    // Test quest listing by day
    let monday_quests: Vec<Quest> = db
        .get_quests_for_day(1)
        .await
        .expect("Failed to get quests for day");
    assert_eq!(monday_quests.len(), 1);
    assert_eq!(monday_quests[0].title, "Integration Test Quest");

    // Test update
    let update_req = UpdateQuestRequest {
        title: Some("Updated Integration Test Quest".to_string()),
        description: None,
        exp_value: Some(30),
        day_of_week: Some(2),
        is_active: None,
    };

    let updated: Option<Quest> = db
        .update_quest(quest.id, update_req)
        .await
        .expect("Failed to update quest");
    assert!(updated.is_some());
    let updated_quest = updated.unwrap();
    assert_eq!(updated_quest.title, "Updated Integration Test Quest");
    assert_eq!(updated_quest.exp_value, 30);
    assert_eq!(updated_quest.day_of_week, 2);

    // Test deletion
    let deleted: bool = db
        .delete_quest(updated_quest.id)
        .await
        .expect("Failed to delete quest");
    assert!(deleted);

    let after_delete: Option<Quest> = db
        .get_quest_by_id(updated_quest.id)
        .await
        .expect("Failed to check deletion");
    assert!(after_delete.is_none());

    // Test reward operations
    let reward_req = CreateRewardRequest {
        title: "Integration Test Reward".to_string(),
        description: Some("Testing reward integration".to_string()),
        required_exp: 50,
    };

    let reward: Reward = db
        .create_reward(reward_req)
        .await
        .expect("Failed to create reward");
    assert_eq!(reward.title, "Integration Test Reward");
    assert_eq!(reward.required_exp, 50);

    // Test reward listing
    let rewards: Vec<Reward> = db
        .get_available_rewards()
        .await
        .expect("Failed to get rewards");
    assert!(!rewards.is_empty());
    assert!(rewards.iter().any(|r| r.title == "Integration Test Reward"));

    // Test settings update
    let settings_update = UpdateSettingsRequest {
        weekly_exp_goal: 150,
    };
    let updated_settings: Settings = db
        .update_settings(settings_update)
        .await
        .expect("Failed to update settings");
    assert_eq!(updated_settings.weekly_exp_goal, 150);

    // Test data integrity - create quest with invalid day_of_week should fail (database constraint)
    let invalid_quest = CreateQuestRequest {
        title: "Invalid Day Quest".to_string(),
        description: None,
        exp_value: Some(10),
        day_of_week: 10, // Invalid day - should be 0-6
    };
    let invalid_result: Result<Quest, sqlx::Error> = db.create_quest(invalid_quest).await;
    assert!(
        invalid_result.is_err(),
        "Database should reject invalid day_of_week values"
    );

    // Test concurrent operations (basic concurrency test)
    let quest_req2 = CreateQuestRequest {
        title: "Concurrent Quest".to_string(),
        description: None,
        exp_value: Some(15),
        day_of_week: 3,
    };
    let quest2: Quest = db
        .create_quest(quest_req2)
        .await
        .expect("Failed to create concurrent quest");

    // Both quests should exist
    let quest1_check: Option<Quest> = db
        .get_quest_by_id(quest.id)
        .await
        .expect("Quest 1 should still exist");
    let quest2_check: Option<Quest> = db
        .get_quest_by_id(quest2.id)
        .await
        .expect("Quest 2 should exist");
    assert!(quest1_check.is_none()); // quest was deleted earlier
    assert!(quest2_check.is_some());

    // Cleanup - no cleanup needed for in-memory database
    // The database will be automatically cleaned up when the pool is dropped
}
