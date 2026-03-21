//! Editor reward CRUD handlers

use std::fmt::Write;

use axum::{
    extract::Path,
    extract::State,
    http::HeaderMap,
    response::IntoResponse,
    response::sse::{Event, Sse},
};
use datastar::axum::ReadSignals;
use datastar::patch_elements::PatchElements;
use datastar::patch_signals::PatchSignals;
use futures::Stream;

use crate::database::SetOrRemove;
use crate::handlers::AppError;
use crate::models::{CreateRewardRequest, FileUpload, RewardJsonRequest, UpdateRewardRequest};
use crate::sse_response;
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
            r#"<tr id="reward-row-{}" class="{}" style="view-transition-name: editor-row;"><td>{}</td><td>{}</td><td>{}</td><td><div class="action-buttons"><button class="editor-btn editor-btn--small" data-on:click="@get('/editor/rewards/{}/edit')">Edit</button><button class="editor-btn editor-btn--danger" data-on:click="@delete('/editor/rewards/{}')">Delete</button></div></td></tr>"#,
            reward.id,
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

struct RewardData {
    title: String,
    description: Option<String>,
    required_exp: i32,
    image_data: Option<Vec<u8>>,
    image_content_type: Option<String>,
}

fn extract_reward_from_request(req: RewardJsonRequest) -> Result<RewardData, AppError> {
    let title = req.reward_title.trim();
    if title.is_empty() {
        return Err(AppError::ValidationError("Title is required".into()));
    }

    if let Some(file) = req.reward_image.first() {
        if file.is_too_large() {
            return Err(AppError::ValidationError(
                "Image file is too large (max 5MB)".into(),
            ));
        }
        tracing::debug!(filename = %file.filename(), mime = %file.mime, "Uploading image file");
    }

    let (image_data, image_content_type) = req
        .reward_image
        .first()
        .and_then(FileUpload::decode)
        .unzip();

    Ok(RewardData {
        title: title.to_string(),
        description: req.reward_description,
        required_exp: req.reward_required_exp,
        image_data,
        image_content_type,
    })
}

/// Create a new reward
///
/// # Errors
///
/// Returns an error if session validation, database operation, or JSON processing fails
pub async fn create_reward_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    ReadSignals(req): ReadSignals<RewardJsonRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let data = extract_reward_from_request(req)?;

    let reward = if data.image_data.is_some() {
        state
            .db
            .create_reward_with_image(
                data.title,
                data.description,
                data.required_exp,
                data.image_data,
                data.image_content_type,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create reward with image");
                AppError::Database(e)
            })?
    } else {
        let req = CreateRewardRequest {
            description: data.description,
            required_exp: data.required_exp,
            title: data.title,
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

    let signals = r#"{"_showRewardForm": false, "_rewardTitle": "", "_rewardDescription": "", "_rewardRequiredExp": 50, "_rewardImage": []}"#;

    let events: Vec<Event> = vec![
        PatchElements::new(html).use_view_transition(true).into(),
        PatchSignals::new(signals).into(),
    ];

    Ok(sse_response!(events))
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
        PatchElements::new(html).use_view_transition(true).into(),
        PatchSignals::new(signals.to_string()).into(),
    ];

    Ok(sse_response!(events))
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

/// Populate form for editing a reward
///
/// # Errors
///
/// Returns an error if session validation, database operation, or reward not found
pub async fn edit_reward_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let reward = state
        .db
        .get_reward_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, reward_id = id, "Failed to load reward for edit");
            AppError::Database(e)
        })?
        .ok_or(AppError::NotFound)?;

    let signals = serde_json::json!({
        "_showRewardForm": true,
        "_editingRewardId": id,
        "_rewardTitle": reward.title,
        "_rewardDescription": reward.description,
        "_rewardRequiredExp": reward.required_exp,
        "_rewardImage": []
    });

    let events: Vec<Event> = vec![PatchSignals::new(signals.to_string()).into()];
    Ok(sse_response!(events))
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
    ReadSignals(req): ReadSignals<RewardJsonRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let data = extract_reward_from_request(req)?;

    let is_active: Option<bool> = Some(true);

    let reward = if let Some(img_data) = data.image_data {
        let img_content_type = data.image_content_type.unwrap_or_default();
        state
            .db
            .update_reward_with_image(
                id,
                Some(data.title),
                data.description,
                Some(data.required_exp),
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
            description: data.description,
            is_active,
            required_exp: Some(data.required_exp),
            title: Some(data.title),
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

    let signals = r#"{"_showRewardForm": false, "_editingRewardId": null, "_rewardTitle": "", "_rewardDescription": "", "_rewardRequiredExp": 50, "_rewardImage": []}"#;

    let events: Vec<Event> = vec![
        PatchElements::new(html).use_view_transition(true).into(),
        PatchSignals::new(signals).into(),
    ];

    Ok(sse_response!(events))
}
