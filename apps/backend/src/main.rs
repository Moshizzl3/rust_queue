use std::sync::Arc;

use dotenv::dotenv;
use envy::from_env;
use rust_queue::{
    build_router,
    config::Config,
    state::AppState,
    worker::{self, JobRegistry, spawn_reaper, spawn_workers},
};
use tokio_util::sync::CancellationToken;
use tracing::info;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let config: Config = from_env().expect("Failed to load settings from env");

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(!config.running_in_cloud)
        .init();

    let state = AppState::new(
        &config.database_url,
        &config.password_pepper,
        &config.jwt_secret,
        config.jwt_access_expiry_mins.unwrap_or(15),
        config.jwt_refresh_expiry_days.unwrap_or(15),
    )
    .await
    .expect("Failed to connect to database");

    // ── Job registry ───────────────────────────────────────────────────
    let mut registry = JobRegistry::new();
    worker::handlers::register_demo_handlers(&mut registry);
    let registry = Arc::new(registry);

    info!(
        "Registered job handlers: {:?}",
        registry.registered_types()
    );

    // ── Cancellation token ─────────────────────────────────────────────
    let cancel_token = CancellationToken::new();

    // ── Spawn workers ──────────────────────────────────────────────────
    let worker_handles = spawn_workers(
        4,
        state.jobs.clone(),
        registry,
        cancel_token.clone(),
        std::time::Duration::from_secs(1),
    );

    info!("Spawned {} workers", worker_handles.len());

    // ── Spawn stale job reaper ─────────────────────────────────────────
    // Checks every 30 seconds for jobs stuck in 'running' for over 5 minutes.
    // 5 minutes is generous — our slowest demo job takes ~15s. In production,
    // tune this based on your longest-running job type.
    let reaper_handle = spawn_reaper(
        state.jobs.clone(),
        cancel_token.clone(),
        std::time::Duration::from_secs(30),  // check every 30s
        std::time::Duration::from_secs(300), // 5 min = stale
    );

    // ── Start HTTP server ──────────────────────────────────────────────
    let app = build_router(state);
    let bind_address = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&bind_address)
        .await
        .unwrap();

    info!("Listening on {}", listener.local_addr().unwrap());

    // ── Graceful shutdown ──────────────────────────────────────────────
    let cancel_for_signal = cancel_token.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            info!("Shutdown signal received, starting graceful shutdown...");
            cancel_for_signal.cancel();
        })
        .await
        .unwrap();

    // ── Wait for everything to finish ──────────────────────────────────
    info!("Server stopped, waiting for workers to finish current jobs...");
    for handle in worker_handles {
        let _ = handle.await;
    }
    let _ = reaper_handle.await;

    info!("All workers stopped. Goodbye!");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
