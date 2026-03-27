use axum::extract::State;
use axum::response::Json;
use branchdeck_core::models::sat::SatScoreResult;
use log::debug;
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
    let sat_runs_dir = state.workspace_root.join("sat").join("runs");
    debug!("Looking for SAT runs in {}", sat_runs_dir.display());

    let latest_run = find_latest_run_dir(&sat_runs_dir);

    let summary = match latest_run {
        Some(run_dir) => {
            let scores_path = run_dir.join("scores.json");
            match std::fs::read_to_string(&scores_path) {
                Ok(content) => {
                    match serde_json::from_str::<SatScoreResult>(&content) {
                        Ok(result) => SatScoreSummary {
                            aggregate_score: Some(result.aggregate_score),
                            scenario_count: result.scenario_scores.len(),
                            finding_count: result.all_findings.len(),
                            run_id: Some(result.run_id),
                        },
                        Err(_) => no_scores(),
                    }
                }
                Err(_) => no_scores(),
            }
        }
        None => no_scores(),
    };

    Ok(Json(summary))
}

fn no_scores() -> SatScoreSummary {
    SatScoreSummary {
        aggregate_score: None,
        scenario_count: 0,
        finding_count: 0,
        run_id: None,
    }
}

/// Find the latest run directory by lexicographic sort (timestamps sort naturally).
fn find_latest_run_dir(runs_dir: &std::path::Path) -> Option<std::path::PathBuf> {
    let entries = std::fs::read_dir(runs_dir).ok()?;
    let mut dirs: Vec<std::path::PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs.pop()
}
