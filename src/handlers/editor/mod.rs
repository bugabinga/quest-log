//! Editor handlers for the Quest Log
//!
//! Provides HTTP handlers for the editor page, authentication, and CRUD operations.

use axum::{
    body::Body,
    extract::Multipart,
    extract::{Form, Path, State},
    http::{HeaderMap, Response, StatusCode},
    response::sse::{Event, Sse},
    response::{Html, IntoResponse},
};
use chrono::Utc;
use datastar::patch_elements::PatchElements;
use datastar::patch_signals::PatchSignals;
use futures::stream::{self, Stream};
use serde::Deserialize;

use crate::auth;
use crate::models::{
    CreateQuestRequest, CreateRewardRequest, Settings, UpdateQuestRequest, UpdateRewardRequest,
    UpdateSettingsRequest,
};
use crate::state::AppState;
use crate::ui;
use tracing::{debug, error, info, warn};

/// Get the client IP from headers
fn get_client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("x-real-ip").and_then(|v| v.to_str().ok()))
        .unwrap_or("unknown")
        .split(',')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Editor page - shows auth modal if not authenticated, otherwise shows editor
pub async fn editor_page_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, EditorError> {
    let client_ip = get_client_ip(&headers);
    debug!(ip = %client_ip, "Editor page requested");

    // For simplicity, check for session token in a custom header
    let session_token = headers
        .get("x-editor-session")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let is_authenticated = if let Some(ref token) = session_token {
        state.validate_session(token).await
    } else {
        false
    };

    if !is_authenticated {
        debug!("User not authenticated, showing login");
        let html = ui::editor::editor_page(
            false,
            "quests",
            &[],
            &[],
            &Settings {
                id: 1,
                weekly_exp_goal: 100,
                updated_at: Utc::now(),
            },
        );
        return Ok(Html(html.into_string()));
    }

    // User is authenticated, load data
    let quests = state.db.get_all_quests().await.map_err(|e| {
        error!(error = %e, "Failed to load quests");
        EditorError::Database
    })?;

    let rewards = state.db.get_all_rewards().await.map_err(|e| {
        error!(error = %e, "Failed to load rewards");
        EditorError::Database
    })?;

    let settings = state.db.get_settings().await.map_err(|e| {
        error!(error = %e, "Failed to load settings");
        EditorError::Database
    })?;

    let html = ui::editor::editor_page(true, "quests", &quests, &rewards, &settings);
    Ok(Html(html.into_string()))
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    password: String,
}

/// Process login attempt
pub async fn login_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(request): Form<LoginRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, EditorError> {
    let client_ip = get_client_ip(&headers);
    debug!(ip = %client_ip, "Login attempt");

    // Check rate limiting
    if state.login_rate_limiter.is_rate_limited(&client_ip).await {
        warn!(ip = %client_ip, "Rate limited login attempt");

        let signals = serde_json::json!({
            "loginError": "Too many login attempts. Please wait a minute.",
            "isRateLimited": true
        });

        let events: Vec<Event> = vec![PatchSignals::new(signals.to_string()).into()];
        return Ok(Sse::new(stream::iter(events.into_iter().map(Ok))));
    }

    // Get password hash from environment
    let password_hash = match auth::get_password_hash_from_env() {
        Some(h) => h,
        None => {
            error!("QUEST_LOG_EDITOR_PASSWORD_HASH not configured");
            let _ = auth::verify_password(
                &request.password,
                "$argon2id$v=19$m=65536,t=3,p=4$fake$fake",
            );

            let signals = serde_json::json!({
                "loginError": "Editor not configured. Contact administrator.",
                "isRateLimited": false
            });

            let events: Vec<Event> = vec![PatchSignals::new(signals.to_string()).into()];
            return Ok(Sse::new(stream::iter(events.into_iter().map(Ok))));
        }
    };

    // Verify password
    if !auth::verify_password(&request.password, &password_hash) {
        warn!(ip = %client_ip, "Invalid password attempt");
        state
            .login_rate_limiter
            .record_failed_attempt(&client_ip)
            .await;

        let signals = serde_json::json!({
            "loginError": "Invalid password. Please try again.",
            "isRateLimited": false
        });

        let events: Vec<Event> = vec![PatchSignals::new(signals.to_string()).into()];
        return Ok(Sse::new(stream::iter(events.into_iter().map(Ok))));
    }

    // Login successful - clear rate limit and create session
    state.login_rate_limiter.clear_attempts(&client_ip).await;
    let token = auth::generate_session_token();
    let expiry = state.create_session(token.clone()).await;
    info!(ip = %client_ip, expires = ?expiry, "Login successful");

    // Build the page HTML
    let quests = state.db.get_all_quests().await.unwrap_or_default();
    let rewards = state.db.get_all_rewards().await.unwrap_or_default();
    let settings = state.db.get_settings().await.unwrap_or(Settings {
        id: 1,
        weekly_exp_goal: 100,
        updated_at: Utc::now(),
    });

    let html = ui::editor::editor_page(true, "quests", &quests, &rewards, &settings);

    let signals = serde_json::json!({
        "isAuthenticated": true,
        "loginError": null
    });

    let combined_html = format!(
        r#"<div class="editor-wrapper">{}</div>"#,
        html.into_string()
    );

    let events: Vec<Event> = vec![
        PatchElements::new(combined_html).into(),
        PatchSignals::new(signals.to_string()).into(),
    ];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}

