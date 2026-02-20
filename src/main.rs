mod cli;
mod database;
mod handlers;
mod models;
mod state;
mod tui;
#[cfg(all(feature = "systemd", target_os = "linux"))]
mod systemd;

use crate::database::Database;
use crate::handlers::ServerMessage;
use crate::state::AppState;
use axum::{
    Router,
    routing::{get, post},
};
use static_serve::embed_assets;
use std::net::SocketAddr;
use tokio::sync::broadcast;
use tokio::time::Duration;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(debug_assertions)]
embed_assets!("static", compress = true);

#[cfg(not(debug_assertions))]
embed_assets!("static", compress = true, ignore_paths = ["*.map"]);

fn setup_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "quest_log=debug,tokio=info,axum=warn".into());

    #[cfg(debug_assertions)]
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(true)
        .without_time()
        .compact();

    #[cfg(not(debug_assertions))]
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(true);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();

    #[cfg(debug_assertions)]
    eprintln!("⏰ Timestamps disabled in debug mode for cleaner output (灬•́_•̀灬)");
}

async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() {
    setup_logging();

    #[cfg(debug_assertions)]
    dotenvy::dotenv().ok();

    tracing::info!("✨ Quest Log starting up...");

    // Dispatch CLI; returns Ok(true) if server should run
    let should_run_server = match cli::run_cli().await {
        Ok(should_run) => should_run,
        Err(e) => {
            eprintln!("❌ CLI error: {}", e);
            std::process::exit(1);
        }
    };

    if !should_run_server {
        return;
    }

    let data_dir = std::env::var("QUEST_LOG_DATA_DIR").unwrap_or_else(|_| ".".to_string());
    tracing::info!(data_dir = %data_dir, "📂 Data directory set");

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    tracing::debug!(port = %port, "🔌 Port configured");

    tracing::info!("🗄️  Initializing database...");
    let db = Database::new()
        .await
        .expect("💥 Failed to connect to database");
    tracing::info!("✅ Database ready! (灬♥ω♥灬)");

    let (bcast_tx, _rx) = broadcast::channel::<ServerMessage>(128);

    let app_state = AppState {
        db,
        bcast: bcast_tx,
    };

    tracing::debug!("🏗️  Building router...");
    let mut router = Router::new()
        .route("/", get(handlers::quests))
        .route("/day/{date}", get(handlers::quests))
        .route("/navigate/{date}", get(handlers::navigate))
        .route("/quests/toggle", post(handlers::toggle_quest))
        .route("/events", get(handlers::events))
        .route("/health", get(health))
        .merge(static_router())
        .with_state(app_state.clone());

    // Test-only endpoints are enabled via ENABLE_TEST_ENDPOINTS=1 at runtime.
    // This avoids exposing test routes in normal production runs.
    if std::env::var("ENABLE_TEST_ENDPOINTS").unwrap_or_default() == "1" {
        tracing::debug!("🔧 Test endpoints enabled");
        router = router.route("/test/slow", get(handlers::slow));
    }

    let app = router;

    let port: u16 = port.parse().expect("💢 PORT must be a number");
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    tracing::info!(port, "🚀 Starting HTTP server...");

    // attach middleware to count active requests
    // middleware: no-op when active_requests not present in AppState for test suites
    let app = app;

    // If systemd provided socket activation fds, convert the first one to a TcpListener
    // and use it. Otherwise, bind normally to the configured address.
    let server = {
        #[cfg(all(feature = "systemd", target_os = "linux"))]
        {
            let fds = systemd::take_listen_fds();
            if let Some(fd) = fds.get(0) {
                use std::os::unix::io::FromRawFd;
                unsafe {
                    match std::net::TcpListener::from_raw_fd(*fd) {
                        std_listener => match tokio::net::TcpListener::from_std(std_listener) {
                            Ok(tokio_listener) => {
                                tracing::info!(fd = %fd, "🔌 Serving on socket-activated fd");
                                axum::serve(tokio_listener, app)
                            }
                            Err(e) => {
                                tracing::error!(error = %e, "💥 Failed to use socket-activated fd - falling back to bind");
                                let listener = match tokio::net::TcpListener::bind(addr).await {
                                    Ok(l) => l,
                                    Err(e) => {
                                        tracing::error!(error = %e, port = %port, "💥 Failed to bind to port - address may be in use");
                                        return;
                                    }
                                };
                                axum::serve(listener, app)
                            }
                        },
                    }
                }
            } else {
                // No socket activation fds; bind normally
                let listener = match tokio::net::TcpListener::bind(addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::error!(error = %e, port = %port, "💥 Failed to bind to port - address may be in use");
                        return;
                    }
                };
                axum::serve(listener, app)
            }
        }

        #[cfg(not(all(feature = "systemd", target_os = "linux")))]
        {
            let listener = match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!(error = %e, port = %port, "💥 Failed to bind to port - address may be in use");
                    return;
                }
            };
            axum::serve(listener, app)
        }
    };

    tracing::info!(url = %format!("http://{}", addr), "🎉 Server listening! (◕‿◕)");
    // If built with systemd support, send READY and start watchdog if enabled.
    #[cfg(all(feature = "systemd", target_os = "linux"))]
    {
        // If systemd handed us sockets, prefer them instead of binding above.
        let fds = systemd::take_listen_fds();
        if !fds.is_empty() {
            // Systemd passed socket activation fds; we detect and log them here.
            // A fuller integration would create a listener from the first fd and
            // avoid the manual bind above. For now we just log the presence of fds.
            if let Some(fd) = fds.get(0) {
                tracing::info!(fd = %fd, "🔌 Systemd provided socket activation fd(s) detected");
            }
        }

        systemd::notify_ready(Some("HTTP server listening"));
        let watchdog = systemd::start_watchdog();

        // Ensure watchdog task is dropped on shutdown
        // We attach it to a scope so the handle is dropped when main continues to shutdown.
        if let Some(handle) = watchdog {
            // spawn a task that awaits the handle so it keeps running until the handle is aborted
            tokio::spawn(async move {
                let _ = handle.await;
            });
        }
    }
    tracing::info!("💡 Open your browser and start questing!");

    // a oneshot bridge we will trigger when a shutdown signal arrives
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let graceful = server.with_graceful_shutdown(async {
        let _ = shutdown_rx.await;
    });

    // run the server in background so we can listen for signals concurrently
    let server_handle = tokio::spawn(async move {
        if let Err(e) = graceful.await {
            tracing::error!(error = %e, "💥 Server error - HTTP service failed");
        }
    });

    // wait for either ctrl-c or unix SIGTERM
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("🛑 SIGINT received, initiating graceful shutdown...");
        }
        _ = async {
            #[cfg(unix)]{
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm = signal(SignalKind::terminate()).expect("Failed to install SIGTERM handler");
                sigterm.recv().await
            }
            #[cfg(not(unix))]{
                futures::future::pending::<()>().await
            }
        } => {
            tracing::info!("🛑 SIGTERM received, initiating graceful shutdown...");
        }
    }

    tracing::info!(
        "⏳ Triggering server graceful shutdown (allowing in-flight requests to finish)..."
    );
    // best-effort send; ignore if receiver already dropped
    let _ = shutdown_tx.send(());

    // wait for the server task to finish with a generous timeout for CI
    match tokio::time::timeout(Duration::from_secs(30), server_handle).await {
        Ok(join_res) => {
            if let Err(e) = join_res {
                tracing::error!(error = %e, "💥 Server task panicked during shutdown");
            }
        }
        Err(_) => {
            tracing::error!("💥 Server did not shut down within 30s, forcing exit");
        }
    }
}
