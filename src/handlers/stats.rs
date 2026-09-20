//! Highscore/Stats handlers for the Quest Log

use axum::{extract::State, response::Html, response::IntoResponse};
use chrono::NaiveDate;

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
    let aggregates = state
        .db
        .get_highscore_stats()
        .await
        .map_err(AppError::Database)?;
    let completions_by_date = state
        .db
        .get_recent_completion_counts(30)
        .await
        .map_err(AppError::Database)?;

    let data = HighscoreData {
        total_exp: aggregates.total_exp,
        quests_completed: aggregates.quests_completed,
        rewards_claimed: aggregates.rewards_claimed,
        weekly_champions: aggregates.weekly_champions,
        completions_by_date,
    };

    let html = ui::highscore::highscore_page(&data);
    Ok(Html(html.into_string()).into_response())
}