/// Logout handler
pub async fn logout_handler(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, EditorError> {
    let client_ip = get_client_ip(&headers);
    debug!(ip = %client_ip, "Logout requested");

    let signals = serde_json::json!({
        "isAuthenticated": false
    });

    let settings = Settings {
        id: 1,
        weekly_exp_goal: 100,
        updated_at: Utc::now(),
    };
    let html = ui::editor::editor_page(false, "quests", &[], &[], &settings);

    let events: Vec<Event> = vec![
        PatchElements::new(html.into_string()).into(),
        PatchSignals::new(signals.to_string()).into(),
    ];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}

// ===== Quest CRUD =====

/// Get all quests as JSON (for tab loading)
pub async fn get_quests_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, EditorError> {
    let quests = state.db.get_all_quests().await.map_err(|e| {
        error!(error = %e, "Failed to load quests");
        EditorError::Database
    })?;

    Ok(axum::Json(quests))
}

/// Create a new quest
pub async fn create_quest_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, EditorError> {
    let mut title = String::new();
    let mut description: Option<String> = None;
    let mut exp_value: Option<i32> = Some(10);
    let mut day_of_week: i32 = 0;
    let mut image_data: Option<Vec<u8>> = None;
    let mut image_content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| EditorError::Validation)?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "title" => {
                title = field.text().await.map_err(|_| EditorError::Validation)?;
            }
            "description" => {
                description = Some(field.text().await.map_err(|_| EditorError::Validation)?);
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
                let content_type = field.content_type().map(|s| s.to_string());
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
        return Err(EditorError::Validation);
    }

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
                error!(error = %e, "Failed to create quest with image");
                EditorError::Database
            })?
    } else {
        let req = CreateQuestRequest {
            title,
            description,
            exp_value,
            day_of_week,
        };
        state.db.create_quest(req).await.map_err(|e| {
            error!(error = %e, "Failed to create quest");
            EditorError::Database
        })?
    };

    info!(quest_id = quest.id, "Quest created");

    // Return updated quests list
    let quests = state
        .db
        .get_all_quests()
        .await
        .map_err(|_| EditorError::Database)?;
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

/// Update a quest
pub async fn update_quest_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(request): Form<UpdateQuestRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, EditorError> {
    let quest = state.db.update_quest(id, request).await.map_err(|e| {
        error!(error = %e, quest_id = id, "Failed to update quest");
        EditorError::Database
    })?;

    if quest.is_none() {
        warn!(quest_id = id, "Quest not found for update");
        return Err(EditorError::NotFound);
    }

    info!(quest_id = id, "Quest updated");

    let quests = state
        .db
        .get_all_quests()
        .await
        .map_err(|_| EditorError::Database)?;
    let html = render_quests_table(&quests);

    let events: Vec<Event> = vec![PatchElements::new(html).into()];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}

