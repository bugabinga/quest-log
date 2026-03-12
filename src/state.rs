use crate::auth::{LoginRateLimiter, SESSION_DURATION_HOURS};
use crate::database::Database;
use crate::handlers::ServerMessage;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, broadcast};

/// Application state shared across all handlers
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub bcast: broadcast::Sender<ServerMessage>,
    /// Editor session tokens mapped to their expiry time
    pub editor_sessions: Arc<RwLock<std::collections::HashMap<String, Instant>>>,
    /// Rate limiter for login attempts
    pub login_rate_limiter: Arc<LoginRateLimiter>,
}

impl AppState {
    pub fn new(db: Database, bcast: broadcast::Sender<ServerMessage>) -> Self {
        Self {
            db,
            bcast,
            editor_sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            login_rate_limiter: Arc::new(LoginRateLimiter::new()),
        }
    }

    /// Get the session duration from environment or use default
    pub fn session_duration(&self) -> Duration {
        let hours: i64 = std::env::var("QUEST_LOG_EDITOR_SESSION_DURATION_HOURS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(SESSION_DURATION_HOURS);
        Duration::from_secs(hours as u64 * 3600)
    }

    /// Create a new editor session
    pub async fn create_session(&self, token: String) -> Instant {
        let expiry = Instant::now() + self.session_duration();
        let mut sessions = self.editor_sessions.write().await;
        sessions.insert(token, expiry);
        expiry
    }

    /// Validate a session token
    pub async fn validate_session(&self, token: &str) -> bool {
        let sessions = self.editor_sessions.read().await;
        if let Some(expiry) = sessions.get(token)
            && *expiry > Instant::now()
        {
            return true;
        }
        false
    }
}
