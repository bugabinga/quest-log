//! Editor authentication handlers

use axum::{
    extract::Form,
    extract::State,
    http::HeaderMap,
    http::header::SET_COOKIE,
    response::Html,
    response::IntoResponse,
    response::sse::{Event, Sse},
};
use chrono::Utc;
use datastar::execute_script::ExecuteScript;
use datastar::patch_elements::PatchElements;
use datastar::patch_signals::PatchSignals;
use futures::Stream;
use serde::Deserialize;
use tracing::{debug, error, info, warn};

use crate::auth;
use crate::handlers::AppError;
use crate::models::Settings;
use crate::sse_response;
use crate::state::AppState;
use crate::ui;

/// Session cookie configuration - Secure flag only in release builds
#[cfg(debug_assertions)]
const SESSION_COOKIE_OPTS: &str = "Path=/; HttpOnly; SameSite=Strict; Max-Age=86400";

#[cfg(not(debug_assertions))]
const SESSION_COOKIE_OPTS: &str = "Path=/; HttpOnly; SameSite=Strict; Secure; Max-Age=86400";

/// Get the rate-limit key for editor auth.
fn get_client_ip(_headers: &HeaderMap) -> String {
    "local".to_string()
}

/// Extract and validate session token from cookie header
pub(super) async fn extract_and_validate_session(
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
/// Panics if the cookie header construction fails unexpectedly
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

    if state.login_rate_limiter.is_rate_limited(&client_ip).await {
        warn!(ip = %client_ip, "Rate limited login attempt");

        let signals = serde_json::json!({
            "loginError": "Too many login attempts. Please wait a minute.",
            "isRateLimited": true
        });

        let events: Vec<Event> = vec![PatchSignals::new(signals.to_string()).into()];
        let response_headers = HeaderMap::new();
        return Ok((response_headers, sse_response!(events)));
    }

    let password_hash = auth::get_password_hash_or_default();

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
        return Ok((response_headers, sse_response!(events)));
    }

    state.login_rate_limiter.clear_attempts(&client_ip).await;
    let token = auth::generate_session_token();
    let expiry = state.create_session(token.clone()).await;
    info!(ip = %client_ip, expires = ?expiry, "Login successful");

    let signals = serde_json::json!({
        "isAuthenticated": true,
        "loginError": null
    });

    let events: Vec<Event> = vec![
        PatchSignals::new(signals.to_string()).into(),
        ExecuteScript::new("window.location.href = '/editor'").into(),
    ];

    let mut response_headers = HeaderMap::new();
    let cookie = format!("editor_session={token}; {SESSION_COOKIE_OPTS}");
    let cookie_header_value = cookie.parse().unwrap_or_else(|_| {
        panic!("Cookie string is valid; constructed with known format: {cookie}")
    });
    response_headers.insert(SET_COOKIE, cookie_header_value);

    Ok((response_headers, sse_response!(events)))
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
    let wrapped_html = format!(
        r#"<div class="editor-wrapper" style="view-transition-name: editor-page;">{}</div>"#,
        html.into_string()
    );

    let events: Vec<Event> = vec![
        PatchElements::new(wrapped_html)
            .use_view_transition(true)
            .into(),
        PatchSignals::new(signals.to_string()).into(),
    ];

    Ok(sse_response!(events))
}
