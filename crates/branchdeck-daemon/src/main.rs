use axum::routing::{get, post};
use axum::Router;
use branchdeck_core::services::activity_store::ActivityStore;
use branchdeck_core::services::event_bus::EventBus;
use branchdeck_core::services::run_manager;
use branchdeck_core::traits::EventEmitter;
use log::{error, info, warn};
use std::sync::Arc;

mod emitter;
mod error;
mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() {
    env_logger::init();

    let event_bus = Arc::new(EventBus::new());

    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
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

    // Sidecar path — resolve from data dir; placeholder until CLI config is added
    let sidecar_path = data_dir.join("sidecar").join("index.js");
    let daemon_emitter: Arc<dyn EventEmitter> = Arc::new(emitter::DaemonEmitter);
    let run_manager_state = run_manager::create_run_manager_state(
        sidecar_path,
        Arc::clone(&event_bus),
        daemon_emitter,
        0, // hook_port — configured at runtime via CLI args
    );

    let app_state = AppState {
        event_bus,
        activity_store,
        run_manager: run_manager_state,
    };

    let app = Router::new()
        .route("/api/events", get(routes::events::sse_handler))
        .route(
            "/api/runs/{session_id}/activity",
            get(routes::activity::get_session_activity),
        )
        .route(
            "/api/agents/active",
            get(routes::activity::get_active_agents),
        )
        .route("/api/runs/cancel", post(routes::runs::cancel_run))
        .with_state(app_state);

    info!("branchdeck-daemon starting on 127.0.0.1:13371");

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:13371").await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind to 127.0.0.1:13371: {e}");
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        error!("Server error: {e}");
        std::process::exit(1);
    }
}
