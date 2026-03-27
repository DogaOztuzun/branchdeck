use axum::extract::State;
use axum::response::Json;

use branchdeck_core::services::update_manager;

use crate::state::AppState;

/// GET /api/updates/status — get current update status.
///
/// Returns the user-visible update status summary including whether an
/// update is available, its version, and whether it's waiting for
/// workflows to complete.
pub async fn get_update_status(
    State(state): State<AppState>,
) -> Json<update_manager::UpdateStatusSummary> {
    let update_state = state.update_state.lock().await;
    Json(update_manager::status_summary(&update_state))
}
