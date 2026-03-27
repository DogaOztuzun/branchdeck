use axum::extract::{Path, State};
use axum::response::Json;
use branchdeck_core::models::run::{RunInfo, RunStatus};
use branchdeck_core::services::run_manager;
use serde::Deserialize;
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::state::AppState;

/// Request body for creating a new run.
#[derive(Debug, Deserialize, ToSchema)]
#[allow(dead_code)] // worktree_path used when RunManager is wired into AppState
#[serde(rename_all = "camelCase")]
pub struct CreateRunRequest {
    /// Path to the task file.
    pub task_path: String,
    /// Worktree path to run in.
    #[serde(default)]
    pub worktree_path: Option<String>,
}


/// Minimal run status response.
#[derive(Debug, serde::Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub session_id: Option<String>,
    pub task_path: String,
    pub status: RunStatus,
    pub started_at: String,
    pub cost_usd: f64,
}

impl From<&RunInfo> for RunSummary {
    fn from(info: &RunInfo) -> Self {
        Self {
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
        (status = 201, description = "Run created (stub)", body = RunSummary),
        (status = 400, description = "Invalid request", body = crate::error::ProblemDetails)
    ),
    tag = "runs"
)]
pub async fn create_run(
    State(_state): State<AppState>,
    Json(_req): Json<CreateRunRequest>,
) -> Result<(axum::http::StatusCode, Json<RunSummary>), ApiError> {
    // RunManager requires process orchestration not yet wired (stories 8.1-8.4)
    Err(branchdeck_core::error::AppError::RunError(
        "Not implemented: RunManager not yet wired (requires stories 8.1-8.4)".to_string(),
    )
    .into())
}

#[utoipa::path(
    get,
    path = "/api/runs",
    responses(
        (status = 200, description = "List all runs", body = Vec<RunSummary>)
    ),
    tag = "runs"
)]
pub async fn list_runs(State(_state): State<AppState>) -> Result<Json<Vec<RunSummary>>, ApiError> {
    // RunManager not yet wired (stories 8.1-8.4) — return empty list (not an error, just no runs)
    Ok(Json(Vec::new()))
}

#[utoipa::path(
    get,
    path = "/api/runs/{id}",
    params(
        ("id" = String, Path, description = "Run session ID")
    ),
    responses(
        (status = 200, description = "Run details", body = RunSummary),
        (status = 404, description = "Run not found", body = crate::error::ProblemDetails)
    ),
    tag = "runs"
)]
pub async fn get_run(
    Path(_id): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<RunSummary>, ApiError> {
    // RunManager not yet wired (stories 8.1-8.4)
    Err(branchdeck_core::error::AppError::RunError(
        "Not implemented: RunManager not yet wired (requires stories 8.1-8.4)".to_string(),
    )
    .into())
}

#[utoipa::path(
    post,
    path = "/api/runs/{id}/cancel",
    params(
        ("id" = String, Path, description = "Run session ID")
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
        ("id" = String, Path, description = "Run session ID")
    ),
    responses(
        (status = 200, description = "Run approved"),
        (status = 404, description = "Run not found", body = crate::error::ProblemDetails)
    ),
    tag = "runs"
)]
pub async fn approve_run(
    Path(_id): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // RunManager not yet wired (stories 8.1-8.4)
    Err(branchdeck_core::error::AppError::RunError(
        "Not implemented: RunManager not yet wired (requires stories 8.1-8.4)".to_string(),
    )
    .into())
}
