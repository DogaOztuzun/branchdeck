use crate::error::AppError;
use crate::models::sat::{SatPipelineConfig, SatPipelineResult};
use crate::services::sat_pipeline;
use log::{error, info};
use std::path::Path;

/// Trigger a complete SAT quality audit cycle.
///
/// This is the manual trigger entry point (FR20). It chains:
/// generate -> execute -> score -> create-issues.
///
/// The `project_root` must be a git repository with a `sat/` directory
/// containing personas and scenarios.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn trigger_sat_cycle(project_root: String) -> Result<SatPipelineResult, AppError> {
    info!("Manual SAT cycle triggered for {project_root}");

    let root = Path::new(&project_root);
    if !root.exists() {
        return Err(AppError::Sat(format!(
            "project root does not exist: {project_root}"
        )));
    }

    let root_path = root.to_path_buf();

    // Use no-op implementations for the integration boundaries.
    // In production, the agent-driven path handles LLM and GitHub API calls.
    // The manual trigger uses the pipeline to chain stages; actual LLM/GitHub
    // calls happen through the sat_score and sat_issues services which require
    // trait implementations injected here.
    //
    // For now, create placeholder impls that return errors — the real
    // implementations will be wired when the agent bridge is connected.
    let judge = PlaceholderJudge;
    let creator = PlaceholderCreator;

    // Run resolve_repo_info + pipeline on a blocking thread to avoid
    // blocking the Tauri async runtime (git2 I/O is synchronous)
    let result = tokio::task::spawn_blocking(move || {
        let (owner, repo) = sat_pipeline::resolve_repo_info(&root_path)?;
        let config = SatPipelineConfig::new(root_path);
        sat_pipeline::run_sat_pipeline(&config, &judge, &creator, &owner, &repo)
    })
    .await
    .map_err(|e| {
        error!("SAT pipeline task panicked: {e}");
        AppError::Sat(format!("pipeline task failed: {e}"))
    })??;

    info!("SAT cycle result: {}", result.status);
    Ok(result)
}

/// Placeholder LLM judge that returns an error.
///
/// In production, the workflow agent handles LLM calls via CLI.
/// This placeholder allows the pipeline structure to work for testing
/// the generate and execute stages without an LLM connection.
struct PlaceholderJudge;

impl crate::services::sat_score::LlmJudge for PlaceholderJudge {
    fn score(
        &self,
        _system_prompt: &str,
        _user_prompt: &str,
    ) -> Result<(String, crate::models::sat::TokenUsage), AppError> {
        Err(AppError::Sat(
            "LLM judge not configured — use the workflow agent path for scoring".into(),
        ))
    }
}

/// Placeholder issue creator that returns an error.
///
/// In production, the workflow agent handles GitHub API calls.
struct PlaceholderCreator;

impl crate::services::sat_issues::IssueCreator for PlaceholderCreator {
    fn create_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _title: &str,
        _body: &str,
        _labels: &[String],
    ) -> Result<(u64, String), AppError> {
        Err(AppError::Sat(
            "Issue creator not configured — use the workflow agent path for issue creation".into(),
        ))
    }

    fn issue_exists_with_fingerprint(
        &self,
        _owner: &str,
        _repo: &str,
        _fingerprint: &str,
    ) -> Result<bool, AppError> {
        Err(AppError::Sat(
            "Issue creator not configured — use the workflow agent path for dedup checks".into(),
        ))
    }
}
