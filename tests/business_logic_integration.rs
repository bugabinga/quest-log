//! Integration tests for core business logic: quest creation, completion, and reward tracking.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]
use chrono::{Datelike, NaiveDate};
use quest_log::{database::Database, models::*, time};
use sqlx::SqlitePool;

#[tokio::test]
async fn test_business_logic_integration() {
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

    // Set today to Sunday to enable reward claiming
    let sunday = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();
    time::set_today(sunday);

    // Test reward claiming workflow

    // Should not be able to claim any rewards yet (need 50+ EXP for Bronze)
    let can_claim_bronze = db
        .claim_reward_for_week_on(r1.id, week_start, sunday)
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
        .claim_reward_for_week_on(r1.id, week_start, sunday)
        .await
        .expect("Failed to claim bronze again");
    assert!(
        !claim_again,
        "Should not be able to claim same reward twice"
    );

    // Should still be able to claim silver (100 EXP required, we have 110)
    let can_claim_silver = db
        .claim_reward_for_week_on(r2.id, week_start, sunday)
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
        .claim_reward_for_week_on(r3.id, week_start, sunday)
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
        .claim_reward_for_week_on(impossible_reward.id, week_start, sunday)
        .await
        .expect("Failed to check impossible reward");
    assert!(
        !can_claim_impossible,
        "Should not be able to claim reward requiring more EXP than earned"
    );

    // Test quest completion toggle (now bidirectional - can complete and un-complete)
    // First, ensure quest1 is in a known state by toggling it if already completed
    let was_already_completed = db
        .is_quest_completed_today(q1.id, monday)
        .await
        .expect("Failed to check quest completion status");

    // Toggle to get to a known state (uncompleted)
    if was_already_completed {
        db.toggle_quest_completion(q1.id, monday)
            .await
            .expect("Failed to un-complete quest");
    }

    // Now toggle to complete
    let was_completed = db
        .toggle_quest_completion(q1.id, monday)
        .await
        .expect("Failed to toggle quest 1 completion");
    assert_eq!(
        was_completed,
        ToggleResult::NewlyCompleted,
        "Quest should be newly completed"
    );

    let exp_after_toggle = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .expect("Failed to calculate EXP after toggle");
    assert_eq!(
        exp_after_toggle, 110,
        "Weekly EXP should be 110 (first quest completed)"
    );

    // Toggle again - should un-complete
    let completed_again = db
        .toggle_quest_completion(q1.id, monday)
        .await
        .expect("Failed to un-complete quest 1");
    assert_eq!(
        completed_again,
        ToggleResult::NewlyUncompleted,
        "Quest should be newly un-completed"
    );

    let final_exp = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .expect("Failed to calculate final EXP");
    assert_eq!(final_exp, 60, "Weekly EXP should be 60 after un-completing");

    // Toggle third time - should complete again
    let future_completion = db
        .toggle_quest_completion(q1.id, monday)
        .await
        .expect("Failed to complete quest again");
    assert_eq!(
        future_completion,
        ToggleResult::NewlyCompleted,
        "Quest should be newly completed again"
    );

    // Verify the completion is counted in total EXP
    let total_with_future = db
        .get_total_exp_earned()
        .await
        .expect("Failed to get total EXP after re-completing");
    assert_eq!(
        total_with_future, 110,
        "Total EXP should be 110 after re-completing quest1"
    );
}

// Test using set_today to simulate different days
#[tokio::test]
async fn test_todays_quests_with_set_today() {
    // Set today to a known Wednesday: Jan 3, 2024 is a Wednesday
    let wednesday = NaiveDate::from_ymd_opt(2024, 1, 3).unwrap();
    time::set_today(wednesday);

    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db: Database = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create quests for different days
    let monday_quest_req = CreateQuestRequest {
        title: "Monday Only Quest".to_string(),
        description: None,
        exp_value: Some(10),
        day_of_week: 1,
    };
    let wednesday_quest_req = CreateQuestRequest {
        title: "Wednesday Quest".to_string(),
        description: None,
        exp_value: Some(20),
        day_of_week: 3,
    };
    let any_day_quest_req = CreateQuestRequest {
        title: "Any Day Quest".to_string(),
        description: None,
        exp_value: Some(15),
        day_of_week: 0, // Sunday
    };

    let _monday_q = db
        .create_quest(monday_quest_req)
        .await
        .expect("Failed to create Monday quest");
    let wednesday_q = db
        .create_quest(wednesday_quest_req)
        .await
        .expect("Failed to create Wednesday quest");
    let _unused = db
        .create_quest(any_day_quest_req)
        .await
        .expect("Failed to create Sunday quest");

    // Use time::today() to get today's quests (now returns fake Wednesday)
    let today = time::today();
    let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

    // Verify we're on Wednesday
    assert_eq!(day_of_week, 3, "Should be Wednesday (3)");
    assert_eq!(today, wednesday);

    // Get today's quests using the handler pattern
    let todays_quests = db
        .get_quests_for_day(day_of_week)
        .await
        .expect("Failed to get today's quests");

    // Should only get Wednesday quest (day_of_week = 3)
    assert_eq!(todays_quests.len(), 1);
    assert_eq!(todays_quests[0].title, "Wednesday Quest");
    assert_eq!(todays_quests[0].exp_value, 20);

    // Now change to Monday and verify we get different quests
    let monday = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    time::set_today(monday);

    let today_monday = time::today();
    let monday_dow = today_monday.weekday().num_days_from_sunday().cast_signed();

    assert_eq!(monday_dow, 1, "Should be Monday (1)");

    let monday_quests = db
        .get_quests_for_day(monday_dow)
        .await
        .expect("Failed to get Monday quests");

    assert_eq!(monday_quests.len(), 1);
    assert_eq!(monday_quests[0].title, "Monday Only Quest");

    // Test completion tracking with fake today
    let wednesday_again = time::today();
    let was_completed = db
        .is_quest_completed_today(wednesday_q.id, wednesday_again)
        .await
        .expect("Failed to check completion");
    assert!(!was_completed, "Should not be completed yet");

    // Complete the quest (using fake today)
    db.toggle_quest_completion(wednesday_q.id, wednesday_again)
        .await
        .expect("Failed to complete quest");

    // Verify it's now completed
    let is_completed = db
        .is_quest_completed_today(wednesday_q.id, wednesday_again)
        .await
        .expect("Failed to check completion after toggle");
    assert!(is_completed, "Quest should be completed after toggle");

    println!("set_today test completed - successfully simulated different days");

    // Cleanup
    time::reset_today();
}
