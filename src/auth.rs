//! Authentication module for the Quest Log Editor
//!
//! Provides password hashing with Argon2, session token generation,
//! and rate limiting for login attempts.

use crate::config;

/// Simple hex encoding for session tokens (no external dependency)
mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";

    pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len().checked_mul(2).unwrap_or(0));
        for &b in bytes {
            s.push(HEX_CHARS[(b >> 4) as usize] as char);
            s.push(HEX_CHARS[(b & 0xf) as usize] as char);
        }
        s
    }
}

use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{Error as PasswordHashError, SaltString, rand_core::OsRng, rand_core::RngCore},
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Authentication errors
#[derive(Debug, Error)]
pub enum AuthError {
    /// Password hashing failed
    #[error("Password hashing failed: {0}")]
    HashingFailed(String),
}

impl From<PasswordHashError> for AuthError {
    fn from(err: PasswordHashError) -> Self {
        AuthError::HashingFailed(err.to_string())
    }
}

/// Default session duration in hours
#[allow(dead_code, reason = "Moved to config module")]
pub const SESSION_DURATION_HOURS: u64 = 24;

/// Maximum login attempts per minute per IP
const MAX_LOGIN_ATTEMPTS_PER_MINUTE: usize = 5;

/// Time window for rate limiting (in seconds)
const RATE_LIMIT_WINDOW_SECS: u64 = 60;

/// Rate limiter for login attempts
#[derive(Clone)]
pub struct LoginRateLimiter {
    attempts: Arc<RwLock<HashMap<String, Vec<Instant>>>>,
}

impl Default for LoginRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl LoginRateLimiter {
    /// Clear rate limit for an IP (on successful login)
    pub async fn clear_attempts(&self, ip: &str) {
        let mut attempts = self.attempts.write().await;
        attempts.remove(ip);
    }

    /// Check if the IP has exceeded the rate limit
    /// Note: This only checks and cleans old attempts - it does NOT add an attempt.
    /// The handler must call `record_failed_attempt()` on failed login.
    pub async fn is_rate_limited(&self, ip: &str) -> bool {
        let mut attempts = self.attempts.write().await;
        let now = Instant::now();
        let window_start = now
            .checked_sub(Duration::from_secs(RATE_LIMIT_WINDOW_SECS))
            .unwrap_or_else(|| {
                tracing::warn!("Rate limit window calculation underflow - using current time");
                now
            });

        // Clean old attempts
        if let Some(ip_attempts) = attempts.get_mut(ip) {
            ip_attempts.retain(|&time| time > window_start);
            let count = ip_attempts.len();
            debug!(ip = %ip, attempts = count, "Login attempts for IP");

            if count >= MAX_LOGIN_ATTEMPTS_PER_MINUTE {
                warn!(ip = %ip, attempts = count, "Rate limit exceeded for IP");
                return true;
            }
            // Don't add attempt here - handler calls record_failed_attempt() on failure
        } else {
            // No existing attempts, ip is not rate limited
            attempts.insert(ip.to_string(), vec![]);
        }

        false
    }

    /// Creates a new rate limiter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            attempts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record a failed login attempt
    pub async fn record_failed_attempt(&self, ip: &str) {
        let mut attempts = self.attempts.write().await;
        let now = Instant::now();
        let window_start = now
            .checked_sub(Duration::from_secs(RATE_LIMIT_WINDOW_SECS))
            .unwrap_or_else(|| {
                tracing::warn!("Rate limit window calculation underflow - using current time");
                now
            });

        let ip_attempts = attempts.entry(ip.to_string()).or_insert_with(Vec::new);
        ip_attempts.retain(|&time| time > window_start);
        ip_attempts.push(now);
    }
}

/// Generate a cryptographically secure session token
pub fn generate_session_token() -> String {
    let salt = SaltString::generate(&mut OsRng);
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!("{}_{}", salt, hex::encode(&bytes))
}

/// Get the password hash from environment variable
#[must_use]
pub fn get_password_hash_from_env() -> Option<String> {
    config::editor_password_hash()
}

/// Get password hash, using a default in debug builds
///
/// # Panics
///
/// In debug builds, panics if hashing the default "dev" password fails.
/// This should never happen as the hashing is deterministic.
#[cfg(debug_assertions)]
pub fn get_password_hash_or_default() -> String {
    static DEV_HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();

    if let Some(hash) = get_password_hash_from_env() {
        hash
    } else {
        tracing::warn!("Using default dev password 'dev' - DO NOT USE IN PRODUCTION!");
        DEV_HASH
            .get_or_init(|| match hash_password("dev") {
                Ok(hash) => hash,
                Err(e) => {
                    tracing::error!("Failed to hash dev password: {}", e);
                    panic!("Failed to hash dev password: {e}");
                }
            })
            .clone()
    }
}

/// Get password hash from environment variable, or panic in production
///
/// # Panics
///
/// Panics if `QUEST_LOG_EDITOR_PASSWORD_HASH` environment variable is not set.
/// In production, this environment variable MUST be set for security.
#[cfg(not(debug_assertions))]
pub fn get_password_hash_or_default() -> String {
    get_password_hash_from_env().expect("QUEST_LOG_EDITOR_PASSWORD_HASH must be set in production")
}

/// Hash a password using Argon2
///
/// # Errors
///
/// Returns an error if the password cannot be hashed due to:
/// - Cryptographic random number generator failure
/// - Invalid password encoding
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(password_hash.to_string())
}

/// Verify a password against a stored hash
pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(e) => {
            warn!(error = %e, "Failed to parse password hash");
            return false;
        }
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let password = "test_password_123";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash));
        assert!(!verify_password("wrong_password", &hash));
    }

    #[test]
    fn test_generate_session_token() {
        let token1 = generate_session_token();
        let token2 = generate_session_token();

        assert!(!token1.is_empty());
        assert!(!token2.is_empty());
        assert_ne!(token1, token2);
    }

    #[tokio::test]
    async fn test_rate_limiter_allows_first_attempts() {
        let limiter = LoginRateLimiter::new();

        // First 5 attempts should be allowed
        for _ in 0..5 {
            assert!(!limiter.is_rate_limited("127.0.0.1").await);
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_excess() {
        let limiter = LoginRateLimiter::new();

        // First 5 failed attempts should be allowed
        for _ in 0..5 {
            assert!(!limiter.is_rate_limited("127.0.0.1").await);
            limiter.record_failed_attempt("127.0.0.1").await;
        }

        // 6th attempt should be blocked
        assert!(limiter.is_rate_limited("127.0.0.1").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_different_ips() {
        let limiter = LoginRateLimiter::new();

        // Different IPs should have separate limits
        for _ in 0..5 {
            assert!(!limiter.is_rate_limited("127.0.0.1").await);
            assert!(!limiter.is_rate_limited("192.168.1.1").await);
        }
    }

    #[test]
    fn test_get_password_hash_or_default_dev() {
        // In debug builds, should return a hash for "dev" password
        let hash = get_password_hash_or_default();
        assert!(!hash.is_empty());
        // Verify the hash works for password "dev"
        assert!(verify_password("dev", &hash));
        // Wrong password should fail
        assert!(!verify_password("wrong", &hash));
    }
}
