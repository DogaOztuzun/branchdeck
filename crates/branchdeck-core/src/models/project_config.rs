use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Project-specific configuration stored in `.branchdeck/config.yaml`.
/// Created by the guided setup flow and persisted per-project.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProjectConfig {
    /// Absolute path to the repository root.
    pub repo_path: String,
    /// How the GitHub token is sourced.
    pub github_token_source: TokenSource,
    /// How the Anthropic API key is sourced.
    pub anthropic_key_source: TokenSource,
    /// Names of workflows enabled for this project.
    #[serde(default)]
    pub enabled_workflows: Vec<String>,
    /// Minimum severity for SAT-generated tasks.
    #[serde(default = "default_min_severity")]
    pub min_severity: Severity,
    /// SAT confidence threshold (0-100). Findings below this are ignored.
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: u8,
    /// Maximum number of concurrent workflow runs. Default 1.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: u32,
}

/// How a secret token is sourced at runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", tag = "type")]
pub enum TokenSource {
    /// Read from an environment variable.
    EnvVar { name: String },
    /// Obtained via the `gh` CLI authentication.
    GhCli,
    /// Not configured.
    None,
}

/// Severity level for SAT findings.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

fn default_min_severity() -> Severity {
    Severity::High
}

fn default_confidence_threshold() -> u8 {
    70
}

fn default_max_concurrent() -> u32 {
    1
}

impl ProjectConfig {
    /// Validate semantic constraints that serde cannot enforce.
    ///
    /// # Errors
    /// Returns `AppError::Config` if `repo_path` is not absolute or
    /// `confidence_threshold` exceeds 100.
    pub fn validate(&self) -> Result<(), AppError> {
        if !Path::new(&self.repo_path).is_absolute() {
            return Err(AppError::Config("repo_path must be absolute".to_string()));
        }
        if self.confidence_threshold > 100 {
            return Err(AppError::Config(
                "confidence_threshold must be 0-100".to_string(),
            ));
        }
        Ok(())
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            repo_path: String::new(),
            github_token_source: TokenSource::None,
            anthropic_key_source: TokenSource::None,
            enabled_workflows: Vec::new(),
            min_severity: default_min_severity(),
            confidence_threshold: default_confidence_threshold(),
            max_concurrent: default_max_concurrent(),
        }
    }
}

/// Result of validating token availability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TokenValidation {
    pub github_available: bool,
    pub github_source: String,
    pub anthropic_available: bool,
    pub anthropic_source: String,
}

/// Summary of available workflows for the setup flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WorkflowOption {
    pub name: String,
    pub description: String,
}

/// Setup status: whether a project has been configured.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SetupStatus {
    pub configured: bool,
    pub config_path: String,
    #[serde(default)]
    pub config: Option<ProjectConfig>,
}
