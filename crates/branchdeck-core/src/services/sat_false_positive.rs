//! SAT false positive labeling service (Story 6.3).
//!
//! Records false positive labels on GitHub issues created by SAT,
//! persists FP data for metric computation, and computes FP rate
//! and classification accuracy updates.
//!
//! Architecture:
//! - Pure functions: `build_false_positive_record`, `compute_false_positive_metrics`,
//!   `update_accuracy_with_false_positive`
//! - I/O functions: `load_false_positive_data`, `save_false_positive_data`,
//!   `record_false_positive`

use log::{debug, error, info};
use std::path::Path;

use crate::error::AppError;
use crate::models::sat::{
    ClassificationAccuracy, FalsePositiveData, FalsePositiveLabel, FalsePositiveMetrics,
    FalsePositiveRecord, FalsePositiveResponse, SatLearningsFile,
};

// ---------------------------------------------------------------------------
// Path validation
// ---------------------------------------------------------------------------

/// Validate and canonicalize a user-provided repo path.
///
/// Ensures the path exists, is a directory, and is a valid git repository.
/// Returns the canonicalized path.
///
/// # Errors
/// Returns `AppError::Sat` if the path is invalid or not a git repository.
pub fn validate_repo_path(raw_path: &str) -> Result<std::path::PathBuf, AppError> {
    let path = std::path::Path::new(raw_path);
    let canonical = std::fs::canonicalize(path).map_err(|e| {
        error!("Invalid repo path {raw_path:?}: {e}");
        AppError::Sat(format!("invalid project path: {e}"))
    })?;
    // Verify it's a git repository
    git2::Repository::open(&canonical).map_err(|e| {
        error!("Not a git repository at {}: {e}", canonical.display());
        AppError::Sat(format!("not a git repository: {e}"))
    })?;
    Ok(canonical)
}

// ---------------------------------------------------------------------------
// Record building (pure)
// ---------------------------------------------------------------------------

/// Build a `FalsePositiveRecord` from the input parameters.
#[must_use]
pub fn build_false_positive_record(
    issue_number: u64,
    repo: &str,
    label: FalsePositiveLabel,
    scenario_id: Option<&str>,
    reason: Option<&str>,
    recorded_at: &str,
) -> FalsePositiveRecord {
    FalsePositiveRecord {
        recorded_at: recorded_at.to_string(),
        issue_number,
        repo: repo.to_string(),
        label,
        scenario_id: scenario_id.map(String::from),
        reason: reason.map(String::from),
    }
}

// ---------------------------------------------------------------------------
// Metrics computation (pure)
// ---------------------------------------------------------------------------

/// Compute false positive metrics from accumulated data and cycle learnings.
///
/// Counts FPs from both the explicit FP records file and from cycle learnings.
/// Total issues created comes from cycle learnings `issues_found` sum.
///
/// FP rate = `total_false_positives / total_issues_created`
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn compute_false_positive_metrics(
    fp_data: &FalsePositiveData,
    learnings: &SatLearningsFile,
) -> FalsePositiveMetrics {
    // Count FPs from the explicit records
    let runner_count = fp_data
        .records
        .iter()
        .filter(|r| r.label == FalsePositiveLabel::Runner)
        .count();
    let scenario_count = fp_data
        .records
        .iter()
        .filter(|r| r.label == FalsePositiveLabel::Scenario)
        .count();

    // Also include FPs from cycle learnings (these are auto-detected during verification)
    let cycle_fps: usize = learnings
        .cycle_learnings
        .iter()
        .map(|c| c.false_positives)
        .sum();

    let total_false_positives = fp_data.records.len() + cycle_fps;

    // Total issues created from cycle learnings. When cycles exist, FP records
    // refer to issues already counted in cycle.issues_found — don't double-count.
    // Only use FP records as the denominator when no cycles exist yet.
    let cycle_issues: usize = learnings
        .cycle_learnings
        .iter()
        .map(|c| c.issues_found)
        .sum();
    let total_issues_created = if cycle_issues > 0 {
        cycle_issues
    } else {
        fp_data.records.len()
    };

    let false_positive_rate = if total_issues_created > 0 {
        Some(total_false_positives as f64 / total_issues_created as f64)
    } else {
        None
    };

    FalsePositiveMetrics {
        total_issues_created,
        total_false_positives,
        false_positive_rate,
        runner_count,
        scenario_count,
    }
}

