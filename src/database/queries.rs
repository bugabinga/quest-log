use chrono::{Datelike, NaiveDate, TimeDelta, Utc};
use sqlx::{Row, SqlitePool};
use std::env;
use std::path::Path;
use tracing::instrument;

use crate::models::{
    ClaimState, CreateQuestRequest, CreateRewardRequest, Quest, QuestCompletion, Reward, Settings,
    ToggleResult, UpdateQuestRequest, UpdateRewardRequest, UpdateSettingsRequest, WeeklyChampion,
    WeeklyRewardDisplay,
};

use crate::time;

use super::{Database, SetOrRemove};

impl Database {
    /// Create a quest with optional image
    ///
    /// # Arguments
    ///
    /// * `title` - The quest title
    /// * `description` - Optional quest description
    /// * `exp_value` - Optional experience value (defaults to 10 if not provided)
    /// * `day_of_week` - Day of week (0-6, where 0 is Sunday)
    /// * `image_data` - Optional image data as bytes
    /// * `image_content_type` - Optional image content type (MIME type)
    ///
    /// # Returns
    ///
    /// The created quest
    ///
    /// # Errors
    ///
    /// Returns an error if the database insert fails
    pub async fn create_quest_with_image(
        &self,
        title: String,
        description: Option<String>,
        exp_value: Option<i32>,
        day_of_week: i32,
        image_data: Option<Vec<u8>>,
        image_content_type: Option<String>,
    ) -> Result<Quest, sqlx::Error> {
        tracing::debug!(title = %title, day = day_of_week, "📝 Creating quest with image");
        let now = Utc::now();
        let quest = sqlx::query_as::<_, Quest>(
            "INSERT INTO quests (title, description, exp_value, day_of_week, image_data, image_content_type, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(&title)
        .bind(&description)
        .bind(exp_value.unwrap_or(10))
        .bind(day_of_week)
        .bind(&image_data)
        .bind(&image_content_type)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!(quest_id = quest.id, title = %quest.title, "✨ Quest created with image!");
        Ok(quest)
    }

    /// Update a quest with optional image
    ///
    /// # Arguments
    ///
    /// * `id` - The quest ID to update
    /// * `title` - Optional new title
    /// * `description` - Optional new description
    /// * `exp_value` - Optional new experience value
    /// * `day_of_week` - Optional new day of week (0-6, where 0 is Sunday)
    /// * `is_active` - Optional new active status
    /// * `image_data` - Optional new image data as bytes
    /// * `image_content_type` - Optional new image content type (MIME type)
    ///
    /// # Returns
    ///
    /// The updated quest if found and updated, None if not found
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails
    pub async fn update_quest_with_image(
        &self,
        id: i64,
        title: Option<String>,
        description: Option<String>,
        exp_value: Option<i32>,
        day_of_week: Option<i32>,
        is_active: Option<bool>,
        image_data: SetOrRemove<Vec<u8>>,
        image_content_type: SetOrRemove<String>,
    ) -> Result<Option<Quest>, sqlx::Error> {
        tracing::debug!(quest_id = id, "🔄 Updating quest with image");
        let now = Utc::now();

        if let Some(title) = &title {
            sqlx::query("UPDATE quests SET title = ?, updated_at = ? WHERE id = ?")
                .bind(title)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(description) = &description {
            sqlx::query("UPDATE quests SET description = ?, updated_at = ? WHERE id = ?")
                .bind(description)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(exp_value) = exp_value {
            sqlx::query("UPDATE quests SET exp_value = ?, updated_at = ? WHERE id = ?")
                .bind(exp_value)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(day_of_week) = day_of_week {
            sqlx::query("UPDATE quests SET day_of_week = ?, updated_at = ? WHERE id = ?")
                .bind(day_of_week)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(is_active) = is_active {
            sqlx::query("UPDATE quests SET is_active = ?, updated_at = ? WHERE id = ?")
                .bind(is_active)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        // Handle image update - Unchanged means don't change, Removed means remove image
        if !image_data.is_unchanged() {
            sqlx::query("UPDATE quests SET image_data = ?, image_content_type = ?, updated_at = ? WHERE id = ?")
                .bind(image_data.as_option())
                .bind(image_content_type.as_option())
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        let result = self.get_quest_by_id(id).await?;
        if result.is_some() {
            tracing::info!(quest_id = id, "✅ Quest updated with image!");
        }
        Ok(result)
    }

    /// Create a reward with optional image
    ///
    /// # Arguments
    ///
    /// * `title` - The reward title
    /// * `description` - Optional reward description
    /// * `required_exp` - The required experience to claim the reward
    /// * `image_data` - Optional image data as bytes
    /// * `image_content_type` - Optional image content type (MIME type)
    ///
    /// # Returns
    ///
    /// The created reward
    ///
    /// # Errors
    ///
    /// Returns an error if the database insert fails
    pub async fn create_reward_with_image(
        &self,
        title: String,
        description: Option<String>,
        required_exp: i32,
        image_data: Option<Vec<u8>>,
        image_content_type: Option<String>,
    ) -> Result<Reward, sqlx::Error> {
        tracing::debug!(title = %title, exp = required_exp, "🎁 Creating reward with image");
        let now = Utc::now();
        let reward = sqlx::query_as::<_, Reward>(
            "INSERT INTO rewards (title, description, required_exp, image_data, image_content_type, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(&title)
        .bind(&description)
        .bind(required_exp)
        .bind(&image_data)
        .bind(&image_content_type)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!(reward_id = reward.id, title = %reward.title, "✨ Reward created with image!");
        Ok(reward)
    }

    /// Update a reward with optional image
    ///
    /// # Arguments
    ///
    /// * `id` - The reward ID to update
    /// * `title` - Optional new title
    /// * `description` - Optional new description
    /// * `required_exp` - Optional new required experience
    /// * `is_active` - Optional new active status
    /// * `image_data` - Optional new image data as bytes
    /// * `image_content_type` - Optional new image content type (MIME type)
    ///
    /// # Returns
    ///
    /// The updated reward if found and updated, None if not found
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails
    pub async fn update_reward_with_image(
        &self,
        id: i64,
        title: Option<String>,
        description: Option<String>,
        required_exp: Option<i32>,
        is_active: Option<bool>,
        image_data: SetOrRemove<Vec<u8>>,
        image_content_type: SetOrRemove<String>,
    ) -> Result<Option<Reward>, sqlx::Error> {
        tracing::debug!(reward_id = id, "🔄 Updating reward with image");
        let now = Utc::now();

        if let Some(title) = &title {
            sqlx::query("UPDATE rewards SET title = ?, updated_at = ? WHERE id = ?")
                .bind(title)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(description) = &description {
            sqlx::query("UPDATE rewards SET description = ?, updated_at = ? WHERE id = ?")
                .bind(description)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(required_exp) = required_exp {
            sqlx::query("UPDATE rewards SET required_exp = ?, updated_at = ? WHERE id = ?")
                .bind(required_exp)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(is_active) = is_active {
            sqlx::query("UPDATE rewards SET is_active = ?, updated_at = ? WHERE id = ?")
                .bind(is_active)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        // Handle image update - Unchanged means don't change, Removed means remove image
        if !image_data.is_unchanged() {
            sqlx::query("UPDATE rewards SET image_data = ?, image_content_type = ?, updated_at = ? WHERE id = ?")
                .bind(image_data.as_option())
                .bind(image_content_type.as_option())
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        let result = self.get_reward_by_id(id).await?;
        if result.is_some() {
            tracing::info!(reward_id = id, "✅ Reward updated with image!");
        }
        Ok(result)
    }
}
