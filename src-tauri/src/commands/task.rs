use crate::error::AppError;
use crate::models::task::{TaskInfo, TaskType};
use crate::services::task_watcher::TaskWatcherState;
use tauri::State;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn create_task_cmd(
    worktree_path: String,
    task_type: TaskType,
    repo: String,
    branch: String,
    pr: Option<u64>,
    description: Option<String>,
) -> Result<TaskInfo, AppError> {
    crate::services::task::create_task(
        &worktree_path,
        task_type,
        &repo,
        &branch,
        pr,
        description.as_deref(),
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn get_task_cmd(worktree_path: String) -> Result<TaskInfo, AppError> {
    crate::services::task::get_task(&worktree_path)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn list_tasks_cmd(worktree_paths: Vec<String>) -> Result<Vec<TaskInfo>, AppError> {
    crate::services::task::list_tasks(&worktree_paths)
}

#[tauri::command]
pub async fn start_task_watcher(
    watcher: State<'_, TaskWatcherState>,
    app_handle: tauri::AppHandle,
    worktree_paths: Vec<String>,
) -> Result<(), AppError> {
    let mut w = watcher.lock().await;
    w.start(&app_handle, &worktree_paths)
}

#[tauri::command]
pub async fn stop_task_watcher(watcher: State<'_, TaskWatcherState>) -> Result<(), AppError> {
    let mut w = watcher.lock().await;
    w.stop();
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn watch_task_path(
    watcher: State<'_, TaskWatcherState>,
    worktree_path: String,
) -> Result<bool, AppError> {
    let mut w = watcher.lock().await;
    w.watch_path(&worktree_path)
}
