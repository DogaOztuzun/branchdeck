use std::collections::HashMap;
use std::path::{Path, PathBuf};

use log::{debug, error, warn};
use yaml_front_matter::YamlFrontMatter;

use crate::error::AppError;
use crate::models::workflow::{
    OutcomeDetector, TriggerContext, TriggerEvent, WorkflowConfig, WorkflowDef,
};

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
    // Pre-check: content must start with `---` frontmatter delimiter
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(AppError::Workflow(
            "workflow file must start with YAML frontmatter (---). See docs/specs/workflow-schema.md".into(),
        ));
    }
    // Check for closing delimiter
    if trimmed.match_indices("---").count() < 2 {
        return Err(AppError::Workflow(
            "workflow file has unclosed frontmatter — missing closing ---".into(),
        ));
    }

    let document: yaml_front_matter::Document<WorkflowConfig> = YamlFrontMatter::parse(content)
        .map_err(|e| {
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
        debug!(
            "Workflow definition '{}' validated successfully",
            config.name
        );
    }

    errors
}

/// Registry of discovered workflow definitions.
///
/// Scans ordered directory tiers for `*/WORKFLOW.md` files, parses and validates
/// each, and applies override precedence (later directories override earlier ones
/// by workflow `name` field).
/// Build the default search directories for workflow discovery.
///
/// Order (later overrides earlier):
/// 1. Global: `~/.config/branchdeck/workflows/`
/// 2. Project-local: `{repo_path}/.branchdeck/workflows/`
#[must_use]
pub fn default_search_dirs(repo_path: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Global config dir
    if let Some(config_dir) = dirs::config_dir() {
        dirs.push(config_dir.join("branchdeck").join("workflows"));
    }

    // Project-local
    dirs.push(PathBuf::from(repo_path).join(".branchdeck").join("workflows"));

    dirs
}

#[derive(Debug, Clone)]
pub struct WorkflowRegistry {
    workflows: HashMap<String, WorkflowDef>,
}

impl WorkflowRegistry {
    /// Scan directories for workflow definitions.
    ///
    /// Directories are processed in order — later directories override earlier ones
    /// when workflow `name` fields collide. Each directory is expected to contain
    /// subdirectories with a `WORKFLOW.md` file inside.
    ///
    /// Invalid definitions are logged as warnings and skipped.
    /// Missing directories are logged at debug level and skipped.
    #[must_use]
    pub fn scan(search_dirs: &[PathBuf]) -> Self {
        let mut workflows = HashMap::new();

        for dir in search_dirs {
            if !dir.is_dir() {
                debug!("Workflow search dir does not exist, skipping: {}", dir.display());
                continue;
            }

            let entries = match std::fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(e) => {
                    warn!("Failed to read workflow directory {}: {e}", dir.display());
                    continue;
                }
            };

            for entry in entries.filter_map(Result::ok) {
                let subdir = entry.path();
                if !subdir.is_dir() {
                    continue;
                }

                let workflow_file = subdir.join("WORKFLOW.md");
                if !workflow_file.is_file() {
                    continue;
                }

                match parse_workflow_file(&workflow_file) {
                    Ok(def) => {
                        let errors = validate_workflow_def(&def);
                        if errors.is_empty() {
                            let name = def.config.name.clone();
                            if workflows.contains_key(&name) {
                                debug!(
                                    "Workflow '{}' overridden by {}",
                                    name,
                                    workflow_file.display()
                                );
                            }
                            debug!("Loaded workflow '{}' from {}", name, workflow_file.display());
                            workflows.insert(name, def);
                        } else {
                            let error_msgs: Vec<String> =
                                errors.iter().map(ToString::to_string).collect();
                            warn!(
                                "Invalid workflow at {}: {}",
                                workflow_file.display(),
                                error_msgs.join("; ")
                            );
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Failed to parse workflow at {}: {e}",
                            workflow_file.display()
                        );
                    }
                }
            }
        }

        debug!("WorkflowRegistry loaded {} workflow(s)", workflows.len());
        Self { workflows }
    }

    /// Return all loaded workflow definitions.
    #[must_use]
    pub fn list_workflows(&self) -> Vec<&WorkflowDef> {
        self.workflows.values().collect()
    }

    /// Look up a workflow by name.
    #[must_use]
    pub fn get_workflow(&self, name: &str) -> Option<&WorkflowDef> {
        self.workflows.get(name)
    }

    /// Number of loaded workflows.
    #[must_use]
    pub fn len(&self) -> usize {
        self.workflows.len()
    }

    /// Whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.workflows.is_empty()
    }

    /// Find all workflows whose trigger type matches the event, applying filter criteria.
    ///
    /// Returns workflows in no guaranteed order. If multiple workflows match,
    /// all are returned (caller decides how to handle — typically first match wins).
    #[must_use]
    pub fn match_workflows(&self, event: &TriggerEvent) -> Vec<&WorkflowDef> {
        self.workflows
            .values()
            .filter(|def| matches_trigger(def, event))
            .collect()
    }
}

// === Trigger matching logic ===

/// Check if a workflow definition's trigger matches an incoming event.
/// Compares tracker kind and applies filter criteria from the workflow def.
fn matches_trigger(def: &WorkflowDef, event: &TriggerEvent) -> bool {
    // Kind must match
    if def.config.tracker.kind != event.kind {
        return false;
    }

    // For manual triggers, match by workflow name
    if let TriggerContext::Manual { workflow_name, .. } = &event.context {
        return def.config.name == *workflow_name;
    }

    // Apply filter criteria if defined
    let Some(filter) = &def.config.tracker.filter else {
        return true; // No filter = match all events of this kind
    };

    match &event.context {
        TriggerContext::GithubIssue { labels, .. } => {
            if let Some(label_filter) = filter.get("label") {
                let Some(label_str) = label_filter.as_str() else {
                    warn!("Filter 'label' has non-string value, rejecting match");
                    return false;
                };
                return labels.iter().any(|l| l == label_str);
            }
            true
        }
        TriggerContext::GithubPr {
            ci_status,
            review_decision,
            ..
        } => {
            if let Some(ci_filter) = filter.get("ci_status") {
                let Some(ci_str) = ci_filter.as_str() else {
                    warn!("Filter 'ci_status' has non-string value, rejecting match");
                    return false;
                };
                if ci_status.as_deref() != Some(ci_str) {
                    return false;
                }
            }
            if let Some(review_filter) = filter.get("review_decision") {
                let Some(review_str) = review_filter.as_str() else {
                    warn!("Filter 'review_decision' has non-string value, rejecting match");
                    return false;
                };
                if review_decision.as_deref() != Some(review_str) {
                    return false;
                }
            }
            true
        }
        TriggerContext::PostMerge { .. } | TriggerContext::Manual { .. } => true,
    }
}
