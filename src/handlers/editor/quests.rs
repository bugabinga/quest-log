//! Editor quest CRUD handlers

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
use crate::models::{CreateQuestRequest, QuestJsonRequest, UpdateQuestRequest};
use crate::sse_response;
use crate::state::AppState;
use crate::ui::editor::quests_table;

use super::auth::extract_and_validate_session;

struct QuestData {
    title: String,
    description: Option<String>,
    exp_value: Option<i32>,
    day_of_week: i32,
    image_data: Option<Vec<u8>>,
    image_content_type: Option<String>,
}

fn extract_quest_from_request(req: QuestJsonRequest) -> Result<QuestData, AppError> {
    let title = req.title.trim();
    if title.is_empty() {
        return Err(AppError::ValidationError("Title is required".into()));
    }

    if !(0..=6).contains(&req.day_of_week) {
        return Err(AppError::ValidationError("Day must be 0-6".into()));
    }

    if req.exp_value.is_some_and(|exp| exp < 0) {
        return Err(AppError::ValidationError("EXP must be non-negative".into()));
    }

    let image = req
        .image
        .first()
        .map(|file| {
            tracing::debug!(filename = %file.filename(), mime = %file.mime, "Uploading image file");
            file.decode()
        })
        .transpose()
        .map_err(|message| AppError::ValidationError(message.into()))?;

    let (image_data, image_content_type) = image.unzip();

    Ok(QuestData {
        title: title.to_string(),
        description: req.description,
        exp_value: req.exp_value,
        day_of_week: req.day_of_week,
        image_data,
        image_content_type,
    })
}

/// Create a new quest
///
/// # Errors
///
/// Returns an error if session validation, database operation, or JSON processing fails
pub async fn create_quest_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    ReadSignals(req): ReadSignals<QuestJsonRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let data = extract_quest_from_request(req)?;

    let quest = if data.image_data.is_some() {
        state
            .db
            .create_quest_with_image(
                data.title,
                data.description,
                data.exp_value,
                data.day_of_week,
                data.image_data,
                data.image_content_type,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create quest with image");
                AppError::Database(e)
            })?
    } else {
        let req = CreateQuestRequest {
            day_of_week: data.day_of_week,
            description: data.description,
            exp_value: data.exp_value,
            title: data.title,
        };
        state.db.create_quest(req).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to create quest");
            AppError::Database(e)
        })?
    };

    tracing::info!(quest_id = quest.id, "Quest created");

    let quests = state
        .db
        .get_all_quests()
        .await
        .map_err(AppError::Database)?;
    let html = quests_table(&quests).into_string();

    let signals = r#"{"_showQuestForm": false, "_questTitle": "", "_questDescription": "", "_questExpValue": 10, "_questDayOfWeek": 0, "_questImage": []}"#;

    let events: Vec<Event> = vec![
        PatchElements::new(html)
            .selector("#quests-table")
            .use_view_transition(true)
            .into(),
        PatchSignals::new(signals).into(),
    ];

    Ok(sse_response!(events))
}

/// Delete a quest
///
/// # Errors
///
/// Returns an error if session validation, database operation, or quest not found
pub async fn delete_quest_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let deleted = state.db.delete_quest(id).await.map_err(|e| {
        tracing::error!(error = %e, quest_id = id, "Failed to delete quest");
        AppError::Database(e)
    })?;

    if !deleted {
        tracing::warn!(quest_id = id, "Quest not found for deletion");
        return Err(AppError::NotFound);
    }

    tracing::info!(quest_id = id, "Quest deleted");

    let quests = state
        .db
        .get_all_quests()
        .await
        .map_err(AppError::Database)?;
    let html = quests_table(&quests).into_string();

    let signals = serde_json::json!({
        "questDeleted": id
    });

    let events: Vec<Event> = vec![
        PatchElements::new(html)
            .selector("#quests-table")
            .use_view_transition(true)
            .into(),
        PatchSignals::new(signals.to_string()).into(),
    ];

    Ok(sse_response!(events))
}

/// Get all quests as JSON (for tab loading)
///
/// # Errors
///
/// Returns an error if database query fails
pub async fn get_quests_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let quests = state.db.get_all_quests().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to load quests");
        AppError::Database(e)
    })?;

    Ok(axum::Json(quests))
}

/// Populate form for editing a quest
///
/// # Errors
///
/// Returns an error if session validation, database operation, or quest not found
pub async fn edit_quest_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let quest = state
        .db
        .get_quest_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, quest_id = id, "Failed to load quest for edit");
            AppError::Database(e)
        })?
        .ok_or(AppError::NotFound)?;

    let signals = serde_json::json!({
        "_showQuestForm": true,
        "_editingQuestId": id,
        "_questTitle": quest.title,
        "_questDescription": quest.description,
        "_questExpValue": quest.exp_value,
        "_questDayOfWeek": quest.day_of_week,
        "_questImage": []
    });

    let events: Vec<Event> = vec![PatchSignals::new(signals.to_string()).into()];
    Ok(sse_response!(events))
}

/// Update a quest
///
/// # Errors
///
/// Returns an error if session validation, database operation, or quest not found
pub async fn update_quest_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    ReadSignals(req): ReadSignals<QuestJsonRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let data = extract_quest_from_request(req)?;

    let is_active: Option<bool> = None;

    let quest = if let Some(img_data) = data.image_data {
        let img_content_type = data.image_content_type.unwrap_or_default();
        state
            .db
            .update_quest_with_image(
                id,
                Some(data.title),
                data.description,
                data.exp_value,
                Some(data.day_of_week),
                is_active,
                SetOrRemove::set(img_data),
                SetOrRemove::set(img_content_type),
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, quest_id = id, "Failed to update quest with image");
                AppError::Database(e)
            })?
    } else {
        let req = UpdateQuestRequest {
            day_of_week: Some(data.day_of_week),
            description: data.description,
            exp_value: data.exp_value,
            is_active,
            title: Some(data.title),
        };
        state.db.update_quest(id, req).await.map_err(|e| {
            tracing::error!(error = %e, quest_id = id, "Failed to update quest");
            AppError::Database(e)
        })?
    };

    if quest.is_none() {
        tracing::warn!(quest_id = id, "Quest not found for update");
        return Err(AppError::NotFound);
    }

    tracing::info!(quest_id = id, "Quest updated");

    let quests = state
        .db
        .get_all_quests()
        .await
        .map_err(AppError::Database)?;
    let html = quests_table(&quests).into_string();

    let signals = r#"{"_showQuestForm": false, "_editingQuestId": null, "_questTitle": "", "_questDescription": "", "_questExpValue": 10, "_questDayOfWeek": 0, "_questImage": []}"#;

    let events: Vec<Event> = vec![
        PatchElements::new(html)
            .selector("#quests-table")
            .use_view_transition(true)
            .into(),
        PatchSignals::new(signals).into(),
    ];

    Ok(sse_response!(events))
}
