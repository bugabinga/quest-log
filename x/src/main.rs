use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use reqwest::Client;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const VERSION: &str = env!("APP_VERSION");
const IMAGE_REPOSITORY: &str = "ghcr.io/bugabinga/quest-log";
const QUEST_LOG_DATA_DIR: &str = "QUEST_LOG_DATA_DIR";
const QUEST_LOG_X_SERVER_ID: &str = "QUEST_LOG_X_SERVER_ID";
const DEV_STATE_DIR: &str = "target/quest-log";

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
    /// Build application binary
    Build {
        /// Release build
        #[arg(long)]
        release: bool,
        /// Enable all features
        #[arg(long)]
        all_features: bool,
        /// Enable features
        #[arg(long, value_delimiter = ',')]
        features: Vec<String>,
        /// Build target triple
        #[arg(long)]
        triple: Option<String>,
    },
    /// Run unit tests
    Test { args: Vec<String> },
    /// Run all tests
    Verify { args: Vec<String> },
    /// Format Rust and JS code
    Fmt { args: Vec<String> },
    /// Quality gate: formatting check, clippy, JS lint, policy lint
    Lint,
    /// Fast typecheck
    Check,
    /// Generate coverage report
    Coverage,
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
    /// Clean build artifacts
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
    /// Update one locked dependency
    Update {
        package: String,
        #[arg(long)]
        precise: String,
    },
    /// Validate release tag against package version
    ReleaseCheck,
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
        Commands::Build {
            release,
            all_features,
            features,
            triple,
        } => build(release, all_features, &features, triple.as_deref()),
        Commands::Test { args } => test(&args),
        Commands::Verify { args } => verify(&args),
        Commands::Fmt { args } => fmt(&args),
        Commands::Lint => lint(),
        Commands::Check => check(),
        Commands::Coverage => coverage(),
        Commands::Run { subcommand, args } => run(&subcommand, &args),
        Commands::Serve { subcommand, args } => serve(&subcommand, &args),
        Commands::Kill => kill(),
        Commands::Watch => watch(),
        Commands::Clean => clean(),
        Commands::Container { command } => container(command),
        Commands::Bundle { command } => bundle(command),
        Commands::Commit { command } => commit(command),
        Commands::Update { package, precise } => update(&package, &precise),
        Commands::ReleaseCheck => release_check(),
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

fn build(
    release: bool,
    all_features: bool,
    features: &[String],
    target: Option<&str>,
) -> Result<()> {
    let mut args = vec![
        "build".to_string(),
        "--package".to_string(),
        "quest-log".to_string(),
    ];
    if release {
        args.push("--release".to_string());
    }
    if all_features {
        args.push("--all-features".to_string());
    }
    if !features.is_empty() {
        args.push("--features".to_string());
        args.push(features.join(","));
    }
    if let Some(target) = target {
        args.push("--target".to_string());
        args.push(target.to_string());
    }

    let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_cargo(&refs)
}

fn test(args: &[String]) -> Result<()> {
    let mut app_args = vec![
        "test",
        "--package",
        "quest-log",
        "--lib",
        "--features",
        "test-utils",
    ];
    app_args.extend(args.iter().map(|s| s.as_str()));
    run_cargo(&app_args)?;

    run_cargo(&["test", "--package", "x"])
}

#[derive(Debug, PartialEq, Eq)]
enum VerifyStep {
    Rust,
    Xtask,
    Browser,
}

fn verify_steps() -> [VerifyStep; 3] {
    [VerifyStep::Rust, VerifyStep::Xtask, VerifyStep::Browser]
}

fn verify(args: &[String]) -> Result<()> {
    for step in verify_steps() {
        match step {
            VerifyStep::Rust => {
                let mut app_args =
                    vec!["test", "--package", "quest-log", "--features", "test-utils"];
                app_args.extend(args.iter().map(|s| s.as_str()));
                run_cargo(&app_args)?;
            }
            VerifyStep::Xtask => run_cargo(&["test", "--package", "x"])?,
            VerifyStep::Browser => browser(false)?,
        }
    }

    Ok(())
}

fn fmt(args: &[String]) -> Result<()> {
    let cargo_args = cargo_fmt_args(args);
    let cargo_refs = cargo_args.iter().map(String::as_str).collect::<Vec<_>>();
    run_cargo(&cargo_refs)?;

    ensure_deno()?;
    let deno_args = deno_fmt_args(args);
    let deno_refs = deno_args.iter().map(String::as_str).collect::<Vec<_>>();
    run_cmd("deno", &deno_refs)?;

    ensure_dictator()?;
    run_cmd(
        "dictator",
        &["dictate", "src/", "x/src/", "tests/", "static/js/"],
    )
}