/// Delete a quest
pub async fn delete_quest_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, EditorError> {
    let deleted = state.db.delete_quest(id).await.map_err(|e| {
        error!(error = %e, quest_id = id, "Failed to delete quest");
        EditorError::Database
    })?;

    if !deleted {
        warn!(quest_id = id, "Quest not found for deletion");
        return Err(EditorError::NotFound);
    }

    info!(quest_id = id, "Quest deleted");

    let quests = state
        .db
        .get_all_quests()
        .await
        .map_err(|_| EditorError::Database)?;
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

// ===== Reward CRUD =====

/// Get all rewards as JSON
pub async fn get_rewards_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, EditorError> {
    let rewards = state.db.get_all_rewards().await.map_err(|e| {
        error!(error = %e, "Failed to load rewards");
        EditorError::Database
    })?;

    Ok(axum::Json(rewards))
}

/// Create a new reward
pub async fn create_reward_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, EditorError> {
    let mut title = String::new();
    let mut description: Option<String> = None;
    let mut required_exp: i32 = 50;
    let mut image_data: Option<Vec<u8>> = None;
    let mut image_content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| EditorError::Validation)?
    {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "title" => {
                title = field.text().await.map_err(|_| EditorError::Validation)?;
            }
            "description" => {
                description = Some(field.text().await.map_err(|_| EditorError::Validation)?);
            }
            "required_exp" => {
                if let Ok(text) = field.text().await {
                    required_exp = text.parse().unwrap_or(50);
                }
            }
            "image" => {
                let content_type = field.content_type().map(|s| s.to_string());
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
        return Err(EditorError::Validation);
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
                error!(error = %e, "Failed to create reward with image");
                EditorError::Database
            })?
    } else {
        let req = CreateRewardRequest {
            title,
            description,
            required_exp,
        };
        state.db.create_reward(req).await.map_err(|e| {
            error!(error = %e, "Failed to create reward");
            EditorError::Database
        })?
    };

    info!(reward_id = reward.id, "Reward created");

    let rewards = state
        .db
        .get_all_rewards()
        .await
        .map_err(|_| EditorError::Database)?;
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

/// Update a reward
pub async fn update_reward_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(request): Form<UpdateRewardRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, EditorError> {
    let reward = state.db.update_reward(id, request).await.map_err(|e| {
        error!(error = %e, reward_id = id, "Failed to update reward");
        EditorError::Database
    })?;

    if reward.is_none() {
        warn!(reward_id = id, "Reward not found for update");
        return Err(EditorError::NotFound);
    }

    info!(reward_id = id, "Reward updated");

    let rewards = state
        .db
        .get_all_rewards()
        .await
        .map_err(|_| EditorError::Database)?;
    let html = render_rewards_table(&rewards);

    let events: Vec<Event> = vec![PatchElements::new(html).into()];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}

