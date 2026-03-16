use crate::error::AppError;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GlobalConfig {
    pub window: WindowConfig,
    pub default_shell: String,
    pub repos: Vec<String>,
    #[serde(default)]
    pub last_active_repo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RepoConfig {
    pub path: String,
    pub last_worktree: Option<String>,
    pub sidebar_collapsed: bool,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig {
                width: 1200,
                height: 800,
                x: 0,
                y: 0,
            },
            default_shell: "/bin/bash".to_string(),
            repos: Vec::new(),
            last_active_repo: None,
        }
    }
}

pub fn config_dir() -> Result<PathBuf, AppError> {
    let dir = dirs::config_dir()
        .ok_or_else(|| AppError::Config("Could not determine config directory".to_string()))?
        .join("branchdeck");

    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }

    Ok(dir)
}

pub fn repo_config_path(repo_path: &str) -> Result<PathBuf, AppError> {
    let mut hasher = Sha256::new();
    hasher.update(repo_path.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    let dir = config_dir()?;
    Ok(dir.join(format!("{hash}.json")))
}

pub fn load_global_config() -> Result<GlobalConfig, AppError> {
    let path = config_dir()?.join("config.json");

    if !path.exists() {
        return Ok(GlobalConfig::default());
    }

    let contents = std::fs::read_to_string(&path)?;
    let config = serde_json::from_str(&contents).map_err(|e| {
        error!("Failed to parse global config: {e}");
        AppError::Config(e.to_string())
    })?;

    debug!("Loaded global config from {}", path.display());

    Ok(config)
}

pub fn save_global_config(config: &GlobalConfig) -> Result<(), AppError> {
    let path = config_dir()?.join("config.json");
    let contents =
        serde_json::to_string_pretty(config).map_err(|e| AppError::Config(e.to_string()))?;
    std::fs::write(&path, contents)?;
    info!("Saved global config to {}", path.display());
    Ok(())
}

pub fn load_repo_config(repo_path: &str) -> Result<RepoConfig, AppError> {
    let path = repo_config_path(repo_path)?;

    if !path.exists() {
        return Ok(RepoConfig {
            path: repo_path.to_string(),
            ..RepoConfig::default()
        });
    }

    let contents = std::fs::read_to_string(&path)?;
    let config = serde_json::from_str(&contents).map_err(|e| {
        error!("Failed to parse repo config for {repo_path:?}: {e}");
        AppError::Config(e.to_string())
    })?;

    debug!("Loaded repo config for {repo_path:?}");

    Ok(config)
}

pub fn save_repo_config(repo_path: &str, config: &RepoConfig) -> Result<(), AppError> {
    let path = repo_config_path(repo_path)?;
    let contents =
        serde_json::to_string_pretty(config).map_err(|e| AppError::Config(e.to_string()))?;
    std::fs::write(&path, contents)?;
    info!("Saved repo config for {repo_path:?}");
    Ok(())
}
