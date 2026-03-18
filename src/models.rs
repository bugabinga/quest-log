use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Quest model representing a daily quest that players can complete.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Quest {
    /// Timestamp when the quest was created.
    pub created_at: DateTime<Utc>,
    /// Day of the week (0=Sunday, 6=Saturday).
    pub day_of_week: i32,
    /// Optional description providing quest details.
    pub description: Option<String>,
    /// Experience points awarded for completing the quest.
    pub exp_value: i32,
    /// Unique identifier for the quest.
    pub id: i64,
    /// MIME content type of the quest image (if any).
    pub image_content_type: Option<String>,
    /// Binary image data for the quest (if any).
    pub image_data: Option<Vec<u8>>,
    /// Whether the quest is currently active and available.
    pub is_active: bool,
    /// Title of the quest.
    pub title: String,
    /// Timestamp when the quest was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Quest completion tracking - records when a quest was completed on a specific date.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct QuestCompletion {
    /// The date when the quest was completed.
    pub completed_date: NaiveDate,
    /// Timestamp when the completion record was created.
    pub created_at: DateTime<Utc>,
    /// Unique identifier for the completion record.
    pub id: i64,
    /// Reference to the completed quest.
    pub quest_id: i64,
}

/// Reward model representing an unlockable reward based on experience points.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Reward {
    /// Timestamp when the reward was created.
    pub created_at: DateTime<Utc>,
    /// Optional description of the reward.
    pub description: Option<String>,
    /// Unique identifier for the reward.
    pub id: i64,
    /// MIME content type of the reward image (if any).
    pub image_content_type: Option<String>,
    /// Binary image data for the reward (if any).
    pub image_data: Option<Vec<u8>>,
    /// Whether the reward is currently active and claimable.
    pub is_active: bool,
    /// Experience points required to claim this reward.
    pub required_exp: i32,
    /// Title of the reward.
    pub title: String,
    /// Timestamp when the reward was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Application settings stored as a single row in the database.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Settings {
    /// Always 1 - single row table identifier.
    pub id: i32,
    /// Timestamp when settings were last updated.
    pub updated_at: DateTime<Utc>,
    /// Weekly experience goal for players.
    pub weekly_exp_goal: i32,
}

/// Request payload for creating a new quest.
#[derive(Debug, Deserialize)]
pub struct CreateQuestRequest {
    /// Day of the week (0=Sunday, 6=Saturday).
    pub day_of_week: i32,
    /// Optional description for the quest.
    pub description: Option<String>,
    /// Optional experience value (defaults to a standard value).
    pub exp_value: Option<i32>,
    /// Title of the quest.
    pub title: String,
}

/// Request payload for updating an existing quest.
#[derive(Debug, Deserialize)]
pub struct UpdateQuestRequest {
    /// Updated day of the week.
    pub day_of_week: Option<i32>,
    /// Updated description.
    pub description: Option<String>,
    /// Updated experience value.
    pub exp_value: Option<i32>,
    /// Updated active status.
    pub is_active: Option<bool>,
    /// Updated title.
    pub title: Option<String>,
}

/// Request payload for creating a new reward.
#[derive(Debug, Deserialize)]
pub struct CreateRewardRequest {
    /// Optional description for the reward.
    pub description: Option<String>,
    /// Experience points required to claim the reward.
    pub required_exp: i32,
    /// Title of the reward.
    pub title: String,
}

/// Request payload for updating an existing reward.
#[derive(Debug, Deserialize)]
pub struct UpdateRewardRequest {
    /// Updated description.
    pub description: Option<String>,
    /// Updated active status.
    pub is_active: Option<bool>,
    /// Updated required experience points.
    pub required_exp: Option<i32>,
    /// Updated title.
    pub title: Option<String>,
}

/// Request payload for updating application settings.
#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    /// New weekly experience goal.
    pub weekly_exp_goal: i32,
}

/// Result of toggling a quest completion status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleResult {
    /// Quest was previously incomplete and is now completed.
    NewlyCompleted,
    /// Quest was previously completed and is now incomplete.
    NewlyUncompleted,
    /// Quest status did not change (already in requested state).
    NoChange,
}

/// Claim state of a weekly reward.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ClaimState {
    /// Reward can be claimed with current experience.
    Claimable,
    /// Reward has already been claimed.
    Claimed,
    /// Reward cannot be claimed yet (insufficient experience).
    Locked,
}

/// Display model for a weekly reward with current claim status.
#[derive(Debug, Clone, Serialize)]
pub struct WeeklyRewardDisplay {
    /// Whether the reward can be claimed today.
    pub can_claim_today: bool,
    /// Description of the reward.
    pub description: Option<String>,
    /// Unique identifier for the reward.
    pub id: i64,
    /// Experience points required to claim.
    pub required_exp: i32,
    /// Current claim state.
    pub state: ClaimState,
    /// Title of the reward.
    pub title: String,
    /// Player's current weekly experience total.
    pub weekly_exp: i32,
}

/// Statistics about player quest progress.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct QuestStats {
    /// Experience earned today.
    pub exp_today: i32,
    /// Maximum experience available today.
    pub exp_today_max: i32,
    /// Number of quests completed today.
    pub quests_completed: i32,
    /// Total number of active quests.
    pub quests_total: i32,
    /// Experience earned this week.
    pub week_exp: i32,
    /// Maximum experience available this week.
    pub week_exp_max: i32,
}

/// Weekly champion - player with highest experience in a given week.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct WeeklyChampion {
    /// Timestamp when the champion title was earned.
    pub earned_at: DateTime<Utc>,
    /// Unique identifier for the champion record.
    pub id: i64,
    /// Start date of the week.
    pub week_start: NaiveDate,
}
