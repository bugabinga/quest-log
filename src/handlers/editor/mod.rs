//! Editor handlers for the Quest Log
//!
//! Provides HTTP handlers for the editor page, authentication, and CRUD operations.

use axum::{
    extract::Multipart,
    extract::{Form, Path, State},
    http::{HeaderMap, header::SET_COOKIE},
    response::sse::{Event, Sse},
    response::{Html, IntoResponse},
};
use chrono::Utc;
use datastar::patch_elements::PatchElements;
use datastar::patch_signals::PatchSignals;
use futures::stream::{self, Stream};
use serde::Deserialize;
use std::fmt::Write;

use crate::auth;
use crate::database::SetOrRemove;
use crate::handlers::AppError;
use crate::models::{
    CreateQuestRequest, CreateRewardRequest, Settings, UpdateQuestRequest, UpdateRewardRequest,
    UpdateSettingsRequest,
};
use crate::state::AppState;
use crate::ui;
use tracing::{debug, error, info, warn};

/// Session cookie configuration - Secure flag only in release builds
#[cfg(debug_assertions)]
const SESSION_COOKIE_OPTS: &str = "Path=/; HttpOnly; SameSite=Strict; Max-Age=86400";

#[cfg(not(debug_assertions))]
const SESSION_COOKIE_OPTS: &str = "Path=/; HttpOnly; SameSite=Strict; Secure; Max-Age=86400";

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

/// Extract and validate session token from cookie header
async fn extract_and_validate_session(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<String, AppError> {
    let cookie =
        headers
            .get("cookie")
            .and_then(|c| c.to_str().ok())
            .ok_or(AppError::Authentication(
                "No session cookie found".to_string(),
            ))?;

    // Parse cookie to find editor_session
    let token = cookie
        .split(';')
        .find_map(|c| {
            let c = c.trim();
            c.strip_prefix("editor_session=")
        })
        .ok_or(AppError::Authentication(
            "No session token in cookie".to_string(),
        ))?;

    if !state.validate_session(token).await {
        return Err(AppError::Authentication("Invalid session".to_string()));
    }

    Ok(token.to_string())
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    password: String,
}

/// Editor page - shows auth modal if not authenticated, otherwise shows editor
///
/// # Errors
///
/// Returns an error if session validation fails or if there's an issue rendering the page
pub async fn editor_page_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    let client_ip = get_client_ip(&headers);
    debug!(ip = %client_ip, "Editor page requested");

    // Check for session token in header or cookie
    let session_token = headers
        .get("x-editor-session")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string)
        .or_else(|| {
            headers
                .get("cookie")
                .and_then(|c| c.to_str().ok())
                .and_then(|cookie| {
                    cookie
                        .split(';')
                        .find_map(|c| {
                            let c = c.trim();
                            c.strip_prefix("editor_session=")
                        })
                        .map(ToString::to_string)
                })
        });

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
        AppError::Database(e)
    })?;

    let rewards = state.db.get_all_rewards().await.map_err(|e| {
        error!(error = %e, "Failed to load rewards");
        AppError::Database(e)
    })?;

    let settings = state.db.get_settings().await.map_err(|e| {
        error!(error = %e, "Failed to load settings");
        AppError::Database(e)
    })?;

    let html = ui::editor::editor_page(true, "quests", &quests, &rewards, &settings);
    Ok(Html(html.into_string()))
}

/// Process login attempt
///
/// # Errors
///
/// Returns an error if authentication fails or if there's an issue with the session
///
/// # Panics
///
/// Panics if the session cookie header cannot be parsed (should never happen with valid cookie format)
pub async fn login_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(request): Form<LoginRequest>,
) -> Result<
    (
        HeaderMap,
        Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>,
    ),
    AppError,
> {
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
        let response_headers = HeaderMap::new();
        return Ok((
            response_headers,
            Sse::new(stream::iter(events.into_iter().map(Ok))),
        ));
    }

    // Get password hash (uses default in debug builds)
    let password_hash = auth::get_password_hash_or_default();

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
        let response_headers = HeaderMap::new();
        return Ok((
            response_headers,
            Sse::new(stream::iter(events.into_iter().map(Ok))),
        ));
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

    // Set cookie header for session persistence
    let mut response_headers = HeaderMap::new();
    let cookie = format!("editor_session={token}; {SESSION_COOKIE_OPTS}");
    // Parsing should succeed since we constructed the cookie string with valid format
    let cookie_header_value = cookie.parse().unwrap_or_else(|_| {
        panic!("Cookie string is valid; constructed with known format: {cookie}")
    });
    response_headers.insert(SET_COOKIE, cookie_header_value);

    Ok((
        response_headers,
        Sse::new(stream::iter(events.into_iter().map(Ok))),
    ))
}

