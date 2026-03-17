use crate::error::AppError;
use crate::models::agent::{AgentDefinition, AgentState, FileAccess};
use crate::services::activity_store::ActivityStore;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

pub struct AgentMonitorConfig {
    pub script_path: PathBuf,
    #[allow(dead_code)]
    pub port: u16,
}

#[tauri::command]
pub async fn get_agents(
    store: State<'_, Arc<ActivityStore>>,
    tab_id: String,
) -> Result<Vec<AgentState>, AppError> {
    Ok(store.get_agents_for_tab(&tab_id).await)
}

#[tauri::command]
pub async fn get_file_activity(
    store: State<'_, Arc<ActivityStore>>,
) -> Result<Vec<FileAccess>, AppError> {
    Ok(store.get_all_files().await)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn list_agent_definitions(repo_path: String) -> Result<Vec<AgentDefinition>, AppError> {
    crate::services::agent_scanner::scan_agent_definitions(&repo_path)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn install_agent_hooks(
    config: State<'_, AgentMonitorConfig>,
    repo_path: String,
) -> Result<(), AppError> {
    crate::services::hook_config::install_hooks(&repo_path, &config.script_path)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn remove_agent_hooks(
    config: State<'_, AgentMonitorConfig>,
    repo_path: String,
) -> Result<(), AppError> {
    crate::services::hook_config::remove_hooks(&repo_path, &config.script_path)
}