/// Update classification accuracy to include a newly recorded false positive.
///
/// Takes the existing learnings file and the FP data (including the new record),
/// and recomputes accuracy. The FP records count as additional false positives
/// on top of those already tracked in cycle learnings.
#[must_use]
pub fn compute_accuracy_with_fp_data(
    learnings: &SatLearningsFile,
    fp_data: &FalsePositiveData,
) -> ClassificationAccuracy {
    // Start from cycle learning totals
    let mut true_positives: usize = 0;
    let mut false_positives: usize = 0;
    let mut total_classifications: usize = 0;

    for cycle in &learnings.cycle_learnings {
        total_classifications += cycle.issues_found;
        true_positives += cycle.issues_fixed;
        false_positives += cycle.false_positives;
    }

    // Add explicit FP records (user-labeled FPs). When cycles exist, these
    // issues are already counted in cycle.issues_found — only add to
    // total_classifications when no cycles exist yet.
    false_positives += fp_data.records.len();
    if learnings.cycle_learnings.is_empty() {
        total_classifications += fp_data.records.len();
    }

    let denominator = true_positives + false_positives;
    #[allow(clippy::cast_precision_loss)]
    let accuracy = if denominator > 0 {
        Some(true_positives as f64 / denominator as f64)
    } else {
        None
    };

    ClassificationAccuracy {
        total_classifications,
        true_positives,
        false_positives,
        accuracy,
        cycles_counted: learnings.cycle_learnings.len(),
    }
}

// ---------------------------------------------------------------------------
// Persistence (I/O)
// ---------------------------------------------------------------------------

/// Load false positive data from the persisted file.
///
/// Returns an empty `FalsePositiveData` if the file does not exist.
///
/// # Errors
/// Returns `AppError::Sat` if the file exists but cannot be read or parsed.
pub fn load_false_positive_data(path: &Path) -> Result<FalsePositiveData, AppError> {
    if !path.exists() {
        return Ok(FalsePositiveData::default());
    }

    let content = std::fs::read_to_string(path).map_err(|e| {
        error!(
            "Failed to read false positive data from {}: {e}",
            path.display()
        );
        AppError::Sat(format!("failed to read false positive data: {e}"))
    })?;

    serde_json::from_str(&content).map_err(|e| {
        error!("Failed to parse false positive data JSON: {e}");
        AppError::Sat(format!("false positive data parse error: {e}"))
    })
}

/// Save false positive data atomically.
///
/// # Errors
/// Returns `AppError::Sat` if serialization or write fails.
pub fn save_false_positive_data(
    path: &Path,
    data: &FalsePositiveData,
) -> Result<(), AppError> {
    let json = serde_json::to_string_pretty(data).map_err(|e| {
        error!("Failed to serialize false positive data: {e}");
        AppError::Sat(format!("false positive data serialization error: {e}"))
    })?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            error!(
                "Failed to create directory {}: {e}",
                parent.display()
            );
            AppError::Sat(format!("failed to create directory: {e}"))
        })?;
    }

    crate::util::write_atomic(path, json.as_bytes()).map_err(|e| {
        error!(
            "Failed to write false positive data to {}: {e}",
            path.display()
        );
        AppError::Sat(format!("failed to write false positive data: {e}"))
    })?;

    info!(
        "Saved false positive data ({} records) to {}",
        data.records.len(),
        path.display()
    );
    Ok(())
}

