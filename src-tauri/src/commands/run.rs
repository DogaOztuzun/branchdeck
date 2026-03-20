use crate::error::AppError;
use crate::models::run::{LaunchOptions, RunInfo};
use crate::services::run_manager::{self, QueueStatus, RunManagerState};
use crate::services::run_state;
use crate::services::shepherd;
use std::sync::Arc;
use tauri::{Emitter, State};

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn launch_run_cmd(
    run_manager: State<'_, RunManagerState>,
    app_handle: tauri::AppHandle,
    task_path: String,
    worktree_path: String,
    max_turns: Option<u32>,
    max_budget_usd: Option<f64>,
) -> Result<RunInfo, AppError> {
    let options = LaunchOptions {
        max_turns,
        max_budget_usd,
    };
    let state = Arc::clone(&run_manager);
    run_manager::launch_run(state, app_handle, &task_path, &worktree_path, options).await
}

#[tauri::command]
pub async fn cancel_run_cmd(run_manager: State<'_, RunManagerState>) -> Result<(), AppError> {
    let mut rm = run_manager.lock().await;
    rm.cancel_run().await
}

#[tauri::command]
pub async fn get_run_status_cmd(
    run_manager: State<'_, RunManagerState>,
) -> Result<Option<RunInfo>, AppError> {
    let rm = run_manager.lock().await;
    Ok(rm.get_status())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn recover_runs_cmd(worktree_paths: Vec<String>) -> Vec<RunInfo> {
    run_state::scan_all_run_states(&worktree_paths)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn retry_run_cmd(
    run_manager: State<'_, RunManagerState>,
    app_handle: tauri::AppHandle,
    task_path: String,
    worktree_path: String,
) -> Result<RunInfo, AppError> {
    let state = Arc::clone(&run_manager);
    run_manager::retry_run(state, app_handle, &task_path, &worktree_path).await
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn resume_run_cmd(
    run_manager: State<'_, RunManagerState>,
    app_handle: tauri::AppHandle,
    task_path: String,
    worktree_path: String,
) -> Result<RunInfo, AppError> {
    let state = Arc::clone(&run_manager);
    run_manager::resume_run(state, app_handle, &task_path, &worktree_path).await
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn shepherd_pr_cmd(
    run_manager: State<'_, RunManagerState>,
    #[cfg(feature = "knowledge")] knowledge: State<
        '_,
        std::sync::Arc<crate::services::knowledge::KnowledgeService>,
    >,
    app_handle: tauri::AppHandle,
    repo_path: String,
    pr_number: u64,
    auto_launch: Option<bool>,
) -> Result<shepherd::ShepherdResult, AppError> {
    let result = shepherd::shepherd_pr(
        &repo_path,
        pr_number,
        #[cfg(feature = "knowledge")]
        Some(knowledge.inner()),
    )
    .await?;

    if auto_launch.unwrap_or(false) {
        let state = Arc::clone(&run_manager);
        let options = LaunchOptions {
            max_turns: None,
            max_budget_usd: None,
        };
        if let Err(e) = run_manager::launch_run(
            state,
            app_handle,
            &result.task.path,
            &result.worktree_path,
            options,
        )
        .await
        {
            // Return the shepherd result even on launch failure so the frontend
            // knows a worktree/task was created and can offer manual launch.
            log::error!(
                "launch_run failed after shepherd_pr for PR #{pr_number}: {e} — returning result with worktree intact"
            );
        }
    }

    Ok(result)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn batch_launch_cmd(
    run_manager: State<'_, RunManagerState>,
    app_handle: tauri::AppHandle,
    pairs: Vec<(String, String)>,
) -> Result<QueueStatus, AppError> {
    let state = Arc::clone(&run_manager);
    run_manager::batch_launch(state, app_handle, pairs).await
}

#[tauri::command]
pub async fn cancel_queue_cmd(run_manager: State<'_, RunManagerState>) -> Result<(), AppError> {
    let mut rm = run_manager.lock().await;
    // Cancel active run first (if any), then clear the queue
    if rm.get_status().is_some() {
        rm.cancel_run().await?;
    }
    rm.cancel_queue();
    Ok(())
}

#[tauri::command]
pub async fn queue_status_cmd(
    run_manager: State<'_, RunManagerState>,
) -> Result<QueueStatus, AppError> {
    let rm = run_manager.lock().await;
    Ok(rm.get_queue_status())
}

#[tauri::command]
pub async fn respond_to_permission_cmd(
    run_manager: State<'_, RunManagerState>,
    app_handle: tauri::AppHandle,
    tool_use_id: String,
    decision: String,
    reason: Option<String>,
) -> Result<(), AppError> {
    let mut rm = run_manager.lock().await;
    rm.respond_to_permission(&tool_use_id, &decision, reason.as_deref())
        .await?;
    if let Some(run) = rm.get_status() {
        if let Err(e) = app_handle.emit("run:status_changed", &run) {
            log::error!("Failed to emit run:status_changed after permission response: {e}");
        }
    }
    Ok(())
}
