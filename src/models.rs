use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Quest model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Quest {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub exp_value: i32,
    pub day_of_week: i32, // 0=Sunday, 6=Saturday
    pub image_data: Option<Vec<u8>>,
    pub image_content_type: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Quest completion tracking
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct QuestCompletion {
    pub id: i64,
    pub quest_id: i64,
    pub completed_date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

// Reward model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Reward {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub required_exp: i32,
    pub image_data: Option<Vec<u8>>,
    pub image_content_type: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Reward claim tracking
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RewardClaim {
    pub id: i64,
    pub reward_id: i64,
    pub claimed_date: NaiveDate,
    pub created_at: DateTime<Utc>,
}

// Application settings (single row table)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Settings {
    pub id: i32, // Always 1
    pub weekly_exp_goal: i32,
    pub updated_at: DateTime<Utc>,
}

// Data transfer objects for API requests
#[derive(Debug, Deserialize)]
pub struct CreateQuestRequest {
    pub title: String,
    pub description: Option<String>,
    pub exp_value: Option<i32>,
    pub day_of_week: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQuestRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub exp_value: Option<i32>,
    pub day_of_week: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRewardRequest {
    pub title: String,
    pub description: Option<String>,
    pub required_exp: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRewardRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub required_exp: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub weekly_exp_goal: i32,
}

// Response types
#[derive(Debug, Serialize)]
pub struct QuestWithCompletion {
    #[serde(flatten)]
    pub quest: Quest,
    pub completed_today: bool,
}

#[derive(Debug, Serialize)]
pub struct WeeklyStats {
    pub total_exp: i32,
    pub weekly_goal: i32,
    pub completion_percentage: f32,
    pub rewards_claimed: i32,
    pub available_rewards: Vec<Reward>,
}
