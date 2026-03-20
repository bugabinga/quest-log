use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use reqwest::Client;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const VERSION: &str = env!("APP_VERSION");

#[derive(Parser)]
#[command(name = "x")]
#[command(version = VERSION)]
#[command(about = "Quest Log development tasks")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run unit tests
    Test { args: Vec<String> },
    /// Run all tests (integration)
    Verify { args: Vec<String> },
    /// Format Rust and JS code
    Fmt { args: Vec<String> },
    /// Check formatting, clippy, and lint
    Lint,
    /// Full check (lint + verify + cargo check)
    Check,
    /// Run the application in foreground (blocks)
    Run {
        #[arg(default_value = "serve")]
        subcommand: String,
        args: Vec<String>,
    },
    /// Run the application in background (non-blocking)
    Serve {
        #[arg(default_value = "serve")]
        subcommand: String,
        args: Vec<String>,
    },
    /// Kill the background server
    Kill,
    /// Watch for changes and rebuild
    Watch,
    /// Clean build artifacts and database
    Clean,
    /// Container operations
    Container {
        #[command(subcommand)]
        command: ContainerCommands,
    },
    /// Bundle datastar from CDN
    Bundle {
        #[command(subcommand)]
        command: BundleCommands,
    },
    /// Validate commit message
    Commit {
        #[command(subcommand)]
        command: CommitCommands,
    },
    /// Process assets (favicons, icons)
    Assets,
    /// Run browser/E2E tests
    Browser {
        #[arg(long, action = clap::ArgAction::SetTrue)]
        headed: bool,
    },
}

#[derive(Subcommand)]
enum ContainerCommands {
    /// Build container image
    Build {
        /// Release build (requires clean git)
        #[arg(long)]
        release: bool,
    },
    /// Push container to registry
    Push {
        /// Release build (requires clean git)
        #[arg(long)]
        release: bool,
    },
    /// Tag container with version
    Tag,
    /// Run database migration in container
    Migrate {
        /// Volume name
        #[arg(default_value = "quest-log-data")]
        volume: String,
    },
}

#[derive(Subcommand)]
enum BundleCommands {
    /// Bundle datastar JS library
    Datastar {
        /// Version to bundle
        #[arg(default_value = "1.0.0-RC.8")]
        version: String,
    },
}

#[derive(Subcommand)]
enum CommitCommands {
    /// Validate commit message format
    Validate {
        /// Path to commit message file
        commit_msg_file: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Test { args } => test(&args),
        Commands::Verify { args } => verify(&args),
        Commands::Fmt { args } => fmt(&args),
        Commands::Lint => lint(),
        Commands::Check => check(),
        Commands::Run { subcommand, args } => run(&subcommand, &args),
        Commands::Serve { subcommand, args } => serve(&subcommand, &args),
        Commands::Kill => kill(),
        Commands::Watch => watch(),
        Commands::Clean => clean(),
        Commands::Container { command } => container(command),
        Commands::Bundle { command } => bundle(command),
        Commands::Commit { command } => commit(command),
        Commands::Assets => assets(),
        Commands::Browser { headed } => browser(headed),
    }
}

fn run_cargo(args: &[&str]) -> Result<()> {
    let status = Command::new("cargo")
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("Failed to run cargo")?;
    if !status.success() {
        bail!("cargo command failed");
    }
    Ok(())
}

fn run_cmd(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context(format!("Failed to run {program}"))?;
    if !status.success() {
        bail!("{program} command failed");
    }
    Ok(())
}

fn test(args: &[String]) -> Result<()> {
    let mut cmd_args = vec!["test", "--lib", "--features", "test-utils"];
    cmd_args.extend(args.iter().map(|s| s.as_str()));
    run_cargo(&cmd_args)
}

fn verify(args: &[String]) -> Result<()> {
    let mut cmd_args = vec!["test", "--features", "test-utils"];
    cmd_args.extend(args.iter().map(|s| s.as_str()));
    run_cargo(&cmd_args)
}

fn fmt(args: &[String]) -> Result<()> {
    let mut cargo_args = vec!["fmt"];
    cargo_args.extend(args.iter().map(|s| s.as_str()));
    run_cargo(&cargo_args)?;

    let mut deno_args = vec!["fmt"];
    deno_args.extend(args.iter().map(|s| s.as_str()));
    deno_args.push("static/");
    ensure_deno()?;
    deno_args.extend(args.iter().map(|s| s.as_str()));
    run_cmd("deno", &deno_args)
}

fn lint() -> Result<()> {
    run_cargo(&["fmt", "--check"])?;

    run_cargo(&[
        "clippy",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ])?;

    ensure_deno()?;
    run_cmd("deno", &["lint", "static/js/"])
}

fn check() -> Result<()> {
    run_cargo(&["check", "--features", "test-utils"])
}

