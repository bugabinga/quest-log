use chrono::NaiveDate;

use crate::models::HighscoreStats;

use super::Database;

fn capped_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}

impl Database {
    /// Returns all aggregate values shown on the highscore page.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn get_highscore_stats(&self) -> Result<HighscoreStats, sqlx::Error> {
        tracing::trace!("🏆 Fetching highscore aggregates");
        let result: (i64, i64, i64, i64) = sqlx::query_as(
            "SELECT
                COALESCE(SUM(q.exp_value), 0),
                COUNT(qc.id),
                (SELECT COUNT(*) FROM reward_claims),
                (SELECT COUNT(*) FROM weekly_champions)
             FROM quest_completions qc
             JOIN quests q ON q.id = qc.quest_id",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(HighscoreStats {
            total_exp: capped_i32(result.0),
            quests_completed: capped_i32(result.1),
            rewards_claimed: capped_i32(result.2),
            weekly_champions: capped_i32(result.3),
        })
    }

    /// Returns recent quest-completion counts grouped by date, newest first.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn get_recent_completion_counts(
        &self,
        limit: u32,
    ) -> Result<Vec<(NaiveDate, i32)>, sqlx::Error> {
        tracing::trace!(limit, "📜 Fetching recent completion counts");
        let rows: Vec<(NaiveDate, i64)> = sqlx::query_as(
            "SELECT completed_date, COUNT(*)
             FROM quest_completions
             GROUP BY completed_date
             ORDER BY completed_date DESC
             LIMIT ?",
        )
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(date, count)| (date, capped_i32(count)))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeDelta};
    use sqlx::SqlitePool;

    use crate::models::CreateQuestRequest;

    use super::Database;

    #[tokio::test]
    async fn highscore_queries_aggregate_in_sql_and_limit_history() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory database should open");
        let db = Database::with_pool(pool);
        db.migrate().await.expect("migrations should succeed");
        let quest = db
            .create_quest(CreateQuestRequest {
                title: "History fixture".to_string(),
                description: None,
                exp_value: Some(10),
                day_of_week: 1,
            })
            .await
            .expect("quest should be created");
        let first_date = NaiveDate::from_ymd_opt(2025, 1, 1).expect("fixture date should be valid");

        for offset in 0..40 {
            let date = first_date
                .checked_add_signed(TimeDelta::days(offset))
                .expect("fixture date should remain valid");
            db.toggle_quest_completion(quest.id, date)
                .await
                .expect("completion should be created");
        }

        let stats = db
            .get_highscore_stats()
            .await
            .expect("aggregates should load");
        assert_eq!(stats.total_exp, 400);
        assert_eq!(stats.quests_completed, 40);
        assert_eq!(stats.rewards_claimed, 0);
        assert_eq!(stats.weekly_champions, 0);

        let history = db
            .get_recent_completion_counts(30)
            .await
            .expect("history should load");
        assert_eq!(history.len(), 30);
        assert_eq!(
            history.first().copied(),
            Some((
                first_date
                    .checked_add_signed(TimeDelta::days(39))
                    .expect("fixture date should remain valid"),
                1,
            ))
        );
        assert_eq!(
            history.last().copied(),
            Some((
                first_date
                    .checked_add_signed(TimeDelta::days(10))
                    .expect("fixture date should remain valid"),
                1,
            ))
        );
    }
}
