use axum::extract::State;
use axum::response::Json;
use branchdeck_core::models::sat::{
    FalsePositiveMetricsResponse, FalsePositiveRequest, FalsePositiveResponse,
};
use branchdeck_core::services::sat_score;
use branchdeck_core::services::{github, sat_false_positive};
use log::{error, info};
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

/// POST `/api/sat/false-positive` — label a SAT issue as false positive.
pub async fn label_false_positive(
    Json(req): Json<FalsePositiveRequest>,
) -> Result<Json<FalsePositiveResponse>, ApiError> {
    if req.repo_path.trim().is_empty() {
        return Err(
            branchdeck_core::error::AppError::Sat("repo_path is required".to_owned()).into(),
        );
    }

    let response = sat_false_positive::label_false_positive_for_project(
        &req.repo_path,
        req.issue_number,
        req.label,
        req.scenario_id.as_deref(),
        req.reason.as_deref(),
    )?;

    // Apply the GitHub label (non-fatal — local record is the primary concern)
    // Use the canonicalized owner/repo from the record, not the raw request path
    let github_label = response.record.label.github_label().to_string();
    let (owner, repo_name) = match response.record.repo.split_once('/') {
        Some((o, r)) => (o.to_string(), r.to_string()),
        None => {
            error!("Malformed repo in FP record: {:?}", response.record.repo);
            return Ok(Json(response));
        }
    };
    if let Err(e) =
        github::add_labels_to_issue(&owner, &repo_name, req.issue_number, &[github_label]).await
    {
        error!(
            "Failed to apply GitHub label to {owner}/{repo_name}#{}: {e} (local record persisted)",
            req.issue_number
        );
    } else {
        info!(
            "Applied GitHub label to {owner}/{repo_name}#{}",
            req.issue_number
        );
    }

    Ok(Json(response))
}

/// GET `/api/sat/false-positive/metrics` — get current FP rate and classification accuracy.
pub async fn get_false_positive_metrics(
    axum::extract::Query(params): axum::extract::Query<MetricsQuery>,
) -> Result<Json<FalsePositiveMetricsResponse>, ApiError> {
    if params.repo_path.trim().is_empty() {
        return Err(
            branchdeck_core::error::AppError::Sat("repo_path is required".to_owned()).into(),
        );
    }

    let response = sat_false_positive::get_metrics_for_project(&params.repo_path)?;
    Ok(Json(response))
}

#[derive(serde::Deserialize)]
pub struct MetricsQuery {
    pub repo_path: String,
}
