//! Integration tests for graceful shutdown behavior and signal handling.
#![allow(
    clippy::tests_outside_test_module,
    reason = "Integration tests in tests/ are only compiled during cargo test"
)]
#![allow(
    clippy::expect_used,
    reason = "Expects are used for test setup that should not fail"
)]

use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::time::{Duration, Instant};

fn get_bin_path() -> std::path::PathBuf {
    let exe = std::env::current_exe().expect("failed to get current exe");
    exe.parent()
        .and_then(|p| p.parent())
        .expect("failed to get grandparent")
        .join("quest-log")
}

#[cfg(unix)]
#[tokio::test]
async fn instant_shutdown_completes_within_300ms() {
    let bin_path = get_bin_path();

    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = std_listener.local_addr().unwrap().port();
    drop(std_listener);

    let mut child = Command::new(&bin_path)
        .arg("serve")
        .env("PORT", port.to_string())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn server");

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let addr = format!("127.0.0.1:{port}");
    let mut ready = false;
    for _ in 0..100 {
        if let Ok(resp) = client.get(format!("http://{addr}/health")).send().await
            && resp.status().is_success()
        {
            ready = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(ready, "server did not become ready in time");

    let client2 = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let sse_handle = tokio::spawn(async move {
        let resp = client2
            .get(format!("http://{addr}/events"))
            .send()
            .await
            .expect("SSE request failed");
        let _unused = resp.bytes().await;
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    let pid = child.id().cast_signed();
    let start = Instant::now();
    // SAFETY: kill is a POSIX function that sends a signal to a process.
    // We use it here to send SIGTERM to the child process for testing shutdown.
    unsafe {
        libc::kill(pid, libc::SIGINT);
    }

    let max_wait = Duration::from_millis(300);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let elapsed = start.elapsed();
                assert!(
                    elapsed < max_wait,
                    "Shutdown took {}ms (expected <300ms)",
                    elapsed.as_millis()
                );
                assert!(status.success() || status.signal().is_some());
                let _unused = sse_handle.await;
                return;
            }
            Ok(None) => {
                if start.elapsed() >= max_wait {
                    let _unused = child.kill();
                    panic!(
                        "Server did not exit within 300ms (took {:?})",
                        start.elapsed()
                    );
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(e) => panic!("error waiting for child: {e}"),
        }
    }
}
