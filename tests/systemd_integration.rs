#![cfg(target_os = "linux")]

use sd_notify::NotifyState;

#[test]
fn systemd_watchdog_and_notify_smoke() {
    // Simulate systemd setting WATCHDOG_USEC and LISTEN_PID for the current PID.
    // The test ensures the sd-notify crate is available and the basic APIs behave.
    unsafe {
        std::env::set_var("WATCHDOG_USEC", "30000000");
        std::env::set_var("WATCHDOG_PID", std::process::id().to_string());
    }

    let mut usec: u64 = 0;
    let enabled = sd_notify::watchdog_enabled(true, &mut usec);
    assert!(
        enabled,
        "watchdog should be reported as enabled when WATCHDOG_USEC is set"
    );
    assert_eq!(usec, 30_000_000);

    // Call notify READY/STATUS and ensure it doesn't panic. We cannot assert systemd received it
    // because runners typically don't have NOTIFY_SOCKET, but the call should be safe.
    let _ = sd_notify::notify(false, &[NotifyState::Status("test")]);
    let _ = sd_notify::notify(false, &[NotifyState::Ready]);
}
