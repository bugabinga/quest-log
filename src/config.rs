//! Centralized configuration module for environment variables.
//!
//! This module provides type-safe access to all application configuration
//! via environment variables. All config values have sensible defaults.
//!
//! ## Usage
//!
//! ```rust
//! use crate::config::{data_dir, port};
//!
//! let dir = data_dir()?;      // Returns Result<String, ConfigError>
//! let port = port()?;          // Returns Result<u16, ConfigError>
//! ```
//!
//! ## Required vs Optional
//!
//! - **Required in production**: [`ENV_EDITOR_PASSWORD_HASH`] (editor password)
//! - **Required (absolute path)**: [`ENV_DATA_DIR`] (database directory)
//! - **Optional with defaults**: [`ENV_PORT`], [`ENV_EDITOR_SESSION_DURATION_HOURS`]
//! - **Debug/Test only**: [`ENV_TODAY`], [`ENV_ENABLE_TEST_ENDPOINTS`]

use std::path::Path;
use thiserror::Error;

const QUEST_LOG_DATA_DIR: &str = "QUEST_LOG_DATA_DIR";
const QUEST_LOG_PORT: &str = "PORT";
const QUEST_LOG_EDITOR_PASSWORD_HASH: &str = "QUEST_LOG_EDITOR_PASSWORD_HASH";
const QUEST_LOG_EDITOR_SESSION_DURATION_HOURS: &str = "QUEST_LOG_EDITOR_SESSION_DURATION_HOURS";
const QUEST_LOG_TODAY: &str = "QUEST_LOG_TODAY";

#[cfg(feature = "test-utils")]
const ENABLE_TEST_ENDPOINTS: &str = "ENABLE_TEST_ENDPOINTS";

const DEFAULT_PORT: &str = "3000";
const DEFAULT_DATA_DIR: &str = ".";
const DEFAULT_SESSION_DURATION_HOURS: u64 = 24;

/// Environment variable name for the database directory.
/// Must be an absolute path.
#[allow(dead_code, reason = "Public API for documentation")]
pub const ENV_DATA_DIR: &str = QUEST_LOG_DATA_DIR;

/// Environment variable name for the server port.
#[allow(dead_code, reason = "Public API for documentation")]
pub const ENV_PORT: &str = QUEST_LOG_PORT;

/// Environment variable name for the editor password hash.
/// Required in production; debug builds use a default "dev" password.
#[allow(dead_code, reason = "Public API for documentation")]
pub const ENV_EDITOR_PASSWORD_HASH: &str = QUEST_LOG_EDITOR_PASSWORD_HASH;

/// Environment variable name for session duration in hours.
#[allow(dead_code, reason = "Public API for documentation")]
pub const ENV_EDITOR_SESSION_DURATION_HOURS: &str = QUEST_LOG_EDITOR_SESSION_DURATION_HOURS;

/// Environment variable name for date override (debug builds only).
#[allow(dead_code, reason = "Public API for documentation")]
pub const ENV_TODAY: &str = QUEST_LOG_TODAY;

/// Environment variable name to enable test endpoints.
/// Only available with `test-utils` feature flag.
#[cfg(feature = "test-utils")]
#[allow(dead_code, reason = "Public API for documentation")]
pub const ENV_ENABLE_TEST_ENDPOINTS: &str = ENABLE_TEST_ENDPOINTS;

/// Default port if not specified.
#[allow(dead_code, reason = "Public API for documentation")]
pub const DEFAULT_PORT_VALUE: &str = DEFAULT_PORT;

/// Default session duration in hours.
#[allow(dead_code, reason = "Public API for documentation")]
pub const DEFAULT_SESSION_DURATION: u64 = DEFAULT_SESSION_DURATION_HOURS;

/// Default data directory (current working directory).
#[allow(dead_code, reason = "Public API for documentation")]
pub const DEFAULT_DATA_DIR_VALUE: &str = DEFAULT_DATA_DIR;

/// Configuration errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The path must be absolute.
    #[error("{0} must be an absolute path, got: {1}")]
    NotAbsolute(String, String),
    /// Failed to create the directory.
    #[error("failed to create directory {0}: {1}")]
    DirectoryCreation(String, std::io::Error),
    /// Invalid port value.
    #[error("invalid port: {0}")]
    InvalidPort(String),
    /// Failed to get current directory.
    #[error("failed to get current directory: {0}")]
    CurrentDirectory(std::io::Error),
}

/// Get the data directory path.
///
/// This function:
/// 1. Reads `QUEST_LOG_DATA_DIR` env var, or uses current directory as default
/// 2. Validates the path is absolute
/// 3. Creates the directory if it doesn't exist
///
/// # Errors
///
/// Returns [`ConfigError::NotAbsolute`] if the path is not absolute.
/// Returns [`ConfigError::DirectoryCreation`] if the directory cannot be created.
pub fn data_dir() -> Result<String, ConfigError> {
    let path = match std::env::var(QUEST_LOG_DATA_DIR) {
        Ok(val) => val,
        Err(_) => std::env::current_dir()
            .map_err(ConfigError::CurrentDirectory)?
            .to_string_lossy()
            .to_string(),
    };

    let path_obj = Path::new(&path);
    if !path_obj.is_absolute() {
        let cwd = std::env::current_dir().map_err(ConfigError::CurrentDirectory)?;
        return Err(ConfigError::NotAbsolute(
            QUEST_LOG_DATA_DIR.to_string(),
            format!("{}. CWD would be: {}", path, cwd.display()),
        ));
    }

    if !path_obj.exists() {
        std::fs::create_dir_all(path_obj)
            .map_err(|e| ConfigError::DirectoryCreation(path.clone(), e))?;
    }

    Ok(path)
}

