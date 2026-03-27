// Allow `needless_for_each` from utoipa's OpenApi derive macro
#![allow(clippy::needless_for_each)]

use axum::routing::{get, post};
use axum::Router;
use branchdeck_core::services::activity_store::ActivityStore;
use branchdeck_core::services::event_bus::EventBus;
use branchdeck_core::util::write_atomic;
use clap::Parser;
use log::{error, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::set_header::SetResponseHeaderLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod cli;
mod cli_client;
mod error;
mod routes;
mod state;

use cli::{Cli, Commands};
use state::AppState;

/// Schema version for the X-Branchdeck-Schema header.
const SCHEMA_VERSION: &str = "0.2.0";

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Branchdeck Daemon API",
        version = "0.2.0",
        description = "REST API for the Branchdeck autonomous development daemon",
        license(name = "MIT")
    ),
    paths(
        routes::health::health,
        routes::events::sse_handler,
        routes::workflows::list_workflows,
        routes::workflows::get_workflow,
        routes::runs::create_run,
        routes::runs::list_runs,
        routes::runs::get_run,
        routes::runs::cancel_run,
        routes::runs::approve_run,
        routes::repos::get_repo,
        routes::sat::get_sat_scores,
    ),
    components(schemas(
        routes::events::SseEnvelope,
        routes::health::HealthResponse,
        routes::workflows::WorkflowSummary,
        routes::workflows::WorkflowDetail,
        routes::runs::CreateRunRequest,
        routes::runs::RunSummary,
        routes::repos::RepoDetail,
        routes::sat::SatScoreSummary,
        error::ProblemDetails,
        branchdeck_core::models::workflow::WorkflowConfig,
        branchdeck_core::models::workflow::TrackerDef,
        branchdeck_core::models::workflow::TrackerKind,
        branchdeck_core::models::workflow::PollingDef,
        branchdeck_core::models::workflow::WorkspaceDef,
        branchdeck_core::models::workflow::HooksDef,
        branchdeck_core::models::workflow::AgentDef,
        branchdeck_core::models::workflow::OutcomeDef,
        branchdeck_core::models::workflow::OutcomeDetector,
        branchdeck_core::models::workflow::OutcomeAction,
        branchdeck_core::models::workflow::LifecycleDef,
        branchdeck_core::models::workflow::RetryDef,
        branchdeck_core::models::workflow::BackoffStrategy,
        branchdeck_core::models::run::RunInfo,
        branchdeck_core::models::run::RunStatus,
        branchdeck_core::models::RepoInfo,
        branchdeck_core::models::WorktreeInfo,
    )),
    tags(
        (name = "events", description = "Server-Sent Events stream"),
        (name = "health", description = "Health check endpoints"),
        (name = "workflows", description = "Workflow management"),
        (name = "runs", description = "Run management"),
        (name = "repos", description = "Repository information"),
        (name = "sat", description = "SAT satisfaction scores")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve { port, workspace } => run_serve(port, workspace).await,
        Commands::Status { port, json } => {
            let client = cli_client::DaemonClient::new(port, json);
            std::process::exit(client.status().await);
        }
        Commands::Trigger {
            workflow,
            port,
            task_path,
            worktree_path,
            json,
        } => {
            let client = cli_client::DaemonClient::new(port, json);
            std::process::exit(
                client
                    .trigger(&workflow, task_path.as_deref(), worktree_path.as_deref())
                    .await,
            );
        }
        Commands::Runs { action, port, json } => {
            let client = cli_client::DaemonClient::new(port, json);
            let code = match action {
                Some(cli::RunsAction::Cancel { id }) => client.cancel_run(&id).await,
                None => client.list_runs().await,
            };
            std::process::exit(code);
        }
        Commands::Update { port, json } => {
            let client = cli_client::DaemonClient::new(port, json);
            std::process::exit(client.update());
        }
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

    let schema_header_value =
        http::HeaderValue::from_str(SCHEMA_VERSION).unwrap_or_else(|_| {
            http::HeaderValue::from_static("unknown")
        });

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
        .route("/api/workflows", get(routes::workflows::list_workflows))
        .route(
            "/api/workflows/{name}",
            get(routes::workflows::get_workflow),
        )
        .route("/api/runs", post(routes::runs::create_run).get(routes::runs::list_runs))
        .route("/api/runs/{id}", get(routes::runs::get_run))
        .route(
            "/api/runs/{id}/cancel",
            post(routes::runs::cancel_run),
        )
        .route(
            "/api/runs/{id}/approve",
            post(routes::runs::approve_run),
        )
        .route("/api/repos", get(routes::repos::get_repo))
        .route("/api/sat/scores", get(routes::sat::get_sat_scores))
        .merge(
            SwaggerUi::new("/api/docs/{_:.*}")
                .url("/api/openapi.json", ApiDoc::openapi()),
        )
        .layer(SetResponseHeaderLayer::overriding(
            http::HeaderName::from_static("x-branchdeck-schema"),
            schema_header_value,
        ))
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
