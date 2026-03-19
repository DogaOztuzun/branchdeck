use crate::error::AppError;
use crate::models::run::{LaunchOptions, RunInfo};
use crate::services::run_manager::{self, RunManagerState};
use std::sync::Arc;
use tauri::State;

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
