use axum::extract::State;
use axum::response::Json;
use branchdeck_core::models::run::RunInfo;
use branchdeck_core::services::run_manager;

use crate::state::AppState;

/// POST /api/runs/cancel — immediately cancel the active run.
pub async fn cancel_run(
    State(state): State<AppState>,
) -> Result<Json<RunInfo>, crate::error::ApiError> {
    let cancelled = run_manager::force_cancel_run(state.run_manager).await?;
    Ok(Json(cancelled))
}
