//! HTTP handlers for the Quest Log application

use axum::body::Body;
use axum::http::{Response, StatusCode};
use axum::response::Html;
use axum::response::IntoResponse;

use crate::ui;

/// Server message sent to connected clients via SSE
#[derive(Clone, Debug)]
pub enum ServerMessage {
    /// HTML elements to patch into the DOM
    Elements(String, Option<String>),
    /// Signals to update client-side state
    Signals(String, Option<String>),
}

pub mod bounty;
pub mod editor;
pub mod events;
pub mod navigate;
pub mod quests;
pub mod stats;

/// Application errors that can occur during request handling
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Database operation failed
    #[error("Something went wrong")]
    Database(#[from] sqlx::Error),
    /// Resource not found
    #[error("Not found")]
    NotFound,
    /// Validation failed
    #[error("Validation error: {0}")]
    ValidationError(String),
    /// Authentication failed
    #[error("Authentication error: {0}")]
    Authentication(String),
}

impl AppError {
    fn heading(&self) -> &'static str {
        match self {
            Self::Database(_) => "Something went wrong",
            Self::NotFound => "Gone.",
            Self::ValidationError(_) => "Wrong Day!",
            Self::Authentication(_) => "Access Denied",
        }
    }

    fn message(&self) -> String {
        match self {
            Self::Database(_) => "Something went wrong".to_string(),
            Self::NotFound => "Like your motivation. Or your quests.".to_string(),
            Self::ValidationError(msg) | Self::Authentication(msg) => msg.clone(),
        }
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::ValidationError(_) => StatusCode::BAD_REQUEST,
            Self::Authentication(_) => StatusCode::UNAUTHORIZED,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Self::Database(_) => "Oops!",
            Self::NotFound => "Nothing Here",
            Self::ValidationError(_) => "Can't Do That",
            Self::Authentication(_) => "Access Denied",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response<Body> {
        let message = self.message();
        let html = ui::error::error_page(self.title(), self.heading(), &message);
        (self.status_code(), Html(html.into_string())).into_response()
    }
}
