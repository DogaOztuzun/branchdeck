use std::path::{Path, PathBuf};

use log::{debug, error, info};

use crate::error::AppError;
use crate::models::project_config::{
    ProjectConfig, SetupStatus, TokenValidation, WorkflowOption,
};
use crate::services::workflow::WorkflowRegistry;

const CONFIG_DIR: &str = ".branchdeck";
const CONFIG_FILE: &str = "config.yaml";

/// Resolve the path to the project config file for a given repo.
#[must_use]
pub fn config_path(repo_path: &str) -> PathBuf {
    Path::new(repo_path).join(CONFIG_DIR).join(CONFIG_FILE)
}

/// Check whether a project has been configured.
///
/// # Errors
/// Returns `AppError` if the config file exists but cannot be read or parsed.
pub fn get_setup_status(repo_path: &str) -> Result<SetupStatus, AppError> {
    let path = config_path(repo_path);
    let path_str = path.display().to_string();

    if !path.exists() {
        debug!("No project config at {path_str}");
        return Ok(SetupStatus {
            configured: false,
            config_path: path_str,
            config: None,
        });
    }

    let config = load_project_config(repo_path)?;
    debug!("Project config found at {path_str}");
    Ok(SetupStatus {
        configured: true,
        config_path: path_str,
        config: Some(config),
    })
}

/// Load the project config from `.branchdeck/config.yaml`.
///
/// # Errors
/// Returns `AppError::Config` if the file cannot be read or parsed.
pub fn load_project_config(repo_path: &str) -> Result<ProjectConfig, AppError> {
    let path = config_path(repo_path);
    let content = std::fs::read_to_string(&path).map_err(|e| {
        error!(
            "Failed to read project config at {}: {e}",
            path.display()
        );
        AppError::Config(format!("failed to read {}: {e}", path.display()))
    })?;

    let config: ProjectConfig = serde_yaml::from_str(&content).map_err(|e| {
        error!(
            "Failed to parse project config at {}: {e}",
            path.display()
        );
        AppError::Config(format!("failed to parse {}: {e}", path.display()))
    })?;

    debug!("Loaded project config for {repo_path:?}");
    Ok(config)
}

/// Save the project config to `.branchdeck/config.yaml`.
///
/// Creates the `.branchdeck/` directory if it does not exist.
/// The `enabled_workflows` field is persisted here and read by the orchestrator
/// at workflow dispatch time to filter which workflows are active for this project.
///
/// # Errors
/// Returns `AppError` if serialization or file write fails.
pub fn save_project_config(config: &ProjectConfig) -> Result<(), AppError> {
    let path = config_path(&config.repo_path);
    let yaml = serde_yaml::to_string(config).map_err(|e| {
        error!("Failed to serialize project config: {e}");
        AppError::Config(format!("serialization error: {e}"))
    })?;

    crate::util::write_atomic(&path, yaml.as_bytes())?;
    info!(
        "Saved project config for {:?} at {}",
        config.repo_path,
        path.display()
    );
    Ok(())
}

/// Validate availability of GitHub and Anthropic tokens by checking
/// environment variables and the `gh` CLI auth status.
///
/// # Errors
/// Returns `AppError` only on unexpected system failures.
#[must_use]
pub fn validate_tokens() -> TokenValidation {
    let (github_available, github_source) = check_github_token();
    let (anthropic_available, anthropic_source) = check_anthropic_key();

    debug!(
        "Token validation: github={github_available} ({github_source}), anthropic={anthropic_available} ({anthropic_source})"
    );

    TokenValidation {
        github_available,
        github_source,
        anthropic_available,
        anthropic_source,
    }
}

/// List available workflows from the registry for a given repo path.
#[must_use]
pub fn list_available_workflows(repo_path: &str) -> Vec<WorkflowOption> {
    let search_dirs = crate::services::workflow::default_search_dirs(repo_path);
    let registry = WorkflowRegistry::scan(&search_dirs);

    registry
        .list_workflows()
        .iter()
        .map(|def| WorkflowOption {
            name: def.config.name.clone(),
            description: def
                .config
                .description
                .clone()
                .unwrap_or_default(),
        })
        .collect()
}

fn check_github_token() -> (bool, String) {
    // Check GITHUB_TOKEN env var first
    if let Ok(token) = std::env::var("GITHUB_TOKEN") {
        if !token.is_empty() {
            return (true, "env:GITHUB_TOKEN".to_string());
        }
    }

    // Check GH_TOKEN env var (GitHub CLI's preferred env var)
    if let Ok(token) = std::env::var("GH_TOKEN") {
        if !token.is_empty() {
            return (true, "env:GH_TOKEN".to_string());
        }
    }

    // Check gh CLI auth status
    if check_gh_cli_auth() {
        return (true, "gh-cli".to_string());
    }

    (false, "not-found".to_string())
}

fn check_anthropic_key() -> (bool, String) {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            return (true, "env:ANTHROPIC_API_KEY".to_string());
        }
    }

    (false, "not-found".to_string())
}

fn check_gh_cli_auth() -> bool {
    std::process::Command::new("gh")
        .args(["auth", "status"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::project_config::{Severity, TokenSource};
    use tempfile::TempDir;

    #[test]
    fn setup_status_unconfigured() {
        let tmp = TempDir::new().unwrap();
        let repo_path = tmp.path().to_str().unwrap();

        let status = get_setup_status(repo_path).unwrap();
        assert!(!status.configured);
        assert!(status.config.is_none());
    }

    #[test]
    fn save_and_load_round_trip() {
        let tmp = TempDir::new().unwrap();
        let repo_path = tmp.path().to_str().unwrap().to_string();

        let config = ProjectConfig {
            repo_path: repo_path.clone(),
            github_token_source: TokenSource::EnvVar {
                name: "GITHUB_TOKEN".to_string(),
            },
            anthropic_key_source: TokenSource::EnvVar {
                name: "ANTHROPIC_API_KEY".to_string(),
            },
            enabled_workflows: vec!["pr-shepherd".to_string()],
            min_severity: Severity::High,
            confidence_threshold: 70,
        };

        save_project_config(&config).unwrap();

        let loaded = load_project_config(&repo_path).unwrap();
        assert_eq!(loaded.repo_path, repo_path);
        assert_eq!(loaded.enabled_workflows, vec!["pr-shepherd"]);
        assert_eq!(loaded.min_severity, Severity::High);
        assert_eq!(loaded.confidence_threshold, 70);
    }

    #[test]
    fn setup_status_after_save() {
        let tmp = TempDir::new().unwrap();
        let repo_path = tmp.path().to_str().unwrap().to_string();

        let config = ProjectConfig {
            repo_path: repo_path.clone(),
            ..ProjectConfig::default()
        };
        save_project_config(&config).unwrap();

        let status = get_setup_status(&repo_path).unwrap();
        assert!(status.configured);
        assert!(status.config.is_some());
    }
}
