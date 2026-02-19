//! Common test utilities and fixtures for Quest Log tests

use chrono::{NaiveDate, Utc};
use quest_log::{
    database::Database,
    models::{CreateQuestRequest, CreateRewardRequest, Quest, Reward},
};
use sqlx::SqlitePool;

/// Setup a test database with migrations applied
pub async fn setup_test_db() -> Database {
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    Database::with_pool(pool)
}

/// Create a test quest with default values
pub async fn create_test_quest(db: &Database, overrides: TestQuestOverrides) -> Quest {
    let req = CreateQuestRequest {
        title: overrides.title.unwrap_or_else(|| "Test Quest".to_string()),
        description: overrides.description,
        exp_value: Some(overrides.exp_value.unwrap_or(10)),
        day_of_week: overrides.day_of_week.unwrap_or(1),
    };
    db.create_quest(req)
        .await
        .expect("Failed to create test quest")
}

/// Create a test reward with default values
pub async fn create_test_reward(db: &Database, overrides: TestRewardOverrides) -> Reward {
    let req = CreateRewardRequest {
        title: overrides.title.unwrap_or_else(|| "Test Reward".to_string()),
        description: overrides.description,
        required_exp: overrides.required_exp.unwrap_or(50),
    };
    db.create_reward(req)
        .await
        .expect("Failed to create test reward")
}

/// Test quest creation overrides
#[derive(Default)]
pub struct TestQuestOverrides {
    pub title: Option<String>,
    pub description: Option<String>,
    pub exp_value: Option<i32>,
    pub day_of_week: Option<i32>,
}

/// Test reward creation overrides
#[derive(Default)]
pub struct TestRewardOverrides {
    pub title: Option<String>,
    pub description: Option<String>,
    pub required_exp: Option<i32>,
}

/// Generate a sequence of test dates for the current week
pub struct TestWeek {
    pub monday: NaiveDate,
    pub tuesday: NaiveDate,
    pub wednesday: NaiveDate,
    pub thursday: NaiveDate,
    pub friday: NaiveDate,
    pub saturday: NaiveDate,
    pub sunday: NaiveDate,
}

impl TestWeek {
    /// Create a test week starting from a known Monday
    pub fn new() -> Self {
        let monday = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // Known Monday
        Self {
            monday,
            tuesday: monday + chrono::Duration::days(1),
            wednesday: monday + chrono::Duration::days(2),
            thursday: monday + chrono::Duration::days(3),
            friday: monday + chrono::Duration::days(4),
            saturday: monday + chrono::Duration::days(5),
            sunday: monday + chrono::Duration::days(6),
        }
    }

    /// Get all dates in the week as a vector
    pub fn all_dates(&self) -> Vec<NaiveDate> {
        vec![
            self.monday,
            self.tuesday,
            self.wednesday,
            self.thursday,
            self.friday,
            self.saturday,
            self.sunday,
        ]
    }
}

/// Assert that a quest has expected properties
pub fn assert_quest_properties(
    quest: &Quest,
    expected_title: &str,
    expected_exp: i32,
    expected_day: i32,
) {
    assert_eq!(quest.title, expected_title);
    assert_eq!(quest.exp_value, expected_exp);
    assert_eq!(quest.day_of_week, expected_day);
    assert!(quest.created_at <= Utc::now());
    assert!(quest.updated_at <= Utc::now());
}

/// Assert that a reward has expected properties
pub fn assert_reward_properties(reward: &Reward, expected_title: &str, expected_exp: i32) {
    assert_eq!(reward.title, expected_title);
    assert_eq!(reward.required_exp, expected_exp);
    assert!(reward.created_at <= Utc::now());
}

/// Common test data for consistent testing
pub mod test_data {
    use super::*;

    pub const VALID_QUEST_TITLES: &[&str] = &[
        "Morning Exercise",
        "Read for 30 minutes",
        "Help with chores",
        "Practice instrument",
        "Learn something new",
    ];

    pub const VALID_EXP_VALUES: &[i32] = &[5, 10, 15, 20, 25, 50, 100];

    pub const VALID_DAYS_OF_WEEK: &[i32] = &[0, 1, 2, 3, 4, 5, 6]; // Sunday = 0, Saturday = 6

    pub const INVALID_DAYS_OF_WEEK: &[i32] = &[-1, 7, 10, 100];

    pub const XSS_ATTEMPTS: &[&str] = &[
        "<script>alert('xss')</script>",
        "<img src=x onerror=alert('xss')>",
        "javascript:alert('xss')",
        "<iframe src='javascript:alert(\"xss\")'>",
    ];

    pub const SQL_INJECTION_ATTEMPTS: &[&str] = &[
        "'; DROP TABLE quests; --",
        "' OR '1'='1",
        "'; SELECT * FROM settings; --",
        "admin'--",
    ];
}
