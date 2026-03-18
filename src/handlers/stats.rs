//! Highscore/Stats handlers for the Quest Log

use axum::{extract::State, response::Html, response::IntoResponse};
use chrono::NaiveDate;
use std::collections::HashMap;

use crate::handlers::AppError;
use crate::state::AppState;
use crate::ui;

/// Highscore statistics data
pub struct HighscoreData {
    /// Total experience points earned
    pub total_exp: i32,
    /// Number of quests completed
    pub quests_completed: i32,
    /// Number of rewards claimed
    pub rewards_claimed: i32,
    /// Number of weekly champions
    pub weekly_champions: i32,
    /// Quest completions grouped by date
    pub completions_by_date: Vec<(NaiveDate, i32)>,
}

/// Get highscore statistics
///
/// # Errors
///
/// Returns an error if database queries fail
pub async fn highscore(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;

    // Fetch all required data
    let total_exp = db
        .get_total_exp_earned()
        .await
        .map_err(AppError::Database)?;

    let quests_completed = db
        .get_total_completions_count()
        .await
        .map_err(AppError::Database)?;

    let rewards_claimed = db
        .get_rewards_claimed_count()
        .await
        .map_err(AppError::Database)?;

    let weekly_champions = db
        .get_all_weekly_champions()
        .await
        .map_err(AppError::Database)?;

    let all_completions = db.get_all_completions().await.map_err(AppError::Database)?;

    // Group completions by date
    let mut completions_map: HashMap<NaiveDate, i32> = HashMap::new();
    for completion in &all_completions {
        let count = completions_map
            .entry(completion.completed_date)
            .or_insert(0);
        *count = count.saturating_add(1);
    }

    // Convert to sorted vector (most recent first)
    let mut completions_by_date: Vec<(NaiveDate, i32)> = completions_map.into_iter().collect();
    completions_by_date.sort_by_key(|b| std::cmp::Reverse(b.0));

    // Limit to last 30 days for MVP
    completions_by_date.truncate(30);

    let data = HighscoreData {
        total_exp,
        quests_completed,
        rewards_claimed,
        weekly_champions: i32::try_from(weekly_champions.len()).unwrap_or(i32::MAX),
        completions_by_date,
    };

    let html = ui::highscore::highscore_page(&data);
    Ok(Html(html.into_string()).into_response())
}