/// Get the data directory path without creating it.
///
/// Useful when you just need the path for display or validation purposes.
///
/// # Errors
///
/// Returns [`ConfigError::NotAbsolute`] if the path is not absolute.
#[allow(dead_code, reason = "May be useful for future use cases")]
pub fn data_dir_path() -> Result<std::path::PathBuf, ConfigError> {
    let path = match std::env::var(QUEST_LOG_DATA_DIR) {
        Ok(val) => val,
        Err(_) => std::env::current_dir()
            .map_err(ConfigError::CurrentDirectory)?
            .to_string_lossy()
            .to_string(),
    };

    let path_obj = Path::new(&path);
    if !path_obj.is_absolute() {
        let cwd = std::env::current_dir().map_err(ConfigError::CurrentDirectory)?;
        return Err(ConfigError::NotAbsolute(
            QUEST_LOG_DATA_DIR.to_string(),
            format!("{}. CWD would be: {}", path, cwd.display()),
        ));
    }

    Ok(path_obj.to_path_buf())
}

/// Get the server port as a u16.
///
/// # Errors
///
/// Returns [`ConfigError::InvalidPort`] if the port cannot be parsed as a number.
pub fn port() -> Result<u16, ConfigError> {
    let port_str = std::env::var(QUEST_LOG_PORT).unwrap_or_else(|_| DEFAULT_PORT.to_string());

    port_str
        .parse()
        .map_err(|_| ConfigError::InvalidPort(port_str))
}

/// Get the editor password hash from environment.
///
/// Returns `None` if not set.
#[must_use]
pub fn editor_password_hash() -> Option<String> {
    std::env::var(QUEST_LOG_EDITOR_PASSWORD_HASH).ok()
}

/// Get the session duration in hours.
///
/// Returns the value from `QUEST_LOG_EDITOR_SESSION_DURATION_HOURS`,
/// or [`DEFAULT_SESSION_DURATION_HOURS`] (24) if not set.
#[must_use]
pub fn editor_session_duration_hours() -> u64 {
    std::env::var(QUEST_LOG_EDITOR_SESSION_DURATION_HOURS)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SESSION_DURATION_HOURS)
}

/// Get the date override for testing (debug builds only).
///
/// Returns `None` if not set or in release builds.
///
/// Valid formats:
/// - `YYYY-MM-DD` (e.g., `2024-01-15`)
/// - Weekday number 0-6 (0=Sunday, e.g., `1` = Monday)
/// - Weekday name (e.g., `Monday`, `mon`)
#[cfg(debug_assertions)]
#[must_use]
pub fn today_override() -> Option<String> {
    std::env::var(QUEST_LOG_TODAY).ok()
}

/// Check if test endpoints should be enabled.
///
/// Only available with `test-utils` feature flag.
/// Returns `true` if `ENABLE_TEST_ENDPOINTS=1`.
#[cfg(feature = "test-utils")]
#[must_use]
pub fn test_endpoints_enabled() -> bool {
    std::env::var(ENABLE_TEST_ENDPOINTS).unwrap_or_default() == "1"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_port() {
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe { std::env::remove_var(QUEST_LOG_PORT) };
        assert_eq!(port().unwrap(), 3000);
    }

    #[test]
    fn test_custom_port() {
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe { std::env::set_var(QUEST_LOG_PORT, "8080") };
        assert_eq!(port().unwrap(), 8080);
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe { std::env::remove_var(QUEST_LOG_PORT) };
    }

    #[test]
    fn test_invalid_port() {
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe { std::env::set_var(QUEST_LOG_PORT, "not-a-port") };
        assert!(port().is_err());
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe { std::env::remove_var(QUEST_LOG_PORT) };
    }

    #[test]
    fn test_default_session_duration() {
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe { std::env::remove_var(QUEST_LOG_EDITOR_SESSION_DURATION_HOURS) };
        assert_eq!(editor_session_duration_hours(), 24);
    }

    #[test]
    fn test_custom_session_duration() {
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe { std::env::set_var(QUEST_LOG_EDITOR_SESSION_DURATION_HOURS, "48") };
        assert_eq!(editor_session_duration_hours(), 48);
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe { std::env::remove_var(QUEST_LOG_EDITOR_SESSION_DURATION_HOURS) };
    }

    #[test]
    fn test_editor_password_hash() {
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe { std::env::remove_var(QUEST_LOG_EDITOR_PASSWORD_HASH) };
        assert!(editor_password_hash().is_none());

        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe { std::env::set_var(QUEST_LOG_EDITOR_PASSWORD_HASH, "test-hash") };
        assert_eq!(editor_password_hash(), Some("test-hash".to_string()));
        // SAFETY: Test-only manipulation of env var, restored immediately
        unsafe { std::env::remove_var(QUEST_LOG_EDITOR_PASSWORD_HASH) };
    }
}
