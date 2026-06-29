#![allow(clippy::module_inception)]
#[cfg(test)]
mod tests {
    use crate::database::Database;
    use crate::models::*;
    use crate::time;
    use chrono::NaiveDate;
    use sqlx::SqlitePool;
    use std::path::Path;
    use std::sync::Arc;

    async fn setup_test_db() -> Database {
        // Use in-memory SQLite for tests to avoid interference between tests
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        Database { pool }
    }

    #[tokio::test]
    async fn test_create_and_get_quest() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: Some("A test quest".to_string()),
            exp_value: Some(20),
            title: "Test Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();
        assert_eq!(quest.title, "Test Quest");
        assert_eq!(quest.exp_value, 20);
        assert_eq!(quest.day_of_week, 1);

        let retrieved = db.get_quest_by_id(quest.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, quest.title);
    }

    #[tokio::test]
    async fn test_get_quests_for_day() {
        let db = setup_test_db().await;

        // Create quests for different days
        let req1 = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: "Monday Quest".to_string(),
        };
        let req2 = CreateQuestRequest {
            day_of_week: 2,
            description: None,
            exp_value: Some(15),
            title: "Tuesday Quest".to_string(),
        };

        db.create_quest(req1).await.unwrap();
        db.create_quest(req2).await.unwrap();

        let monday_quests = db.get_quests_for_day(1).await.unwrap();
        assert_eq!(monday_quests.len(), 1);
        assert_eq!(monday_quests[0].title, "Monday Quest");

        let tuesday_quests = db.get_quests_for_day(2).await.unwrap();
        assert_eq!(tuesday_quests.len(), 1);
        assert_eq!(tuesday_quests[0].title, "Tuesday Quest");
    }

    #[tokio::test]
    async fn test_update_quest() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: "Original Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();

        let update_req = UpdateQuestRequest {
            title: Some("Updated Quest".to_string()),
            description: None,
            exp_value: Some(25),
            day_of_week: Some(3),
            is_active: None,
        };

        let updated = db
            .update_quest(quest.id, update_req)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.title, "Updated Quest");
        assert_eq!(updated.exp_value, 25);
        assert_eq!(updated.day_of_week, 3);
    }

    #[tokio::test]
    async fn test_delete_quest() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: "Quest to Delete".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();

        let deleted = db.delete_quest(quest.id).await.unwrap();
        assert!(deleted);

        let retrieved = db.get_quest_by_id(quest.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_toggle_quest_completion() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: "Completable Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();
        let today = time::today();

        // Complete quest
        let completed = db.toggle_quest_completion(quest.id, today).await.unwrap();
        assert_eq!(completed, ToggleResult::NewlyCompleted);

        // Check completion status
        let is_completed = db.is_quest_completed_today(quest.id, today).await.unwrap();
        assert!(is_completed);

        // Toggle again - should un-complete
        let completed_again = db.toggle_quest_completion(quest.id, today).await.unwrap();
        assert_eq!(completed_again, ToggleResult::NewlyUncompleted);

        let is_completed_after = db.is_quest_completed_today(quest.id, today).await.unwrap();
        assert!(!is_completed_after);
    }

    #[tokio::test]
    async fn test_calculate_weekly_exp() {
        let db = setup_test_db().await;

        // Create quest
        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(15),
            title: "EXP Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();

        // Complete quest on Monday and Wednesday of the current week
        let _today = time::today();
        // For simplicity, use a known Monday
        let monday = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // This was a Monday
        let wednesday = monday + chrono::Duration::days(2);

        db.toggle_quest_completion(quest.id, monday).await.unwrap();
        db.toggle_quest_completion(quest.id, wednesday)
            .await
            .unwrap();

        // Calculate EXP for the week
        let week_start = monday;
        let week_end = monday + chrono::Duration::days(6);
        let total_exp = db.calculate_weekly_exp(week_start, week_end).await.unwrap();

        assert_eq!(total_exp, 30); // 15 * 2 completions
    }

    #[tokio::test]
    async fn test_get_settings() {
        let db = setup_test_db().await;

        let settings = db.get_settings().await.unwrap();
        assert_eq!(settings.weekly_exp_goal, 100);
    }

    #[tokio::test]
    async fn test_update_settings() {
        let db = setup_test_db().await;

        let update_req = UpdateSettingsRequest {
            weekly_exp_goal: 150,
        };

        let updated = db.update_settings(update_req).await.unwrap();
        assert_eq!(updated.weekly_exp_goal, 150);
    }

    #[tokio::test]
    async fn test_create_and_get_rewards() {
        let db = setup_test_db().await;

        let req = CreateRewardRequest {
            description: Some("A test reward".to_string()),
            required_exp: 50,
            title: "Test Reward".to_string(),
        };

        let reward = db.create_reward(req).await.unwrap();
        assert_eq!(reward.title, "Test Reward");
        assert_eq!(reward.required_exp, 50);

        let rewards = db.get_available_rewards().await.unwrap();
        assert_eq!(rewards.len(), 1);
        assert_eq!(rewards[0].title, "Test Reward");
    }

    #[tokio::test]
    async fn test_claim_reward() {
        let db = setup_test_db().await;

        // Create reward requiring 30 EXP
        let reward_req = CreateRewardRequest {
            description: None,
            required_exp: 30,
            title: "Test Reward".to_string(),
        };
        let reward = db.create_reward(reward_req).await.unwrap();

        // Create quest worth 20 EXP
        let quest_req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(20),
            title: "Test Quest".to_string(),
        };
        let quest = db.create_quest(quest_req).await.unwrap();

        // Use test dates within the week
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // Monday
        let today = week_start + chrono::Duration::days(2); // Wednesday within the week

        // Try to claim reward without enough EXP - should fail
        let claimed = db.claim_reward(reward.id, week_start).await.unwrap();
        assert!(!claimed);

        // Complete quest to get 20 EXP
        db.toggle_quest_completion(quest.id, today).await.unwrap();

        // Try again - should still fail (need 30, have 20)
        let claimed = db.claim_reward(reward.id, week_start).await.unwrap();
        assert!(!claimed);

        // Create another quest and complete it
        let quest_2_req = CreateQuestRequest {
            day_of_week: 2,
            description: None,
            exp_value: Some(15),
            title: "Test Quest 2".to_string(),
        };
        let quest2 = db.create_quest(quest_2_req).await.unwrap();
        db.toggle_quest_completion(quest2.id, today).await.unwrap();

        // Now have 35 EXP, should be able to claim
        let claimed = db.claim_reward(reward.id, week_start).await.unwrap();
        assert!(claimed);

        // Try to claim again - should fail
        let claimed_again = db.claim_reward(reward.id, week_start).await.unwrap();
        assert!(!claimed_again);

        // Check that claim was recorded
        let claimed_count = db.get_rewards_claimed_count().await.unwrap();
        assert_eq!(claimed_count, 1);
    }

    // ===== QUEST_LOG_DATA_DIR INTEGRATION TESTS =====

    #[tokio::test]
    async fn test_quest_log_data_dir_absolute_path_validation() {
        use std::env;
        use tempfile::tempdir;

        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe {
            env::set_var("QUEST_LOG_DATA_DIR", "relative/path");
        }
        let result = Database::new().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must be an absolute path")
        );
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe {
            env::remove_var("QUEST_LOG_DATA_DIR");
        }

        // Test that absolute paths work and directory is created
        let temp_dir = tempdir().unwrap();
        let absolute_path = temp_dir
            .path()
            .join("test_data_dir")
            .to_string_lossy()
            .to_string();
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe {
            env::set_var("QUEST_LOG_DATA_DIR", &absolute_path);
        }

        // Directory shouldn't exist yet
        assert!(!Path::new(&absolute_path).exists());

        let result = Database::new().await;
        assert!(result.is_ok(), "Should succeed with absolute path");

        // Directory should now exist
        assert!(Path::new(&absolute_path).exists());

        // Database file should exist
        let db_path = Path::new(&absolute_path).join("quests.db");
        assert!(db_path.exists());

        // Clean up environment variable
        unsafe {
            // SAFETY: Only called in tests after verifying the variable was read first
            env::remove_var("QUEST_LOG_DATA_DIR");
        }
    }

    #[tokio::test]
    async fn test_create_quest_empty_title() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: String::new(),
        };

        // Empty title should still work (database doesn't enforce this constraint)
        let result = db.create_quest(req).await;
        assert!(result.is_ok(), "Empty title should be allowed by database");
    }

    #[tokio::test]
    async fn test_update_quest_not_found() {
        let db = setup_test_db().await;

        let update_req = UpdateQuestRequest {
            title: Some("Updated Title".to_string()),
            description: None,
            exp_value: None,
            day_of_week: None,
            is_active: None,
        };

        let result = db.update_quest(99999, update_req).await.unwrap();
        assert!(
            result.is_none(),
            "Updating non-existent quest should return None"
        );
    }

    #[tokio::test]
    async fn test_delete_quest_not_found() {
        let db = setup_test_db().await;

        let result = db.delete_quest(99999).await.unwrap();
        assert!(!result, "Deleting non-existent quest should return false");
    }

    #[tokio::test]
    async fn test_toggle_completion_nonexistent_quest() {
        let db = setup_test_db().await;
        let today = time::today();

        // Should return an error for non-existent quest
        let result = db.toggle_quest_completion(99999, today).await;
        assert!(
            result.is_err(),
            "Should return error for non-existent quest"
        );

        // Verify no completion was recorded
        let completions = db.get_completions_for_date(today).await.unwrap();
        assert!(
            completions.is_empty(),
            "No completions should be recorded for non-existent quest"
        );
    }

    #[tokio::test]
    async fn test_get_quest_by_id_not_found() {
        let db = setup_test_db().await;

        let result = db.get_quest_by_id(99999).await.unwrap();
        assert!(result.is_none(), "Non-existent quest ID should return None");
    }

    // ===== SECURITY UNIT TESTS =====

    #[tokio::test]
    async fn test_sql_injection_prevention() {
        let db = setup_test_db().await;

        // Test SQL injection attempts in quest titles
        let injection_attempts = vec![
            "'; DROP TABLE quests; --",
            "' OR '1'='1",
            "'; SELECT * FROM settings; --",
            "admin'--",
        ];

        for attempt in injection_attempts {
            let req = CreateQuestRequest {
                day_of_week: 1,
                description: Some("Injection attempt".to_string()),
                exp_value: Some(10),
                title: attempt.to_string(),
            };

            // Should succeed (data is properly escaped by sqlx)
            let quest = db.create_quest(req).await.unwrap();
            assert_eq!(quest.title, attempt, "Title should be stored as-is");

            // Verify we can retrieve it safely
            let retrieved = db.get_quest_by_id(quest.id).await.unwrap().unwrap();
            assert_eq!(retrieved.title, attempt, "Should retrieve safely");
        }
    }

    #[tokio::test]
    async fn test_xss_prevention_in_storage() {
        let db = setup_test_db().await;

        // Test XSS attempts in quest data
        let xss_attempts = vec![
            "<script>alert('xss')</script>",
            "<img src=x onerror=alert('xss')>",
            "javascript:alert('xss')",
            "<iframe src='javascript:alert(\"xss\")'>",
        ];

        for attempt in xss_attempts {
            let req = CreateQuestRequest {
                day_of_week: 1,
                description: Some("XSS attempt".to_string()),
                exp_value: Some(10),
                title: attempt.to_string(),
            };

            // Should succeed (data is properly escaped by sqlx)
            let quest = db.create_quest(req).await.unwrap();
            assert_eq!(quest.title, attempt, "Title should be stored as-is");

            // Verify we can retrieve it safely
            let retrieved = db.get_quest_by_id(quest.id).await.unwrap().unwrap();
            assert_eq!(retrieved.title, attempt, "Should retrieve safely");
        }
    }

    // ===== BOUNDARY AND EDGE CASE UNIT TESTS =====

    #[tokio::test]
    async fn test_zero_exp_quest() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(0),
            title: "Zero EXP Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();
        assert_eq!(quest.exp_value, 0);

        // Complete the quest
        let today = time::today();
        let completed = db.toggle_quest_completion(quest.id, today).await.unwrap();
        assert_eq!(completed, ToggleResult::NewlyCompleted);

        // Calculate weekly EXP - should include zero
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let week_end = week_start + chrono::Duration::days(6);
        let total_exp = db.calculate_weekly_exp(week_start, week_end).await.unwrap();
        assert_eq!(total_exp, 0, "Zero EXP quest should contribute 0 to total");
    }

    #[tokio::test]
    async fn test_negative_exp_quest() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(-10),
            title: "Negative EXP Quest".to_string(),
        };

        let result = db.create_quest(req).await;

        assert!(result.is_err(), "negative quest EXP must be rejected");
    }

    #[tokio::test]
    async fn test_max_exp_values() {
        let db = setup_test_db().await;

        // Test with very large EXP values
        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(i32::MAX),
            title: "Max EXP Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();
        assert_eq!(quest.exp_value, i32::MAX);

        // Complete the quest using consistent dates
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let today = week_start + chrono::Duration::days(2); // Wednesday within the week
        let completed = db.toggle_quest_completion(quest.id, today).await.unwrap();
        assert_eq!(completed, ToggleResult::NewlyCompleted);

        // Calculate weekly EXP - should handle large values
        let week_end = week_start + chrono::Duration::days(6);
        let total_exp = db.calculate_weekly_exp(week_start, week_end).await.unwrap();
        assert_eq!(total_exp, i32::MAX, "Should handle maximum i32 values");
    }

    #[tokio::test]
    async fn test_empty_quest_list() {
        let db = setup_test_db().await;

        // Test getting quests for a day with no quests
        let quests = db.get_quests_for_day(5).await.unwrap();
        assert!(
            quests.is_empty(),
            "Should return empty list for day with no quests"
        );

        // Test weekly EXP calculation with no quests
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let week_end = week_start + chrono::Duration::days(6);
        let total_exp = db.calculate_weekly_exp(week_start, week_end).await.unwrap();
        assert_eq!(total_exp, 0, "Empty quest list should result in 0 EXP");
    }

    #[tokio::test]
    async fn test_get_quests_for_invalid_day() {
        let db = setup_test_db().await;

        // Test edge cases for day_of_week
        let quests = db.get_quests_for_day(-1).await.unwrap();
        assert!(quests.is_empty(), "Invalid day should return empty list");

        let quests = db.get_quests_for_day(7).await.unwrap();
        assert!(quests.is_empty(), "Invalid day should return empty list");
    }

    #[tokio::test]
    async fn test_reward_claiming_boundary_conditions() {
        let db = setup_test_db().await;

        // Create reward requiring exactly 0 EXP
        let reward_req = CreateRewardRequest {
            description: None,
            required_exp: 0,
            title: "Free Reward".to_string(),
        };
        let reward = db.create_reward(reward_req).await.unwrap();

        // Should be able to claim immediately
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let claimed = db.claim_reward(reward.id, week_start).await.unwrap();
        assert!(claimed, "Should be able to claim reward requiring 0 EXP");

        // Create reward requiring exact EXP match
        let reward_req2 = CreateRewardRequest {
            description: None,
            required_exp: 25,
            title: "Exact Match Reward".to_string(),
        };
        let reward2 = db.create_reward(reward_req2).await.unwrap();

        // Create quest worth exactly 25 EXP
        let quest_req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(25),
            title: "Exact EXP Quest".to_string(),
        };
        let quest = db.create_quest(quest_req).await.unwrap();

        // Complete the quest
        let today = week_start + chrono::Duration::days(2);
        db.toggle_quest_completion(quest.id, today).await.unwrap();

        // Should be able to claim with exact EXP match
        let claimed = db.claim_reward(reward2.id, week_start).await.unwrap();
        assert!(claimed, "Should be able to claim with exact EXP match");
    }

    // ===== CONCURRENT OPERATION UNIT TESTS =====

    #[tokio::test]
    async fn test_concurrent_quest_completions() {
        use tempfile::tempdir;

        // Use file-based SQLite for concurrent test (in-memory doesn't share across connections)
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("concurrent_test.db");

        // Create the database file first (required for SQLite)
        std::fs::File::create(&db_path).unwrap();

        let db_url = format!("sqlite:{}", db_path.display());
        let pool = SqlitePool::connect(&db_url).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let db = Database { pool };

        // Create a quest
        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: "Concurrent Quest".to_string(),
        };
        let quest = db.create_quest(req).await.unwrap();

        let today = time::today();

        // Wrap in Arc so all concurrent tasks share the same database instance
        let db_arc = Arc::new(db);

        // Spawn multiple concurrent tasks trying to toggle the same quest
        let mut handles = vec![];
        for _ in 0..10 {
            let db_clone = db_arc.clone();
            let quest_id = quest.id;
            let date = today;

            let handle =
                tokio::spawn(async move { db_clone.toggle_quest_completion(quest_id, date).await });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap().is_ok() {
                success_count += 1;
            }
        }

        // All operations should succeed since completing is idempotent
        assert_eq!(
            success_count, 10,
            "All completion operations should succeed"
        );

        // Note: We don't check final completion state here because toggle is not
        // idempotent - with concurrent toggles the final state is non-deterministic
        // The important thing is that all operations succeeded without errors/race conditions
    }

    #[tokio::test]
    async fn test_concurrent_reward_claiming() {
        let db = setup_test_db().await;

        // Create reward requiring 10 EXP
        let reward_req = CreateRewardRequest {
            description: None,
            required_exp: 10,
            title: "Concurrent Reward".to_string(),
        };
        let reward = db.create_reward(reward_req).await.unwrap();

        // Create quest and complete it to get EXP
        let quest_req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(20),
            title: "EXP Quest".to_string(),
        };
        let quest = db.create_quest(quest_req).await.unwrap();

        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let today = week_start + chrono::Duration::days(2);
        db.toggle_quest_completion(quest.id, today).await.unwrap();

        // Spawn multiple concurrent tasks trying to claim the same reward
        let mut handles = vec![];
        for _ in 0..5 {
            let db_clone = Database::with_pool(db.pool().clone());
            let reward_id = reward.id;
            let week = week_start;

            let handle = tokio::spawn(async move { db_clone.claim_reward(reward_id, week).await });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut success_count = 0;
        let mut failure_count = 0;
        for handle in handles {
            match handle.await.unwrap() {
                Ok(result) => {
                    if result {
                        success_count += 1;
                    } else {
                        failure_count += 1;
                    }
                }
                Err(_) => failure_count += 1,
            }
        }

        // Exactly one should succeed, others should fail
        assert_eq!(success_count, 1, "Exactly one reward claim should succeed");
        assert_eq!(failure_count, 4, "Four reward claims should fail");

        // Verify reward was claimed
        let claimed_count = db.get_rewards_claimed_count().await.unwrap();
        assert_eq!(claimed_count, 1, "Reward should be claimed exactly once");
    }
}
