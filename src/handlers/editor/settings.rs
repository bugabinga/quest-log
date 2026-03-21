//! Editor settings handlers

use axum::{
    extract::Form,
    extract::State,
    http::HeaderMap,
    response::IntoResponse,
    response::sse::{Event, Sse},
};
use datastar::patch_signals::PatchSignals;
use futures::Stream;

use crate::handlers::AppError;
use crate::models::UpdateSettingsRequest;
use crate::sse_response;
use crate::state::AppState;

use super::auth::extract_and_validate_session;

/// Get current settings
///
/// # Errors
///
/// Returns an error if database query fails
pub async fn get_settings_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let settings = state.db.get_settings().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to load settings");
        AppError::Database(e)
    })?;

    Ok(axum::Json(settings))
}

/// Update settings
///
/// # Errors
///
/// Returns an error if session validation or database operation fails
pub async fn update_settings_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(request): Form<UpdateSettingsRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let settings = state.db.update_settings(request).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to update settings");
        AppError::Database(e)
    })?;

    tracing::info!(
        weekly_exp_goal = settings.weekly_exp_goal,
        "Settings updated"
    );

    let signals = serde_json::json!({
        "settingsSaved": true
    });

    let events: Vec<Event> = vec![PatchSignals::new(signals.to_string()).into()];

    Ok(sse_response!(events))
}
