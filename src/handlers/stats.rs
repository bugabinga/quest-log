//! Highscore/Stats handlers for the Quest Log

use axum::{extract::State, response::Html, response::IntoResponse};
use chrono::NaiveDate;
use std::collections::HashMap;

use crate::state::AppState;
use crate::ui;

#[derive(Debug)]
pub enum AppError {
    Database,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Database => Html("Database error".to_string()).into_response(),
        }
    }
}

pub struct HighscoreData {
    pub total_exp: i32,
    pub quests_completed: i32,
    pub rewards_claimed: i32,
    pub weekly_champions: i32,
    pub completions_by_date: Vec<(NaiveDate, i32)>,
}

pub async fn highscore(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;

    // Fetch all required data
    let total_exp = db
        .get_total_exp_earned()
        .await
        .map_err(|_| AppError::Database)?;

    let quests_completed = db
        .get_total_completions_count()
        .await
        .map_err(|_| AppError::Database)?;

    let rewards_claimed = db
        .get_rewards_claimed_count()
        .await
        .map_err(|_| AppError::Database)?;

    let weekly_champions = db
        .get_all_weekly_champions()
        .await
        .map_err(|_| AppError::Database)?;

    let all_completions = db
        .get_all_completions()
        .await
        .map_err(|_| AppError::Database)?;

    // Group completions by date
    let mut completions_map: HashMap<NaiveDate, i32> = HashMap::new();
    for completion in &all_completions {
        *completions_map
            .entry(completion.completed_date)
            .or_insert(0) += 1;
    }

    // Convert to sorted vector (most recent first)
    let mut completions_by_date: Vec<(NaiveDate, i32)> = completions_map.into_iter().collect();
    completions_by_date.sort_by(|a, b| b.0.cmp(&a.0));

    // Limit to last 30 days for MVP
    completions_by_date.truncate(30);

    let data = HighscoreData {
        total_exp,
        quests_completed,
        rewards_claimed,
        weekly_champions: weekly_champions.len() as i32,
        completions_by_date,
    };

    let html = ui::highscore::highscore_page(data);
    Ok(Html(html.into_string()).into_response())
}
