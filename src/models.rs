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

/// Maximum file size for uploads (5MB decoded = ~6.7MB base64 encoded)
pub const MAX_FILE_SIZE: usize = 7_000_000;

/// File upload from Datastar (base64 encoded data URL)
/// Format: [{ name: string, contents: string, type: string }]
/// where contents is a data URL like "data:image/png;base64,..."
#[derive(Debug, Clone, Deserialize)]
pub struct FileUpload {
    /// Original filename
    pub name: String,
    /// Base64 encoded file contents (data URL format)
    pub contents: String,
    /// MIME type from data URL
    #[serde(rename = "type")]
    pub mime: String,
}

use base64::Engine;

impl FileUpload {
    /// Decode base64 data URL to raw bytes
    /// Returns (bytes, `mime_type`) or None if invalid
    #[must_use]
    pub fn decode(&self) -> Option<(Vec<u8>, String)> {
        let base64_start = self.contents.find(";base64,")?;
        let encoded = &self.contents[base64_start + 8..];
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .ok()
            .map(|bytes| (bytes, self.mime.clone()))
    }

    /// Check if file exceeds size limit
    #[must_use]
    pub fn is_too_large(&self) -> bool {
        self.contents.len() > MAX_FILE_SIZE
    }

    /// Get original filename for logging/audit
    #[must_use]
    pub fn filename(&self) -> &str {
        &self.name
    }
}

/// Request payload for creating/updating quest via Datastar JSON signals
#[derive(Debug, Deserialize)]
#[allow(
    clippy::struct_field_names,
    reason = "Frontend uses camelCase, Rust uses snake_case"
)]
pub struct QuestJsonRequest {
    /// Quest title from form
    #[serde(rename = "questTitle")]
    pub quest_title: String,
    /// Quest description from form
    #[serde(rename = "questDescription")]
    pub quest_description: Option<String>,
    /// Quest EXP value from form
    #[serde(rename = "questExpValue")]
    pub quest_exp_value: Option<i32>,
    /// Quest day of week from form
    #[serde(rename = "questDayOfWeek")]
    pub quest_day_of_week: i32,
    /// Quest image upload from form
    #[serde(rename = "questImage")]
    pub quest_image: Vec<FileUpload>,
}

/// Request payload for creating/updating reward via Datastar JSON signals
#[derive(Debug, Deserialize)]
#[allow(
    clippy::struct_field_names,
    reason = "Frontend uses camelCase, Rust uses snake_case"
)]
pub struct RewardJsonRequest {
    /// Reward title from form
    #[serde(rename = "rewardTitle")]
    pub reward_title: String,
    /// Reward description from form
    #[serde(rename = "rewardDescription")]
    pub reward_description: Option<String>,
    /// Reward required EXP from form
    #[serde(rename = "rewardRequiredExp")]
    pub reward_required_exp: i32,
    /// Reward image upload from form
    #[serde(rename = "rewardImage")]
    pub reward_image: Vec<FileUpload>,
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
