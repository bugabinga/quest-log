//! Bounty and reward handlers for the Quest Log

use std::convert::Infallible;

use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use chrono::Datelike;
use datastar::axum::ReadSignals;
use futures::Stream;
use serde::Deserialize;

use crate::extractors::Timezone;
use crate::handlers::AppError;
use crate::models::ClaimState;
use crate::state::AppState;
use crate::time;
use crate::ui;
use tracing::instrument;

use super::ServerMessage;

/// Request to claim a weekly reward
#[derive(Deserialize)]
pub struct ClaimRewardRequest {
    /// Client identifier for SSE targeting
    pub client_id: Option<String>,
    /// The reward to claim
    pub reward_id: i64,
}

/// Get bounty page
///
/// # Errors
///
/// Returns an error if database query fails
#[instrument(name = "🏴‍☠️ GET /bounty", skip(state, headers))]
pub async fn bounty(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    tracing::debug!("🏴‍☠️ GET /bounty request received");
    bounty_handler(state, headers).await
}

async fn bounty_handler(
    state: AppState,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let db = &state.db;
    let tz = Timezone::from_headers(&headers);
    let today = time::today_with_timezone(tz.as_deref());
    let (week_start, week_end) = time::get_week_bounds(today);

    let week_exp = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "⚠️ Failed to calculate weekly EXP");
            0
        });

    let rewards = db
        .get_weekly_reward_status(week_start, today)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "⚠️ Failed to get reward status");
            Vec::default()
        });

    let all_rewards_claimed =
        !rewards.is_empty() && rewards.iter().all(|r| r.state == ClaimState::Claimed);

    let html = ui::bounty::bounty_page(week_exp, &rewards, all_rewards_claimed);

    Ok(axum::response::Html(html.into_string()).into_response())
}

/// Claim a reward
///
/// # Errors
///
/// Returns an error if database operation fails
#[instrument(name = "🏆 claim_reward", skip(state, request, headers))]
pub async fn claim_reward(
    State(state): State<AppState>,
    headers: HeaderMap,
    ReadSignals(request): ReadSignals<ClaimRewardRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    let db = &state.db;
    let bcast = state.bcast.clone();
    let tz = Timezone::from_headers(&headers);
    let today = time::today_with_timezone(tz.as_deref());
    let reward_id = request.reward_id;

    tracing::debug!(reward_id, "🏆 Claim reward request received");

    let (week_start, week_end) = time::get_week_bounds(today);

    let is_sunday = today.weekday().num_days_from_sunday() == 0;
    if !is_sunday {
        tracing::warn!(reward_id, "Claim attempted on non-Sunday");
        return Err(AppError::ValidationError(
            "Claim attempted on non-Sunday".to_string(),
        ));
    }

    if today < week_start || today > week_end {
        tracing::warn!(reward_id, "Claim attempted outside current week");
        return Err(AppError::ValidationError(
            "Claim attempted outside current week".to_string(),
        ));
    }

    let claim_result = db
        .claim_reward_for_week(reward_id, week_start)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, reward_id, "💥 Database error claiming reward");
            AppError::Database(e)
        })?;

    if !claim_result {
        tracing::warn!(
            reward_id,
            "Reward claim failed - either insufficient EXP or already claimed"
        );
        return Err(AppError::ValidationError(
            "Reward claim failed - either insufficient EXP or already claimed".to_string(),
        ));
    }

    let rewards = db
        .get_weekly_reward_status(week_start, today)
        .await
        .unwrap_or_default();

    let week_exp = db
        .calculate_weekly_exp(week_start, week_end)
        .await
        .unwrap_or(0);

    let all_rewards_claimed =
        !rewards.is_empty() && rewards.iter().all(|r| r.state == ClaimState::Claimed);

    let rewards_html =
        ui::fragments::weekly_rewards::weekly_rewards(week_exp, &rewards, all_rewards_claimed)
            .into_string();

    if all_rewards_claimed && let Err(e) = db.create_weekly_champion(week_start).await {
        tracing::warn!(error = %e, "Failed to create weekly champion");
    }

    let signals_json = serde_json::json!({
        "rewardClaimed": reward_id,
        "rewards": rewards,
        "weekExp": week_exp,
        "allRewardsClaimed": all_rewards_claimed
    });

    let origin = request.client_id.clone();
    if let Err(e) = bcast.send(ServerMessage::Elements(
        rewards_html.clone(),
        origin.clone(),
    )) {
        tracing::warn!(error = %e, "Failed to broadcast elements");
    }
    if let Err(e) = bcast.send(ServerMessage::Signals(signals_json.to_string(), origin)) {
        tracing::warn!(error = %e, "Failed to broadcast signals");
    }

    let events: Vec<Event> = vec![
        datastar::patch_elements::PatchElements::new(rewards_html)
            .use_view_transition(true)
            .into(),
        datastar::patch_signals::PatchSignals::new(signals_json.to_string()).into(),
    ];

    let stream = futures::stream::iter(events.into_iter().map(Ok));
    Ok(Sse::new(stream))
}
