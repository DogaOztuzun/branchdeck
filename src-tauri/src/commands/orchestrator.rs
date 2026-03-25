use crate::error::AppError;
use crate::models::github::PrSummary;
use crate::models::orchestrator::{
    AnalysisPlan, ApprovedPlan, LifecycleEvent, LifecycleStatus, RunningEntry,
};
use crate::services::event_bus::EventBusState;
use crate::services::orchestrator::{self as orch_service, OrchestratorState};
use crate::services::pr_poller::DiscoveredPrsState;
use crate::services::run_manager::RunManagerState;
use crate::traits::EventEmitter;
use log::{debug, error, info};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn relaunch_pr_cmd(
    orchestrator: State<'_, OrchestratorState>,
    run_manager: State<'_, RunManagerState>,
    event_bus: State<'_, EventBusState>,
    emitter: State<'_, Arc<dyn EventEmitter>>,
    pr_key: String,
    worktree_path: String,
) -> Result<(), AppError> {
    let effects = {
        let mut orch = orchestrator.lock().await;
        orch_service::apply_relaunch(
            &mut orch,
            &pr_key,
            &worktree_path,
            crate::models::agent::now_ms(),
        )
    };

    let orch_state = Arc::clone(&orchestrator);
    let rm_state = Arc::clone(&run_manager);
    orch_service::execute_effects(effects, &orch_state, &rm_state, &emitter, Some(&event_bus))
        .await;

    info!("Relaunched PR {pr_key}");
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn skip_pr_cmd(
    orchestrator: State<'_, OrchestratorState>,
    run_manager: State<'_, RunManagerState>,
    event_bus: State<'_, EventBusState>,
    emitter: State<'_, Arc<dyn EventEmitter>>,
    pr_key: String,
) -> Result<(), AppError> {
    let effects = {
        let mut orch = orchestrator.lock().await;
        orch_service::apply_skip(&mut orch, &pr_key)
    };

    let orch_state = Arc::clone(&orchestrator);
    let rm_state = Arc::clone(&run_manager);
    orch_service::execute_effects(effects, &orch_state, &rm_state, &emitter, Some(&event_bus))
        .await;

    info!("Skipped PR {pr_key}");
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn get_lifecycles_cmd(
    orchestrator: State<'_, OrchestratorState>,
) -> Result<Vec<LifecycleEvent>, AppError> {
    let orch = orchestrator.lock().await;
    let mut events = Vec::new();

    for entry in orch.running.values() {
        events.push(LifecycleEvent {
            pr_key: entry.pr_key.clone(),
            worktree_path: entry.worktree_path.clone(),
            status: LifecycleStatus::Running,
            attempt: entry.attempt,
            started_at: entry.started_at,
            session_id: Some(entry.tab_id.clone()),
        });
    }

    for entry in orch.retry_queue.values() {
        events.push(LifecycleEvent {
            pr_key: entry.pr_key.clone(),
            worktree_path: entry.worktree_path.clone(),
            status: LifecycleStatus::Retrying,
            attempt: entry.attempt,
            started_at: entry.due_at_ms,
            session_id: None,
        });
    }

    for entry in orch.review_ready.values() {
        let status = if entry.stale {
            LifecycleStatus::Stale
        } else {
            LifecycleStatus::ReviewReady
        };
        events.push(LifecycleEvent {
            pr_key: entry.pr_key.clone(),
            worktree_path: entry.worktree_path.clone(),
            status,
            attempt: entry.attempt,
            started_at: entry.started_at,
            session_id: None,
        });
    }

    // Include completed lifecycles for Done section
    for event in orch.completed_lifecycles.values() {
        events.push(event.clone());
    }

    Ok(events)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn toggle_orchestrator_cmd(
    orchestrator: State<'_, OrchestratorState>,
    enabled: bool,
) -> Result<(), AppError> {
    let mut orch = orchestrator.lock().await;
    orch.config.enabled = enabled;
    info!(
        "Orchestrator {}",
        if enabled { "enabled" } else { "disabled" }
    );
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn orchestrator_shepherd_pr_cmd(
    orchestrator: State<'_, OrchestratorState>,
    run_manager: State<'_, RunManagerState>,
    event_bus: State<'_, EventBusState>,
    emitter: State<'_, Arc<dyn EventEmitter>>,
    repo_path: String,
    pr_number: u64,
) -> Result<(), AppError> {
    use crate::models::orchestrator::pr_key;
    use crate::services::github;
    use std::path::Path;

    // Resolve owner/repo from repo path
    let (owner, repo) = github::resolve_owner_repo(Path::new(&repo_path))?;
    let full_repo = format!("{owner}/{repo}");
    let key = pr_key(&full_repo, pr_number);

    // Fetch PR from GitHub for branch info
    let client = github::get_client_pub().await?;
    let pr = client
        .pulls(&owner, &repo)
        .get(pr_number)
        .await
        .map_err(|e| {
            error!("Failed to fetch PR #{pr_number}: {e}");
            AppError::GitHub(format!("PR #{pr_number} not found: {e}"))
        })?;

    let branch = pr.head.ref_field.clone();
    let base_branch = pr.base.ref_field.clone();

    let worktree_path = crate::services::orchestrator::build_worktree_path(&repo_path, &branch);

    let pr_context = crate::models::orchestrator::PrContext {
        repo: full_repo,
        number: pr_number,
        branch,
        base_branch,
    };

    let effects = {
        let mut orch = orchestrator.lock().await;

        // Claim and dispatch directly
        orch.claimed.insert(key.clone());

        vec![
            crate::models::orchestrator::OrchestratorEffect::DispatchSession {
                pr_key: key.clone(),
                worktree_path: worktree_path.clone(),
                pr_context,
                attempt: 1,
            },
            crate::models::orchestrator::OrchestratorEffect::EmitLifecycleEvent {
                event: LifecycleEvent {
                    pr_key: key,
                    worktree_path,
                    status: LifecycleStatus::Running,
                    attempt: 1,
                    started_at: crate::models::agent::now_ms(),
                    session_id: None, // populated by executor after dispatch
                },
            },
        ]
    };

    let orch_state = Arc::clone(&orchestrator);
    let rm_state = Arc::clone(&run_manager);
    orch_service::execute_effects(effects, &orch_state, &rm_state, &emitter, Some(&event_bus))
        .await;

    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn list_discovered_prs_cmd(
    discovered_prs: State<'_, DiscoveredPrsState>,
) -> Result<Vec<PrSummary>, AppError> {
    let state = discovered_prs.read().map_err(|e| {
        error!("Failed to read discovered PRs: {e}");
        AppError::Config(format!("Lock poisoned: {e}"))
    })?;
    let flat: Vec<PrSummary> = state.values().flat_map(|v| v.iter().cloned()).collect();
    debug!("list_discovered_prs_cmd: returning {} PRs", flat.len());
    Ok(flat)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn get_running_entries_cmd(
    orchestrator: State<'_, OrchestratorState>,
) -> Result<Vec<RunningEntry>, AppError> {
    let orch = orchestrator.lock().await;
    let entries: Vec<RunningEntry> = orch.running.values().cloned().collect();
    debug!(
        "get_running_entries_cmd: returning {} entries",
        entries.len()
    );
    Ok(entries)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn read_analysis_cmd(worktree_path: String) -> Result<Option<String>, AppError> {
    let path = format!("{worktree_path}/.branchdeck/analysis.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(Some(content)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => {
            error!("Failed to read analysis.json: {e}");
            Err(AppError::Io(e))
        }
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn write_approval_cmd(
    worktree_path: String,
    approved_plan: ApprovedPlan,
) -> Result<(), AppError> {
    let path = format!("{worktree_path}/.branchdeck/analysis.json");

    let content = std::fs::read_to_string(&path).map_err(|e| {
        error!("Failed to read analysis.json for approval: {e}");
        AppError::Io(e)
    })?;

    let mut analysis: AnalysisPlan = serde_json::from_str(&content).map_err(|e| {
        error!("Failed to parse analysis.json: {e}");
        AppError::Config(format!("Invalid analysis.json: {e}"))
    })?;

    analysis.approved = true;
    analysis.approved_plan = Some(approved_plan);

    let json = serde_json::to_string_pretty(&analysis).map_err(|e| {
        error!("Failed to serialize analysis.json: {e}");
        AppError::Config(format!("Serialization error: {e}"))
    })?;

    std::fs::write(&path, json).map_err(|e| {
        error!("Failed to write analysis.json: {e}");
        AppError::Io(e)
    })?;

    info!("Wrote approval to {path}");
    Ok(())
}