fn build_server_command(subcommand: &str, args: &[String]) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.env(
        "RUST_LOG",
        std::env::var("RUST_LOG").unwrap_or_else(|_| "debug".into()),
    )
    .args(["run", "--", subcommand])
    .args(args)
    .stdout(Stdio::inherit())
    .stderr(Stdio::inherit());
    cmd
}

fn run(subcommand: &str, args: &[String]) -> Result<()> {
    let status = build_server_command(subcommand, args)
        .status()
        .context("Failed to run cargo")?;
    if !status.success() {
        bail!("cargo run failed");
    }
    Ok(())
}

fn serve(subcommand: &str, args: &[String]) -> Result<()> {
    let (old_pid, old_was_running) = kill_server_internal();

    if old_was_running {
        println!("🛑 Stopping existing server (PID: {})", old_pid.unwrap());
    }

    let mut cmd = build_server_command(subcommand, args);
    let log_file_path = std::env::current_dir()?.join(".server.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_file_path)?;
    cmd.stdout(Stdio::from(log_file.try_clone()?))
        .stderr(Stdio::from(log_file));

    let child = cmd.spawn().context("Failed to spawn server")?;
    let pid = child.id();

    std::fs::write(pid_path(), pid.to_string())?;
    println!("🚀 Starting quest-log {} (PID: {})", subcommand, pid);
    println!("   Logs: {}", log_file_path.display());
    Ok(())
}

fn kill() -> Result<()> {
    let (pid, was_running) = kill_server_internal();

    if was_running {
        println!("🛑 Stopped server (PID: {})", pid.unwrap());
    } else {
        println!("No server running");
    }
    Ok(())
}

fn kill_server_internal() -> (Option<u32>, bool) {
    let pid: Option<u32> = match std::fs::read_to_string(pid_path()) {
        Ok(s) => s.trim().parse().ok(),
        Err(_) => None,
    };

    let mut was_running = false;

    if let Some(pid) = pid {
        was_running = process_is_alive(pid);
        if was_running {
            #[cfg(unix)]
            let _ = Command::new("kill")
                .arg("-TERM")
                .arg(pid.to_string())
                .status();
            #[cfg(windows)]
            let _ = Command::new("taskkill")
                .args(["/F", "/PID", &pid.to_string()])
                .status();
        }
    }

    let _ = std::fs::remove_file(pid_path());

    (pid, was_running)
}

#[cfg(unix)]
fn process_is_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(windows)]
fn process_is_alive(pid: u32) -> bool {
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}")])
        .output()
        .map(|o| {
            let output = String::from_utf8_lossy(&o.stdout);
            output.lines().any(|l| l.contains(&pid.to_string()))
        })
        .unwrap_or(false)
}

fn pid_path() -> PathBuf {
    PathBuf::from(".server.pid")
}

fn watch() -> Result<()> {
    run_cmd(
        "cargo",
        &[
            "watch", "--delay", "1", "--exec", "run", "--notify", "--clear",
        ],
    )
}

fn clean() -> Result<()> {
    run_cargo(&["clean"])?;
    let db_path = std::path::Path::new("quests.db");
    if db_path.exists() {
        std::fs::remove_file(db_path).context("Failed to remove quests.db")?;
    }
    Ok(())
}

fn container(command: ContainerCommands) -> Result<()> {
    match command {
        ContainerCommands::Build { release } => {
            if release {
                check_clean()?;
            }
            build_container()
        }
        ContainerCommands::Push { release } => {
            if release {
                check_clean()?;
            }
            build_container()?;
            tag_container()?;
            push_container()
        }
        ContainerCommands::Tag => tag_container(),
        ContainerCommands::Migrate { volume } => run_migrate(&volume),
    }
}

fn build_container() -> Result<()> {
    run_cmd(
        "podman",
        &[
            "build",
            "--build-arg",
            &format!("VERSION={}", VERSION),
            "-t",
            "bugabinga/quest-log:local",
            "-f",
            "Containerfile",
            ".",
        ],
    )
}

fn tag_container() -> Result<()> {
    run_cmd(
        "podman",
        &[
            "tag",
            "bugabinga/quest-log:local",
            &format!("bugabinga/quest-log:{}", VERSION),
        ],
    )?;
    run_cmd(
        "podman",
        &[
            "tag",
            "bugabinga/quest-log:local",
            "bugabinga/quest-log:latest",
        ],
    )?;
    println!("Tagged as bugabinga/quest-log:{} and latest", VERSION);
    Ok(())
}

fn push_container() -> Result<()> {
    run_cmd(
        "podman",
        &["push", &format!("bugabinga/quest-log:{}", VERSION)],
    )?;
    run_cmd("podman", &["push", "bugabinga/quest-log:latest"])
}

fn run_migrate(volume: &str) -> Result<()> {
    run_cmd(
        "podman",
        &[
            "run",
            "--rm",
            "-v",
            &format!("{}:/data", volume),
            "bugabinga/quest-log:local",
            "migrate-only",
        ],
    )
}

