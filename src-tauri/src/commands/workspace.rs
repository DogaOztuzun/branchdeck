use crate::error::AppError;
use crate::services::config::{self, GlobalConfig, RepoConfig};
use log::{debug, info};

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

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn get_presets(repo_path: String) -> Result<Vec<config::Preset>, AppError> {
    debug!("Loading presets for repo: {repo_path:?}");
    let config = config::load_repo_config(&repo_path)?;
    Ok(config.presets)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn save_presets(repo_path: String, presets: Vec<config::Preset>) -> Result<(), AppError> {
    info!("Saving {} presets for repo: {repo_path:?}", presets.len());
    let mut config = config::load_repo_config(&repo_path)?;
    config.presets = presets;
    config::save_repo_config(&repo_path, &config)
}
