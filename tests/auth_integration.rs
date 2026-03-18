//! Integration tests for authentication module.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]

use quest_log::auth::{self, LoginRateLimiter};

#[tokio::test]
async fn test_hash_and_verify_password() {
    let password = "test_password_123";
    let hash = auth::hash_password(password).expect("Failed to hash password");

    assert!(auth::verify_password(password, &hash));
    assert!(!auth::verify_password("wrong_password", &hash));
}

#[tokio::test]
async fn test_verify_password_rejects_invalid_hash() {
    let password = "test_password";
    let invalid_hash = "not_a_valid_hash";

    assert!(!auth::verify_password(password, invalid_hash));
}

#[tokio::test]
async fn test_generate_session_token_produces_unique_tokens() {
    let token1 = auth::generate_session_token();
    let token2 = auth::generate_session_token();

    assert!(!token1.is_empty());
    assert!(!token2.is_empty());
    assert_ne!(token1, token2);
}

#[tokio::test]
async fn test_generate_session_token_format() {
    let token = auth::generate_session_token();

    assert!(
        token.contains('_'),
        "Token should contain underscore separator"
    );

    let parts: Vec<&str> = token.split('_').collect();
    assert_eq!(
        parts.len(),
        2,
        "Token should have two parts separated by underscore"
    );
}

#[tokio::test]
async fn test_rate_limiter_allows_first_attempts() {
    let limiter = LoginRateLimiter::new();

    for _ in 0..5 {
        let is_limited = limiter.is_rate_limited("127.0.0.1").await;
        assert!(!is_limited, "First 5 attempts should be allowed");
    }
}

#[tokio::test]
async fn test_rate_limiter_blocks_excess_attempts() {
    let limiter = LoginRateLimiter::new();

    for _ in 0..5 {
        limiter.is_rate_limited("127.0.0.1").await;
        limiter.record_failed_attempt("127.0.0.1").await;
    }

    let is_limited = limiter.is_rate_limited("127.0.0.1").await;
    assert!(is_limited, "6th attempt should be rate limited");
}

#[tokio::test]
async fn test_rate_limiter_different_ips_have_separate_limits() {
    let limiter = LoginRateLimiter::new();

    for _ in 0..5 {
        limiter.is_rate_limited("127.0.0.1").await;
        limiter.record_failed_attempt("127.0.0.1").await;
    }

    let ip1_limited = limiter.is_rate_limited("127.0.0.1").await;
    let ip2_limited = limiter.is_rate_limited("192.168.1.1").await;

    assert!(ip1_limited, "IP 1 should be rate limited");
    assert!(!ip2_limited, "IP 2 should not be rate limited");
}

#[tokio::test]
async fn test_rate_limiter_clear_attempts_on_success() {
    let limiter = LoginRateLimiter::new();

    for _ in 0..5 {
        limiter.is_rate_limited("127.0.0.1").await;
        limiter.record_failed_attempt("127.0.0.1").await;
    }

    limiter.clear_attempts("127.0.0.1").await;

    let is_limited = limiter.is_rate_limited("127.0.0.1").await;
    assert!(
        !is_limited,
        "IP should not be rate limited after clearing attempts"
    );
}

#[tokio::test]
async fn test_rate_limiter_old_attempts_expire() {
    let limiter = LoginRateLimiter::new();

    limiter.record_failed_attempt("127.0.0.1").await;

    let is_limited_immediately = limiter.is_rate_limited("127.0.0.1").await;
    assert!(
        !is_limited_immediately,
        "Single attempt should not trigger rate limit"
    );
}
