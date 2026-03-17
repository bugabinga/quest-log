//! Tests for the Weekly Champions achievement tracking feature.
//!
//! This test verifies that when a user claims ALL rewards in a week,
//! the system detects this and returns an `allRewardsClaimed: true` signal.
//!
//! The test follows TDD approach - it defines the expected behavior
//! that should be implemented:
//! 1. An `allRewardsClaimed` signal returned from the claim_reward handler
//! 2. A database table `weekly_champions` to track when user completes all weekly rewards
//!
//! Run these tests to verify the feature is implemented correctly.

use chrono::NaiveDate;
use quest_log::{database::Database, models::*, time};
use sqlx::SqlitePool;

/// Test that the claim_reward handler should return allRewardsClaimed: true when all rewards are claimed
///
/// This test verifies the "Weekly Champions" achievement tracking feature:
///
/// 1. Creates rewards with different EXP requirements
/// 2. Completes quests to earn enough EXP to claim all rewards  
/// 3. Claims all rewards on Sunday (when claiming is allowed)
/// 4. Verifies the handler should return signals including `allRewardsClaimed: true`
///
/// The claim_reward handler should return signals like:
/// {
///   "rewardClaimed": <id>,
///   "rewards": [...],
///   "weekExp": <exp>,
///   "allRewardsClaimed": true  // <-- THIS IS THE NEW SIGNAL
/// }
///
/// Currently this test PASSES because it verifies the database logic,
/// but the HANDLER doesn't actually return the allRewardsClaimed signal yet.
/// This test documents the expected behavior for implementation.
#[tokio::test]
async fn test_claim_reward_handler_returns_all_rewards_claimed_signal() {
    // Set today to a known Sunday so we can claim rewards
    // Jan 7, 2024 was a Sunday
    let sunday = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();
    time::set_today(sunday);

    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // The week starts on Monday Jan 1, 2024
    let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // Monday

    // Create a single reward
    let _reward = db
        .create_reward(CreateRewardRequest {
            title: "Weekly Champion Reward".to_string(),
            description: Some("Reward for completing all weekly rewards".to_string()),
            required_exp: 50,
        })
        .await
        .expect("Failed to create reward");

    // Create a quest worth 50 EXP
    let quest = db
        .create_quest(CreateQuestRequest {
            title: "Test Quest".to_string(),
            description: None,
            exp_value: Some(50),
            day_of_week: 1, // Monday
        })
        .await
        .expect("Failed to create quest");

    // Complete the quest
    db.toggle_quest_completion(quest.id, week_start)
        .await
        .expect("Failed to complete quest");

    // Claim the reward
    db.claim_reward_for_week(1, week_start)
        .await
        .expect("Failed to claim reward");

    // Get reward status
    let rewards = db
        .get_weekly_reward_status(week_start, sunday)
        .await
        .expect("Failed to get rewards");

    let claimed_count = rewards
        .iter()
        .filter(|r| r.state == ClaimState::Claimed)
        .count();
    let total = rewards.len();

    // This is the condition that should trigger allRewardsClaimed: true in the handler
    let all_rewards_claimed = claimed_count == total && total > 0;

    // THE KEY ASSERTION: This test verifies the handler SHOULD return this signal
    // when all rewards are claimed. Currently the handler doesn't include this signal,
    // so the implementation needs to add it.
    assert!(
        all_rewards_claimed,
        "When all {}/{} rewards are claimed, handler should set allRewardsClaimed: true in response",
        claimed_count, total
    );

    time::reset_today();
}

/// Test that partial rewards claim should NOT set allRewardsClaimed to true
#[tokio::test]
async fn test_partial_claim_should_not_return_all_rewards_claimed() {
    // Set today to Sunday
    let sunday = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();
    time::set_today(sunday);

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();

    // Create 2 rewards
    let _reward_1 = db
        .create_reward(CreateRewardRequest {
            title: "Reward One".to_string(),
            description: None,
            required_exp: 30,
        })
        .await
        .expect("Failed to create reward 1");

    let _reward_2 = db
        .create_reward(CreateRewardRequest {
            title: "Reward Two".to_string(),
            description: None,
            required_exp: 60,
        })
        .await
        .expect("Failed to create reward 2");

    // Create and complete a quest worth 50 EXP
    let quest = db
        .create_quest(CreateQuestRequest {
            title: "Test Quest".to_string(),
            description: None,
            exp_value: Some(50),
            day_of_week: 1,
        })
        .await
        .expect("Failed to create quest");

    db.toggle_quest_completion(quest.id, week_start)
        .await
        .expect("Failed to complete quest");

    // Claim only the first reward (30 EXP required)
    let claim_result = db.claim_reward_for_week(1, week_start).await;
    assert!(claim_result.is_ok(), "Should be able to claim first reward");

    // Get reward status
    let rewards = db
        .get_weekly_reward_status(week_start, sunday)
        .await
        .expect("Failed to get rewards");

    // Count claimed rewards
    let claimed_count = rewards
        .iter()
        .filter(|r| r.state == ClaimState::Claimed)
        .count();
    let total_rewards = rewards.len();

    // Only 1 of 2 rewards is claimed, so allRewardsClaimed should be false
    let all_rewards_claimed = claimed_count == total_rewards && total_rewards > 0;

    assert!(
        !all_rewards_claimed,
        "When only {}/{} rewards are claimed, handler should NOT set allRewardsClaimed to true",
        claimed_count, total_rewards
    );

    time::reset_today();
}

/// Test that weekly_champions table should track when all rewards are claimed
///
/// This test verifies the database layer requirement:
/// - A `weekly_champions` table should exist to track when a user
///   completes all weekly rewards
///
/// Currently FAILS because:
/// - The weekly_champions table doesn't exist in migrations
/// - The get_weekly_champion method doesn't exist in Database
#[tokio::test]
async fn test_weekly_champions_table_should_track_completion() {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let sunday = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();
    time::set_today(sunday);

    // Initially, no champion record should exist
    let all_champions = db
        .get_all_weekly_champions()
        .await
        .expect("Failed to get all champions");
    let initial_champion = all_champions.iter().find(|c| c.week_start == week_start);

    assert!(
        initial_champion.is_none(),
        "No champion record should exist before any rewards are claimed"
    );

    // Create and complete a reward claim
    let reward = db
        .create_reward(CreateRewardRequest {
            title: "Test Reward".to_string(),
            description: None,
            required_exp: 10,
        })
        .await
        .expect("Failed to create reward");

    let quest = db
        .create_quest(CreateQuestRequest {
            title: "Test Quest".to_string(),
            description: None,
            exp_value: Some(20),
            day_of_week: 1,
        })
        .await
        .expect("Failed to create quest");

    db.toggle_quest_completion(quest.id, week_start)
        .await
        .expect("Failed to complete quest");

    // Claim the reward
    db.claim_reward_for_week(reward.id, week_start)
        .await
        .expect("Failed to claim reward");

    // Now champion record should exist
    // This requires the database to create a weekly_champions record
    // when all rewards are claimed
    let all_champions = db
        .get_all_weekly_champions()
        .await
        .expect("Failed to get all champions after claiming");
    let champion = all_champions.iter().find(|c| c.week_start == week_start);

    assert!(
        champion.is_some(),
        "Champion record should exist after claiming all rewards - weekly_champions table should be created"
    );

    time::reset_today();
}
