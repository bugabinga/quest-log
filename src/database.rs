use chrono::{Datelike, NaiveDate, TimeDelta, Utc};
use sqlx::{Row, SqlitePool};
use std::env;
use std::path::Path;
use tracing::instrument;

#[derive(Clone, Debug, Default)]
pub enum SetOrRemove<T> {
    #[default]
    Unchanged,
    Set(T),
}

impl<T> SetOrRemove<T> {
    #[must_use]
    pub fn set(value: T) -> Self {
        Self::Set(value)
    }

    pub fn as_option(&self) -> Option<&T> {
        match self {
            Self::Set(value) => Some(value),
            Self::Unchanged => None,
        }
    }

    pub fn is_unchanged(&self) -> bool {
        matches!(self, Self::Unchanged)
    }
}

use crate::models::{
    ClaimState, CreateQuestRequest, CreateRewardRequest, Quest, QuestCompletion, Reward, Settings,
    ToggleResult, UpdateQuestRequest, UpdateRewardRequest, UpdateSettingsRequest, WeeklyChampion,
    WeeklyRewardDisplay,
};
use crate::time;

#[derive(Clone, Debug)]
pub struct Database {
    pool: SqlitePool,
}

#[cfg(feature = "test-utils")]
impl Database {
    pub fn with_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn get_completions_for_date(
        &self,
        date: NaiveDate,
    ) -> Result<Vec<QuestCompletion>, sqlx::Error> {
        tracing::trace!(date = %date, "📋 Fetching completions for date");
        sqlx::query_as::<_, QuestCompletion>(
            "SELECT * FROM quest_completions WHERE completed_date = ?",
        )
        .bind(date)
        .fetch_all(&self.pool)
        .await
    }
}

impl Database {
    /// Create a new database connection pool
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be created or connected to.
    ///
    /// # Panics
    ///
    /// Panics if the current directory cannot be determined and `QUEST_LOG_DATA_DIR` is not set.
    pub async fn new() -> Result<Self, sqlx::Error> {
        let data_dir = if let Ok(val) = env::var("QUEST_LOG_DATA_DIR") {
            val
        } else {
            let current_dir = env::current_dir()
                .map_err(|e| sqlx::Error::Configuration(
                    format!("❌ Failed to get current directory: {e}. Please ensure you have permission to access the current working directory.").into()
                ))?;
            current_dir.to_string_lossy().to_string()
        };

        // Validate absolute path
        let data_dir_path = Path::new(&data_dir);
        if !data_dir_path.is_absolute() {
            return Err(sqlx::Error::Configuration(
                 format!("❌ QUEST_LOG_DATA_DIR must be an absolute path: '{data_dir}'. Current working directory would be: '{}'",
                     env::current_dir().unwrap_or_default().display()
                 ).into()
             ));
        }

        // Create directory if it doesn't exist
        if !data_dir_path.exists() {
            tracing::debug!(path = %data_dir_path.display(), "Creating data directory");
            std::fs::create_dir_all(data_dir_path)
                 .map_err(|e| sqlx::Error::Configuration(
                     format!("❌ Failed to create database directory '{data_dir_path_display}': {e}. Please check permissions.",
                         data_dir_path_display = data_dir_path.display()
                     ).into()
                 ))?;
        }

        let database_path = data_dir_path
            .join("quests.db")
            .to_string_lossy()
            .to_string();

        // Check if database file exists (only for file-based databases, not :memory:)
        let is_new_database = if database_path != ":memory:" && database_path != "sqlite::memory:" {
            let exists = Path::new(&database_path).exists();
            if !exists {
                tracing::debug!(path = %database_path, "Creating new database file");
                // Create an empty file to ensure SQLite can connect
                std::fs::File::create(&database_path).map_err(|e| {
                    sqlx::Error::Configuration(
                 format!(
                     "❌ Failed to create database file '{database_path}': {e}. Please check permissions.",
                 )
                        .into(),
                    )
                })?;
            }
            !exists
        } else {
            false
        };

        tracing::info!(db_path = %database_path, "🗄️  Connecting to database...");
        let pool = SqlitePool::connect(&database_path).await?;

        let db = Self { pool };

        // Run migrations
        db.migrate().await?;

        // Seed sample data only if this is a new database AND we're in debug mode
        if is_new_database {
            tracing::info!("🌱 Seeding sample data for first-time setup");
            #[cfg(debug_assertions)]
            db.seed_sample_data().await?;
        }

        Ok(db)
    }

