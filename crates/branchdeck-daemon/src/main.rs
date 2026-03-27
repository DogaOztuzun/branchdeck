use axum::routing::get;
use axum::Router;
use branchdeck_core::services::activity_store::ActivityStore;
use branchdeck_core::services::event_bus::EventBus;
use branchdeck_core::util::write_atomic;
use clap::Parser;
use log::{error, info, warn};
use std::path::PathBuf;
use std::sync::Arc;

mod cli;
mod routes;
mod state;

use cli::{Cli, Commands};
use state::AppState;

#[tokio::main]
async fn main() {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, workspace } => run_serve(port, workspace).await,
    }
}

async fn run_serve(port: u16, workspace_arg: Option<PathBuf>) {
    let workspace_root = workspace_arg.unwrap_or_else(|| {
        std::env::current_dir().unwrap_or_else(|e| {
            error!("Failed to determine current directory: {e}");
            std::process::exit(1);
        })
    });

    info!("Workspace root: {}", workspace_root.display());

    let event_bus = Arc::new(EventBus::new());

    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("branchdeck");

    let activity_store = match ActivityStore::new_with_persistence(&data_dir) {
        Ok(store) => {
            info!("Activity store initialized with file-backed persistence");
            Arc::new(store)
        }
        Err(e) => {
            warn!("Failed to initialize persistent activity store, falling back to in-memory: {e}");
            Arc::new(ActivityStore::new())
        }
    };

    activity_store.start_subscriber(&event_bus);
    activity_store.start_gc(300_000); // 5 minute TTL

    let app_state = AppState {
        event_bus,
        activity_store,
        workspace_root: workspace_root.clone(),
    };

    let app = Router::new()
        .route("/api/health", get(routes::health::health))
        .route("/api/events", get(routes::events::sse_handler))
        .route(
            "/api/runs/{session_id}/activity",
            get(routes::activity::get_session_activity),
        )
        .route(
            "/api/agents/active",
            get(routes::activity::get_active_agents),
        )
        .with_state(app_state);

    let bind_addr = format!("127.0.0.1:{port}");
    info!("branchdeck-daemon starting on {bind_addr}");

    let listener = match tokio::net::TcpListener::bind(&bind_addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to {bind_addr}: {e}");
            std::process::exit(1);
        }
    };

    // Write daemon.json so other processes can discover this daemon
    write_daemon_json(&workspace_root, port);

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {e}");
        std::process::exit(1);
    }
}

/// Write daemon discovery info to `{workspace}/.branchdeck/daemon.json`.
fn write_daemon_json(workspace_root: &std::path::Path, port: u16) {
    let daemon_info = serde_json::json!({
        "pid": std::process::id(),
        "port": port,
        "version": env!("CARGO_PKG_VERSION"),
    });

    let path = workspace_root.join(".branchdeck").join("daemon.json");

    match write_atomic(&path, daemon_info.to_string().as_bytes()) {
        Ok(()) => info!("Wrote daemon.json to {}", path.display()),
        Err(e) => warn!("Failed to write daemon.json: {e}"),
    }
}
