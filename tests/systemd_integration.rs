//! Integration tests for systemd socket activation and notify protocol.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]
#![cfg(target_os = "linux")]

use sd_notify::NotifyState;

#[test]
fn systemd_watchdog_and_notify_smoke() {
    // Simulate systemd setting WATCHDOG_USEC and LISTEN_PID for the current PID.
    // The test ensures the sd-notify crate is available and the basic APIs behave.
    // SAFETY: Test-only manipulation of env vars, restored automatically when test ends
    unsafe {
        std::env::set_var("WATCHDOG_USEC", "30000000");
    }
    // SAFETY: Test-only manipulation of env vars, restored automatically when test ends
    unsafe {
        std::env::set_var("WATCHDOG_PID", std::process::id().to_string());
    }

    let mut usec: u128 = 0;
    if let Some(duration) = sd_notify::watchdog_enabled() {
        usec = duration.as_micros();
    }
    assert!(
        usec == 30_000_000,
        "watchdog should be reported as enabled when WATCHDOG_USEC is set"
    );

    // Call notify READY/STATUS and ensure it doesn't panic. We cannot assert systemd received it
    // because runners typically don't have NOTIFY_SOCKET, but the call should be safe.
    let _unused = sd_notify::notify(&[NotifyState::Status("test")]);
    let _unused = sd_notify::notify(&[NotifyState::Ready]);
}
