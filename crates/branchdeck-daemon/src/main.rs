// Allow `needless_for_each` from utoipa's OpenApi derive macro
#![allow(clippy::needless_for_each)]

use axum::routing::{get, post};
use axum::Router;
use branchdeck_core::services::activity_store::ActivityStore;
use branchdeck_core::services::event_bus::EventBus;
use branchdeck_core::services::run_manager;
use branchdeck_core::traits::EventEmitter;
use branchdeck_core::util::write_atomic;
use clap::Parser;
use log::{error, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

mod auth;
mod cli;
mod cli_client;
mod emitter;
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
        routes::mcp::mcp_handler,
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
        (name = "sat", description = "SAT satisfaction scores"),
        (name = "mcp", description = "MCP-over-HTTP endpoint (JSON-RPC 2.0)")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {
            port,
            bind,
            workspace,
            static_dir,
            require_auth,
        } => run_serve(port, &bind, workspace, static_dir, require_auth).await,
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
        Commands::Token { action } => match action {
            cli::TokenAction::Generate => {
                let token = auth::generate_token();
                match auth::save_token(&token) {
                    Ok(()) => {
                        println!("{token}");
                    }
                    Err(e) => {
                        error!("Failed to save token: {e}");
                        std::process::exit(1);
                    }
                }
            }
        },
        Commands::Update { port, json } => {
            let client = cli_client::DaemonClient::new(port, json);
            std::process::exit(client.update());
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn run_serve(port: u16, bind: &str, workspace_arg: Option<PathBuf>, static_dir: Option<PathBuf>, require_auth_flag: bool) {
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

    // Auto-enable auth when binding to non-localhost address
    let is_loopback = bind == "127.0.0.1" || bind == "::1" || bind == "localhost";
    let require_auth = require_auth_flag || !is_loopback;

    let auth_token = if require_auth {
        match auth::load_token() {
            Ok(Some(token)) => {
                info!("Authentication enabled — token loaded");
                Some(token)
            }
            Ok(None) => {
                error!("Authentication required but no token found. Run `branchdeck-daemon token generate` first.");
                std::process::exit(1);
            }
            Err(e) => {
                error!("Failed to load auth token: {e}");
                std::process::exit(1);
            }
        }
    } else {
        info!("Authentication disabled (localhost-only)");
        None
    };

    let search_dirs = branchdeck_core::services::workflow::default_search_dirs(
        &workspace_root.display().to_string(),
    );
    let workflow_registry = Arc::new(
        branchdeck_core::services::workflow::WorkflowRegistry::scan(&search_dirs),
    );
    info!("Loaded {} workflow(s)", workflow_registry.list_workflows().len());

    // RunManager + DaemonEmitter from main
    let sidecar_path = data_dir.join("sidecar").join("index.js");
    let daemon_emitter: Arc<dyn EventEmitter> = Arc::new(emitter::DaemonEmitter);
    let run_manager_state = run_manager::create_run_manager_state(
        sidecar_path,
        Arc::clone(&event_bus),
        daemon_emitter,
        0, // hook_port — configured at runtime via CLI args
    );

    let update_state = Arc::new(tokio::sync::Mutex::new(
        branchdeck_core::services::update_manager::UpdateState::default(),
    ));

    let app_state = AppState {
        event_bus,
        activity_store,
        workflow_registry,
        workspace_root: workspace_root.clone(),
        require_auth,
        auth_token,
        run_manager: run_manager_state,
        update_state,
    };

    let schema_header_value =
        http::HeaderValue::from_str(SCHEMA_VERSION).unwrap_or_else(|_| {
            http::HeaderValue::from_static("unknown")
        });

    // Auth-protected routes with timeout + body limit
    let protected_api = Router::new()
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
        .route(
            "/api/setup/status",
            get(routes::setup::get_setup_status),
        )
        .route(
            "/api/setup/validate",
            get(routes::setup::validate_tokens),
        )
        .route(
            "/api/setup/workflows",
            get(routes::setup::list_workflows),
        )
        .route(
            "/api/setup/save",
            post(routes::setup::save_config),
        )
        .route(
            "/api/sat/false-positive",
            post(routes::sat::label_false_positive),
        )
        .route(
            "/api/sat/false-positive/metrics",
            get(routes::sat::get_false_positive_metrics),
        )
        .route(
            "/api/updates/status",
            get(routes::updates::get_update_status),
        )
        .route("/mcp", post(routes::mcp::mcp_handler))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .layer(TimeoutLayer::with_status_code(
            http::StatusCode::REQUEST_TIMEOUT,
            std::time::Duration::from_secs(30),
        ));

    // SSE is auth-protected but exempt from timeout (long-lived stream)
    let protected = Router::new()
        .route("/api/events", get(routes::events::sse_handler))
        .merge(protected_api)
        .layer(axum::middleware::from_fn_with_state(
            app_state.clone(),
            auth::auth_middleware,
        ));

    // Swagger UI is auth-protected (leaks full API surface)
    let protected_with_docs = protected.merge(
        SwaggerUi::new("/api/docs")
            .url("/api/openapi.json", ApiDoc::openapi()),
    );

    // Health endpoint is exempt from auth so monitoring tools and desktop startup can probe
    let mut app = Router::new()
        .route("/api/health", get(routes::health::health))
        .merge(protected_with_docs)
        .layer(SetResponseHeaderLayer::overriding(
            http::HeaderName::from_static("x-branchdeck-schema"),
            schema_header_value,
        ))
        .with_state(app_state);

    // Serve static frontend files if the directory exists (Docker/web mode)
    let resolved_static_dir = static_dir
        .or_else(|| {
            let candidate = std::env::current_exe()
                .ok()?
                .parent()?
                .join("dist");
            if candidate.is_dir() { Some(candidate) } else { None }
        });

    if let Some(dir) = resolved_static_dir {
        if dir.is_dir() {
            let index_html = dir.join("index.html");
            let serve_dir = ServeDir::new(&dir)
                .not_found_service(ServeFile::new(&index_html));
            app = app.fallback_service(serve_dir);
            info!("Serving static frontend from {}", dir.display());
        } else {
            warn!("Static dir {} does not exist, skipping frontend serving", dir.display());
        }
    }

    let bind_addr = format!("{bind}:{port}");
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