/// Record a false positive: append to the data file and return updated metrics.
///
/// This is the main I/O entry point for the false positive labeling flow.
///
/// # Errors
/// Returns `AppError::Sat` on persistence failures.
pub fn record_false_positive(
    fp_data_path: &Path,
    learnings_path: &Path,
    record: &FalsePositiveRecord,
) -> Result<(FalsePositiveData, FalsePositiveMetrics, ClassificationAccuracy), AppError> {
    let mut fp_data = load_false_positive_data(fp_data_path)?;

    // Dedup: don't re-append if this issue+repo is already recorded
    let already_recorded = fp_data
        .records
        .iter()
        .any(|r| r.issue_number == record.issue_number && r.repo == record.repo);

    if already_recorded {
        debug!(
            "Skipping duplicate false positive for issue #{} in {}",
            record.issue_number, record.repo
        );
    } else {
        fp_data.records.push(record.clone());
        save_false_positive_data(fp_data_path, &fp_data)?;
        info!(
            "Recorded false positive for issue #{} in {} (total: {})",
            record.issue_number, record.repo, fp_data.records.len()
        );
    }

    let learnings = crate::services::sat_score::load_learnings(learnings_path)?;
    let metrics = compute_false_positive_metrics(&fp_data, &learnings);
    let accuracy = compute_accuracy_with_fp_data(&learnings, &fp_data);

    Ok((fp_data, metrics, accuracy))
}

// ---------------------------------------------------------------------------
// High-level orchestration (I/O)
// ---------------------------------------------------------------------------

/// Label a false positive for a project: validate path, record locally,
/// return updated metrics.
///
/// GitHub label application is handled separately by the caller (async).
///
/// # Errors
/// Returns `AppError::Sat` on validation or persistence failures.
pub fn label_false_positive_for_project(
    raw_repo_path: &str,
    issue_number: u64,
    label: FalsePositiveLabel,
    scenario_id: Option<&str>,
    reason: Option<&str>,
) -> Result<FalsePositiveResponse, AppError> {
    let project_root = validate_repo_path(raw_repo_path)?;

    let (owner, repo_name) = crate::services::github::resolve_owner_repo(&project_root)?;
    let repo_full = format!("{owner}/{repo_name}");

    let fp_data_path = project_root
        .join(".branchdeck")
        .join("sat-false-positives.json");
    let learnings_path = project_root.join("sat").join("learnings.yaml");

    let record = build_false_positive_record(
        issue_number,
        &repo_full,
        label,
        scenario_id,
        reason,
        &chrono::Utc::now().to_rfc3339(),
    );

    let (_fp_data, false_positive_metrics, classification_accuracy) =
        record_false_positive(&fp_data_path, &learnings_path, &record)?;

    Ok(FalsePositiveResponse {
        record,
        classification_accuracy,
        false_positive_metrics,
    })
}