/// Delete a reward
pub async fn delete_reward_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, EditorError> {
    let deleted = state.db.delete_reward(id).await.map_err(|e| {
        error!(error = %e, reward_id = id, "Failed to delete reward");
        EditorError::Database
    })?;

    if !deleted {
        warn!(reward_id = id, "Reward not found for deletion");
        return Err(EditorError::NotFound);
    }

    info!(reward_id = id, "Reward deleted");

    let rewards = state
        .db
        .get_all_rewards()
        .await
        .map_err(|_| EditorError::Database)?;
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

// ===== Settings =====

/// Get current settings
pub async fn get_settings_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, EditorError> {
    let settings = state.db.get_settings().await.map_err(|e| {
        error!(error = %e, "Failed to load settings");
        EditorError::Database
    })?;

    Ok(axum::Json(settings))
}

/// Update settings
pub async fn update_settings_handler(
    State(state): State<AppState>,
    Form(request): Form<UpdateSettingsRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, EditorError> {
    let settings = state.db.update_settings(request).await.map_err(|e| {
        error!(error = %e, "Failed to update settings");
        EditorError::Database
    })?;

    info!(
        weekly_exp_goal = settings.weekly_exp_goal,
        "Settings updated"
    );

    let signals = serde_json::json!({
        "settingsSaved": true
    });

    let events: Vec<Event> = vec![PatchSignals::new(signals.to_string()).into()];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}

// ===== Helper Functions =====

const DAY_NAMES: &[&str] = &[
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

fn render_quests_table(quests: &[crate::models::Quest]) -> String {
    let mut html = String::new();

    html.push_str(r#"<table class="editor-table"><thead><tr><th>Title</th><th>EXP</th><th>Day</th><th>Status</th><th>Actions</th></tr></thead><tbody>"#);

    for quest in quests {
        let row_class = if !quest.is_active { "inactive-row" } else { "" };
        let status_badge = if quest.is_active {
            r#"<span class="status-badge status-badge--active">Active</span>"#
        } else {
            r#"<span class="status-badge status-badge--inactive">Inactive</span>"#
        };

        html.push_str(&format!(
            r#"<tr class="{}"><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td><div class="action-buttons"><button class="editor-btn editor-btn--small" data-on:click="@get('/editor/quests/{}/edit')">Edit</button><button class="editor-btn editor-btn--danger" data-on:click="@delete('/editor/quests/{}')">Delete</button></div></td></tr>"#,
            row_class,
            escape_html(&quest.title),
            quest.exp_value,
            DAY_NAMES[quest.day_of_week as usize],
            status_badge,
            quest.id,
            quest.id
        ));
    }

    if quests.is_empty() {
        html.push_str(r#"<tr><td colspan="5" class="empty-message">No quests yet. Add your first quest!</td></tr>"#);
    }

    html.push_str("</tbody></table>");
    html
}

fn render_rewards_table(rewards: &[crate::models::Reward]) -> String {
    let mut html = String::new();

    html.push_str(r#"<table class="editor-table"><thead><tr><th>Title</th><th>Required EXP</th><th>Status</th><th>Actions</th></tr></thead><tbody>"#);

    for reward in rewards {
        let row_class = if !reward.is_active {
            "inactive-row"
        } else {
            ""
        };
        let status_badge = if reward.is_active {
            r#"<span class="status-badge status-badge--active">Active</span>"#
        } else {
            r#"<span class="status-badge status-badge--inactive">Inactive</span>"#
        };

        html.push_str(&format!(
            r#"<tr class="{}"><td>{}</td><td>{}</td><td>{}</td><td><div class="action-buttons"><button class="editor-btn editor-btn--small" data-on:click="@get('/editor/rewards/{}/edit')">Edit</button><button class="editor-btn editor-btn--danger" data-on:click="@delete('/editor/rewards/{}')">Delete</button></div></td></tr>"#,
            row_class,
            escape_html(&reward.title),
            reward.required_exp,
            status_badge,
            reward.id,
            reward.id
        ));
    }

    if rewards.is_empty() {
        html.push_str(r#"<tr><td colspan="4" class="empty-message">No rewards yet. Add your first reward!</td></tr>"#);
    }

    html.push_str("</tbody></table>");
    html
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Editor-specific errors
#[derive(Debug, Clone, Copy)]
pub enum EditorError {
    NotFound,
    Database,
    Validation,
}

impl IntoResponse for EditorError {
    fn into_response(self) -> Response<Body> {
        let message = match self {
            EditorError::NotFound => "Not found",
            EditorError::Database => "Database error",
            EditorError::Validation => "Invalid request",
        };

        let status = match self {
            EditorError::NotFound => StatusCode::NOT_FOUND,
            EditorError::Database => StatusCode::INTERNAL_SERVER_ERROR,
            EditorError::Validation => StatusCode::BAD_REQUEST,
        };

        (status, Html(message)).into_response()
    }
}