/// Logout handler
///
/// # Errors
///
/// Returns an error if session invalidation fails
pub async fn logout_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    let client_ip = get_client_ip(&headers);
    debug!(ip = %client_ip, "Logout requested");

    // Try to invalidate the session if token is present
    if let Ok(token) = extract_and_validate_session(&headers, &state).await {
        state.invalidate_session(&token).await;
        info!(ip = %client_ip, "Session invalidated");
    }

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
    // Validate session
    extract_and_validate_session(&headers, &state).await?;

    // Process multipart form data
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
                error!(error = %e, "Failed to create quest with image");
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
            error!(error = %e, "Failed to create quest");
            AppError::Database(e)
        })?
    };

    info!(quest_id = quest.id, "Quest created");

    // Return updated quests list
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

/// Process multipart form fields for quest creation
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
    // Validate session
    extract_and_validate_session(&headers, &state).await?;

    let deleted = state.db.delete_quest(id).await.map_err(|e| {
        error!(error = %e, quest_id = id, "Failed to delete quest");
        AppError::Database(e)
    })?;

    if !deleted {
        warn!(quest_id = id, "Quest not found for deletion");
        return Err(AppError::NotFound);
    }

    info!(quest_id = id, "Quest deleted");

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
        error!(error = %e, "Failed to load quests");
        AppError::Database(e)
    })?;

    Ok(axum::Json(quests))
}

/// Update a quest
///
/// # Errors
///
/// Returns an error if session validation, database operation, or quest not found
///
/// # Panics
///
/// Panics if multipart form data is invalid
pub async fn update_quest_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    // Validate session
    extract_and_validate_session(&headers, &state).await?;

    // Process multipart form data
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
                error!(error = %e, quest_id = id, "Failed to update quest with image");
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
            error!(error = %e, quest_id = id, "Failed to update quest");
            AppError::Database(e)
        })?
    };

    if quest.is_none() {
        warn!(quest_id = id, "Quest not found for update");
        return Err(AppError::NotFound);
    }

    info!(quest_id = id, "Quest updated");

    let quests = state
        .db
        .get_all_quests()
        .await
        .map_err(AppError::Database)?;
    let html = render_quests_table(&quests);

    let events: Vec<Event> = vec![PatchElements::new(html).into()];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}

// ===== Reward CRUD =====

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
    // Validate session
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
                error!(error = %e, "Failed to create reward with image");
                AppError::Database(e)
            })?
    } else {
        let req = CreateRewardRequest {
            description,
            required_exp,
            title,
        };
        state.db.create_reward(req).await.map_err(|e| {
            error!(error = %e, "Failed to create reward");
            AppError::Database(e)
        })?
    };

    info!(reward_id = reward.id, "Reward created");

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
    // Validate session
    extract_and_validate_session(&headers, &state).await?;

    let deleted = state.db.delete_reward(id).await.map_err(|e| {
        error!(error = %e, reward_id = id, "Failed to delete reward");
        AppError::Database(e)
    })?;

    if !deleted {
        warn!(reward_id = id, "Reward not found for deletion");
        return Err(AppError::NotFound);
    }

    info!(reward_id = id, "Reward deleted");

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
        error!(error = %e, "Failed to load rewards");
        AppError::Database(e)
    })?;

    Ok(axum::Json(rewards))
}

/// Update a reward
///
/// # Errors
///
/// Returns an error if session validation, database operation, or reward not found
///
/// # Panics
///
/// Panics if multipart form data is invalid
pub async fn update_reward_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>>, AppError> {
    // Validate session
    extract_and_validate_session(&headers, &state).await?;

    // Process multipart form data
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
                error!(error = %e, reward_id = id, "Failed to update reward with image");
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
            error!(error = %e, reward_id = id, "Failed to update reward");
            AppError::Database(e)
        })?
    };

    if reward.is_none() {
        warn!(reward_id = id, "Reward not found for update");
        return Err(AppError::NotFound);
    }

    info!(reward_id = id, "Reward updated");

    let rewards = state
        .db
        .get_all_rewards()
        .await
        .map_err(AppError::Database)?;
    let html = render_rewards_table(&rewards);

    let events: Vec<Event> = vec![PatchElements::new(html).into()];

    Ok(Sse::new(stream::iter(events.into_iter().map(Ok))))
}

// ===== Settings =====

/// Get current settings
///
/// # Errors
///
/// Returns an error if database query fails
pub async fn get_settings_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let settings = state.db.get_settings().await.map_err(|e| {
        error!(error = %e, "Failed to load settings");
        AppError::Database(e)
    })?;

    Ok(axum::Json(settings))
}

const DAY_NAMES: &[&str] = &[
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

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
    // Validate session
    extract_and_validate_session(&headers, &state).await?;

    let settings = state.db.update_settings(request).await.map_err(|e| {
        error!(error = %e, "Failed to update settings");
        AppError::Database(e)
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
