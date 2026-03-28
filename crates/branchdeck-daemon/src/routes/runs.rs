use axum::extract::{Path, State};
use axum::response::Json;
use branchdeck_core::models::run::{LaunchOptions, RunInfo, RunStatus};
use branchdeck_core::services::run_manager;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::state::AppState;

/// Request body for creating a new run.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRunRequest {
    /// Path to the task file.
    pub task_path: String,
    /// Worktree path to run in.
    pub worktree_path: String,
    /// Maximum agent turns.
    #[serde(default)]
    pub max_turns: Option<u32>,
    /// Per-run cost budget in USD.
    #[serde(default)]
    pub max_budget_usd: Option<f64>,
}

/// Request body for permission response.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PermissionResponse {
    pub tool_use_id: String,
    pub decision: String,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Minimal run status response.
#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub run_id: String,
    pub session_id: Option<String>,
    pub task_path: String,
    pub status: RunStatus,
    pub started_at: String,
    pub cost_usd: f64,
}

impl From<&RunInfo> for RunSummary {
    fn from(info: &RunInfo) -> Self {
        Self {
            run_id: info.run_id.clone(),
            session_id: info.session_id.clone(),
            task_path: info.task_path.clone(),
            status: info.status,
            started_at: info.started_at.clone(),
            cost_usd: info.cost_usd,
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/runs",
    request_body = CreateRunRequest,
    responses(
        (status = 201, description = "Run created", body = RunSummary),
        (status = 400, description = "Invalid request", body = crate::error::ProblemDetails)
    ),
    tag = "runs"
)]
pub async fn create_run(
    State(state): State<AppState>,
    Json(req): Json<CreateRunRequest>,
) -> Result<(axum::http::StatusCode, Json<RunSummary>), ApiError> {
    let options = LaunchOptions {
        max_turns: req.max_turns,
        max_budget_usd: req.max_budget_usd,
        permission_mode: None,
        allowed_directories: Vec::new(),
    };
    let run = run_manager::launch_run(
        state.run_manager,
        &req.task_path,
        &req.worktree_path,
        options,
    )
    .await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(RunSummary::from(&run)),
    ))
}

#[utoipa::path(
    get,
    path = "/api/runs",
    responses(
        (status = 200, description = "List all runs", body = Vec<RunSummary>)
    ),
    tag = "runs"
)]
pub async fn list_runs(State(state): State<AppState>) -> Result<Json<Vec<RunSummary>>, ApiError> {
    let manager = state.run_manager.lock().await;
    let runs: Vec<RunSummary> = manager
        .get_all_runs()
        .iter()
        .map(RunSummary::from)
        .collect();
    Ok(Json(runs))
}

#[utoipa::path(
    get,
    path = "/api/runs/{id}",
    params(
        ("id" = String, Path, description = "Run ID")
    ),
    responses(
        (status = 200, description = "Run details", body = RunInfo),
        (status = 404, description = "Run not found", body = crate::error::ProblemDetails)
    ),
    tag = "runs"
)]
pub async fn get_run(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<RunInfo>, ApiError> {
    let manager = state.run_manager.lock().await;
    let run = manager.get_run(&id).ok_or_else(|| {
        branchdeck_core::error::AppError::RunError(format!("No run with id {id}"))
    })?;
    Ok(Json(run))
}

#[utoipa::path(
    post,
    path = "/api/runs/{id}/cancel",
    params(
        ("id" = String, Path, description = "Run ID")
    ),
    responses(
        (status = 200, description = "Run cancelled", body = RunInfo),
        (status = 404, description = "Run not found", body = crate::error::ProblemDetails)
    ),
    tag = "runs"
)]
pub async fn cancel_run(
    Path(run_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<RunInfo>, ApiError> {
    let cancelled = run_manager::force_cancel_run(state.run_manager, &run_id).await?;
    Ok(Json(cancelled))
}

#[utoipa::path(
    post,
    path = "/api/runs/{id}/approve",
    params(
        ("id" = String, Path, description = "Run ID")
    ),
    request_body = PermissionResponse,
    responses(
        (status = 200, description = "Permission responded"),
        (status = 404, description = "Run not found", body = crate::error::ProblemDetails)
    ),
    tag = "runs"
)]
pub async fn approve_run(
    Path(run_id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<PermissionResponse>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut manager = state.run_manager.lock().await;
    manager
        .respond_to_permission(
            &run_id,
            &req.tool_use_id,
            &req.decision,
            req.reason.as_deref(),
        )
        .await?;
    Ok(Json(serde_json::json!({"ok": true})))
}
