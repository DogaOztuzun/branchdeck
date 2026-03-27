//! SAT pipeline orchestration service.
//!
//! Chains the 4 SAT stages in sequence:
//! 1. Generate — build manifest from personas + scenarios
//! 2. Execute — run scenarios via `WebDriver`
//! 3. Score — evaluate results with LLM-as-judge
//! 4. Create Issues — file GitHub issues for high-confidence findings
//!
//! Architecture:
//! - Pure function `build_pipeline_result` for assembling results
//! - `run_sat_pipeline` is the top-level orchestrator that calls each stage
//! - Trait objects (`LlmJudge`, `IssueCreator`) are injected by the caller
//! - If any stage fails, the pipeline stops and returns a partial result

use std::path::Path;
use std::time::Instant;

use log::{error, info};

use crate::error::AppError;
use crate::models::sat::{
    SatGenerationConfig, SatIssueConfig, SatManifestEntry, SatPipelineConfig, SatPipelineResult,
    SatPipelineStage, SatPipelineStatus, SatRunConfig, SatScoreConfig, SatStageResult,
};
use crate::services::sat_execute;
use crate::services::sat_generate;
use crate::services::sat_issues::{self, IssueCreator};
use crate::services::sat_score::{self, LlmJudge};
use crate::services::sat_threshold_config;

// ---------------------------------------------------------------------------
// Pure result builder
// ---------------------------------------------------------------------------

/// Build a pipeline result from accumulated stage results.
#[must_use]
pub fn build_pipeline_result(
    stages: Vec<SatStageResult>,
    status: SatPipelineStatus,
    total_duration_ms: u64,
    run_id: Option<String>,
    aggregate_score: Option<u32>,
    issues_created: Option<usize>,
) -> SatPipelineResult {
    SatPipelineResult {
        status,
        stages,
        total_duration_ms,
        run_id,
        aggregate_score,
        issues_created,
    }
}

/// Create a stage result for a successful stage.
#[must_use]
pub fn stage_ok(stage: SatPipelineStage, duration_ms: u64) -> SatStageResult {
    SatStageResult {
        stage,
        success: true,
        duration_ms,
        error: None,
    }
}

/// Create a stage result for a failed stage.
#[must_use]
pub fn stage_err(stage: SatPipelineStage, duration_ms: u64, error: String) -> SatStageResult {
    SatStageResult {
        stage,
        success: false,
        duration_ms,
        error: Some(error),
    }
}

// ---------------------------------------------------------------------------
// Pipeline execution
// ---------------------------------------------------------------------------

/// Convert `Duration::as_millis()` (u128) to u64 safely.
/// Durations under ~584 million years fit in u64, so truncation is not a concern.
#[allow(clippy::cast_possible_truncation)]
fn millis_u64(d: std::time::Duration) -> u64 {
    d.as_millis() as u64
}

