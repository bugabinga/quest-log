use chrono::NaiveDate;
use quest_log::{database::Database, models::*};
use sqlx::SqlitePool;

#[tokio::test]
async fn test_phase3_business_logic_integration() {
    // Setup test database with in-memory SQLite
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Test complex business logic scenarios

    // Create test data - quests with different EXP values
    let quest1 = CreateQuestRequest {
        title: "High EXP Quest".to_string(),
        description: Some("Worth 50 EXP".to_string()),
        exp_value: Some(50),
        day_of_week: 1, // Monday
    };
    let quest2 = CreateQuestRequest {
        title: "Medium EXP Quest".to_string(),
        description: Some("Worth 25 EXP".to_string()),
        exp_value: Some(25),
        day_of_week: 2, // Tuesday
    };
    let quest3 = CreateQuestRequest {
        title: "Low EXP Quest".to_string(),
        description: Some("Worth 10 EXP".to_string()),
        exp_value: Some(10),
        day_of_week: 3, // Wednesday
    };

    let q1 = db
        .create_quest(quest1)
        .await
        .expect("Failed to create quest 1");
    let q2 = db
        .create_quest(quest2)
        .await
        .expect("Failed to create quest 2");
    let q3 = db
        .create_quest(quest3)
        .await
        .expect("Failed to create quest 3");

    // Create rewards with different EXP requirements
    let reward1 = CreateRewardRequest {
        title: "Bronze Reward".to_string(),
        description: Some("50 EXP reward".to_string()),
        required_exp: 50,
    };
    let reward2 = CreateRewardRequest {
        title: "Silver Reward".to_string(),
        description: Some("100 EXP reward".to_string()),
        required_exp: 100,
    };
    let reward3 = CreateRewardRequest {
        title: "Gold Reward".to_string(),
        description: Some("200 EXP reward".to_string()),
        required_exp: 200,
    };

    let r1 = db
        .create_reward(reward1)
        .await
        .expect("Failed to create reward 1");
    let r2 = db
        .create_reward(reward2)
        .await
        .expect("Failed to create reward 2");
    let r3 = db
        .create_reward(reward3)
        .await
        .expect("Failed to create reward 3");

    // Test weekly EXP calculation - no completions yet
    let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // Monday
    let week_end = week_start + chrono::Duration::days(6);
    let initial_exp = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .expect("Failed to calculate initial EXP");
    assert_eq!(initial_exp, 0, "Initial weekly EXP should be 0");

    // Complete quests throughout the week
    let monday = week_start;
    let tuesday = week_start + chrono::Duration::days(1);
    let wednesday = week_start + chrono::Duration::days(2);
    let thursday = week_start + chrono::Duration::days(3);

    // Monday: Complete high EXP quest (50 EXP)
    db.toggle_quest_completion(q1.id, monday)
        .await
        .expect("Failed to complete quest 1");

    // Tuesday: Complete medium EXP quest (25 EXP)
    db.toggle_quest_completion(q2.id, tuesday)
        .await
        .expect("Failed to complete quest 2");

    // Wednesday: Complete low EXP quest (10 EXP)
    db.toggle_quest_completion(q3.id, wednesday)
        .await
        .expect("Failed to complete quest 3");

    // Thursday: Complete medium quest again (25 EXP) - same quest, different day
    db.toggle_quest_completion(q2.id, thursday)
        .await
        .expect("Failed to complete quest 2 again");

    // Calculate weekly EXP after completions
    let weekly_exp = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .expect("Failed to calculate weekly EXP");
    assert_eq!(
        weekly_exp, 110,
        "Weekly EXP should be 50 + 25 + 10 + 25 = 110"
    );

    // Test reward claiming workflow

    // Should not be able to claim any rewards yet (need 50+ EXP for Bronze)
    let can_claim_bronze = db
        .claim_reward(r1.id, week_start)
        .await
        .expect("Failed to check bronze reward claim");
    assert!(
        can_claim_bronze,
        "Should be able to claim Bronze reward with 110 EXP"
    );

    // Check that bronze reward was claimed
    let claimed_count = db
        .get_rewards_claimed_count()
        .await
        .expect("Failed to get claimed count");
    assert_eq!(claimed_count, 1, "Should have 1 reward claimed");

    // Try to claim bronze again - should fail
    let claim_again = db
        .claim_reward(r1.id, week_start)
        .await
        .expect("Failed to claim bronze again");
    assert!(
        !claim_again,
        "Should not be able to claim same reward twice"
    );

    // Should still be able to claim silver (100 EXP required, we have 110)
    let can_claim_silver = db
        .claim_reward(r2.id, week_start)
        .await
        .expect("Failed to check silver reward claim");
    assert!(
        can_claim_silver,
        "Should be able to claim Silver reward with 110 EXP"
    );

    let claimed_count_after_silver = db
        .get_rewards_claimed_count()
        .await
        .expect("Failed to get claimed count after silver");
    assert_eq!(
        claimed_count_after_silver, 2,
        "Should have 2 rewards claimed"
    );

    // Should not be able to claim gold (200 EXP required, we only have 110)
    let can_claim_gold = db
        .claim_reward(r3.id, week_start)
        .await
        .expect("Failed to check gold reward claim");
    assert!(
        !can_claim_gold,
        "Should not be able to claim Gold reward with only 110 EXP"
    );

    // Test total EXP earned calculation
    let total_exp = db
        .get_total_exp_earned()
        .await
        .expect("Failed to get total EXP earned");
    assert_eq!(total_exp, 110, "Total EXP earned should be 110");

    // Test edge cases

    // Try to claim reward with insufficient EXP
    let insufficient_reward = CreateRewardRequest {
        title: "Impossible Reward".to_string(),
        description: Some("Requires 1000 EXP".to_string()),
        required_exp: 1000,
    };
    let impossible_reward = db
        .create_reward(insufficient_reward)
        .await
        .expect("Failed to create impossible reward");
    let can_claim_impossible = db
        .claim_reward(impossible_reward.id, week_start)
        .await
        .expect("Failed to check impossible reward");
    assert!(
        !can_claim_impossible,
        "Should not be able to claim reward requiring more EXP than earned"
    );

    // Test quest completion toggle (uncomplete and complete again)
    let was_completed = db
        .toggle_quest_completion(q1.id, monday)
        .await
        .expect("Failed to toggle quest 1 completion");
    assert!(!was_completed, "Quest should now be incomplete");

    let exp_after_toggle = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .expect("Failed to calculate EXP after toggle");
    assert_eq!(
        exp_after_toggle, 60,
        "Weekly EXP should be reduced by 50 (110 - 50 = 60)"
    );

    // Complete it again
    let completed_again = db
        .toggle_quest_completion(q1.id, monday)
        .await
        .expect("Failed to complete quest 1 again");
    assert!(completed_again, "Quest should now be completed again");

    let final_exp = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .expect("Failed to calculate final EXP");
    assert_eq!(final_exp, 110, "Weekly EXP should be back to 110");

    // Test business logic validation - completing quest on wrong day should still work
    // (The database allows any date, business logic validation happens at application level)
    let future_date = week_start + chrono::Duration::days(30);
    let future_completion = db
        .toggle_quest_completion(q1.id, future_date)
        .await
        .expect("Failed to complete quest in future");
    assert!(
        future_completion,
        "Should be able to complete quest on any date"
    );

    // Verify the future completion is counted in total EXP
    let total_with_future = db
        .get_total_exp_earned()
        .await
        .expect("Failed to get total EXP with future completion");
    assert_eq!(
        total_with_future, 160,
        "Total EXP should include future completion (110 + 50 = 160)"
    );
}
