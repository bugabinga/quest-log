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

use super::{Database, SetOrRemove};

impl Database {
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

        let today_quests = self.get_quests_for_day(day_of_week).await?;
        let exp_today_max: i32 = today_quests.iter().map(|q| q.exp_value).sum();
        let quests_total = i32::try_from(today_quests.len()).unwrap_or(i32::MAX);

        let quest_ids: Vec<i64> = today_quests.iter().map(|q| q.id).collect();
        let completion_status = self.get_quests_completion_status(&quest_ids, today).await?;

        let mut exp_today = 0i32;
        let mut quests_completed = 0i32;
        for quest in &today_quests {
            if completion_status.get(&quest.id).copied().unwrap_or(false) {
                exp_today = exp_today
                    .checked_add(quest.exp_value)
                    .ok_or_else(|| sqlx::Error::Protocol("Integer overflow in exp_today".into()))?;
                quests_completed = quests_completed.checked_add(1).ok_or_else(|| {
                    sqlx::Error::Protocol("Integer overflow in quests_completed".into())
                })?;
            }
        }

        let week_exp = self.calculate_weekly_exp(week_start, week_end).await?;
        let week_exp_max_raw: (i64,) =
            sqlx::query_as("SELECT COALESCE(SUM(exp_value), 0) FROM quests WHERE is_active = TRUE")
                .fetch_one(&self.pool)
                .await?;
        let week_exp_max = i32::try_from(week_exp_max_raw.0)
            .map_err(|_| sqlx::Error::Protocol("Integer overflow in week_exp_max".into()))?;

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
    /// Claim a reward for a specific week using an explicit request-local date.
    ///
    /// Returns false if the date is not that week's Sunday, EXP is too low, or the reward is already claimed.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    ///
    /// # Panics
    ///
    /// Panics if the week end date overflows.
    pub async fn claim_reward_for_week_on(
        &self,
        reward_id: i64,
        week_start: NaiveDate,
        today: NaiveDate,
    ) -> Result<bool, sqlx::Error> {
        let week_end = week_start
            .checked_add_days(chrono::Days::new(6))
            .unwrap_or_else(|| panic!("date overflow"));
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
        match sqlx::query("INSERT INTO reward_claims (reward_id, claimed_date) VALUES (?, ?)")
            .bind(reward_id)
            .bind(claimed_date)
            .execute(&self.pool)
            .await
        {
            Ok(_) => {}
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
                return Ok(false);
            }
            Err(e) => return Err(e),
        }

        let rewards = self.get_weekly_reward_status(week_start, today).await?;
        let all_rewards_claimed = rewards
            .iter()
            .filter(|r| r.state == ClaimState::Claimed)
            .count()
            == rewards.len()
            && !rewards.is_empty();

        if all_rewards_claimed {
            // Intentionally ignore result - weekly champion creation is an optional bonus
            drop(self.create_weekly_champion(week_start).await);
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
    ///
    /// # Errors
    ///
    /// Returns `sqlx::Error` if the database query fails.
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