/// Run the complete SAT pipeline: generate -> execute -> score -> create issues.
///
/// If any stage fails, the pipeline stops immediately and returns a partial
/// result with the error. The caller (Tauri command or workflow agent) decides
/// whether to retry.
///
/// # Arguments
/// * `config` — Pipeline-level configuration (project root, budget, filter).
/// * `judge` — LLM judge implementation for the scoring stage.
/// * `creator` — Issue creator implementation for the issue creation stage.
/// * `owner` — GitHub repo owner (for issue creation).
/// * `repo` — GitHub repo name (for issue creation).
///
/// # Errors
/// Returns `AppError::Sat` if a stage fails (error is also captured in the result).
#[allow(clippy::too_many_lines)]
pub fn run_sat_pipeline(
    config: &SatPipelineConfig,
    judge: &dyn LlmJudge,
    creator: &dyn IssueCreator,
    owner: &str,
    repo: &str,
) -> Result<SatPipelineResult, AppError> {
    let pipeline_start = Instant::now();
    let mut stages: Vec<SatStageResult> = Vec::new();
    #[allow(unused_assignments)]
    let mut run_id: Option<String> = None;
    #[allow(unused_assignments)]
    let mut aggregate_score: Option<u32> = None;
    #[allow(unused_assignments)]
    let mut issues_created: Option<usize> = None;
    #[allow(unused_assignments)]
    let mut manifest_scenarios: Vec<SatManifestEntry> = Vec::new();

    info!(
        "Starting SAT pipeline for {}",
        config.project_root.display()
    );

    // --- Stage 1: Generate ---
    let gen_start = Instant::now();
    let gen_config = SatGenerationConfig::new(config.project_root.clone());
    match sat_generate::generate_manifest(&gen_config) {
        Ok(manifest) => {
            let duration = millis_u64(gen_start.elapsed());
            info!(
                "Pipeline: generate complete — {} personas, {} scenarios ({duration}ms)",
                manifest.persona_count, manifest.scenario_count
            );
            manifest_scenarios.clone_from(&manifest.scenarios);
            stages.push(stage_ok(SatPipelineStage::Generate, duration));

            // Fail early if there are no scenarios to execute
            if manifest.scenario_count == 0 {
                let msg = "manifest contains 0 scenarios — nothing to execute".to_string();
                error!("Pipeline: {msg}");
                let total = millis_u64(pipeline_start.elapsed());
                return Ok(build_pipeline_result(
                    stages,
                    SatPipelineStatus::Failed {
                        stage: SatPipelineStage::Generate,
                        error: msg,
                    },
                    total,
                    None,
                    None,
                    None,
                ));
            }
        }
        Err(e) => {
            let duration = millis_u64(gen_start.elapsed());
            let msg = format!("{e}");
            error!("Pipeline: generate failed — {msg}");
            stages.push(stage_err(SatPipelineStage::Generate, duration, msg.clone()));
            let total = millis_u64(pipeline_start.elapsed());
            return Ok(build_pipeline_result(
                stages,
                SatPipelineStatus::Failed {
                    stage: SatPipelineStage::Generate,
                    error: msg,
                },
                total,
                None,
                None,
                None,
            ));
        }
    }

    // --- Stage 2: Execute ---
    let exec_start = Instant::now();
    let exec_config = SatRunConfig::new(config.project_root.clone());
    match sat_execute::execute_run(&exec_config, &config.scenario_filter) {
        Ok(exec_result) => {
            let duration = millis_u64(exec_start.elapsed());
            run_id = Some(exec_result.run_id.clone());
            info!(
                "Pipeline: execute complete — run {} ({} scenarios, {duration}ms)",
                exec_result.run_id, exec_result.scenarios_total
            );
            stages.push(stage_ok(SatPipelineStage::Execute, duration));
        }
        Err(e) => {
            let duration = millis_u64(exec_start.elapsed());
            let msg = format!("{e}");
            error!("Pipeline: execute failed — {msg}");
            stages.push(stage_err(SatPipelineStage::Execute, duration, msg.clone()));
            let total = millis_u64(pipeline_start.elapsed());
            return Ok(build_pipeline_result(
                stages,
                SatPipelineStatus::Failed {
                    stage: SatPipelineStage::Execute,
                    error: msg,
                },
                total,
                None,
                None,
                None,
            ));
        }
    }

    let current_run_id = run_id.clone().unwrap_or_default();

    // --- Stage 3: Score ---
    let score_start = Instant::now();
    let mut score_config = SatScoreConfig::new(config.project_root.clone());
    score_config.budget.max_cost_dollars = config.max_budget_usd;
    match sat_score::score_run(&score_config, &current_run_id, judge) {
        Ok(score_result) => {
            let duration = millis_u64(score_start.elapsed());
            aggregate_score = Some(score_result.aggregate_score);
            info!(
                "Pipeline: score complete — {}/100, {} findings ({duration}ms)",
                score_result.aggregate_score, score_result.finding_counts.total
            );
            stages.push(stage_ok(SatPipelineStage::Score, duration));
        }
        Err(e) => {
            let duration = millis_u64(score_start.elapsed());
            let msg = format!("{e}");
            error!("Pipeline: score failed — {msg}");
            stages.push(stage_err(SatPipelineStage::Score, duration, msg.clone()));
            let total = millis_u64(pipeline_start.elapsed());
            return Ok(build_pipeline_result(
                stages,
                SatPipelineStatus::Failed {
                    stage: SatPipelineStage::Score,
                    error: msg,
                },
                total,
                run_id,
                None,
                None,
            ));
        }
    }

    // --- Stage 4: Create Issues ---
    let issue_start = Instant::now();
    let mut issue_config = SatIssueConfig::new(config.project_root.clone());

    // Load threshold config and apply per-scenario severity overrides (Story 6.1).
    // Config is read fresh from disk so changes take effect without restart.
    match sat_threshold_config::load_threshold_config(&config.project_root) {
        Ok(threshold_config) => {
            sat_threshold_config::apply_threshold_config(
                &mut issue_config,
                &threshold_config,
                &manifest_scenarios,
            );
        }
        Err(e) => {
            // Non-fatal: use defaults if threshold config is invalid
            log::warn!("Failed to load SAT threshold config — using defaults: {e}");
        }
    }

    match sat_issues::create_issues_from_run(&issue_config, &current_run_id, owner, repo, creator) {
        Ok(issue_result) => {
            let duration = millis_u64(issue_start.elapsed());
            issues_created = Some(issue_result.created_count);
            info!(
                "Pipeline: create-issues complete — {} created, {} skipped ({duration}ms)",
                issue_result.created_count, issue_result.skipped_count
            );
            stages.push(stage_ok(SatPipelineStage::CreateIssues, duration));
        }
        Err(e) => {
            let duration = millis_u64(issue_start.elapsed());
            let msg = format!("{e}");
            error!("Pipeline: create-issues failed — {msg}");
            stages.push(stage_err(
                SatPipelineStage::CreateIssues,
                duration,
                msg.clone(),
            ));
            let total = millis_u64(pipeline_start.elapsed());
            return Ok(build_pipeline_result(
                stages,
                SatPipelineStatus::Failed {
                    stage: SatPipelineStage::CreateIssues,
                    error: msg,
                },
                total,
                run_id,
                aggregate_score,
                None,
            ));
        }
    }

    // --- All stages complete ---
    let total = millis_u64(pipeline_start.elapsed());
    info!("SAT pipeline complete ({total}ms)");
    Ok(build_pipeline_result(
        stages,
        SatPipelineStatus::Completed,
        total,
        run_id,
        aggregate_score,
        issues_created,
    ))
}

