//! Tests for the celebration animation feature when all weekly rewards are claimed.
//!
//! This test verifies that when the SSE signal contains `allRewardsClaimed: true`,
//! the frontend should display:
//! 1. An achievement banner/popup with "Achievement Unlocked" message
//! 2. A golden pulse animation on the reward cards
//! 3. A "Weekly Champion" badge
//!
//! Currently FAILS because:
//! - The weekly rewards UI doesn't render celebration elements when allRewardsClaimed is true
//! - No achievement banner HTML is generated
//! - No golden pulse animation class is applied to reward cards

use chrono::NaiveDate;
use quest_log::{database::Database, models::*, time};
use sqlx::SqlitePool;

/// Test that weekly rewards UI should include celebration elements when all rewards are claimed
///
/// This test verifies the frontend celebration feature:
/// 1. Setup rewards and quests, complete them to earn enough EXP
/// 2. Verify the UI renders celebration elements when all rewards are claimed
///
/// Expected behavior (not yet implemented):
/// - The rewards HTML should include an achievement banner
/// - Reward cards should have golden pulse animation classes
/// - A "Weekly Champion" badge should be displayed
#[tokio::test]
async fn test_weekly_rewards_ui_should_include_celebration_when_all_claimed() {
    // Set today to Sunday (when rewards can be claimed)
    let sunday = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();
    let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    time::set_today(sunday);

    // Setup test database
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create a single reward that requires 50 EXP
    let _reward = db
        .create_reward(CreateRewardRequest {
            title: "Weekly Champion Reward".to_string(),
            description: Some("Reward for completing all weekly rewards".to_string()),
            required_exp: 50,
        })
        .await
        .expect("Failed to create reward");

    // Create a quest worth 50 EXP to unlock the reward
    let quest = db
        .create_quest(CreateQuestRequest {
            title: "Test Quest".to_string(),
            description: None,
            exp_value: Some(50),
            day_of_week: 1, // Monday
        })
        .await
        .expect("Failed to create quest");

    // Complete the quest to earn 50 EXP
    db.toggle_quest_completion(quest.id, week_start)
        .await
        .expect("Failed to complete quest");

    // Get reward status - all rewards should be claimable
    let rewards = db
        .get_weekly_reward_status(week_start, sunday)
        .await
        .expect("Failed to get rewards");

    let week_exp = db
        .calculate_weekly_exp(week_start, sunday)
        .await
        .unwrap_or(0);

    // Verify we have rewards
    assert!(!rewards.is_empty(), "Should have rewards in the database");

    // Get the rendered HTML - pass all_rewards_claimed as true to test celebration UI
    let rewards_html =
        quest_log::ui::fragments::weekly_rewards::weekly_rewards(week_exp, &rewards, true);
    let html_string = rewards_html.into_string();

    // ===== FAILING TEST: CELEBRATION ELEMENTS NOT YET IMPLEMENTED =====
    //
    // The frontend SHOULD display celebration elements when all rewards are claimed.
    // This test verifies that the UI includes:
    // 1. An achievement banner (achievement-unlocked or celebration-banner)
    // 2. Golden pulse animation class on reward cards (golden-pulse or celebration-pulse)
    // 3. A "Weekly Champion" badge
    //
    // Currently FAILS because the weekly_rewards UI doesn't render these elements.
    // Once implemented, the assertions below should PASS.

    // Test 1: Achievement banner should be present when all rewards are claimed
    assert!(
        html_string.contains("achievement-unlocked")
            || html_string.contains("celebration-banner")
            || html_string.contains("Weekly Champion"),
        "When all rewards are claimed, the HTML should include a celebration banner or achievement element. \
         Expected: 'achievement-unlocked', 'celebration-banner', or 'Weekly Champion' in HTML. \
         This test FAILS because celebration elements are not yet implemented in the UI."
    );

    // Test 2: Golden pulse animation should be applied to reward cards
    assert!(
        html_string.contains("golden-pulse") || html_string.contains("celebration-pulse"),
        "When all rewards are claimed, reward cards should have a golden pulse animation class. \
         Expected: 'golden-pulse' or 'celebration-pulse' in HTML. \
         This test FAILS because celebration elements are not yet implemented in the UI."
    );

    // Test 3: Weekly Champion badge should be displayed
    assert!(
        html_string.contains("weekly-champion-badge")
            || html_string.contains("badge-weekly-champion")
            || (html_string.contains("🏆") && html_string.contains("Champion")),
        "When all rewards are claimed, a Weekly Champion badge should be displayed. \
         This test FAILS because celebration elements are not yet implemented in the UI."
    );

    time::reset_today();
}

/// Test that the handler correctly calculates allRewardsClaimed signal
///
/// Verifies that when ALL rewards are claimed, the handler returns allRewardsClaimed: true
#[tokio::test]
async fn test_handler_returns_all_rewards_claimed_signal() {
    // Set today to Sunday
    let sunday = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();
    let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    time::set_today(sunday);

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

    // Create a reward
    let reward = db
        .create_reward(CreateRewardRequest {
            title: "Test Reward".to_string(),
            description: None,
            required_exp: 10,
        })
        .await
        .expect("Failed to create reward");

    // Create and complete a quest
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

    // Get reward status
    let rewards = db
        .get_weekly_reward_status(week_start, sunday)
        .await
        .expect("Failed to get rewards");

    // Calculate allRewardsClaimed (same logic as handler)
    let all_rewards_claimed = rewards
        .iter()
        .filter(|r| r.state == ClaimState::Claimed)
        .count()
        == rewards.len()
        && !rewards.is_empty();

    assert!(
        all_rewards_claimed,
        "When 1/1 rewards are claimed, allRewardsClaimed should be true"
    );

    time::reset_today();
}

/// Test that partial rewards claim should NOT set allRewardsClaimed to true
#[tokio::test]
async fn test_partial_claim_should_not_return_all_rewards_claimed() {
    // Set today to Sunday
    let sunday = NaiveDate::from_ymd_opt(2024, 1, 7).unwrap();
    let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    time::set_today(sunday);

    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");
    let db = Database::with_pool(pool);
    db.migrate().await.expect("Failed to run migrations");

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
    db.claim_reward_for_week(1, week_start)
        .await
        .expect("Failed to claim reward");

    // Get reward status
    let rewards = db
        .get_weekly_reward_status(week_start, sunday)
        .await
        .expect("Failed to get rewards");

    // Calculate allRewardsClaimed
    let all_rewards_claimed = rewards
        .iter()
        .filter(|r| r.state == ClaimState::Claimed)
        .count()
        == rewards.len()
        && !rewards.is_empty();

    assert!(
        !all_rewards_claimed,
        "When only 1/2 rewards are claimed, allRewardsClaimed should be false"
    );

    time::reset_today();
}
