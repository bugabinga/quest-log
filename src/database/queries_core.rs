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
    #[must_use]
    #[allow(dead_code, reason = "test only")]
    #[cfg(feature = "test-utils")]
    /// Creates a new database instance with an existing pool.
    pub fn with_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    #[must_use]
    #[allow(dead_code, reason = "test only")]
    #[cfg(feature = "test-utils")]
    /// Returns a reference to the underlying connection pool.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Get completions for a specific date.
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` if the database query fails.
    #[allow(dead_code, reason = "test only")]
    #[cfg(feature = "test-utils")]
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
}
