//! Editor reward CRUD handlers

use std::fmt::Write;

use axum::{
    extract::Multipart,
    extract::Path,
    extract::State,
    http::HeaderMap,
    response::IntoResponse,
    response::sse::{Event, Sse},
};
use datastar::patch_elements::PatchElements;
use datastar::patch_signals::PatchSignals;
use futures::stream::{self, Stream};

use crate::database::SetOrRemove;
use crate::handlers::AppError;
use crate::models::{CreateRewardRequest, UpdateRewardRequest};
use crate::state::AppState;

use super::auth::extract_and_validate_session;

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_rewards_table(rewards: &[crate::models::Reward]) -> String {
    let mut html = String::new();

    html.push_str(r#"<table class="editor-table"><thead><tr><th>Title</th><th>Required EXP</th><th>Status</th><th>Actions</th></tr></thead><tbody>"#);

    for reward in rewards {
        let row_class = if reward.is_active { "" } else { "inactive-row" };
        let status_badge = if reward.is_active {
            r#"<span class="status-badge status-badge--active">Active</span>"#
        } else {
            r#"<span class="status-badge status-badge--inactive">Inactive</span>"#
        };

        let _ = write!(
            &mut html,
            r#"<tr class="{}"><td>{}</td><td>{}</td><td>{}</td><td><div class="action-buttons"><button class="editor-btn editor-btn--small" data-on:click="@get('/editor/rewards/{}/edit')">Edit</button><button class="editor-btn editor-btn--danger" data-on:click="@delete('/editor/rewards/{}')">Delete</button></div></td></tr>"#,
            row_class,
            escape_html(&reward.title),
            reward.required_exp,
            status_badge,
            reward.id,
            reward.id
        );
    }

    if rewards.is_empty() {
        html.push_str(r#"<tr><td colspan="4" class="empty-message">No rewards yet. Add your first reward!</td></tr>"#);
    }

    html.push_str("</tbody></table>");
    html
}

/// Create a new reward
///
/// # Errors
///
/// Returns an error if session validation, database operation, or multipart form processing fails
pub async fn create_reward_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let mut title = String::new();
    let mut description: Option<String> = None;
    let mut required_exp: i32 = 50;
    let mut image_data: Option<Vec<u8>> = None;
    let mut image_content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::ValidationError("Validation failed".to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "title" => {
                title = field
                    .text()
                    .await
                    .map_err(|_| AppError::ValidationError("Validation failed".to_string()))?;
            }
            "description" => {
                description = Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::ValidationError("Validation failed".to_string()))?,
                );
            }
            "required_exp" => {
                if let Ok(text) = field.text().await {
                    required_exp = text.parse().unwrap_or(50);
                }
            }
            "image" => {
                let content_type = field.content_type().map(ToString::to_string);
                if let Ok(data) = field.bytes().await
                    && !data.is_empty()
                {
                    image_data = Some(data.to_vec());
                    image_content_type = content_type;
                }
            }
            _ => {}
        }
    }

    if title.is_empty() {
        return Err(AppError::ValidationError("Validation failed".to_string()));
    }

    let reward = if image_data.is_some() {
        state
            .db
            .create_reward_with_image(
                title,
                description,
                required_exp,
                image_data,
                image_content_type,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create reward with image");
                AppError::Database(e)
            })?
    } else {
        let req = CreateRewardRequest {
            description,
            required_exp,
            title,
        };
        state.db.create_reward(req).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to create reward");
            AppError::Database(e)
        })?
    };

    tracing::info!(reward_id = reward.id, "Reward created");

    let rewards = state
        .db
        .get_all_rewards()
        .await
        .map_err(AppError::Database)?;
    let html = render_rewards_table(&rewards);

    let signals = serde_json::json!({
        "showRewardForm": false,
        "rewardSaved": true
    });

    let events: Vec<Event> = vec![
        PatchElements::new(html).into(),
        PatchSignals::new(signals.to_string()).into(),
    ];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}

/// Delete a reward
///
/// # Errors
///
/// Returns an error if session validation, database operation, or reward not found
pub async fn delete_reward_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let deleted = state.db.delete_reward(id).await.map_err(|e| {
        tracing::error!(error = %e, reward_id = id, "Failed to delete reward");
        AppError::Database(e)
    })?;

    if !deleted {
        tracing::warn!(reward_id = id, "Reward not found for deletion");
        return Err(AppError::NotFound);
    }

    tracing::info!(reward_id = id, "Reward deleted");

    let rewards = state
        .db
        .get_all_rewards()
        .await
        .map_err(AppError::Database)?;
    let html = render_rewards_table(&rewards);

    let signals = serde_json::json!({
        "rewardDeleted": id
    });

    let events: Vec<Event> = vec![
        PatchElements::new(html).into(),
        PatchSignals::new(signals.to_string()).into(),
    ];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}

/// Get all rewards as JSON
///
/// # Errors
///
/// Returns an error if database query fails
pub async fn get_rewards_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let rewards = state.db.get_all_rewards().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to load rewards");
        AppError::Database(e)
    })?;

    Ok(axum::Json(rewards))
}

/// Update a reward
///
/// # Errors
///
/// Returns an error if session validation, database operation, or reward not found
pub async fn update_reward_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let mut title: Option<String> = None;
    let mut description: Option<String> = None;
    let mut required_exp: Option<i32> = None;
    let mut is_active: Option<bool> = None;
    let mut image_data: Option<Vec<u8>> = None;
    let mut image_content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError::ValidationError("Validation failed".to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "title" => {
                title = Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::ValidationError("Validation failed".to_string()))?,
                );
            }
            "description" => {
                description = Some(
                    field
                        .text()
                        .await
                        .map_err(|_| AppError::ValidationError("Validation failed".to_string()))?,
                );
            }
            "required_exp" => {
                if let Ok(text) = field.text().await {
                    required_exp = text.parse().ok();
                }
            }
            "is_active" => {
                if let Ok(text) = field.text().await {
                    is_active = text.parse().ok();
                }
            }
            "image" => {
                let content_type = field.content_type().map(ToString::to_string);
                if let Ok(data) = field.bytes().await
                    && !data.is_empty()
                {
                    image_data = Some(data.to_vec());
                    image_content_type = content_type;
                }
            }
            _ => {}
        }
    }

    let reward = if let Some(img_data) = image_data {
        let img_content_type = image_content_type.unwrap_or_default();
        state
            .db
            .update_reward_with_image(
                id,
                title,
                description,
                required_exp,
                is_active,
                SetOrRemove::set(img_data),
                SetOrRemove::set(img_content_type),
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, reward_id = id, "Failed to update reward with image");
                AppError::Database(e)
            })?
    } else {
        let req = UpdateRewardRequest {
            description,
            is_active,
            required_exp,
            title,
        };
        state.db.update_reward(id, req).await.map_err(|e| {
            tracing::error!(error = %e, reward_id = id, "Failed to update reward");
            AppError::Database(e)
        })?
    };

    if reward.is_none() {
        tracing::warn!(reward_id = id, "Reward not found for update");
        return Err(AppError::NotFound);
    }

    tracing::info!(reward_id = id, "Reward updated");

    let rewards = state
        .db
        .get_all_rewards()
        .await
        .map_err(AppError::Database)?;
    let html = render_rewards_table(&rewards);

    let events: Vec<Event> = vec![PatchElements::new(html).into()];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}
