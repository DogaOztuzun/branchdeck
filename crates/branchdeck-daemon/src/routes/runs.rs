use axum::extract::{Path, State};
use axum::response::Json;
use branchdeck_core::models::run::RunInfo;
use branchdeck_core::services::run_manager;

use crate::state::AppState;

/// POST `/api/runs/{run_id}/cancel` — immediately cancel a run by ID.
pub async fn cancel_run(
    Path(run_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<RunInfo>, crate::error::ApiError> {
    let cancelled = run_manager::force_cancel_run(state.run_manager, &run_id).await?;
    Ok(Json(cancelled))
}