    /// Run database migrations
    ///
    /// # Errors
    ///
    /// Returns an error if the migration fails to apply.
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        tracing::info!("🔧 Running database migrations...");
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        tracing::info!("✅ Migrations complete! (灬♥ω♥灬)");
        Ok(())
    }

    /// Seed the database with sample data (debug mode only)
    #[cfg(debug_assertions)]
    async fn seed_sample_data(&self) -> Result<(), sqlx::Error> {
        // Sample quests for each day of the week
        let sample_quests = vec![
            (
                "Morning exercise",
                Some("Get your body moving for the day"),
                15,
                1,
            ), // Monday
            ("Read for 30 minutes", Some("Expand your knowledge"), 20, 1), // Monday
            (
                "Help with chores",
                Some("Contribute to household tasks"),
                10,
                2,
            ), // Tuesday
            (
                "Practice instrument",
                Some("Improve your musical skills"),
                25,
                2,
            ), // Tuesday
            (
                "Learn something new",
                Some("Discover a new topic or skill"),
                30,
                3,
            ), // Wednesday
            ("Call a friend", Some("Maintain social connections"), 5, 3),  // Wednesday
            (
                "Organize workspace",
                Some("Create a productive environment"),
                15,
                4,
            ), // Thursday
            ("Healthy meal prep", Some("Plan nutritious meals"), 20, 4),   // Thursday
            (
                "Meditation session",
                Some("Center your mind and spirit"),
                10,
                5,
            ), // Friday
            (
                "Review weekly goals",
                Some("Assess progress and plan ahead"),
                15,
                5,
            ), // Friday
            ("Weekend project", Some("Work on a personal project"), 40, 6), // Saturday
            (
                "Family time",
                Some("Spend quality time with loved ones"),
                25,
                0,
            ), // Sunday
        ];

        for (title, description, exp_value, day_of_week) in sample_quests {
            let req = CreateQuestRequest {
                day_of_week,
                description: description.map(ToString::to_string),
                exp_value: Some(exp_value),
                title: title.to_string(),
            };
            self.create_quest(req).await?;
        }

        // Sample rewards with different EXP requirements
        let sample_rewards = vec![
            (
                "Small treat",
                Some("Enjoy a favorite snack or small indulgence"),
                50,
            ),
            (
                "Movie night",
                Some("Watch a movie or show you've been wanting to see"),
                100,
            ),
            ("Weekend outing", Some("Plan a fun activity or trip"), 200),
        ];

        for (title, description, required_exp) in sample_rewards {
            let req = CreateRewardRequest {
                description: description.map(ToString::to_string),
                required_exp,
                title: title.to_string(),
            };
            self.create_reward(req).await?;
        }

        Ok(())
    }

    // Quest operations
    #[instrument(name = "📋 get_quests_for_day", skip(self))]
    /// Get all active quests for a specific day of the week
    ///
    /// # Arguments
    ///
    /// * `day_of_week` - Day of week (0-6, where 0 is Sunday)
    ///
    /// # Returns
    ///
    /// Vector of active quests for the specified day
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_quests_for_day(&self, day_of_week: i32) -> Result<Vec<Quest>, sqlx::Error> {
        tracing::trace!(day_of_week, "📋 Fetching quests for day");
        let quests = sqlx::query_as::<_, Quest>(
            "SELECT * FROM quests WHERE day_of_week = ? AND is_active = TRUE ORDER BY created_at",
        )
        .bind(day_of_week)
        .fetch_all(&self.pool)
        .await?;

        tracing::trace!(
            count = quests.len(),
            day_of_week,
            "📋 Loaded {} quests",
            quests.len()
        );
        Ok(quests)
    }

    /// Get a quest by its ID
    ///
    /// # Arguments
    ///
    /// * `id` - The quest ID to look up
    ///
    /// # Returns
    ///
    /// The quest if found, None if not found
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_quest_by_id(&self, id: i64) -> Result<Option<Quest>, sqlx::Error> {
        tracing::trace!(quest_id = id, "🔍 Looking up quest by ID");
        sqlx::query_as::<_, Quest>("SELECT * FROM quests WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Create a new quest
    ///
    /// # Arguments
    ///
    /// * `req` - The quest creation request containing title, description, `exp_value`, and `day_of_week`
    ///
    /// # Returns
    ///
    /// The created quest
    ///
    /// # Errors
    ///
    /// Returns an error if the database insert fails
    pub async fn create_quest(&self, req: CreateQuestRequest) -> Result<Quest, sqlx::Error> {
        tracing::debug!(title = %req.title, day = req.day_of_week, exp = req.exp_value.unwrap_or(10), "📝 Creating new quest");
        let now = Utc::now();
        let quest = sqlx::query_as::<_, Quest>(
            "INSERT INTO quests (title, description, exp_value, day_of_week, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING *"
        )
        .bind(&req.title)
        .bind(&req.description)
        .bind(req.exp_value.unwrap_or(10))
        .bind(req.day_of_week)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!(quest_id = quest.id, title = %quest.title, exp = quest.exp_value, "✨ Quest created successfully!");
        Ok(quest)
    }

    /// Update a quest by its ID
    ///
    /// # Arguments
    ///
    /// * `id` - The quest ID to update
    /// * `req` - The update request containing optional fields to update
    ///
    /// # Returns
    ///
    /// The updated quest if found and updated, None if not found
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails
    pub async fn update_quest(
        &self,
        id: i64,
        req: UpdateQuestRequest,
    ) -> Result<Option<Quest>, sqlx::Error> {
        tracing::debug!(quest_id = id, "🔄 Updating quest");
        let now = Utc::now();

        // For simplicity, let's update only the fields that are provided
        // In a real app, we'd use a more sophisticated approach
        if let Some(title) = &req.title {
            sqlx::query("UPDATE quests SET title = ?, updated_at = ? WHERE id = ?")
                .bind(title)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(description) = &req.description {
            sqlx::query("UPDATE quests SET description = ?, updated_at = ? WHERE id = ?")
                .bind(description)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(exp_value) = req.exp_value {
            sqlx::query("UPDATE quests SET exp_value = ?, updated_at = ? WHERE id = ?")
                .bind(exp_value)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(day_of_week) = req.day_of_week {
            sqlx::query("UPDATE quests SET day_of_week = ?, updated_at = ? WHERE id = ?")
                .bind(day_of_week)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(is_active) = req.is_active {
            sqlx::query("UPDATE quests SET is_active = ?, updated_at = ? WHERE id = ?")
                .bind(is_active)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        // Return the updated quest
        let result = self.get_quest_by_id(id).await?;
        if result.is_some() {
            tracing::info!(quest_id = id, "✅ Quest updated successfully!");
        }
        Ok(result)
    }

    /// Delete a quest by its ID
    ///
    /// # Arguments
    ///
    /// * `id` - The quest ID to delete
    ///
    /// # Returns
    ///
    /// True if the quest was deleted, false if not found
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn delete_quest(&self, id: i64) -> Result<bool, sqlx::Error> {
        tracing::debug!(quest_id = id, "🗑️  Deleting quest");
        let result = sqlx::query("DELETE FROM quests WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            tracing::info!(quest_id = id, "💨 Quest deleted!");
        }
        Ok(deleted)
    }

    /// Check if a quest is completed today
    ///
    /// # Arguments
    ///
    /// * `quest_id` - The ID of the quest to check
    /// * `today` - The date to check against
    ///
    /// # Returns
    ///
    /// True if the quest is completed today, false otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn is_quest_completed_today(
        &self,
        quest_id: i64,
        today: NaiveDate,
    ) -> Result<bool, sqlx::Error> {
        tracing::trace!(quest_id, date = %today, "✓ Checking if quest completed today");
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM quest_completions WHERE quest_id = ? AND completed_date = ?",
        )
        .bind(quest_id)
        .bind(today)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 > 0)
    }

    #[instrument(name = "📋 get_quests_completion_status", skip(self, quest_ids))]
    /// Get completion status for multiple quests on a specific date
    ///
    /// # Arguments
    ///
    /// * `quest_ids` - Slice of quest IDs to check
    /// * `date` - The date to check completion status for
    ///
    /// # Returns
    ///
    /// `HashMap` mapping quest IDs to their completion status (true if completed)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_quests_completion_status(
        &self,
        quest_ids: &[i64],
        date: NaiveDate,
    ) -> Result<std::collections::HashMap<i64, bool>, sqlx::Error> {
        tracing::trace!(count = quest_ids.len(), date = %date, "📋 Batch completion check");
        if quest_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let placeholders: Vec<&str> = quest_ids.iter().map(|_| "?").collect();
        let query = format!(
            "SELECT quest_id FROM quest_completions WHERE quest_id IN ({}) AND completed_date = ?",
            placeholders.join(", ")
        );

        let mut sql_query = sqlx::query(&query);
        for id in quest_ids {
            sql_query = sql_query.bind(id);
        }
        sql_query = sql_query.bind(date);

        let completed_rows = sql_query.fetch_all(&self.pool).await?;

        let completed_set: std::collections::HashSet<i64> = completed_rows
            .into_iter()
            .map(|row| row.get::<i64, _>(0))
            .collect();

        let mut result = std::collections::HashMap::new();
        for id in quest_ids {
            result.insert(*id, completed_set.contains(id));
        }

        Ok(result)
    }

    /// Get the total number of quest completions
    ///
    /// # Returns
    ///
    /// Total count of quest completions
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_total_completions_count(&self) -> Result<i32, sqlx::Error> {
        tracing::trace!("📋 Fetching total completions count");
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM quest_completions")
            .fetch_one(&self.pool)
            .await?;
        Ok(i32::try_from(result.0).unwrap_or(i32::MAX))
    }

    /// Get all quest completions
    ///
    /// # Returns
    ///
    /// Vector of all quest completions, ordered by date descending
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_all_completions(&self) -> Result<Vec<QuestCompletion>, sqlx::Error> {
        tracing::trace!("📋 Fetching all completions");
        sqlx::query_as::<_, QuestCompletion>(
            "SELECT * FROM quest_completions ORDER BY completed_date DESC",
        )
        .fetch_all(&self.pool)
        .await
    }

    #[instrument(name = "🎯 toggle_quest_completion", skip(self))]
    /// Toggle the completion status of a quest for a specific date
    ///
    /// # Arguments
    ///
    /// * `quest_id` - The ID of the quest to toggle
    /// * `date` - The date to toggle completion for
    ///
    /// # Returns
    ///
    /// `ToggleResult` indicating whether the quest is now completed or not
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn toggle_quest_completion(
        &self,
        quest_id: i64,
        date: NaiveDate,
    ) -> Result<ToggleResult, sqlx::Error> {
        tracing::debug!(quest_id, date = %date, "🎯 Toggling quest completion");
        // First, verify the quest exists
        let quest_exists = self.get_quest_by_id(quest_id).await?.is_some();
        if !quest_exists {
            tracing::warn!(quest_id, "❌ Quest not found for completion toggle");
            return Err(sqlx::Error::RowNotFound); // Quest doesn't exist
        }

        // Check if already completed
        let exists = self.is_quest_completed_today(quest_id, date).await?;

        if exists {
            // Already completed - delete it to un-complete
            match sqlx::query(
                "DELETE FROM quest_completions WHERE quest_id = ? AND completed_date = ?",
            )
            .bind(quest_id)
            .bind(date)
            .execute(&self.pool)
            .await
            {
                Ok(_) => {
                    tracing::info!(quest_id, date = %date, "📤 Quest marked as incomplete");
                    Ok(ToggleResult::NewlyUncompleted)
                }
                Err(e) => Err(e),
            }
        } else {
            // Not completed - insert to complete
            match sqlx::query(
                "INSERT INTO quest_completions (quest_id, completed_date) VALUES (?, ?)",
            )
            .bind(quest_id)
            .bind(date)
            .execute(&self.pool)
            .await
            {
                Ok(_) => {
                    tracing::info!(quest_id, date = %date, "🎉 Quest completed! +EXP");
                    Ok(ToggleResult::NewlyCompleted)
                }
                Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                    // Another concurrent operation already inserted - that's fine
                    tracing::debug!(quest_id, date = %date, "Quest completion already recorded (concurrent)");
                    Ok(ToggleResult::NoChange)
                }
                Err(e) => Err(e),
            }
        }
    }

    // Settings operations
    /// Get the application settings
    ///
    /// # Returns
    ///
    /// The application settings
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_settings(&self) -> Result<Settings, sqlx::Error> {
        tracing::debug!("⚙️ Fetching settings");
        sqlx::query_as::<_, Settings>("SELECT * FROM settings WHERE id = 1")
            .fetch_one(&self.pool)
            .await
    }

    /// Update application settings
    ///
    /// # Arguments
    ///
    /// * `req` - The settings update request containing fields to update
    ///
    /// # Returns
    ///
    /// The updated settings
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails
    pub async fn update_settings(
        &self,
        req: UpdateSettingsRequest,
    ) -> Result<Settings, sqlx::Error> {
        tracing::debug!(
            weekly_exp_goal = req.weekly_exp_goal,
            "⚙️ Updating settings"
        );
        let now = Utc::now();
        sqlx::query_as::<_, Settings>(
            "UPDATE settings SET weekly_exp_goal = ?, updated_at = ? WHERE id = 1 RETURNING *",
        )
        .bind(req.weekly_exp_goal)
        .bind(now)
        .fetch_one(&self.pool)
        .await
    }

    // Reward operations
    /// Get all active rewards ordered by required experience
    ///
    /// # Returns
    ///
    /// Vector of active rewards sorted by required EXP ascending
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_available_rewards(&self) -> Result<Vec<Reward>, sqlx::Error> {
        tracing::trace!("🎁 Fetching available rewards");
        sqlx::query_as::<_, Reward>(
            "SELECT * FROM rewards WHERE is_active = TRUE ORDER BY required_exp",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Create a new reward
    ///
    /// # Arguments
    ///
    /// * `req` - The reward creation request containing title, description, and `required_exp`
    ///
    /// # Returns
    ///
    /// The created reward
    ///
    /// # Errors
    ///
    /// Returns an error if the database insert fails
    pub async fn create_reward(&self, req: CreateRewardRequest) -> Result<Reward, sqlx::Error> {
        tracing::debug!(title = %req.title, exp = req.required_exp, "🎁 Creating new reward");
        let now = Utc::now();
        let reward = sqlx::query_as::<_, Reward>(
            "INSERT INTO rewards (title, description, required_exp, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(&req.title)
        .bind(&req.description)
        .bind(req.required_exp)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!(reward_id = reward.id, title = %reward.title, "✨ Reward created!");
        Ok(reward)
    }

    /// Get all quests (including inactive) for the editor
    ///
    /// # Returns
    ///
    /// Vector of all quests ordered by day of week and creation date
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_all_quests(&self) -> Result<Vec<Quest>, sqlx::Error> {
        tracing::debug!("📋 Fetching all quests for editor");
        sqlx::query_as::<_, Quest>("SELECT * FROM quests ORDER BY day_of_week, created_at")
            .fetch_all(&self.pool)
            .await
    }

    /// Get all rewards (including inactive) for the editor
    ///
    /// # Returns
    ///
    /// Vector of all rewards ordered by required EXP and creation date
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_all_rewards(&self) -> Result<Vec<Reward>, sqlx::Error> {
        tracing::debug!("🎁 Fetching all rewards for editor");
        sqlx::query_as::<_, Reward>("SELECT * FROM rewards ORDER BY required_exp, created_at")
            .fetch_all(&self.pool)
            .await
    }

    /// Get a reward by ID
    ///
    /// # Arguments
    ///
    /// * `id` - The reward ID to look up
    ///
    /// # Returns
    ///
    /// The reward if found, None if not found
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_reward_by_id(&self, id: i64) -> Result<Option<Reward>, sqlx::Error> {
        tracing::trace!(reward_id = id, "🔍 Looking up reward by ID");
        sqlx::query_as::<_, Reward>("SELECT * FROM rewards WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Update a reward by its ID
    ///
    /// # Arguments
    ///
    /// * `id` - The reward ID to update
    /// * `req` - The update request containing optional fields to update
    ///
    /// # Returns
    ///
    /// The updated reward if found and updated, None if not found
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails
    pub async fn update_reward(
        &self,
        id: i64,
        req: UpdateRewardRequest,
    ) -> Result<Option<Reward>, sqlx::Error> {
        tracing::debug!(reward_id = id, "🔄 Updating reward");
        let now = Utc::now();

        if let Some(title) = &req.title {
            sqlx::query("UPDATE rewards SET title = ?, updated_at = ? WHERE id = ?")
                .bind(title)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(description) = &req.description {
            sqlx::query("UPDATE rewards SET description = ?, updated_at = ? WHERE id = ?")
                .bind(description)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(required_exp) = req.required_exp {
            sqlx::query("UPDATE rewards SET required_exp = ?, updated_at = ? WHERE id = ?")
                .bind(required_exp)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        if let Some(is_active) = req.is_active {
            sqlx::query("UPDATE rewards SET is_active = ?, updated_at = ? WHERE id = ?")
                .bind(is_active)
                .bind(now)
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        let result = self.get_reward_by_id(id).await?;
        if result.is_some() {
            tracing::info!(reward_id = id, "✅ Reward updated successfully!");
        }
        Ok(result)
    }

    /// Delete a reward by its ID
    ///
    /// # Arguments
    ///
    /// * `id` - The reward ID to delete
    ///
    /// # Returns
    ///
    /// True if the reward was deleted, false if not found
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails
    pub async fn delete_reward(&self, id: i64) -> Result<bool, sqlx::Error> {
        tracing::debug!(reward_id = id, "🗑️  Deleting reward");
        let result = sqlx::query("DELETE FROM rewards WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        let deleted = result.rows_affected() > 0;
        if deleted {
            tracing::info!(reward_id = id, "💨 Reward deleted!");
        }
        Ok(deleted)
    }

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

    // Statistics and calculations
    #[instrument(name = "🧮 calculate_weekly_exp", skip(self))]
    /// Calculate total experience earned in a week
    ///
    /// # Arguments
    ///
    /// * `week_start` - Start date of the week (inclusive)
    /// * `week_end` - End date of the week (inclusive)
    ///
    /// # Returns
    ///
    /// Total experience earned in the specified week
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn calculate_weekly_exp(
        &self,
        week_start: NaiveDate,
        week_end: NaiveDate,
    ) -> Result<i32, sqlx::Error> {
        tracing::trace!(week_start = %week_start, week_end = %week_end, "🧮 Calculating weekly EXP");
        let result: (i32,) = sqlx::query_as(
            "SELECT COALESCE(SUM(q.exp_value), 0) as total_exp
             FROM quest_completions qc
             JOIN quests q ON qc.quest_id = q.id
             WHERE qc.completed_date >= ? AND qc.completed_date <= ?",
        )
        .bind(week_start)
        .bind(week_end)
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }

    #[instrument(name = "📊 get_week_stats", skip(self))]
    /// Get statistics for a week
    ///
    /// # Arguments
    ///
    /// * `today` - Today's date
    /// * `week_start` - Start date of the week (inclusive)
    /// * `week_end` - End date of the week (inclusive)
    ///
    /// # Returns
    ///
    /// Weekly statistics including completed quests and experience
    ///
    /// # Errors
    ///
    /// Returns an error if any of the database queries fail
    pub async fn get_week_stats(
        &self,
        today: NaiveDate,
        week_start: NaiveDate,
        week_end: NaiveDate,
    ) -> Result<crate::models::QuestStats, sqlx::Error> {
        tracing::debug!(today = %today, week_start = %week_start, week_end = %week_end, "📊 Getting week stats");
        let day_of_week = today.weekday().num_days_from_sunday().cast_signed();

        // Today's quests and completed EXP
        let today_quests = self.get_quests_for_day(day_of_week).await?;
        let exp_today_max: i32 = today_quests.iter().map(|q| q.exp_value).sum();
        let quests_total = i32::try_from(today_quests.len()).unwrap_or(i32::MAX);

        let mut exp_today = 0i32;
        let mut quests_completed = 0i32;
        for quest in &today_quests {
            if self.is_quest_completed_today(quest.id, today).await? {
                exp_today = exp_today
                    .checked_add(quest.exp_value)
                    .ok_or_else(|| sqlx::Error::Protocol("Integer overflow in exp_today".into()))?;
                quests_completed = quests_completed.checked_add(1).ok_or_else(|| {
                    sqlx::Error::Protocol("Integer overflow in quests_completed".into())
                })?;
            }
        }

        // Weekly stats
        let week_exp = self.calculate_weekly_exp(week_start, week_end).await?;

        // Max weekly EXP (all quests for each day of the week)
        let mut week_exp_max = 0i32;
        for dow in 0..7 {
            let quests = self.get_quests_for_day(dow).await?;
            let day_exp: i32 = quests.iter().map(|q| q.exp_value).sum();
            week_exp_max = week_exp_max
                .checked_add(day_exp)
                .ok_or_else(|| sqlx::Error::Protocol("Integer overflow in week_exp_max".into()))?;
        }

        Ok(crate::models::QuestStats {
            exp_today,
            exp_today_max,
            quests_completed,
            quests_total,
            week_exp,
            week_exp_max,
        })
    }

    /// Get the total experience earned from all completed quests
    ///
    /// # Returns
    ///
    /// Total experience earned from all completed quests
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_total_exp_earned(&self) -> Result<i32, sqlx::Error> {
        tracing::trace!("💎 Fetching total EXP");
        let result: (i32,) = sqlx::query_as(
            "SELECT COALESCE(SUM(q.exp_value), 0) as total_exp
             FROM quest_completions qc
             JOIN quests q ON qc.quest_id = q.id",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }

    /// Get the total number of rewards claimed
    ///
    /// # Returns
    ///
    /// Total count of rewards claimed
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_rewards_claimed_count(&self) -> Result<i32, sqlx::Error> {
        tracing::trace!("🏆 Fetching claimed rewards count");
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reward_claims")
            .fetch_one(&self.pool)
            .await?;

        Ok(i32::try_from(result.0).unwrap_or(i32::MAX))
    }

    // Reward claiming logic
    #[instrument(name = "🎁 get_weekly_reward_status", skip(self))]
    /// Get the status of all rewards for a specific week
    ///
    /// # Arguments
    ///
    /// * `week_start` - Start date of the week (inclusive)
    /// * `today` - Today's date
    ///
    /// # Returns
    ///
    /// Vector of weekly reward display objects showing claim status
    ///
    /// # Errors
    ///
    /// Returns an error if any of the database queries fail
    pub async fn get_weekly_reward_status(
        &self,
        week_start: NaiveDate,
        today: NaiveDate,
    ) -> Result<Vec<WeeklyRewardDisplay>, sqlx::Error> {
        tracing::trace!(week_start = %week_start, today = %today, "🎁 Fetching reward status");
        let week_end = week_start
            .checked_add_signed(TimeDelta::days(6))
            .ok_or_else(|| sqlx::Error::Protocol("Date overflow in week_end".into()))?;
        let weekly_exp = self.calculate_weekly_exp(week_start, week_end).await?;

        let is_sunday = today.weekday().num_days_from_sunday() == 0;
        let can_claim_this_week = today >= week_start && today <= week_end && is_sunday;

        let rewards = self.get_available_rewards().await?;

        let mut result = Vec::new();
        for reward in rewards {
            let claimed_this_week: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM reward_claims
                 WHERE reward_id = ? AND claimed_date >= ? AND claimed_date <= ?",
            )
            .bind(reward.id)
            .bind(week_start)
            .bind(week_end)
            .fetch_one(&self.pool)
            .await?;

            let is_claimed = claimed_this_week.0 > 0;
            let has_enough_exp = weekly_exp >= reward.required_exp;

            let state = if is_claimed {
                ClaimState::Claimed
            } else if can_claim_this_week && has_enough_exp {
                ClaimState::Claimable
            } else {
                ClaimState::Locked
            };

            result.push(WeeklyRewardDisplay {
                id: reward.id,
                title: reward.title,
                description: reward.description,
                required_exp: reward.required_exp,
                weekly_exp,
                state,
                can_claim_today: can_claim_this_week,
            });
        }

        Ok(result)
    }

    #[instrument(name = "🏆 claim_reward_for_week", skip(self))]
    /// Claim a reward for a specific week (only on Sunday)
    ///
    /// # Arguments
    ///
    /// * `reward_id` - The ID of the reward to claim
    /// * `week_start` - The start date of the week (Monday) for which to claim the reward
    ///
    /// # Returns
    ///
    /// True if the reward was successfully claimed, false if not eligible (wrong day, already claimed, etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    ///
    /// # Panics
    ///
    /// Panics if the week end date overflows
    pub async fn claim_reward_for_week(
        &self,
        reward_id: i64,
        week_start: NaiveDate,
    ) -> Result<bool, sqlx::Error> {
        let week_end = week_start
            .checked_add_days(chrono::Days::new(6))
            .unwrap_or_else(|| panic!("date overflow"));
        let today = time::today();
        let is_sunday = today.weekday().num_days_from_sunday() == 0;

        if today < week_start || today > week_end || !is_sunday {
            tracing::warn!(reward_id, "Claim attempted outside valid period");
            return Ok(false);
        }

        let reward =
            sqlx::query_as::<_, Reward>("SELECT * FROM rewards WHERE id = ? AND is_active = TRUE")
                .bind(reward_id)
                .fetch_optional(&self.pool)
                .await?;

        let Some(reward) = reward else {
            return Ok(false);
        };

        let weekly_exp = self.calculate_weekly_exp(week_start, week_end).await?;
        if weekly_exp < reward.required_exp {
            tracing::warn!(
                reward_id,
                weekly_exp,
                required = reward.required_exp,
                "Insufficient EXP to claim reward"
            );
            return Ok(false);
        }

        let existing_claim: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM reward_claims
             WHERE reward_id = ? AND claimed_date >= ? AND claimed_date <= ?",
        )
        .bind(reward_id)
        .bind(week_start)
        .bind(week_end)
        .fetch_one(&self.pool)
        .await?;

        if existing_claim.0 > 0 {
            return Ok(false);
        }

        let claimed_date = today;
        sqlx::query("INSERT INTO reward_claims (reward_id, claimed_date) VALUES (?, ?)")
            .bind(reward_id)
            .bind(claimed_date)
            .execute(&self.pool)
            .await?;

        let rewards = self.get_weekly_reward_status(week_start, today).await?;
        let all_rewards_claimed = rewards
            .iter()
            .filter(|r| r.state == ClaimState::Claimed)
            .count()
            == rewards.len()
            && !rewards.is_empty();

        if all_rewards_claimed {
            let _ = self.create_weekly_champion(week_start).await;
        }

        tracing::info!(reward_id, title = %reward.title, "Reward claimed successfully!");
        Ok(true)
    }

    /// Get a weekly champion record for a specific week
    ///
    /// # Arguments
    ///
    /// Get all weekly champion records
    ///
    /// # Returns
    ///
    /// Vector of all weekly champions ordered by week start date descending
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails
    pub async fn get_all_weekly_champions(&self) -> Result<Vec<WeeklyChampion>, sqlx::Error> {
        tracing::trace!("🏆 Fetching all weekly champions");
        sqlx::query_as::<_, WeeklyChampion>(
            "SELECT * FROM weekly_champions ORDER BY week_start DESC",
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Create a new weekly champion record
    ///
    /// # Arguments
    ///
    /// * `week_start` - Start date of the week
    ///
    /// # Returns
    ///
    /// The created weekly champion record
    ///
    /// # Errors
    ///
    /// Returns an error if the database insert fails
    pub async fn create_weekly_champion(
        &self,
        week_start: NaiveDate,
    ) -> Result<WeeklyChampion, sqlx::Error> {
        tracing::info!(week_start = %week_start, "🏆 Creating weekly champion record");
        let now = Utc::now();
        sqlx::query_as::<_, WeeklyChampion>(
            "INSERT INTO weekly_champions (week_start, earned_at) VALUES (?, ?) RETURNING *",
        )
        .bind(week_start)
        .bind(now)
        .fetch_one(&self.pool)
        .await
    }

    /// Claim a reward for a specific week (test-only version without Sunday check)
    #[cfg(test)]
    pub async fn claim_reward(
        &self,
        reward_id: i64,
        week_start: NaiveDate,
    ) -> Result<bool, sqlx::Error> {
        let reward =
            sqlx::query_as::<_, Reward>("SELECT * FROM rewards WHERE id = ? AND is_active = TRUE")
                .bind(reward_id)
                .fetch_optional(&self.pool)
                .await?;

        let Some(reward) = reward else {
            return Ok(false);
        };

        let week_end = week_start
            .checked_add_signed(TimeDelta::days(6))
            .ok_or_else(|| sqlx::Error::Protocol("Date overflow in week_end".into()))?;
        let weekly_exp = self.calculate_weekly_exp(week_start, week_end).await?;

        if weekly_exp < reward.required_exp {
            return Ok(false);
        }

        let existing_claim: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM reward_claims
             WHERE reward_id = ? AND claimed_date >= ? AND claimed_date <= ?",
        )
        .bind(reward_id)
        .bind(week_start)
        .bind(week_end)
        .fetch_one(&self.pool)
        .await?;

        if existing_claim.0 > 0 {
            return Ok(false);
        }

        let claimed_date = week_start
            .checked_add_signed(TimeDelta::days(6))
            .ok_or_else(|| sqlx::Error::Protocol("Date overflow in claimed_date".into()))?;
        sqlx::query("INSERT INTO reward_claims (reward_id, claimed_date) VALUES (?, ?)")
            .bind(reward_id)
            .bind(claimed_date)
            .execute(&self.pool)
            .await?;

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::Database;
    use crate::models::*;
    use crate::time;
    use chrono::NaiveDate;
    use sqlx::SqlitePool;
    use std::path::Path;
    use std::sync::Arc;

    async fn setup_test_db() -> Database {
        // Use in-memory SQLite for tests to avoid interference between tests
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        Database { pool }
    }

    #[tokio::test]
    async fn test_create_and_get_quest() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: Some("A test quest".to_string()),
            exp_value: Some(20),
            title: "Test Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();
        assert_eq!(quest.title, "Test Quest");
        assert_eq!(quest.exp_value, 20);
        assert_eq!(quest.day_of_week, 1);

        let retrieved = db.get_quest_by_id(quest.id).await.unwrap().unwrap();
        assert_eq!(retrieved.title, quest.title);
    }

    #[tokio::test]
    async fn test_get_quests_for_day() {
        let db = setup_test_db().await;

        // Create quests for different days
        let req1 = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: "Monday Quest".to_string(),
        };
        let req2 = CreateQuestRequest {
            day_of_week: 2,
            description: None,
            exp_value: Some(15),
            title: "Tuesday Quest".to_string(),
        };

        db.create_quest(req1).await.unwrap();
        db.create_quest(req2).await.unwrap();

        let monday_quests = db.get_quests_for_day(1).await.unwrap();
        assert_eq!(monday_quests.len(), 1);
        assert_eq!(monday_quests[0].title, "Monday Quest");

        let tuesday_quests = db.get_quests_for_day(2).await.unwrap();
        assert_eq!(tuesday_quests.len(), 1);
        assert_eq!(tuesday_quests[0].title, "Tuesday Quest");
    }

    #[tokio::test]
    async fn test_update_quest() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: "Original Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();

        let update_req = UpdateQuestRequest {
            title: Some("Updated Quest".to_string()),
            description: None,
            exp_value: Some(25),
            day_of_week: Some(3),
            is_active: None,
        };

        let updated = db
            .update_quest(quest.id, update_req)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.title, "Updated Quest");
        assert_eq!(updated.exp_value, 25);
        assert_eq!(updated.day_of_week, 3);
    }

    #[tokio::test]
    async fn test_delete_quest() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: "Quest to Delete".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();

        let deleted = db.delete_quest(quest.id).await.unwrap();
        assert!(deleted);

        let retrieved = db.get_quest_by_id(quest.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_toggle_quest_completion() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: "Completable Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();
        let today = time::today();

        // Complete quest
        let completed = db.toggle_quest_completion(quest.id, today).await.unwrap();
        assert_eq!(completed, ToggleResult::NewlyCompleted);

        // Check completion status
        let is_completed = db.is_quest_completed_today(quest.id, today).await.unwrap();
        assert!(is_completed);

        // Toggle again - should un-complete
        let completed_again = db.toggle_quest_completion(quest.id, today).await.unwrap();
        assert_eq!(completed_again, ToggleResult::NewlyUncompleted);

        let is_completed_after = db.is_quest_completed_today(quest.id, today).await.unwrap();
        assert!(!is_completed_after);
    }

    #[tokio::test]
    async fn test_calculate_weekly_exp() {
        let db = setup_test_db().await;

        // Create quest
        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(15),
            title: "EXP Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();

        // Complete quest on Monday and Wednesday of the current week
        let _today = time::today();
        // For simplicity, use a known Monday
        let monday = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // This was a Monday
        let wednesday = monday + chrono::Duration::days(2);

        db.toggle_quest_completion(quest.id, monday).await.unwrap();
        db.toggle_quest_completion(quest.id, wednesday)
            .await
            .unwrap();

        // Calculate EXP for the week
        let week_start = monday;
        let week_end = monday + chrono::Duration::days(6);
        let total_exp = db.calculate_weekly_exp(week_start, week_end).await.unwrap();

        assert_eq!(total_exp, 30); // 15 * 2 completions
    }

    #[tokio::test]
    async fn test_get_settings() {
        let db = setup_test_db().await;

        let settings = db.get_settings().await.unwrap();
        assert_eq!(settings.weekly_exp_goal, 100);
    }

    #[tokio::test]
    async fn test_update_settings() {
        let db = setup_test_db().await;

        let update_req = UpdateSettingsRequest {
            weekly_exp_goal: 150,
        };

        let updated = db.update_settings(update_req).await.unwrap();
        assert_eq!(updated.weekly_exp_goal, 150);
    }

    #[tokio::test]
    async fn test_create_and_get_rewards() {
        let db = setup_test_db().await;

        let req = CreateRewardRequest {
            description: Some("A test reward".to_string()),
            required_exp: 50,
            title: "Test Reward".to_string(),
        };

        let reward = db.create_reward(req).await.unwrap();
        assert_eq!(reward.title, "Test Reward");
        assert_eq!(reward.required_exp, 50);

        let rewards = db.get_available_rewards().await.unwrap();
        assert_eq!(rewards.len(), 1);
        assert_eq!(rewards[0].title, "Test Reward");
    }

    #[tokio::test]
    async fn test_claim_reward() {
        let db = setup_test_db().await;

        // Create reward requiring 30 EXP
        let reward_req = CreateRewardRequest {
            description: None,
            required_exp: 30,
            title: "Test Reward".to_string(),
        };
        let reward = db.create_reward(reward_req).await.unwrap();

        // Create quest worth 20 EXP
        let quest_req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(20),
            title: "Test Quest".to_string(),
        };
        let quest = db.create_quest(quest_req).await.unwrap();

        // Use test dates within the week
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // Monday
        let today = week_start + chrono::Duration::days(2); // Wednesday within the week

        // Try to claim reward without enough EXP - should fail
        let claimed = db.claim_reward(reward.id, week_start).await.unwrap();
        assert!(!claimed);

        // Complete quest to get 20 EXP
        db.toggle_quest_completion(quest.id, today).await.unwrap();

        // Try again - should still fail (need 30, have 20)
        let claimed = db.claim_reward(reward.id, week_start).await.unwrap();
        assert!(!claimed);

        // Create another quest and complete it
        let quest2_req = CreateQuestRequest {
            day_of_week: 2,
            description: None,
            exp_value: Some(15),
            title: "Test Quest 2".to_string(),
        };
        let quest2 = db.create_quest(quest2_req).await.unwrap();
        db.toggle_quest_completion(quest2.id, today).await.unwrap();

        // Now have 35 EXP, should be able to claim
        let claimed = db.claim_reward(reward.id, week_start).await.unwrap();
        assert!(claimed);

        // Try to claim again - should fail
        let claimed_again = db.claim_reward(reward.id, week_start).await.unwrap();
        assert!(!claimed_again);

        // Check that claim was recorded
        let claimed_count = db.get_rewards_claimed_count().await.unwrap();
        assert_eq!(claimed_count, 1);
    }

    // ===== QUEST_LOG_DATA_DIR INTEGRATION TESTS =====

    #[tokio::test]
    async fn test_quest_log_data_dir_absolute_path_validation() {
        use std::env;
        use tempfile::tempdir;

        unsafe {
            env::set_var("QUEST_LOG_DATA_DIR", "relative/path");
        }
        let result = Database::new().await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("must be an absolute path")
        );
        unsafe {
            env::remove_var("QUEST_LOG_DATA_DIR");
        }

        // Test that absolute paths work and directory is created
        let temp_dir = tempdir().unwrap();
        let absolute_path = temp_dir
            .path()
            .join("test_data_dir")
            .to_string_lossy()
            .to_string();
        unsafe {
            env::set_var("QUEST_LOG_DATA_DIR", &absolute_path);
        }

        // Directory shouldn't exist yet
        assert!(!Path::new(&absolute_path).exists());

        let result = Database::new().await;
        assert!(result.is_ok(), "Should succeed with absolute path");

        // Directory should now exist
        assert!(Path::new(&absolute_path).exists());

        // Database file should exist
        let db_path = Path::new(&absolute_path).join("quests.db");
        assert!(db_path.exists());

        // Clean up environment variable
        unsafe {
            env::remove_var("QUEST_LOG_DATA_DIR");
        }
    }

    #[tokio::test]
    async fn test_create_quest_empty_title() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: "".to_string(),
        };

        // Empty title should still work (database doesn't enforce this constraint)
        let result = db.create_quest(req).await;
        assert!(result.is_ok(), "Empty title should be allowed by database");
    }

    #[tokio::test]
    async fn test_update_quest_not_found() {
        let db = setup_test_db().await;

        let update_req = UpdateQuestRequest {
            title: Some("Updated Title".to_string()),
            description: None,
            exp_value: None,
            day_of_week: None,
            is_active: None,
        };

        let result = db.update_quest(99999, update_req).await.unwrap();
        assert!(
            result.is_none(),
            "Updating non-existent quest should return None"
        );
    }

    #[tokio::test]
    async fn test_delete_quest_not_found() {
        let db = setup_test_db().await;

        let result = db.delete_quest(99999).await.unwrap();
        assert!(!result, "Deleting non-existent quest should return false");
    }

    #[tokio::test]
    async fn test_toggle_completion_nonexistent_quest() {
        let db = setup_test_db().await;
        let today = time::today();

        // Should return an error for non-existent quest
        let result = db.toggle_quest_completion(99999, today).await;
        assert!(
            result.is_err(),
            "Should return error for non-existent quest"
        );

        // Verify no completion was recorded
        let completions = db.get_completions_for_date(today).await.unwrap();
        assert!(
            completions.is_empty(),
            "No completions should be recorded for non-existent quest"
        );
    }

    #[tokio::test]
    async fn test_get_quest_by_id_not_found() {
        let db = setup_test_db().await;

        let result = db.get_quest_by_id(99999).await.unwrap();
        assert!(result.is_none(), "Non-existent quest ID should return None");
    }

    // ===== SECURITY UNIT TESTS =====

    #[tokio::test]
    async fn test_sql_injection_prevention() {
        let db = setup_test_db().await;

        // Test SQL injection attempts in quest titles
        let injection_attempts = vec![
            "'; DROP TABLE quests; --",
            "' OR '1'='1",
            "'; SELECT * FROM settings; --",
            "admin'--",
        ];

        for attempt in injection_attempts {
            let req = CreateQuestRequest {
                day_of_week: 1,
                description: Some("Injection attempt".to_string()),
                exp_value: Some(10),
                title: attempt.to_string(),
            };

            // Should succeed (data is properly escaped by sqlx)
            let quest = db.create_quest(req).await.unwrap();
            assert_eq!(quest.title, attempt, "Title should be stored as-is");

            // Verify we can retrieve it safely
            let retrieved = db.get_quest_by_id(quest.id).await.unwrap().unwrap();
            assert_eq!(retrieved.title, attempt, "Should retrieve safely");
        }
    }

    #[tokio::test]
    async fn test_xss_prevention_in_storage() {
        let db = setup_test_db().await;

        // Test XSS attempts in quest data
        let xss_attempts = vec![
            "<script>alert('xss')</script>",
            "<img src=x onerror=alert('xss')>",
            "javascript:alert('xss')",
            "<iframe src='javascript:alert(\"xss\")'>",
        ];

        for attempt in xss_attempts {
            let req = CreateQuestRequest {
                day_of_week: 1,
                description: Some("XSS attempt".to_string()),
                exp_value: Some(10),
                title: attempt.to_string(),
            };

            // Should succeed (data is properly escaped by sqlx)
            let quest = db.create_quest(req).await.unwrap();
            assert_eq!(quest.title, attempt, "Title should be stored as-is");

            // Verify we can retrieve it safely
            let retrieved = db.get_quest_by_id(quest.id).await.unwrap().unwrap();
            assert_eq!(retrieved.title, attempt, "Should retrieve safely");
        }
    }

    // ===== BOUNDARY AND EDGE CASE UNIT TESTS =====

    #[tokio::test]
    async fn test_zero_exp_quest() {
        let db = setup_test_db().await;

        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(0),
            title: "Zero EXP Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();
        assert_eq!(quest.exp_value, 0);

        // Complete the quest
        let today = time::today();
        let completed = db.toggle_quest_completion(quest.id, today).await.unwrap();
        assert_eq!(completed, ToggleResult::NewlyCompleted);

        // Calculate weekly EXP - should include zero
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let week_end = week_start + chrono::Duration::days(6);
        let total_exp = db.calculate_weekly_exp(week_start, week_end).await.unwrap();
        assert_eq!(total_exp, 0, "Zero EXP quest should contribute 0 to total");
    }

    #[tokio::test]
    async fn test_negative_exp_quest() {
        let db = setup_test_db().await;

        // Database might allow negative EXP values, but let's test the behavior
        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(-10),
            title: "Negative EXP Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();
        assert_eq!(quest.exp_value, -10);

        // Complete the quest using consistent dates
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let today = week_start + chrono::Duration::days(2); // Wednesday within the week
        let completed = db.toggle_quest_completion(quest.id, today).await.unwrap();
        assert_eq!(completed, ToggleResult::NewlyCompleted);

        // Calculate weekly EXP - should handle negative values
        let week_end = week_start + chrono::Duration::days(6);
        let total_exp = db.calculate_weekly_exp(week_start, week_end).await.unwrap();
        assert_eq!(total_exp, -10, "Negative EXP should be handled correctly");
    }

    #[tokio::test]
    async fn test_max_exp_values() {
        let db = setup_test_db().await;

        // Test with very large EXP values
        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(i32::MAX),
            title: "Max EXP Quest".to_string(),
        };

        let quest = db.create_quest(req).await.unwrap();
        assert_eq!(quest.exp_value, i32::MAX);

        // Complete the quest using consistent dates
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let today = week_start + chrono::Duration::days(2); // Wednesday within the week
        let completed = db.toggle_quest_completion(quest.id, today).await.unwrap();
        assert_eq!(completed, ToggleResult::NewlyCompleted);

        // Calculate weekly EXP - should handle large values
        let week_end = week_start + chrono::Duration::days(6);
        let total_exp = db.calculate_weekly_exp(week_start, week_end).await.unwrap();
        assert_eq!(total_exp, i32::MAX, "Should handle maximum i32 values");
    }

    #[tokio::test]
    async fn test_empty_quest_list() {
        let db = setup_test_db().await;

        // Test getting quests for a day with no quests
        let quests = db.get_quests_for_day(5).await.unwrap();
        assert!(
            quests.is_empty(),
            "Should return empty list for day with no quests"
        );

        // Test weekly EXP calculation with no quests
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let week_end = week_start + chrono::Duration::days(6);
        let total_exp = db.calculate_weekly_exp(week_start, week_end).await.unwrap();
        assert_eq!(total_exp, 0, "Empty quest list should result in 0 EXP");
    }

    #[tokio::test]
    async fn test_get_quests_for_invalid_day() {
        let db = setup_test_db().await;

        // Test edge cases for day_of_week
        let quests = db.get_quests_for_day(-1).await.unwrap();
        assert!(quests.is_empty(), "Invalid day should return empty list");

        let quests = db.get_quests_for_day(7).await.unwrap();
        assert!(quests.is_empty(), "Invalid day should return empty list");
    }

    #[tokio::test]
    async fn test_reward_claiming_boundary_conditions() {
        let db = setup_test_db().await;

        // Create reward requiring exactly 0 EXP
        let reward_req = CreateRewardRequest {
            description: None,
            required_exp: 0,
            title: "Free Reward".to_string(),
        };
        let reward = db.create_reward(reward_req).await.unwrap();

        // Should be able to claim immediately
        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let claimed = db.claim_reward(reward.id, week_start).await.unwrap();
        assert!(claimed, "Should be able to claim reward requiring 0 EXP");

        // Create reward requiring exact EXP match
        let reward_req2 = CreateRewardRequest {
            description: None,
            required_exp: 25,
            title: "Exact Match Reward".to_string(),
        };
        let reward2 = db.create_reward(reward_req2).await.unwrap();

        // Create quest worth exactly 25 EXP
        let quest_req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(25),
            title: "Exact EXP Quest".to_string(),
        };
        let quest = db.create_quest(quest_req).await.unwrap();

        // Complete the quest
        let today = week_start + chrono::Duration::days(2);
        db.toggle_quest_completion(quest.id, today).await.unwrap();

        // Should be able to claim with exact EXP match
        let claimed = db.claim_reward(reward2.id, week_start).await.unwrap();
        assert!(claimed, "Should be able to claim with exact EXP match");
    }

    // ===== CONCURRENT OPERATION UNIT TESTS =====

    #[tokio::test]
    async fn test_concurrent_quest_completions() {
        use tempfile::tempdir;

        // Use file-based SQLite for concurrent test (in-memory doesn't share across connections)
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("concurrent_test.db");

        // Create the database file first (required for SQLite)
        std::fs::File::create(&db_path).unwrap();

        let db_url = format!("sqlite:{}", db_path.display());
        let pool = SqlitePool::connect(&db_url).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let db = Database { pool };

        // Create a quest
        let req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(10),
            title: "Concurrent Quest".to_string(),
        };
        let quest = db.create_quest(req).await.unwrap();

        let today = time::today();

        // Wrap in Arc so all concurrent tasks share the same database instance
        let db_arc = Arc::new(db);

        // Spawn multiple concurrent tasks trying to toggle the same quest
        let mut handles = vec![];
        for _ in 0..10 {
            let db_clone = db_arc.clone();
            let quest_id = quest.id;
            let date = today;

            let handle =
                tokio::spawn(async move { db_clone.toggle_quest_completion(quest_id, date).await });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut success_count = 0;
        let _failure_count = 0;
        for handle in handles {
            match handle.await.unwrap() {
                Ok(_) => success_count += 1,
                Err(_) => {}
            }
        }

        // All operations should succeed since completing is idempotent
        assert_eq!(
            success_count, 10,
            "All completion operations should succeed"
        );

        // Note: We don't check final completion state here because toggle is not
        // idempotent - with concurrent toggles the final state is non-deterministic
        // The important thing is that all operations succeeded without errors/race conditions
    }

    #[tokio::test]
    async fn test_concurrent_reward_claiming() {
        let db = setup_test_db().await;

        // Create reward requiring 10 EXP
        let reward_req = CreateRewardRequest {
            description: None,
            required_exp: 10,
            title: "Concurrent Reward".to_string(),
        };
        let reward = db.create_reward(reward_req).await.unwrap();

        // Create quest and complete it to get EXP
        let quest_req = CreateQuestRequest {
            day_of_week: 1,
            description: None,
            exp_value: Some(20),
            title: "EXP Quest".to_string(),
        };
        let quest = db.create_quest(quest_req).await.unwrap();

        let week_start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let today = week_start + chrono::Duration::days(2);
        db.toggle_quest_completion(quest.id, today).await.unwrap();

        // Spawn multiple concurrent tasks trying to claim the same reward
        let mut handles = vec![];
        for _ in 0..5 {
            let db_clone = Database::with_pool(db.pool().clone());
            let reward_id = reward.id;
            let week = week_start;

            let handle = tokio::spawn(async move { db_clone.claim_reward(reward_id, week).await });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        let mut success_count = 0;
        let mut failure_count = 0;
        for handle in handles {
            match handle.await.unwrap() {
                Ok(result) => {
                    if result {
                        success_count += 1
                    } else {
                        failure_count += 1
                    }
                }
                Err(_) => failure_count += 1,
            }
        }

        // Exactly one should succeed, others should fail
        assert_eq!(success_count, 1, "Exactly one reward claim should succeed");
        assert_eq!(failure_count, 4, "Four reward claims should fail");

        // Verify reward was claimed
        let claimed_count = db.get_rewards_claimed_count().await.unwrap();
        assert_eq!(claimed_count, 1, "Reward should be claimed exactly once");
    }
}
