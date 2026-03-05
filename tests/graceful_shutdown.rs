use libc;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::time::Duration;

// This test is Unix-only because it sends real signals to the spawned
// process (SIGTERM). Guard compilation on unix so Windows runners skip it.
#[cfg(unix)]
#[tokio::test]
async fn graceful_shutdown_waits_for_active_requests() {
    // ensure the binary exists (build if necessary)
    let bin_path = "target/debug/quest-log";
    if !std::path::Path::new(bin_path).exists() {
        let build = Command::new("cargo")
            .arg("build")
            .output()
            .expect("cargo build failed");
        assert!(build.status.success(), "cargo build failed");
    }

    // pick a free port
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = std_listener.local_addr().unwrap().port();
    drop(std_listener);

    // spawn server process with selected port
    let mut child = Command::new(bin_path)
        .arg("serve")
        .env("PORT", port.to_string())
        .env("ENABLE_TEST_ENDPOINTS", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn server");

    // wait for health endpoint to be ready
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let addr = format!("127.0.0.1:{}", port);
    // wait up to ~10s for readiness (server may be slower on CI)
    let mut ready = false;
    for _ in 0..100 {
        if let Ok(resp) = client.get(format!("http://{}/health", addr)).send().await {
            if resp.status().is_success() {
                ready = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(ready, "server did not become ready in time");

    // start a slow request in background
    let client2 = reqwest::Client::new();
    let slow_handle = tokio::spawn(async move {
        let resp = client2
            .get(format!("http://{}/test/slow", addr))
            .send()
            .await
            .expect("slow request failed");
        resp.text().await.expect("read body")
    });

    // give the request a moment to reach server; increase slightly for CI
    tokio::time::sleep(Duration::from_millis(2000)).await;

    // send SIGTERM to the server process using libc
    let pid = child.id() as libc::pid_t;
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }

    // the slow request should complete successfully despite shutdown
    let slow_res = tokio::time::timeout(Duration::from_secs(60), slow_handle)
        .await
        .expect("slow request timed out")
        .expect("slow task panicked");
    assert_eq!(slow_res, "done");

    // wait for the process to exit (with a margin)
    for _ in 0..35 {
        match child.try_wait() {
            Ok(Some(status)) => {
                assert!(status.success() || status.signal().is_some());
                return;
            }
            Ok(None) => {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(e) => panic!("error waiting for child: {}", e),
        }
    }
    // if still running, try to kill it and include stdout/stderr to aid debugging
    let _ = child.kill();
    match child.wait_with_output() {
        Ok(output) => {
            let out = String::from_utf8_lossy(&output.stdout);
            let err = String::from_utf8_lossy(&output.stderr);
            panic!(
                "server did not exit in time. stdout:\n{}\n--- stderr:\n{}",
                out, err
            );
        }
        Err(e) => panic!(
            "server did not exit in time and reading output failed: {}",
            e
        ),
    }
}
