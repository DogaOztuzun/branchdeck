use axum::extract::State;
use axum::response::Json;
use branchdeck_core::services::sat_score;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::ApiError;
use crate::state::AppState;

/// Summary of the latest SAT score.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SatScoreSummary {
    /// Aggregate satisfaction score (0-100), if a run exists.
    pub aggregate_score: Option<u32>,
    /// Number of scored scenarios.
    pub scenario_count: usize,
    /// Number of findings (app bugs).
    pub finding_count: usize,
    /// Run ID of the latest scored run.
    pub run_id: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/sat/scores",
    responses(
        (status = 200, description = "Latest SAT score summary", body = SatScoreSummary),
        (status = 500, description = "Error reading scores", body = crate::error::ProblemDetails)
    ),
    tag = "sat"
)]
pub async fn get_sat_scores(
    State(state): State<AppState>,
) -> Result<Json<SatScoreSummary>, ApiError> {
    let summary = match sat_score::load_latest_scores(&state.workspace_root) {
        Some(scores) => SatScoreSummary {
            aggregate_score: Some(scores.aggregate_score),
            scenario_count: scores.scenario_count,
            finding_count: scores.finding_count,
            run_id: Some(scores.run_id),
        },
        None => SatScoreSummary {
            aggregate_score: None,
            scenario_count: 0,
            finding_count: 0,
            run_id: None,
        },
    };

    Ok(Json(summary))
}
