use chrono::{NaiveDate, Utc};
use sqlx::SqlitePool;
use std::env;

use crate::models::{
    CreateQuestRequest, CreateRewardRequest, Quest, QuestCompletion, Reward, Settings,
    UpdateQuestRequest, UpdateSettingsRequest,
};

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Create a new database connection pool
    pub async fn new() -> Result<Self, sqlx::Error> {
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "quests.db".to_string());

        let pool = SqlitePool::connect(&database_url).await?;

        Ok(Self { pool })
    }

    /// Create a new database instance with an existing pool (for testing)
    pub fn with_pool(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get the underlying pool (for handlers that need direct access)
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Run database migrations
    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    // Quest operations
    pub async fn get_quests_for_day(&self, day_of_week: i32) -> Result<Vec<Quest>, sqlx::Error> {
        sqlx::query_as::<_, Quest>(
            "SELECT * FROM quests WHERE day_of_week = ? AND is_active = TRUE ORDER BY created_at",
        )
        .bind(day_of_week)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_quest_by_id(&self, id: i64) -> Result<Option<Quest>, sqlx::Error> {
        sqlx::query_as::<_, Quest>("SELECT * FROM quests WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn create_quest(&self, req: CreateQuestRequest) -> Result<Quest, sqlx::Error> {
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

        Ok(quest)
    }

    pub async fn update_quest(
        &self,
        id: i64,
        req: UpdateQuestRequest,
    ) -> Result<Option<Quest>, sqlx::Error> {
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
        self.get_quest_by_id(id).await
    }

    pub async fn delete_quest(&self, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM quests WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // Quest completion operations
    pub async fn get_completions_for_date(
        &self,
        date: NaiveDate,
    ) -> Result<Vec<QuestCompletion>, sqlx::Error> {
        sqlx::query_as::<_, QuestCompletion>(
            "SELECT * FROM quest_completions WHERE completed_date = ?",
        )
        .bind(date)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn is_quest_completed_today(
        &self,
        quest_id: i64,
        today: NaiveDate,
    ) -> Result<bool, sqlx::Error> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM quest_completions WHERE quest_id = ? AND completed_date = ?",
        )
        .bind(quest_id)
        .bind(today)
        .fetch_one(&self.pool)
        .await?;

        Ok(count.0 > 0)
    }

    pub async fn toggle_quest_completion(
        &self,
        quest_id: i64,
        date: NaiveDate,
    ) -> Result<bool, sqlx::Error> {
        // Check if already completed
        let exists = self.is_quest_completed_today(quest_id, date).await?;

        if exists {
            // Remove completion
            sqlx::query("DELETE FROM quest_completions WHERE quest_id = ? AND completed_date = ?")
                .bind(quest_id)
                .bind(date)
                .execute(&self.pool)
                .await?;
            Ok(false) // Now incomplete
        } else {
            // Add completion
            sqlx::query("INSERT INTO quest_completions (quest_id, completed_date) VALUES (?, ?)")
                .bind(quest_id)
                .bind(date)
                .execute(&self.pool)
                .await?;
            Ok(true) // Now complete
        }
    }

    // Settings operations
    pub async fn get_settings(&self) -> Result<Settings, sqlx::Error> {
        sqlx::query_as::<_, Settings>("SELECT * FROM settings WHERE id = 1")
            .fetch_one(&self.pool)
            .await
    }

    pub async fn update_settings(
        &self,
        req: UpdateSettingsRequest,
    ) -> Result<Settings, sqlx::Error> {
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
    pub async fn get_available_rewards(&self) -> Result<Vec<Reward>, sqlx::Error> {
        sqlx::query_as::<_, Reward>(
            "SELECT * FROM rewards WHERE is_active = TRUE ORDER BY required_exp",
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn create_reward(&self, req: CreateRewardRequest) -> Result<Reward, sqlx::Error> {
        let now = Utc::now();
        sqlx::query_as::<_, Reward>(
            "INSERT INTO rewards (title, description, required_exp, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?) RETURNING *",
        )
        .bind(&req.title)
        .bind(&req.description)
        .bind(req.required_exp)
        .bind(now)
        .bind(now)
        .fetch_one(&self.pool)
        .await
    }

    // Statistics and calculations
    pub async fn calculate_weekly_exp(
        &self,
        week_start: NaiveDate,
        week_end: NaiveDate,
    ) -> Result<i32, sqlx::Error> {
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

    pub async fn get_total_exp_earned(&self) -> Result<i32, sqlx::Error> {
        let result: (i32,) = sqlx::query_as(
            "SELECT COALESCE(SUM(q.exp_value), 0) as total_exp
             FROM quest_completions qc
             JOIN quests q ON qc.quest_id = q.id",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.0)
    }

    pub async fn get_rewards_claimed_count(&self) -> Result<i32, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM reward_claims")
            .fetch_one(&self.pool)
            .await?;

        Ok(result.0 as i32)
    }

    // Reward claiming logic
    pub async fn claim_reward(
        &self,
        reward_id: i64,
        week_start: NaiveDate,
    ) -> Result<bool, sqlx::Error> {
        // Get the reward
        let reward =
            sqlx::query_as::<_, Reward>("SELECT * FROM rewards WHERE id = ? AND is_active = TRUE")
                .bind(reward_id)
                .fetch_optional(&self.pool)
                .await?;

        let reward = match reward {
            Some(r) => r,
            None => return Ok(false), // Reward not found or inactive
        };

        // Calculate user's weekly EXP
        let week_end = week_start + chrono::Duration::days(6);
        let weekly_exp = self.calculate_weekly_exp(week_start, week_end).await?;

        // Check if user has enough EXP
        if weekly_exp < reward.required_exp {
            return Ok(false);
        }

        // Check if reward already claimed this week
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
            return Ok(false); // Already claimed this week
        }

        // Claim the reward
        let claimed_date = week_start + chrono::Duration::days(6); // End of the week being claimed
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
    use chrono::{NaiveDate, Utc};
    use sqlx::SqlitePool;

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
            title: "Test Quest".to_string(),
            description: Some("A test quest".to_string()),
            exp_value: Some(20),
            day_of_week: 1,
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
            title: "Monday Quest".to_string(),
            description: None,
            exp_value: Some(10),
            day_of_week: 1,
        };
        let req2 = CreateQuestRequest {
            title: "Tuesday Quest".to_string(),
            description: None,
            exp_value: Some(15),
            day_of_week: 2,
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
            title: "Original Quest".to_string(),
            description: None,
            exp_value: Some(10),
            day_of_week: 1,
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
            title: "Quest to Delete".to_string(),
            description: None,
            exp_value: Some(10),
            day_of_week: 1,
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
            title: "Completable Quest".to_string(),
            description: None,
            exp_value: Some(10),
            day_of_week: 1,
        };

        let quest = db.create_quest(req).await.unwrap();
        let today = Utc::now().date_naive();

        // Complete quest
        let completed = db.toggle_quest_completion(quest.id, today).await.unwrap();
        assert!(completed);

        // Check completion status
        let is_completed = db.is_quest_completed_today(quest.id, today).await.unwrap();
        assert!(is_completed);

        // Toggle back to incomplete
        let uncompleted = db.toggle_quest_completion(quest.id, today).await.unwrap();
        assert!(!uncompleted);

        let is_completed_after = db.is_quest_completed_today(quest.id, today).await.unwrap();
        assert!(!is_completed_after);
    }

    #[tokio::test]
    async fn test_calculate_weekly_exp() {
        let db = setup_test_db().await;

        // Create quest
        let req = CreateQuestRequest {
            title: "EXP Quest".to_string(),
            description: None,
            exp_value: Some(15),
            day_of_week: 1,
        };

        let quest = db.create_quest(req).await.unwrap();

        // Complete quest on Monday and Wednesday of the current week
        let today = Utc::now().date_naive();
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
            title: "Test Reward".to_string(),
            description: Some("A test reward".to_string()),
            required_exp: 50,
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
            title: "Test Reward".to_string(),
            description: None,
            required_exp: 30,
        };
        let reward = db.create_reward(reward_req).await.unwrap();

        // Create quest worth 20 EXP
        let quest_req = CreateQuestRequest {
            title: "Test Quest".to_string(),
            description: None,
            exp_value: Some(20),
            day_of_week: 1,
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
            title: "Test Quest 2".to_string(),
            description: None,
            exp_value: Some(15),
            day_of_week: 2,
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
}