fn cargo_fmt_args(args: &[String]) -> Vec<String> {
    let mut fmt_args = vec!["fmt".to_string()];
    fmt_args.extend(args.iter().cloned());
    fmt_args
}

fn deno_fmt_args(args: &[String]) -> Vec<String> {
    let mut fmt_args = vec!["fmt".to_string()];
    fmt_args.extend(args.iter().cloned());
    fmt_args.push("static/".to_string());
    fmt_args
}

fn lint() -> Result<()> {
    let steps = lint_step_names();

    run_lint_step(steps[0], || {
        ensure_dictator()?;
        run_cmd(
            "dictator",
            &["lint", "src/", "x/src/", "tests/", "static/js/"],
        )
    })?;

    run_lint_step(steps[1], || run_cargo(&["fmt", "--check"]))?;

    run_lint_step(steps[2], || run_cargo(&clippy_lint_args()))?;

    run_lint_step(steps[3], || {
        ensure_deno()?;
        run_cmd("deno", &["lint", "static/js/"])
    })?;

    run_lint_step(steps[4], check_sse_macro_usage)
}

fn run_lint_step(step: &str, action: impl FnOnce() -> Result<()>) -> Result<()> {
    println!("==> {step}");
    action()
}

fn lint_step_names() -> [&'static str; 5] {
    [
        "dictator lint",
        "rustfmt check",
        "clippy workspace",
        "deno lint",
        "sse macro policy",
    ]
}

fn clippy_lint_args() -> [&'static str; 7] {
    [
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ]
}

fn check() -> Result<()> {
    run_cargo(&check_args())
}

fn check_args() -> [&'static str; 5] {
    [
        "check",
        "--workspace",
        "--all-targets",
        "--features",
        "test-utils",
    ]
}

fn coverage() -> Result<()> {
    if !cargo_subcommand_exists("tarpaulin")? {
        bail!("cargo-tarpaulin not installed. Install: cargo install cargo-tarpaulin");
    }

    run_cargo(&coverage_args())
}

fn coverage_args() -> [&'static str; 7] {
    [
        "tarpaulin",
        "--package",
        "quest-log",
        "--out",
        "Xml",
        "--features",
        "test-utils",
    ]
}

fn build_server_command(subcommand: &str, args: &[String]) -> Result<Command> {
    let mut cmd = Command::new("cargo");
    apply_server_env(&mut cmd)?;

    cmd.args(["run", "--", subcommand])
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    Ok(cmd)
}

fn apply_server_env(cmd: &mut Command) -> Result<()> {
    for (key, value) in server_env_vars(
        std::env::var("RUST_LOG").ok(),
        std::env::var_os(QUEST_LOG_DATA_DIR).map(PathBuf::from),
        &std::env::current_dir()?,
    ) {
        cmd.env(key, value);
    }
    Ok(())
}

fn server_env_vars(
    rust_log: Option<String>,
    data_dir: Option<PathBuf>,
    cwd: &std::path::Path,
) -> Vec<(String, String)> {
    vec![
        (
            "RUST_LOG".to_string(),
            rust_log.unwrap_or_else(|| "debug".to_string()),
        ),
        (
            QUEST_LOG_DATA_DIR.to_string(),
            data_dir
                .unwrap_or_else(|| cwd.join(DEV_STATE_DIR).join("data"))
                .display()
                .to_string(),
        ),
    ]
}

fn dev_state_path(name: &str) -> PathBuf {
    PathBuf::from(DEV_STATE_DIR).join(name)
}

