use axum::extract::{Path, State};
use axum::response::Json;
use branchdeck_core::models::run::{RunInfo, RunStatus};
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
    Json(req): Json<CreateRunRequest>,
) -> Result<(axum::http::StatusCode, Json<RunSummary>), ApiError> {
    // Stub: RunManager requires process orchestration (Arc<Mutex>, child processes)
    // which is not available in the stateless daemon yet. Return a placeholder.
    let summary = RunSummary {
        session_id: None,
        task_path: req.task_path,
        status: RunStatus::Created,
        started_at: chrono::Utc::now().to_rfc3339(),
        cost_usd: 0.0,
    };
    Ok((axum::http::StatusCode::CREATED, Json(summary)))
}

#[utoipa::path(
    get,
    path = "/api/runs",
    responses(
        (status = 200, description = "List all runs", body = Vec<RunSummary>)
    ),
    tag = "runs"
)]
pub async fn list_runs(State(_state): State<AppState>) -> Json<Vec<RunSummary>> {
    // Stub: RunManager is not yet wired into AppState
    Json(Vec::new())
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
    Path(id): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<RunSummary>, ApiError> {
    // Stub: RunManager is not yet wired into AppState
    Err(branchdeck_core::error::AppError::RunError(format!("run not found: {id}")).into())
}

#[utoipa::path(
    post,
    path = "/api/runs/{id}/cancel",
    params(
        ("id" = String, Path, description = "Run session ID")
    ),
    responses(
        (status = 200, description = "Run cancelled"),
        (status = 404, description = "Run not found", body = crate::error::ProblemDetails)
    ),
    tag = "runs"
)]
pub async fn cancel_run(
    Path(id): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Stub: RunManager is not yet wired into AppState
    Err(branchdeck_core::error::AppError::RunError(format!("run not found: {id}")).into())
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
    Path(id): Path<String>,
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Stub: RunManager is not yet wired into AppState
    Err(branchdeck_core::error::AppError::RunError(format!("run not found: {id}")).into())
}
