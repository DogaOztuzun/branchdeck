use axum::response::Json;
use log::{error, info};

use branchdeck_core::models::sat::{
    FalsePositiveMetricsResponse, FalsePositiveRequest, FalsePositiveResponse,
};
use branchdeck_core::services::{github, sat_false_positive};

/// POST `/api/sat/false-positive` — label a SAT issue as false positive.
pub async fn label_false_positive(
    Json(req): Json<FalsePositiveRequest>,
) -> Result<Json<FalsePositiveResponse>, crate::error::ApiError> {
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
) -> Result<Json<FalsePositiveMetricsResponse>, crate::error::ApiError> {
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
