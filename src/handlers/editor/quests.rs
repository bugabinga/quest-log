//! Editor quest CRUD handlers

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
use crate::models::{CreateQuestRequest, UpdateQuestRequest};
use crate::state::AppState;

use super::auth::extract_and_validate_session;

const DAY_NAMES: &[&str] = &[
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_quests_table(quests: &[crate::models::Quest]) -> String {
    let mut html = String::new();

    html.push_str(r#"<table class="editor-table"><thead><tr><th>Title</th><th>EXP</th><th>Day</th><th>Status</th><th>Actions</th></tr></thead><tbody>"#);

    for quest in quests {
        let row_class = if quest.is_active { "" } else { "inactive-row" };
        let status_badge = if quest.is_active {
            r#"<span class="status-badge status-badge--active">Active</span>"#
        } else {
            r#"<span class="status-badge status-badge--inactive">Inactive</span>"#
        };

        let day_name = quest
            .day_of_week
            .try_into()
            .ok()
            .and_then(|idx: usize| DAY_NAMES.get(idx))
            .unwrap_or(&"Unknown");
        let _ = write!(
            &mut html,
            r#"<tr class="{}"><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><div class="action-buttons"><button class="editor-btn editor-btn--small" data-on:click="@get('/editor/quests/{}/edit')">Edit</button><button class="editor-btn editor-btn--danger" data-on:click="@delete('/editor/quests/{}')">Delete</button></div></td></tr>"#,
            row_class,
            escape_html(&quest.title),
            quest.exp_value,
            day_name,
            status_badge,
            quest.id,
            quest.id
        );
    }

    if quests.is_empty() {
        html.push_str(r#"<tr><td colspan="5" class="empty-message">No quests yet. Add your first quest!</td></tr>"#);
    }

    html.push_str("</tbody></table>");
    html
}

async fn process_multipart_quest_fields(
    multipart: &mut Multipart,
) -> Result<
    (
        String,
        Option<String>,
        Option<i32>,
        i32,
        Option<Vec<u8>>,
        Option<String>,
    ),
    AppError,
> {
    let mut title = String::new();
    let mut description: Option<String> = None;
    let mut exp_value: Option<i32> = Some(10);
    let mut day_of_week: i32 = 0;
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
            "exp_value" => {
                if let Ok(text) = field.text().await {
                    exp_value = text.parse().ok();
                }
            }
            "day_of_week" => {
                if let Ok(text) = field.text().await {
                    day_of_week = text.parse().unwrap_or(0);
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

    Ok((
        title,
        description,
        exp_value,
        day_of_week,
        image_data,
        image_content_type,
    ))
}

/// Create a new quest
///
/// # Errors
///
/// Returns an error if session validation, database operation, or multipart form processing fails
pub async fn create_quest_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let (title, description, exp_value, day_of_week, image_data, image_content_type) =
        process_multipart_quest_fields(&mut multipart).await?;

    let quest = if image_data.is_some() {
        state
            .db
            .create_quest_with_image(
                title,
                description,
                exp_value,
                day_of_week,
                image_data,
                image_content_type,
            )
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create quest with image");
                AppError::Database(e)
            })?
    } else {
        let req = CreateQuestRequest {
            day_of_week,
            description,
            exp_value,
            title,
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
    let html = render_quests_table(&quests);

    let signals = serde_json::json!({
        "showQuestForm": false,
        "questSaved": true
    });

    let events: Vec<Event> = vec![
        PatchElements::new(html).into(),
        PatchSignals::new(signals.to_string()).into(),
    ];

    Ok(Sse::new(stream::iter(
        events.into_iter().map(Ok::<_, std::convert::Infallible>),
    )))
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
    let html = render_quests_table(&quests);

    let signals = serde_json::json!({
        "questDeleted": id
    });

    let events: Vec<Event> = vec![
        PatchElements::new(html).into(),
        PatchSignals::new(signals.to_string()).into(),
    ];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}

/// Get all quests as JSON (for tab loading)
///
/// # Errors
///
/// Returns an error if database query fails
pub async fn get_quests_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let quests = state.db.get_all_quests().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to load quests");
        AppError::Database(e)
    })?;

    Ok(axum::Json(quests))
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
    mut multipart: Multipart,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    extract_and_validate_session(&headers, &state).await?;

    let (title, description, exp_value, day_of_week, image_data, image_content_type) =
        process_multipart_quest_fields(&mut multipart).await?;

    let is_active: Option<bool> = Some(true);

    let quest = if let Some(img_data) = image_data {
        let img_content_type = image_content_type.unwrap_or_default();
        state
            .db
            .update_quest_with_image(
                id,
                Some(title),
                description,
                exp_value,
                Some(day_of_week),
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
            day_of_week: Some(day_of_week),
            description,
            exp_value,
            is_active,
            title: Some(title),
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
    let html = render_quests_table(&quests);

    let events: Vec<Event> = vec![PatchElements::new(html).into()];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}