fn run(subcommand: &str, args: &[String]) -> Result<()> {
    let status = build_server_command(subcommand, args)?
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

    let mut cmd = build_server_command(subcommand, args)?;
    let server_id = new_server_id();
    cmd.env(QUEST_LOG_X_SERVER_ID, &server_id);
    let state_dir = std::env::current_dir()?.join(DEV_STATE_DIR);
    std::fs::create_dir_all(&state_dir)?;
    let log_file_path = state_dir.join("server.log");
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
    std::fs::write(server_id_path(), server_id)?;
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
    let server_id = std::fs::read_to_string(server_id_path()).ok();

    let mut was_running = false;

    if let (Some(pid), Some(server_id)) = (pid, server_id.as_deref()) {
        was_running = process_is_alive(pid) && process_has_server_id(pid, server_id.trim());
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
    let _ = std::fs::remove_file(server_id_path());

    (pid, was_running)
}

fn new_server_id() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("{}-{now}", std::process::id())
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

#[cfg(target_os = "linux")]
fn process_has_server_id(pid: u32, server_id: &str) -> bool {
    std::fs::read(format!("/proc/{pid}/environ"))
        .map(|environ| environ_has_var(&environ, QUEST_LOG_X_SERVER_ID, server_id))
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn process_has_server_id(_pid: u32, _server_id: &str) -> bool {
    false
}

fn environ_has_var(environ: &[u8], name: &str, value: &str) -> bool {
    let expected = format!("{name}={value}");
    environ
        .split(|byte| *byte == 0)
        .any(|entry| entry == expected.as_bytes())
}

fn pid_path() -> PathBuf {
    dev_state_path("server.pid")
}

fn server_id_path() -> PathBuf {
    dev_state_path("server.id")
}

fn watch() -> Result<()> {
    if !cargo_subcommand_exists("watch")? {
        println!("cargo-watch not installed; running one check instead");
        return check();
    }

    let mut cmd = Command::new("cargo");
    apply_server_env(&mut cmd)?;
    cmd.args(watch_args())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = cmd.status().context("Failed to run cargo watch")?;
    if !status.success() {
        bail!("cargo watch failed");
    }
    Ok(())
}

fn watch_args() -> [&'static str; 7] {
    [
        "watch",
        "--delay",
        "1",
        "--exec",
        "run -- serve",
        "--notify",
        "--clear",
    ]
}

fn clean() -> Result<()> {
    run_cargo(&["clean"])
}

fn container(command: ContainerCommands) -> Result<()> {
    match command {
        ContainerCommands::Build { release } => {
            if release {
                check_clean()?;
                check_release_tag_if_present()?;
            }
            build_container()
        }
        ContainerCommands::Push { release } => {
            if release {
                check_clean()?;
                check_release_tag_if_present()?;
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
            &format!("VERSION={VERSION}"),
            "-t",
            &format!("{IMAGE_REPOSITORY}:local"),
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
            &format!("{IMAGE_REPOSITORY}:local"),
            &format!("{IMAGE_REPOSITORY}:{VERSION}"),
        ],
    )?;
    run_cmd(
        "podman",
        &[
            "tag",
            &format!("{IMAGE_REPOSITORY}:local"),
            &format!("{IMAGE_REPOSITORY}:latest"),
        ],
    )?;
    println!("Tagged as {IMAGE_REPOSITORY}:{VERSION} and latest");
    Ok(())
}

fn push_container() -> Result<()> {
    run_cmd(
        "podman",
        &["push", &format!("{IMAGE_REPOSITORY}:{VERSION}")],
    )?;
    run_cmd("podman", &["push", &format!("{IMAGE_REPOSITORY}:latest")])
}

fn run_migrate(volume: &str) -> Result<()> {
    run_cmd(
        "podman",
        &[
            "run",
            "--rm",
            "-e",
            "QUEST_LOG_DATA_DIR=/data",
            "-v",
            &format!("{}:/data", volume),
            &format!("{IMAGE_REPOSITORY}:local"),
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
    std::fs::create_dir_all("static/vendor")?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    runtime.block_on(async {
        let client = Client::new();

        let js_url = format!(
            "https://cdn.jsdelivr.net/gh/starfederation/datastar@{}/bundles/datastar.js",
            version
        );

        let js_result = client.get(&js_url).send().await?;
        http_status_ok(js_result.status(), &js_url)?;
        let js_content = js_result.bytes().await?;

        std::fs::write("static/vendor/datastar.js", &js_content)?;

        Ok::<(), anyhow::Error>(())
    })
}

fn http_status_ok(status: reqwest::StatusCode, url: &str) -> Result<()> {
    if !status.is_success() {
        bail!("download failed: {url} returned {status}");
    }
    Ok(())
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

fn update(package: &str, precise: &str) -> Result<()> {
    run_cargo(&update_args(package, precise))
}

fn update_args<'a>(package: &'a str, precise: &'a str) -> [&'a str; 5] {
    ["update", "-p", package, "--precise", precise]
}

fn release_check() -> Result<()> {
    let tag = std::env::var("GITHUB_REF_NAME").context("GITHUB_REF_NAME is required")?;
    validate_release_tag(&tag)
}

fn check_release_tag_if_present() -> Result<()> {
    let Ok(github_ref) = std::env::var("GITHUB_REF") else {
        return Ok(());
    };
    if !is_tag_ref(&github_ref) {
        return Ok(());
    }

    let tag = std::env::var("GITHUB_REF_NAME").context("GITHUB_REF_NAME is required")?;
    validate_release_tag(&tag)
}

fn is_tag_ref(github_ref: &str) -> bool {
    github_ref.starts_with("refs/tags/")
}

fn validate_release_tag(tag: &str) -> Result<()> {
    let expected = expected_release_tag();
    if tag != expected {
        bail!("release tag {tag} does not match package version {expected}");
    }
    Ok(())
}

fn expected_release_tag() -> String {
    format!("v{VERSION}")
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
               feat(api)!: remove v1 endpoint\n\
\n\
             Your commit:\n             {}",
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

fn cargo_subcommand_exists(name: &str) -> Result<bool> {
    let output = Command::new("cargo")
        .arg("--list")
        .output()
        .context("Failed to list cargo commands")?;

    if !output.status.success() {
        bail!("cargo --list failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .any(|command| command == name))
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

fn ensure_dictator() -> Result<()> {
    let output = Command::new("dictator")
        .arg("--version")
        .output()
        .context("dictator not found")?;

    if !output.status.success() {
        bail!("dictator not installed. Install: cargo install dictator");
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

fn check_sse_macro_usage() -> Result<()> {
    let patterns = ["Sse::new(stream::iter", "Sse::new(futures::stream::iter"];

    for entry in walkdir::WalkDir::new("src/handlers")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let content = std::fs::read_to_string(entry.path())?;
        for pattern in &patterns {
            if content.contains(pattern) {
                bail!(
                    "Found manual SSE construction in {}.\n\
                     Pattern: '{}'\n\
                     Use `sse_response!(events)` macro instead.\n\
                     See src/sse.rs for documentation.",
                    entry.path().display(),
                    pattern
                );
            }
        }
    }

    Ok(())
}

fn browser(headed: bool) -> Result<()> {
    ensure_deno()?;
    let browser = std::env::var("BROWSER").map_or_else(|_| default_browser(), Ok)?;

    let args = Vec::new();
    serve("serve", &args)?;
    let test_result = wait_for_server().and_then(|()| run_browser_tests(headed, &browser));
    let kill_result = kill();

    test_result?;
    kill_result
}

fn wait_for_server() -> Result<()> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3000);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let deadline = Instant::now() + Duration::from_secs(20);

    while Instant::now() < deadline {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(200)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    bail!("server did not start on {addr}")
}

fn run_browser_tests(headed: bool, browser: &str) -> Result<()> {
    let mut args = vec!["task", "test:e2e"];

    if headed {
        args.push("--");
        args.push("--headed");
    }

    let mut cmd = Command::new("deno");
    cmd.args(&args);

    cmd.env("BROWSER", browser);
    if browser == "chrome"
        && std::env::var_os("BROWSER_EXECUTABLE").is_none()
        && let Some(path) = chrome_executable()
    {
        cmd.env("BROWSER_EXECUTABLE", path);
    }
    if !headed && std::env::var_os("HEADLESS").is_none() {
        cmd.env("HEADLESS", "true");
    }

    let status = cmd.status().context("Failed to run browser tests")?;

    if !status.success() {
        bail!("Browser tests failed");
    }
    Ok(())
}

fn default_browser() -> Result<String> {
    if chrome_executable().is_some() {
        return Ok("chrome".to_string());
    }
    bail!("Chrome not found. Install google-chrome, google-chrome-stable, or chromium")
}

fn chrome_executable() -> Option<String> {
    [
        "google-chrome",
        "google-chrome-stable",
        "chromium",
        "chromium-browser",
    ]
    .iter()
    .find_map(|program| program_path(program))
}

fn program_path(program: &str) -> Option<String> {
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|path| path.join(program))
            .find(|path| path.is_file())
            .map(|path| path.display().to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_runs_rust_tests_then_xtask_tests_then_browser_tests() {
        assert_eq!(
            verify_steps(),
            [VerifyStep::Rust, VerifyStep::Xtask, VerifyStep::Browser]
        );
    }

    #[test]
    fn fmt_check_args_are_not_duplicated_for_deno() {
        let args = vec!["--check".to_string()];

        assert_eq!(cargo_fmt_args(&args), ["fmt", "--check"]);
        assert_eq!(deno_fmt_args(&args), ["fmt", "--check", "static/"]);
    }

    #[test]
    fn check_checks_workspace_targets() {
        assert_eq!(
            check_args(),
            [
                "check",
                "--workspace",
                "--all-targets",
                "--features",
                "test-utils",
            ]
        );
    }

    #[test]
    fn lint_checks_entire_workspace_with_clippy() {
        assert_eq!(
            clippy_lint_args(),
            [
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ]
        );
    }

    #[test]
    fn lint_steps_are_explicit() {
        assert_eq!(
            lint_step_names(),
            [
                "dictator lint",
                "rustfmt check",
                "clippy workspace",
                "deno lint",
                "sse macro policy",
            ]
        );
    }

    fn ci_workflow() -> String {
        let ci_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join(".github/workflows/ci.yml");
        std::fs::read_to_string(ci_path).unwrap()
    }

    #[test]
    fn ci_installs_locked_dictator_version() {
        assert!(ci_workflow().contains("cargo install dictator --version 0.16.5 --locked"));
    }

    #[test]
    fn release_matrix_is_linux_only() {
        let ci = ci_workflow();

        assert!(!ci.contains("windows-"));
        assert!(!ci.contains("macos-"));
        assert!(!ci.contains("pc-windows"));
        assert!(!ci.contains("apple-darwin"));
    }

    #[test]
    fn release_upload_uses_ref_name_tag() {
        let ci = ci_workflow();

        assert!(ci.contains("cargo x release-check"));
        assert!(ci.contains("tag_name: ${{ github.ref_name }}"));
        assert!(!ci.contains("tag_name: ${{ github.ref }}"));
    }

    #[test]
    fn release_tag_must_match_package_version() {
        assert_eq!(expected_release_tag(), format!("v{VERSION}"));
        assert!(validate_release_tag(&expected_release_tag()).is_ok());
        assert!(validate_release_tag("v999.0.0").is_err());
    }

    #[test]
    fn release_tag_validation_only_runs_for_tags() {
        assert!(is_tag_ref("refs/tags/v1.2.3"));
        assert!(!is_tag_ref("refs/heads/trunk"));
    }

    #[test]
    fn ci_migration_pulls_published_image() {
        let ci = ci_workflow();

        assert!(ci.contains("podman pull ghcr.io/bugabinga/quest-log:latest"));
        assert!(ci.contains(
            "podman tag ghcr.io/bugabinga/quest-log:latest ghcr.io/bugabinga/quest-log:local"
        ));
    }

    #[test]
    fn container_image_repository_is_ghcr() {
        let unit_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("container/systemd/quest-log.container");
        let unit = std::fs::read_to_string(unit_path).unwrap();

        assert_eq!(IMAGE_REPOSITORY, "ghcr.io/bugabinga/quest-log");
        assert!(unit.contains("Image=ghcr.io/bugabinga/quest-log:latest"));
    }

    #[test]
    fn watch_uses_server_defaults() {
        assert_eq!(
            watch_args(),
            [
                "watch",
                "--delay",
                "1",
                "--exec",
                "run -- serve",
                "--notify",
                "--clear"
            ]
        );
        assert_eq!(
            server_env_vars(None, None, std::path::Path::new("/repo")),
            [
                ("RUST_LOG".to_string(), "debug".to_string()),
                (
                    QUEST_LOG_DATA_DIR.to_string(),
                    "/repo/target/quest-log/data".to_string(),
                ),
            ]
        );
    }

    #[test]
    fn coverage_fails_if_tool_missing_and_checks_integration_tests() {
        assert_eq!(
            coverage_args(),
            [
                "tarpaulin",
                "--package",
                "quest-log",
                "--out",
                "Xml",
                "--features",
                "test-utils",
            ]
        );
        assert!(!coverage_args().contains(&"--lib"));
    }

    #[test]
    fn datastar_download_rejects_http_errors() {
        assert!(http_status_ok(reqwest::StatusCode::OK, "url").is_ok());
        assert!(http_status_ok(reqwest::StatusCode::NOT_FOUND, "url").is_err());
    }

    #[test]
    fn update_pins_one_package() {
        assert_eq!(
            update_args("rand@0.9.2", "0.9.3"),
            ["update", "-p", "rand@0.9.2", "--precise", "0.9.3"]
        );
    }

    #[test]
    fn server_pid_marker_matches_exact_env_var() {
        let env = b"PATH=/bin\0QUEST_LOG_X_SERVER_ID=abc\0OTHER=x\0";

        assert!(environ_has_var(env, QUEST_LOG_X_SERVER_ID, "abc"));
        assert!(!environ_has_var(env, QUEST_LOG_X_SERVER_ID, "ab"));
        assert!(!environ_has_var(env, "SERVER_ID", "abc"));
    }
}