fn bundle(command: BundleCommands) -> Result<()> {
    match command {
        BundleCommands::Datastar { version } => bundle_datastar(&version),
    }
}

fn bundle_datastar(version: &str) -> Result<()> {
    std::fs::create_dir_all("static/js")?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let client = Client::new();

        let js_url = format!(
            "https://cdn.jsdelivr.net/gh/starfederation/datastar@{}/bundles/datastar.js",
            version
        );
        let map_url = format!(
            "https://cdn.jsdelivr.net/gh/starfederation/datastar@{}/bundles/datastar.js.map",
            version
        );

        let (js_result, map_result) =
            tokio::join!(client.get(&js_url).send(), client.get(&map_url).send());

        let js_content = js_result?.bytes().await?;
        std::fs::write("static/js/datastar.js", &js_content)?;

        let map_content = map_result?.bytes().await?;
        std::fs::write("static/js/datastar.js.map", &map_content)?;

        Ok::<(), anyhow::Error>(())
    })
}

fn assets() -> Result<()> {
    use image::{ImageFormat, ImageReader};

    let static_dir = std::path::Path::new("static/images");
    std::fs::create_dir_all(static_dir)?;

    // Favicon and PWA icons
    let src = std::path::Path::new("assets/favicon.png");
    let img = ImageReader::open(src)?.decode()?;

    let img192 = img.resize(192, 192, image::imageops::FilterType::Lanczos3);
    img192.save_with_format("static/images/icon-192.png", ImageFormat::Png)?;
    println!("Generated: static/images/icon-192.png");

    let img512 = img.resize(512, 512, image::imageops::FilterType::Lanczos3);
    img512.save_with_format("static/images/icon-512.png", ImageFormat::Png)?;
    println!("Generated: static/images/icon-512.png");

    let img32 = img.resize(32, 32, image::imageops::FilterType::Lanczos3);
    img32.save_with_format("static/favicon.png", ImageFormat::Png)?;
    println!("Generated: static/favicon.png");

    // Day sprite sheets - resize to 512x512 with Nearest neighbor for pixel art
    let day_names = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ];
    for day in day_names {
        let src_path = format!("assets/{}.png", day);
        let dst_path = format!("static/images/{}.png", day);

        let img = ImageReader::open(&src_path)?.decode()?;
        let resized = img.resize(512, 512, image::imageops::FilterType::Nearest);
        resized.save_with_format(&dst_path, ImageFormat::Png)?;
        println!("Generated: {}", dst_path);
    }

    println!("\nAll assets processed successfully!");
    Ok(())
}

fn commit(command: CommitCommands) -> Result<()> {
    match command {
        CommitCommands::Validate { commit_msg_file } => validate_commit_msg(&commit_msg_file),
    }
}

fn validate_commit_msg(file_path: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "-q", "--verify", "MERGE_HEAD"])
        .output()
        .context("Failed to check for merge commit")?;

    if !output.stdout.is_empty() {
        return Ok(());
    }

    let msg = std::fs::read_to_string(file_path).context("Failed to read commit message file")?;
    let first_line = msg.lines().next().unwrap_or("");

    let pattern =
        regex::Regex::new(r"^(feat|fix|docs|style|refactor|test|chore|perf|revert)(\(.+\))?!?: .+")
            .expect("Invalid regex");

    if !pattern.is_match(first_line) {
        bail!(
            "Invalid commit message format.\n\n\
             Expected: <type>(<scope>)<!>: <subject>\n\
               - Type: feat, fix, docs, style, refactor, test, chore, perf, revert\n\
               - Add ! before : for breaking changes\n\n\
             Examples:\n\
               feat(auth): add login button\n\
               fix(ui): resolve padding issue\n\
               feat(api)!: remove v1 endpoint\n\n\
             Your commit:\n\
             {}",
            first_line
        );
    }

    let subject = first_line.split(": ").nth(1).unwrap_or("");
    if subject.len() > 72 {
        bail!(
            "Subject line exceeds 72 characters (current: {})",
            subject.len()
        );
    }

    Ok(())
}

fn ensure_deno() -> Result<()> {
    let output = Command::new("deno")
        .arg("--version")
        .output()
        .context("deno not found")?;

    if !output.status.success() {
        bail!("deno not installed. Install: curl -fsSL https://deno.land/install.sh | sh");
    }
    Ok(())
}

fn check_clean() -> Result<()> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to check git status")?;

    if !output.stdout.is_empty() {
        bail!("Working directory is dirty. Commit or stash changes before release.");
    }
    Ok(())
}

fn browser(headed: bool) -> Result<()> {
    let mut args = vec!["task", "test:e2e"];

    if headed {
        args.push("--");
        args.push("--headed");
    }

    let status = Command::new("deno")
        .args(&args)
        .status()
        .context("Failed to run playwright tests")?;

    if !status.success() {
        bail!("Playwright tests failed");
    }
    Ok(())
}
