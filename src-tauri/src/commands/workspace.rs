use crate::error::AppError;
use crate::services::config::{self, GlobalConfig, RepoConfig};

#[tauri::command]
pub fn get_app_config() -> Result<GlobalConfig, AppError> {
    config::load_global_config()
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn save_app_config(config: GlobalConfig) -> Result<(), AppError> {
    config::save_global_config(&config)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn get_repo_config(repo_path: String) -> Result<RepoConfig, AppError> {
    config::load_repo_config(&repo_path)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn save_repo_config_cmd(repo_path: String, config: RepoConfig) -> Result<(), AppError> {
    config::save_repo_config(&repo_path, &config)
}
