use std::path::Path;

use log::{debug, error};
use yaml_front_matter::YamlFrontMatter;

use crate::error::AppError;
use crate::models::workflow::{OutcomeDetector, WorkflowConfig, WorkflowDef};

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// Parse a WORKFLOW.md file from disk (YAML frontmatter + markdown body).
///
/// # Errors
/// Returns `AppError::Workflow` if the file cannot be read or contains invalid frontmatter.
pub fn parse_workflow_file(path: &Path) -> Result<WorkflowDef, AppError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        error!("Failed to read workflow file {}: {e}", path.display());
        AppError::Workflow(format!("failed to read {}: {e}", path.display()))
    })?;
    parse_workflow_md(&content)
}

/// Parse a markdown string with YAML frontmatter into a `WorkflowDef`.
///
/// # Errors
/// Returns `AppError::Workflow` if the frontmatter is malformed or missing.
pub fn parse_workflow_md(content: &str) -> Result<WorkflowDef, AppError> {
    let document: yaml_front_matter::Document<WorkflowConfig> =
        YamlFrontMatter::parse(content).map_err(|e| {
            error!("Failed to parse workflow frontmatter: {e}");
            AppError::Workflow(format!("frontmatter parse error: {e}"))
        })?;

    Ok(WorkflowDef {
        config: document.metadata,
        prompt: document.content,
    })
}

/// Validate a parsed `WorkflowDef` and return all errors found.
/// An empty vec means the definition is valid.
#[must_use]
pub fn validate_workflow_def(def: &WorkflowDef) -> Vec<ValidationError> {
    let mut errors = Vec::new();
    let config = &def.config;

    if config.name.trim().is_empty() {
        errors.push(ValidationError {
            field: "name".into(),
            message: "must not be empty or whitespace-only".into(),
        });
    }

    if let Some(desc) = &config.description {
        if desc.trim().is_empty() {
            errors.push(ValidationError {
                field: "description".into(),
                message: "must not be empty or whitespace-only if provided".into(),
            });
        }
    }

    // Agent validation
    if let Some(agent) = &config.agent {
        if let Some(budget) = agent.max_budget_usd {
            if budget.is_nan() || budget.is_infinite() || budget < 0.0 {
                errors.push(ValidationError {
                    field: "agent.max_budget_usd".into(),
                    message: "must be a finite non-negative number".into(),
                });
            }
        }
    }

    // Outcomes validation
    if config.outcomes.is_empty() && def.prompt.trim().is_empty() {
        errors.push(ValidationError {
            field: "outcomes".into(),
            message: "must have at least one outcome, or provide a prompt body".into(),
        });
    }

    for (i, outcome) in config.outcomes.iter().enumerate() {
        if outcome.name.trim().is_empty() {
            errors.push(ValidationError {
                field: format!("outcomes[{i}].name"),
                message: "must not be empty".into(),
            });
        }
        if outcome.detect == OutcomeDetector::FileExists && outcome.path.is_none() {
            errors.push(ValidationError {
                field: format!("outcomes[{i}].path"),
                message: "required when detect is 'file-exists'".into(),
            });
        }
    }

    // Retry validation
    if let Some(retry) = &config.retry {
        if retry.max_attempts == 0 {
            errors.push(ValidationError {
                field: "retry.max_attempts".into(),
                message: "must be at least 1".into(),
            });
        }
        if retry.base_delay_ms == 0 {
            errors.push(ValidationError {
                field: "retry.base_delay_ms".into(),
                message: "must be at least 1".into(),
            });
        }
    }

    if errors.is_empty() {
        debug!("Workflow definition '{}' validated successfully", config.name);
    }

    errors
}