/// Resolve the GitHub owner/repo from a project root path.
///
/// # Errors
/// Returns `AppError::Sat` if the git remote cannot be parsed.
pub fn resolve_repo_info(project_root: &Path) -> Result<(String, String), AppError> {
    crate::services::github::resolve_owner_repo(project_root).map_err(|e| {
        error!(
            "Failed to resolve owner/repo for {}: {e}",
            project_root.display()
        );
        AppError::Sat(format!("cannot resolve GitHub repo: {e}"))
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::sat::{SatPipelineStage, SatPipelineStatus};

    #[test]
    fn stage_ok_builds_success() {
        let result = stage_ok(SatPipelineStage::Generate, 1500);
        assert!(result.success);
        assert_eq!(result.duration_ms, 1500);
        assert!(result.error.is_none());
        assert_eq!(result.stage, SatPipelineStage::Generate);
    }

    #[test]
    fn stage_err_builds_failure() {
        let result = stage_err(SatPipelineStage::Score, 500, "LLM timeout".into());
        assert!(!result.success);
        assert_eq!(result.duration_ms, 500);
        assert_eq!(result.error.as_deref(), Some("LLM timeout"));
    }

    #[test]
    fn build_pipeline_result_completed() {
        let stages = vec![
            stage_ok(SatPipelineStage::Generate, 100),
            stage_ok(SatPipelineStage::Execute, 200),
            stage_ok(SatPipelineStage::Score, 300),
            stage_ok(SatPipelineStage::CreateIssues, 150),
        ];
        let result = build_pipeline_result(
            stages,
            SatPipelineStatus::Completed,
            750,
            Some("run-test".into()),
            Some(85),
            Some(3),
        );
        assert_eq!(result.status, SatPipelineStatus::Completed);
        assert_eq!(result.stages.len(), 4);
        assert_eq!(result.total_duration_ms, 750);
        assert_eq!(result.run_id.as_deref(), Some("run-test"));
        assert_eq!(result.aggregate_score, Some(85));
        assert_eq!(result.issues_created, Some(3));
    }

    #[test]
    fn build_pipeline_result_failed_partial() {
        let stages = vec![
            stage_ok(SatPipelineStage::Generate, 100),
            stage_err(SatPipelineStage::Execute, 50, "WebDriver down".into()),
        ];
        let result = build_pipeline_result(
            stages,
            SatPipelineStatus::Failed {
                stage: SatPipelineStage::Execute,
                error: "WebDriver down".into(),
            },
            150,
            None,
            None,
            None,
        );
        assert!(matches!(
            result.status,
            SatPipelineStatus::Failed {
                stage: SatPipelineStage::Execute,
                ..
            }
        ));
        assert_eq!(result.stages.len(), 2);
        assert!(result.stages[0].success);
        assert!(!result.stages[1].success);
        assert!(result.run_id.is_none());
    }

    #[test]
    fn pipeline_stage_display() {
        assert_eq!(SatPipelineStage::Generate.to_string(), "generate");
        assert_eq!(SatPipelineStage::Execute.to_string(), "execute");
        assert_eq!(SatPipelineStage::Score.to_string(), "score");
        assert_eq!(SatPipelineStage::CreateIssues.to_string(), "create-issues");
    }

    #[test]
    fn pipeline_status_display() {
        let running = SatPipelineStatus::Running {
            stage: SatPipelineStage::Score,
        };
        assert_eq!(running.to_string(), "running (score)");

        let completed = SatPipelineStatus::Completed;
        assert_eq!(completed.to_string(), "completed");

        let failed = SatPipelineStatus::Failed {
            stage: SatPipelineStage::Execute,
            error: "timeout".into(),
        };
        assert_eq!(failed.to_string(), "failed at execute: timeout");
    }
}