/// Retrieve false positive metrics for a project.
///
/// Validates the repo path, loads data, and computes metrics.
///
/// # Errors
/// Returns `AppError::Sat` on validation or load failures.
pub fn get_metrics_for_project(
    raw_repo_path: &str,
) -> Result<crate::models::sat::FalsePositiveMetricsResponse, AppError> {
    let project_root = validate_repo_path(raw_repo_path)?;

    let fp_data_path = project_root
        .join(".branchdeck")
        .join("sat-false-positives.json");
    let learnings_path = project_root.join("sat").join("learnings.yaml");

    let fp_data = load_false_positive_data(&fp_data_path)?;
    let learnings = crate::services::sat_score::load_learnings(&learnings_path)?;
    let metrics = compute_false_positive_metrics(&fp_data, &learnings);
    let accuracy = compute_accuracy_with_fp_data(&learnings, &fp_data);

    Ok(crate::models::sat::FalsePositiveMetricsResponse {
        classification_accuracy: accuracy,
        false_positive_metrics: metrics,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::sat::{SatCycleLearning, SatLearningsFile, VerificationOutcome};

    fn make_record(
        issue_number: u64,
        label: FalsePositiveLabel,
    ) -> FalsePositiveRecord {
        build_false_positive_record(
            issue_number,
            "owner/repo",
            label,
            Some("scenario-01"),
            Some("not a real bug"),
            "2026-03-27T12:00:00Z",
        )
    }

    fn make_learnings_with_cycles() -> SatLearningsFile {
        SatLearningsFile {
            learnings: Vec::new(),
            cycle_learnings: vec![
                SatCycleLearning {
                    recorded_at: "2026-03-26T12:00:00Z".into(),
                    run_id: "run-1".into(),
                    merged_pr_number: 42,
                    repo: "owner/repo".into(),
                    cycle_iteration: 1,
                    issues_found: 10,
                    issues_fixed: 7,
                    false_positives: 1,
                    score_before: 40,
                    score_after: 75,
                    outcome: VerificationOutcome::Verified,
                },
            ],
        }
    }

    // -- Record building ------------------------------------------------------

    #[test]
    fn build_record_captures_all_fields() {
        let record = build_false_positive_record(
            42,
            "owner/repo",
            FalsePositiveLabel::Runner,
            Some("scenario-01"),
            Some("tauri-driver flake"),
            "2026-03-27T12:00:00Z",
        );

        assert_eq!(record.issue_number, 42);
        assert_eq!(record.repo, "owner/repo");
        assert_eq!(record.label, FalsePositiveLabel::Runner);
        assert_eq!(record.scenario_id.as_deref(), Some("scenario-01"));
        assert_eq!(record.reason.as_deref(), Some("tauri-driver flake"));
        assert_eq!(record.recorded_at, "2026-03-27T12:00:00Z");
    }

    #[test]
    fn build_record_handles_none_optionals() {
        let record = build_false_positive_record(
            99,
            "org/repo",
            FalsePositiveLabel::Scenario,
            None,
            None,
            "2026-03-27T12:00:00Z",
        );

        assert!(record.scenario_id.is_none());
        assert!(record.reason.is_none());
    }

    // -- GitHub label strings -------------------------------------------------

    #[test]
    fn github_label_runner() {
        assert_eq!(
            FalsePositiveLabel::Runner.github_label(),
            "false-positive:runner"
        );
    }

    #[test]
    fn github_label_scenario() {
        assert_eq!(
            FalsePositiveLabel::Scenario.github_label(),
            "false-positive:scenario"
        );
    }

    // -- Metrics computation --------------------------------------------------

    #[test]
    fn metrics_empty_when_no_data() {
        let fp_data = FalsePositiveData::default();
        let learnings = SatLearningsFile::default();
        let metrics = compute_false_positive_metrics(&fp_data, &learnings);

        assert_eq!(metrics.total_issues_created, 0);
        assert_eq!(metrics.total_false_positives, 0);
        assert!(metrics.false_positive_rate.is_none());
        assert_eq!(metrics.runner_count, 0);
        assert_eq!(metrics.scenario_count, 0);
    }

    #[test]
    fn metrics_from_fp_records_only() {
        let fp_data = FalsePositiveData {
            records: vec![
                make_record(1, FalsePositiveLabel::Runner),
                make_record(2, FalsePositiveLabel::Scenario),
                make_record(3, FalsePositiveLabel::Runner),
            ],
        };
        let learnings = SatLearningsFile::default();
        let metrics = compute_false_positive_metrics(&fp_data, &learnings);

        assert_eq!(metrics.total_false_positives, 3);
        assert_eq!(metrics.runner_count, 2);
        assert_eq!(metrics.scenario_count, 1);
        assert_eq!(metrics.total_issues_created, 3);
        // Rate = 3/3 = 1.0
        let rate = metrics.false_positive_rate.unwrap();
        assert!((rate - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn metrics_combined_fp_records_and_cycle_learnings() {
        let fp_data = FalsePositiveData {
            records: vec![make_record(100, FalsePositiveLabel::Runner)],
        };
        let learnings = make_learnings_with_cycles();
        let metrics = compute_false_positive_metrics(&fp_data, &learnings);

        // total FPs = 1 (record) + 1 (from cycle) = 2
        assert_eq!(metrics.total_false_positives, 2);
        // total issues = 10 (from cycle only — FP records don't add to denominator
        // when cycles exist, since those issues are already counted)
        assert_eq!(metrics.total_issues_created, 10);
        // Rate = 2/10
        let rate = metrics.false_positive_rate.unwrap();
        assert!((rate - 0.2).abs() < f64::EPSILON);
    }

    // -- Classification accuracy with FP data ---------------------------------

    #[test]
    fn accuracy_includes_fp_records() {
        let learnings = make_learnings_with_cycles();
        let fp_data = FalsePositiveData {
            records: vec![
                make_record(100, FalsePositiveLabel::Runner),
                make_record(101, FalsePositiveLabel::Scenario),
            ],
        };

        let acc = compute_accuracy_with_fp_data(&learnings, &fp_data);

        // TP = issues_fixed = 7
        // FP from cycle = 1, FP from records = 2, total FP = 3
        // total_classifications = 10 (from cycle only — FP records don't add
        // to classifications when cycles exist)
        // accuracy = 7 / (7 + 3) = 0.7
        assert_eq!(acc.true_positives, 7);
        assert_eq!(acc.false_positives, 3);
        assert_eq!(acc.total_classifications, 10);
        let a = acc.accuracy.unwrap();
        assert!((a - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn accuracy_no_cycles_only_fp_records() {
        let learnings = SatLearningsFile::default();
        let fp_data = FalsePositiveData {
            records: vec![make_record(1, FalsePositiveLabel::Runner)],
        };

        let acc = compute_accuracy_with_fp_data(&learnings, &fp_data);

        // TP = 0, FP = 1
        // accuracy = 0 / (0 + 1) = 0.0
        assert_eq!(acc.true_positives, 0);
        assert_eq!(acc.false_positives, 1);
        let a = acc.accuracy.unwrap();
        assert!(a.abs() < f64::EPSILON);
    }

    // -- Persistence round-trip -----------------------------------------------

    #[test]
    fn save_and_load_fp_data_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join(".branchdeck").join("sat-false-positives.json");

        let data = FalsePositiveData {
            records: vec![
                make_record(1, FalsePositiveLabel::Runner),
                make_record(2, FalsePositiveLabel::Scenario),
            ],
        };

        save_false_positive_data(&path, &data).unwrap();
        let loaded = load_false_positive_data(&path).unwrap();

        assert_eq!(loaded.records.len(), 2);
        assert_eq!(loaded.records[0].issue_number, 1);
        assert_eq!(loaded.records[0].label, FalsePositiveLabel::Runner);
        assert_eq!(loaded.records[1].issue_number, 2);
        assert_eq!(loaded.records[1].label, FalsePositiveLabel::Scenario);
    }

    #[test]
    fn load_returns_empty_for_missing_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("does-not-exist.json");

        let data = load_false_positive_data(&path).unwrap();
        assert!(data.records.is_empty());
    }

    #[test]
    fn record_false_positive_end_to_end() {
        let tmp = tempfile::TempDir::new().unwrap();
        let fp_path = tmp.path().join("sat-false-positives.json");
        let learnings_path = tmp.path().join("sat").join("learnings.yaml");

        let record = make_record(42, FalsePositiveLabel::Runner);
        let (data, metrics, accuracy) =
            record_false_positive(&fp_path, &learnings_path, &record).unwrap();

        assert_eq!(data.records.len(), 1);
        assert_eq!(metrics.total_false_positives, 1);
        assert_eq!(metrics.runner_count, 1);
        // With no cycle learnings, FP records are the only data
        assert!(accuracy.accuracy.is_some());
    }

    #[test]
    fn record_false_positive_dedup_same_issue() {
        let tmp = tempfile::TempDir::new().unwrap();
        let fp_path = tmp.path().join("sat-false-positives.json");
        let learnings_path = tmp.path().join("sat").join("learnings.yaml");

        let record = make_record(42, FalsePositiveLabel::Runner);

        // First record
        let (data1, _, _) =
            record_false_positive(&fp_path, &learnings_path, &record).unwrap();
        assert_eq!(data1.records.len(), 1);

        // Second record with same issue+repo — should NOT duplicate
        let (data2, _, _) =
            record_false_positive(&fp_path, &learnings_path, &record).unwrap();
        assert_eq!(data2.records.len(), 1);
    }
}
