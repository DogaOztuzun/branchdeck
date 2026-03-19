use crate::error::AppError;
use crate::models::run::{LaunchOptions, RunInfo};
use crate::services::run_manager::{self, RunManagerState};
use crate::services::run_state;
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
