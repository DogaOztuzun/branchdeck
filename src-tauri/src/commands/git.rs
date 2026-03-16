use crate::error::AppError;
use crate::models::{BranchInfo, FileStatus, RepoInfo, WorktreeInfo, WorktreePreview};
use crate::services::{config, git};
use std::path::PathBuf;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn add_repository(app: tauri::AppHandle) -> Result<Option<RepoInfo>, AppError> {
    let folder =
        tauri::async_runtime::spawn_blocking(move || app.dialog().file().blocking_pick_folder())
            .await
            .map_err(|e| AppError::Config(e.to_string()))?;

    let Some(path) = folder else {
        return Ok(None);
    };

    let path_buf = path.as_path().ok_or_else(|| {
        AppError::Config("Selected path is not a valid filesystem path".to_string())
    })?;

    let repo_info = git::validate_repo(path_buf)?;

    let mut global_config = config::load_global_config()?;
    let path_str = repo_info.path.to_string_lossy().to_string();
    if !global_config.repos.contains(&path_str) {
        global_config.repos.push(path_str);
        config::save_global_config(&global_config)?;
    }

    Ok(Some(repo_info))
}

#[tauri::command]
pub fn list_repositories() -> Result<Vec<RepoInfo>, AppError> {
    let global_config = config::load_global_config()?;
    let mut repos = Vec::new();

    for repo_path in &global_config.repos {
        if let Ok(info) = git::validate_repo(&PathBuf::from(repo_path)) {
            repos.push(info);
        }
    }

    Ok(repos)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn remove_repository(repo_path: String) -> Result<(), AppError> {
    let mut global_config = config::load_global_config()?;
    global_config.repos.retain(|p| p != &repo_path);
    config::save_global_config(&global_config)?;
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn list_worktrees_cmd(repo_path: String) -> Result<Vec<WorktreeInfo>, AppError> {
    git::list_worktrees(&PathBuf::from(&repo_path))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn create_worktree_cmd(
    repo_path: String,
    name: String,
    branch: Option<String>,
    base_branch: Option<String>,
) -> Result<WorktreeInfo, AppError> {
    git::create_worktree(
        &PathBuf::from(&repo_path),
        &name,
        branch.as_deref(),
        base_branch.as_deref(),
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn remove_worktree_cmd(
    repo_path: String,
    worktree_name: String,
    delete_branch: bool,
) -> Result<(), AppError> {
    git::remove_worktree(&PathBuf::from(&repo_path), &worktree_name, delete_branch)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn preview_worktree_cmd(repo_path: String, name: String) -> Result<WorktreePreview, AppError> {
    git::preview_worktree(&PathBuf::from(&repo_path), &name)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn get_repo_status(worktree_path: String) -> Result<Vec<FileStatus>, AppError> {
    git::get_status(&PathBuf::from(&worktree_path))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn list_branches_cmd(repo_path: String) -> Result<Vec<BranchInfo>, AppError> {
    git::list_branches(&PathBuf::from(&repo_path))
}
