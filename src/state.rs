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
    #[must_use]
    pub fn new(db: Database, bcast: broadcast::Sender<ServerMessage>) -> Self {
        Self {
            db,
            bcast,
            editor_sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            login_rate_limiter: Arc::new(LoginRateLimiter::new()),
        }
    }

    /// Get the session duration from environment or use default
    #[must_use]
    pub fn session_duration() -> Duration {
        let hours: u64 = std::env::var("QUEST_LOG_EDITOR_SESSION_DURATION_HOURS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(SESSION_DURATION_HOURS);
        Duration::from_secs(hours.saturating_mul(3600))
    }

    /// Create a new editor session
    ///
    /// # Panics
    ///
    /// Panics if the session duration cannot be added to the current time
    pub async fn create_session(&self, token: String) -> Instant {
        let expiry = Instant::now()
            .checked_add(Self::session_duration())
            .unwrap_or_else(|| panic!("time overflow"));
        let mut sessions = self.editor_sessions.write().await;
        sessions.insert(token, expiry);
        expiry
    }

    /// Validate a session token
    pub async fn validate_session(&self, token: &str) -> bool {
        let mut sessions = self.editor_sessions.write().await;
        let now = Instant::now();

        // Lazy cleanup: remove expired sessions
        sessions.retain(|_, expiry| *expiry > now);

        sessions.get(token).is_some_and(|expiry| *expiry > now)
    }

    /// Invalidate a session token (logout)
    pub async fn invalidate_session(&self, token: &str) {
        let mut sessions = self.editor_sessions.write().await;
        sessions.remove(token);
    }
}
