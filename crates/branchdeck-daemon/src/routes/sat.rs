use axum::response::Json;

use branchdeck_core::models::sat::{
    FalsePositiveRequest, FalsePositiveResponse,
};
use branchdeck_core::services::github;
use branchdeck_core::services::sat_false_positive;
use branchdeck_core::services::sat_score;

/// POST /api/sat/false-positive — label a SAT issue as false positive.
///
/// Records the FP, persists it for metric computation, and returns
/// updated classification accuracy and FP rate metrics.
pub async fn label_false_positive(
    Json(req): Json<FalsePositiveRequest>,
) -> Result<Json<FalsePositiveResponse>, crate::error::ApiError> {
    let (owner, repo_name) =
        github::resolve_owner_repo(std::path::Path::new(&req.repo_path))?;
    let repo_full = format!("{owner}/{repo_name}");

    // Build paths for persistence
    let project_root = std::path::Path::new(&req.repo_path);
    let fp_data_path = project_root
        .join(".branchdeck")
        .join("sat-false-positives.json");
    let learnings_path = project_root.join("sat").join("learnings.yaml");

    let record = sat_false_positive::build_false_positive_record(
        req.issue_number,
        &repo_full,
        req.label,
        req.scenario_id.as_deref(),
        req.reason.as_deref(),
        &chrono::Utc::now().to_rfc3339(),
    );

    let (_fp_data, false_positive_metrics, classification_accuracy) =
        sat_false_positive::record_false_positive(
            &fp_data_path,
            &learnings_path,
            &record,
        )?;

    // Load learnings for the response accuracy
    let _learnings = sat_score::load_learnings(&learnings_path)?;

    Ok(Json(FalsePositiveResponse {
        record,
        classification_accuracy,
        false_positive_metrics,
    }))
}

/// GET /api/sat/false-positive/metrics — get current FP rate and classification accuracy.
pub async fn get_false_positive_metrics(
    axum::extract::Query(params): axum::extract::Query<MetricsQuery>,
) -> Result<Json<FalsePositiveResponse>, crate::error::ApiError> {
    let project_root = std::path::Path::new(&params.repo_path);
    let fp_data_path = project_root
        .join(".branchdeck")
        .join("sat-false-positives.json");
    let learnings_path = project_root.join("sat").join("learnings.yaml");

    let fp_data = sat_false_positive::load_false_positive_data(&fp_data_path)?;
    let learnings = sat_score::load_learnings(&learnings_path)?;
    let metrics = sat_false_positive::compute_false_positive_metrics(&fp_data, &learnings);
    let accuracy = sat_false_positive::compute_accuracy_with_fp_data(&learnings, &fp_data);

    // Return the latest record if available, or a placeholder
    let record = fp_data
        .records
        .last()
        .cloned()
        .unwrap_or_else(|| branchdeck_core::models::sat::FalsePositiveRecord {
            recorded_at: String::new(),
            issue_number: 0,
            repo: String::new(),
            label: branchdeck_core::models::sat::FalsePositiveLabel::Runner,
            scenario_id: None,
            reason: None,
        });

    Ok(Json(FalsePositiveResponse {
        record,
        classification_accuracy: accuracy,
        false_positive_metrics: metrics,
    }))
}

#[derive(serde::Deserialize)]
pub struct MetricsQuery {
    pub repo_path: String,
}
