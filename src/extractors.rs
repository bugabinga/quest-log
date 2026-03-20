//! Custom extractors for request data

use axum::http::{HeaderMap, header::COOKIE};
use std::str::FromStr;

/// Timezone extracted from request headers or cookies
///
/// The timezone is extracted with the following precedence:
/// 1. `X-Timezone` header (IANA timezone string, e.g., `America/New_York`)
/// 2. `QuestLog-TZ` cookie (fallback)
///
/// Returns `None` if neither is present.
#[derive(Debug, Clone)]
pub struct Timezone(pub Option<String>);

impl Timezone {
    /// Create a new timezone extractor from request headers
    ///
    /// # Arguments
    ///
    /// * `headers` - HTTP headers from the request
    ///
    /// # Returns
    ///
    /// A `Timezone` with the value being the timezone string if found
    pub fn from_headers(headers: &HeaderMap) -> Self {
        // Try X-Timezone header first
        if let Some(tz) = headers
            .get("X-Timezone")
            .and_then(|v| v.to_str().ok())
            .filter(|s| !s.is_empty())
        {
            return Timezone(Some(tz.to_string()));
        }

        // Fall back to QuestLog-TZ cookie
        if let Some(cookie) = headers.get(COOKIE).and_then(|v| v.to_str().ok())
            && let Some(tz) = cookie
                .split(';')
                .find_map(|c| {
                    let c = c.trim();
                    c.strip_prefix("QuestLog-TZ=")
                })
                .filter(|s| !s.is_empty())
        {
            return Timezone(Some(tz.to_string()));
        }

        Timezone(None)
    }

    /// Get the timezone string if present
    #[must_use]
    pub fn as_deref(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// Validate that the timezone string is a valid IANA timezone
    ///
    /// # Arguments
    ///
    /// * `tz` - Optional timezone string to validate
    ///
    /// # Returns
    ///
    /// `true` if the timezone is valid or None, `false` otherwise
    #[must_use]
    #[allow(dead_code, reason = "used in tests")]
    pub fn is_valid(&self) -> bool {
        if let Some(ref tz) = self.0 {
            tz.parse::<chrono_tz::Tz>().is_ok()
        } else {
            true // None is considered valid (will fall back to UTC)
        }
    }

    /// Get the validated timezone as a `chrono_tz::Tz`
    ///
    /// Returns `None` if the timezone is invalid or not present
    #[must_use]
    #[allow(dead_code, reason = "used in tests")]
    pub fn to_chrono_tz(&self) -> Option<chrono_tz::Tz> {
        self.0.as_ref().and_then(|tz| tz.parse().ok())
    }
}

impl FromStr for Timezone {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Timezone(Some(s.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_timezone_from_header() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Timezone", HeaderValue::from_static("America/New_York"));

        let tz = Timezone::from_headers(&headers);
        assert_eq!(tz.0, Some("America/New_York".to_string()));
    }

    #[test]
    fn test_timezone_from_cookie_fallback() {
        let mut headers = HeaderMap::new();
        headers.insert("Cookie", HeaderValue::from_static("QuestLog-TZ=Asia/Tokyo"));

        let tz = Timezone::from_headers(&headers);
        assert_eq!(tz.0, Some("Asia/Tokyo".to_string()));
    }

    #[test]
    fn test_timezone_header_takes_precedence() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Timezone", HeaderValue::from_static("Europe/London"));
        headers.insert(
            "Cookie",
            HeaderValue::from_static("QuestLog-TZ=America/New_York"),
        );

        let tz = Timezone::from_headers(&headers);
        assert_eq!(tz.0, Some("Europe/London".to_string()));
    }

    #[test]
    fn test_timezone_none_when_missing() {
        let headers = HeaderMap::new();
        let tz = Timezone::from_headers(&headers);
        assert_eq!(tz.0, None);
    }

    #[test]
    fn test_timezone_empty_header_ignored() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Timezone", HeaderValue::from_static(""));
        headers.insert("Cookie", HeaderValue::from_static("QuestLog-TZ=Asia/Tokyo"));

        let tz = Timezone::from_headers(&headers);
        assert_eq!(tz.0, Some("Asia/Tokyo".to_string()));
    }

    #[test]
    fn test_is_valid_with_valid_timezone() {
        let tz = Timezone(Some("America/New_York".to_string()));
        assert!(tz.is_valid());
    }

    #[test]
    fn test_is_valid_with_invalid_timezone() {
        let tz = Timezone(Some("Invalid/Timezone".to_string()));
        assert!(!tz.is_valid());
    }

    #[test]
    fn test_is_valid_with_none() {
        let tz = Timezone(None);
        assert!(tz.is_valid());
    }

    #[test]
    fn test_to_chrono_tz_valid() {
        let tz = Timezone(Some("America/New_York".to_string()));
        assert!(tz.to_chrono_tz().is_some());
    }

    #[test]
    fn test_to_chrono_tz_invalid() {
        let tz = Timezone(Some("Invalid/Timezone".to_string()));
        assert!(tz.to_chrono_tz().is_none());
    }

    #[test]
    fn test_to_chrono_tz_none() {
        let tz = Timezone(None);
        assert!(tz.to_chrono_tz().is_none());
    }
}
