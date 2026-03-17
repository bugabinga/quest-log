use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// Quest model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Quest {
    pub created_at: DateTime<Utc>,
    pub day_of_week: i32, // 0=Sunday, 6=Saturday
    pub description: Option<String>,
    pub exp_value: i32,
    pub id: i64,
    pub image_content_type: Option<String>,
    pub image_data: Option<Vec<u8>>,
    pub is_active: bool,
    pub title: String,
    pub updated_at: DateTime<Utc>,
}

// Quest completion tracking
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct QuestCompletion {
    pub completed_date: NaiveDate,
    pub created_at: DateTime<Utc>,
    pub id: i64,
    pub quest_id: i64,
}

// Reward model
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Reward {
    pub created_at: DateTime<Utc>,
    pub description: Option<String>,
    pub id: i64,
    pub image_content_type: Option<String>,
    pub image_data: Option<Vec<u8>>,
    pub is_active: bool,
    pub required_exp: i32,
    pub title: String,
    pub updated_at: DateTime<Utc>,
}

// Application settings (single row table)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Settings {
    pub id: i32, // Always 1
    pub updated_at: DateTime<Utc>,
    pub weekly_exp_goal: i32,
}

// Data transfer objects for API requests
#[derive(Debug, Deserialize)]
pub struct CreateQuestRequest {
    pub day_of_week: i32,
    pub description: Option<String>,
    pub exp_value: Option<i32>,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQuestRequest {
    pub day_of_week: Option<i32>,
    pub description: Option<String>,
    pub exp_value: Option<i32>,
    pub is_active: Option<bool>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRewardRequest {
    pub description: Option<String>,
    pub required_exp: i32,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRewardRequest {
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub required_exp: Option<i32>,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub weekly_exp_goal: i32,
}

// Response types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleResult {
    NewlyCompleted,
    NewlyUncompleted,
    NoChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ClaimState {
    Claimable,
    Claimed,
    Locked,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyRewardDisplay {
    pub can_claim_today: bool,
    pub description: Option<String>,
    pub id: i64,
    pub required_exp: i32,
    pub state: ClaimState,
    pub title: String,
    pub weekly_exp: i32,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct QuestStats {
    pub exp_today: i32,
    pub exp_today_max: i32,
    pub quests_completed: i32,
    pub quests_total: i32,
    pub week_exp: i32,
    pub week_exp_max: i32,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WeeklyChampion {
    pub earned_at: DateTime<Utc>,
    pub id: i64,
    pub week_start: NaiveDate,
}
