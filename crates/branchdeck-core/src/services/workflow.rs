use std::path::Path;

use log::{error, info};

use crate::error::AppError;
use crate::models::workflow::WorkflowDef;

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

/// Parse a YAML workflow definition file from disk.
///
/// # Errors
/// Returns `AppError::Workflow` if the file cannot be read or contains invalid YAML.
pub fn parse_workflow_file(path: &Path) -> Result<WorkflowDef, AppError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        error!("Failed to read workflow file {}: {e}", path.display());
        AppError::Workflow(format!("failed to read {}: {e}", path.display()))
    })?;
    parse_workflow_yaml(&content)
}

/// Parse a YAML string into a `WorkflowDef`.
///
/// # Errors
/// Returns `AppError::Workflow` if the YAML is malformed or doesn't match the schema.
pub fn parse_workflow_yaml(yaml: &str) -> Result<WorkflowDef, AppError> {
    serde_yaml::from_str(yaml).map_err(|e| {
        error!("Failed to parse workflow YAML: {e}");
        AppError::Workflow(format!("YAML parse error: {e}"))
    })
}

/// Validate a parsed `WorkflowDef` and return all errors found.
/// An empty vec means the definition is valid.
#[must_use]
pub fn validate_workflow_def(def: &WorkflowDef) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if def.schema_version != 1 {
        errors.push(ValidationError {
            field: "schema_version".into(),
            message: format!(
                "unsupported schema version '{}'. Valid: 1",
                def.schema_version
            ),
        });
    }

    if def.name.is_empty() {
        errors.push(ValidationError {
            field: "name".into(),
            message: "must not be empty".into(),
        });
    }

    if def.description.is_empty() {
        errors.push(ValidationError {
            field: "description".into(),
            message: "must not be empty".into(),
        });
    }

    if def.context.template.is_empty() {
        errors.push(ValidationError {
            field: "context.template".into(),
            message: "must not be empty".into(),
        });
    }

    if def.context.output.is_empty() {
        errors.push(ValidationError {
            field: "context.output".into(),
            message: "must not be empty".into(),
        });
    }

    if def.execution.skill.is_empty() {
        errors.push(ValidationError {
            field: "execution.skill".into(),
            message: "must not be empty".into(),
        });
    }

    if def.outcomes.is_empty() {
        errors.push(ValidationError {
            field: "outcomes".into(),
            message: "must have at least one outcome".into(),
        });
    }

    for (i, outcome) in def.outcomes.iter().enumerate() {
        if outcome.name.is_empty() {
            errors.push(ValidationError {
                field: format!("outcomes[{i}].name"),
                message: "must not be empty".into(),
            });
        }
    }

    if let Some(retry) = &def.retry {
        if retry.max_attempts == 0 {
            errors.push(ValidationError {
                field: "retry.max_attempts".into(),
                message: "must be at least 1".into(),
            });
        }
        if retry.base_delay_ms == 0 {
            errors.push(ValidationError {
                field: "retry.base_delay_ms".into(),
                message: "must be greater than 0".into(),
            });
        }
    }

    if errors.is_empty() {
        info!("Workflow definition '{}' validated successfully", def.name);
    }

    errors
}
