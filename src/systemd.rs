//! Optional systemd integration (enabled via `features = ["systemd"]`).
//! Compiles only on Linux when the `systemd` feature is enabled.
#![cfg(target_os = "linux")]

use sd_notify::{self, NotifyState};
use std::os::unix::io::RawFd;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Send a READY=1 notification to systemd with an optional STATUS message.
pub fn notify_ready(status: Option<&str>) {
    if let Some(s) = status {
        let _ = sd_notify::notify(false, &[NotifyState::Status(s)]);
    }
    let _ = sd_notify::notify(false, &[NotifyState::Ready]);
}

/// Attempt to start a watchdog heartbeat task if WATCHDOG is enabled.
/// Returns a JoinHandle that should be aborted/joined on shutdown.
pub fn start_watchdog() -> Option<JoinHandle<()>> {
    let mut usec: u64 = 0;
    // Unset the env vars for children so they don't inherit WATCHDOG_USEC
    if !sd_notify::watchdog_enabled(true, &mut usec) {
        return None;
    }
    if usec == 0 {
        return None;
    }

    // Be conservative: send heartbeat at one-third of the watchdog interval
    let interval = Duration::from_micros(usec / 3);

    Some(tokio::spawn(async move {
        loop {
            let _ = sd_notify::notify(false, &[NotifyState::Watchdog]);
            tokio::time::sleep(interval).await;
        }
    }))
}

/// If systemd passed socket file descriptors (socket activation), return a Vec of RawFd.
/// Caller is responsible for consuming them exactly once (FromRawFd).
pub fn take_listen_fds() -> Vec<RawFd> {
    // sd_notify doesn't provide listen_fds helpers; use the environment variable directly.
    // The standard behavior is that systemd sets LISTEN_FDS and LISTEN_PID.
    let listen_fds = std::env::var("LISTEN_FDS").ok();
    let listen_pid = std::env::var("LISTEN_PID").ok();

    if listen_fds.is_none() || listen_pid.is_none() {
        return Vec::new();
    }

    // Ensure the PID matches our PID
    if let Ok(pid_str) = listen_pid.unwrap().parse::<u32>() {
        if pid_str != std::process::id() {
            return Vec::new();
        }
    } else {
        return Vec::new();
    }

    let nfds: i32 = match listen_fds.unwrap().parse() {
        Ok(n) if n > 0 => n,
        _ => return Vec::new(),
    };

    // sd-daemon starts at SD_LISTEN_FDS_START = 3
    let start_fd = 3;
    let mut fds = Vec::new();
    for i in 0..nfds {
        fds.push((start_fd + i) as RawFd);
    }

    // Unset env so children won't inherit and repeated calls won't re-read
    unsafe {
        std::env::remove_var("LISTEN_FDS");
        std::env::remove_var("LISTEN_PID");
    }

    // sd_listen_fds semantics reserve fds starting at 3; we return them in order
    // so the caller can consume them via FromRawFd exactly once.

    fds
}

// Stub implementations when the feature is not enabled are intentionally not provided here;
// consumers should gate usage behind cfg checks or call these from cfg-gated modules.
