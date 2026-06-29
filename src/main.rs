//! Quest Log - A habit tracking application with weekly rewards and gamification.

mod auth;
mod cli;
mod config;
mod database;
mod extractors;
mod handlers;
mod models;
mod sse;
mod state;
mod systemd;
mod time;
mod ui;

use crate::config::{bind_addr, data_dir, port};
use crate::database::Database;
use crate::handlers::ServerMessage;
use crate::state::AppState;
use axum::{
    Router,
    routing::{delete, get, post, put},
};
// static assets are embedded via `static-serve` in normal builds.
use std::net::SocketAddr;
use tokio::sync::broadcast;
use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod static_assets {
    #![allow(
        missing_docs,
        reason = "Macro from static-serve crate generates undocumented function"
    )]

    static_serve::embed_assets!("static", compress = true);
}

fn setup_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "quest_log=debug,tokio=info,axum=warn".into());

    #[cfg(debug_assertions)]
    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(true);

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
        .with(ErrorLayer::default())
        .init();
}

async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
/// Starts the Quest Log server with the configured state and routes.
async fn main() {
    setup_logging();

    #[cfg(debug_assertions)]
    let _unused = dotenvy::dotenv();

    tracing::info!("✨ Quest Log starting up...");

    // Dispatch CLI; returns Ok(true) if server should run
    let should_run_server = match cli::run_cli().await {
        Ok(should_run) => should_run,
        Err(e) => {
            eprintln!("❌ CLI error: {e}");
            std::process::exit(1);
        }
    };

    if !should_run_server {
        return;
    }

    let data_dir = match data_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("❌ Configuration error: {e}");
            std::process::exit(1);
        }
    };
    tracing::info!(data_dir = %data_dir, "📂 Data directory set");

    let port = match port() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Configuration error: {e}");
            std::process::exit(1);
        }
    };
    let bind_addr = match bind_addr() {
        Ok(addr) => addr,
        Err(e) => {
            eprintln!("❌ Configuration error: {e}");
            std::process::exit(1);
        }
    };
    tracing::debug!(bind_addr = %bind_addr, port = %port, "🔌 Address configured");

    if !cfg!(debug_assertions)
        && let Err(e) = config::required_editor_password_hash()
    {
        eprintln!("❌ Configuration error: {e}");
        std::process::exit(1);
    }

    tracing::info!("🗄️  Initializing database...");
    let db = match Database::new().await {
        Ok(db) => db,
        Err(e) => {
            tracing::error!(error = %e, "💥 Failed to connect to database");
            eprintln!("❌ Failed to connect to database: {e}");
            std::process::exit(1);
        }
    };
    tracing::info!("✅ Database ready! (灬♥ω♥灬)");

    let (bcast_tx, _rx) = broadcast::channel::<ServerMessage>(128);

    let app_state = AppState::new(db, bcast_tx);

    tracing::debug!("🏗️  Building router...");
    let router = Router::new()
        .route("/", get(handlers::quests::quests))
        .route("/bounty", get(handlers::bounty::bounty))
        .route("/highscore", get(handlers::stats::highscore))
        .route("/day/{date}", get(handlers::quests::quests_with_date))
        .route("/navigate/{date}", get(handlers::navigate::navigate))
        .route("/quests/toggle", post(handlers::quests::toggle_quest))
        .route("/rewards/claim", post(handlers::bounty::claim_reward))
        .route("/events", get(handlers::events::events))
        .route("/health", get(health))
        // Editor routes
        .route("/editor", get(handlers::editor::auth::editor_page_handler))
        .route("/editor/login", post(handlers::editor::auth::login_handler))
        .route(
            "/editor/logout",
            post(handlers::editor::auth::logout_handler),
        )
        .route(
            "/editor/quests",
            get(handlers::editor::quests::get_quests_handler),
        )
        .route(
            "/editor/quests",
            post(handlers::editor::quests::create_quest_handler),
        )
        .route(
            "/editor/quests/{id}",
            put(handlers::editor::quests::update_quest_handler),
        )
        .route(
            "/editor/quests/{id}",
            delete(handlers::editor::quests::delete_quest_handler),
        )
        .route(
            "/editor/quests/{id}/edit",
            get(handlers::editor::quests::edit_quest_handler),
        )
        .route(
            "/editor/rewards",
            get(handlers::editor::rewards::get_rewards_handler),
        )
        .route(
            "/editor/rewards",
            post(handlers::editor::rewards::create_reward_handler),
        )
        .route(
            "/editor/rewards/{id}",
            put(handlers::editor::rewards::update_reward_handler),
        )
        .route(
            "/editor/rewards/{id}",
            delete(handlers::editor::rewards::delete_reward_handler),
        )
        .route(
            "/editor/rewards/{id}/edit",
            get(handlers::editor::rewards::edit_reward_handler),
        )
        .route(
            "/editor/settings",
            get(handlers::editor::settings::get_settings_handler),
        )
        .route(
            "/editor/settings",
            put(handlers::editor::settings::update_settings_handler),
        )
        .fallback(async |_req: axum::extract::State<AppState>| {
            Err::<axum::response::Html<String>, handlers::AppError>(handlers::AppError::NotFound)
        });

    // Test-only endpoints are enabled via ENABLE_TEST_ENDPOINTS=1 at runtime.
    // Only available with `test-utils` feature flag.
    // This avoids exposing test routes in normal production runs.
    #[cfg(feature = "test-utils")]
    let router = if config::test_endpoints_enabled() {
        tracing::debug!("🔧 Test endpoints enabled");
        router.route("/test/slow", get(handlers::quests::slow))
    } else {
        router
    };

    // Attach embedded static assets router. The `embed_assets!` macro in the
    // `static_assets` module generates a `static_router()` function which
    // returns an `axum::Router` configured to serve the embedded files.
    // We merge it into our application router so static files are served
    // with the same application state type (`AppState`).
    let app = router.merge(static_assets::static_router::<AppState>());

    let addr = SocketAddr::new(bind_addr, port);

    tracing::info!(port, "🚀 Starting HTTP server...");

    // attach middleware to count active requests
    // middleware: no-op when active_requests not present in AppState for test suites
    let app = app;

    // Resolve a tokio TcpListener first (prefer systemd socket activation if
    // available). We then create the make-service from the router and pass
    // it to axum::serve. This avoids moving the router multiple times across
    // branches.
    let listener = {
        #[cfg(target_os = "linux")]
        {
            let fds = systemd::take_listen_fds();
            if let Some(fd) = fds.first() {
                use std::os::unix::io::FromRawFd;
                // Safety: The fd was received from systemd's sd_listen_fds,
                // which guarantees it is a valid, open TCP socket. We transfer
                // ownership to FromRawFd, which consumes the fd and prevents
                // further use of the raw fd.
                unsafe {
                    // Try to construct a std listener from the provided fd and
                    // convert it to a tokio listener. If this fails, fall back
                    // to binding normally.
                    let std_listener = std::net::TcpListener::from_raw_fd(*fd);
                    match tokio::net::TcpListener::from_std(std_listener) {
                        Ok(tokio_listener) => {
                            tracing::info!(fd = %fd, "🔌 Serving on socket-activated fd");
                            tokio_listener
                        }
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                "💥 Failed to use socket-activated fd - falling back to bind"
                            );
                            match tokio::net::TcpListener::bind(addr).await {
                                Ok(l) => l,
                                Err(e) => {
                                    tracing::error!(
                                        error = %e,
                                        port = %port,
                                        "💥 Failed to bind to port - address may be in use"
                                    );
                                    return;
                                }
                            }
                        }
                    }
                }
            } else {
                // No socket activation fds; bind normally
                match tokio::net::TcpListener::bind(addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            port = %port,
                            "💥 Failed to bind to port - address may be in use"
                        );
                        return;
                    }
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            match tokio::net::TcpListener::bind(addr).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!(error = %e, port = %port, "💥 Failed to bind to port - address may be in use");
                    return;
                }
            }
        }
    };

    // Convert the router into a make-service once and hand it to the server.
    // Attach the real application state and turn the router into a make
    // service that `axum::serve` can accept.
    let app = app.with_state(app_state.clone());
    let make_service = app.into_make_service();
    let server = axum::serve(listener, make_service);

    tokio::spawn(async move {
        if let Err(e) = server.await {
            tracing::error!(error = %e, "💥 Server error - HTTP service failed");
        }
    });

    tracing::info!(url = %format!("http://{}", addr), "🎉 Server listening! (◕‿◕)");
    // On Linux, send READY and start watchdog if enabled.
    #[cfg(target_os = "linux")]
    {
        // If systemd handed us sockets, prefer them instead of binding above.
        let fds = systemd::take_listen_fds();
        if !fds.is_empty() {
            // Systemd passed socket activation fds; we detect and log them here.
            // A fuller integration would create a listener from the first fd and
            // avoid the manual bind above. For now we just log the presence of fds.
            if let Some(fd) = fds.first() {
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
                let _unused = handle.await;
            });
        }
    }
    tracing::info!("💡 Open your browser and start questing!");

    // Wait for shutdown signal (ctrl-c or SIGTERM)
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("🛑 SIGINT received, shutting down...");
        }
        _ = async {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                match signal(SignalKind::terminate()) {
                    Ok(mut sigterm) => sigterm.recv().await,
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to install SIGTERM");
                        None
                    }
                }
            }
            #[cfg(not(unix))]
            {
                futures::future::pending::<()>().await
            }
        } => {
            tracing::info!("🛑 SIGTERM received, shutting down...");
        }
    }

    std::process::exit(0);
}
