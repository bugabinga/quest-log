//! Systemd integration for Linux builds.

use sd_notify::NotifyState;
use std::os::unix::io::RawFd;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Send a READY=1 notification to systemd with an optional STATUS message.
pub fn notify_ready(status: Option<&str>) {
    if let Some(s) = status {
        let _unused = sd_notify::notify(&[NotifyState::Status(s)]);
    }
    let _unused = sd_notify::notify(&[NotifyState::Ready]);
}

/// Attempt to start a watchdog heartbeat task if WATCHDOG is enabled.
/// Returns a `JoinHandle` that should be aborted/joined on shutdown.
pub fn start_watchdog() -> Option<JoinHandle<()>> {
    let mut usec: u128 = 0;
    // Unset the env vars for children so they don't inherit WATCHDOG_USEC
    if let Some(duration) = sd_notify::watchdog_enabled() {
        usec = duration.as_micros();
    }
    if usec == 0 {
        return None;
    }

    // Be conservative: send heartbeat at one-third of the watchdog interval
    let interval = Duration::from_micros(u64::try_from(usec / 3).unwrap_or(u64::MAX));

    Some(tokio::spawn(async move {
        loop {
            let _unused = sd_notify::notify(&[NotifyState::Watchdog]);
            tokio::time::sleep(interval).await;
        }
    }))
}

/// If systemd passed socket file descriptors (socket activation), return a Vec of `RawFd`.
/// Caller is responsible for consuming them exactly once (`FromRawFd`).
pub fn take_listen_fds() -> Vec<RawFd> {
    // sd_notify doesn't provide listen_fds helpers; use the environment variable directly.
    // The standard behavior is that systemd sets LISTEN_FDS and LISTEN_PID.
    let listen_fds = std::env::var("LISTEN_FDS").ok();
    let listen_pid = std::env::var("LISTEN_PID").ok();

    let Some(listen_fds) = listen_fds else {
        return Vec::new();
    };
    let Some(listen_pid) = listen_pid else {
        return Vec::new();
    };

    // Ensure the PID matches our PID
    let pid_str = &listen_pid;
    if let Ok(pid) = pid_str.parse::<u32>() {
        if pid != std::process::id() {
            return Vec::new();
        }
    } else {
        return Vec::new();
    }

    let nfds: i32 = match listen_fds.parse() {
        Ok(n) if n > 0 => n,
        _ => return Vec::new(),
    };

    // sd-daemon starts at SD_LISTEN_FDS_START = 3
    let start_fd: i32 = 3;
    let mut fds = Vec::new();
    for i in 0..nfds {
        fds.push(
            start_fd
                .checked_add(i)
                .unwrap_or_else(|| panic!("fd overflow")) as RawFd,
        );
    }

    // Unset env so children won't inherit and repeated calls won't re-read
    // SAFETY: LISTEN_FDS is set by systemd's sd-daemon, removing it prevents double-initialization
    unsafe {
        std::env::remove_var("LISTEN_FDS");
    }
    // SAFETY: LISTEN_PID is set by systemd's sd-daemon, removing it prevents double-initialization
    unsafe {
        std::env::remove_var("LISTEN_PID");
    }

    // sd_listen_fds semantics reserve fds starting at 3; we return them in order
    // so the caller can consume them via FromRawFd exactly once.

    fds
}

// Stub implementations when the feature is not enabled are intentionally not provided here;
// consumers should gate usage behind cfg checks or call these from cfg-gated modules.
