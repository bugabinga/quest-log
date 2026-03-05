use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn start_server() -> Child {
    let mut child = Command::new("cargo")
        .args(["run", "--", "serve"])
        .env("PORT", "3458")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start server");

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let reader = BufReader::new(stdout);

    for line in reader.lines() {
        if let Ok(line) = line {
            if line.contains("Server listening") || line.contains("listening") {
                println!("Server ready: {}", line);
                break;
            }
        }
    }

    thread::sleep(Duration::from_millis(200));
    child
}

fn stop_server(mut child: Child) -> Duration {
    let start = Instant::now();

    #[cfg(unix)]
    {
        let pid = child.id();
        let _ = Command::new("kill")
            .arg("-INT")
            .arg(pid.to_string())
            .output();
    }

    match child.wait_timeout(Duration::from_secs(3)) {
        Ok(Some(status)) => {
            println!("Server exited with: {:?}", status);
        }
        Ok(None) => {
            println!("Server didn't exit in time, forcing...");
            let _ = child.kill();
            let _ = child.wait();
        }
        Err(e) => println!("Error waiting for server: {}", e),
    }

    start.elapsed()
}

trait ChildExt {
    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl ChildExt for Child {
    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>> {
        let start = Instant::now();
        loop {
            match self.try_wait()? {
                Some(status) => return Ok(Some(status)),
                None => {
                    if start.elapsed() > timeout {
                        return Ok(None);
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
}

#[test]
fn test_fast_shutdown() {
    println!("Testing fast graceful shutdown...");
    let server = start_server();
    thread::sleep(Duration::from_millis(200));

    let elapsed = stop_server(server);

    assert!(
        elapsed < Duration::from_secs(2),
        "Graceful shutdown took too long: {:?}",
        elapsed
    );

    println!("Shutdown completed in {:?}", elapsed);
}
