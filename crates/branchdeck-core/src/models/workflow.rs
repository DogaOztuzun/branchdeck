use serde::{Deserialize, Serialize};

/// Top-level workflow definition. External contract for workflow authors.
/// Deserialized from YAML files in `.branchdeck/workflows/` or global config.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowDef {
    pub schema_version: u32,
    pub name: String,
    pub description: String,
    pub trigger: TriggerDef,
    pub context: ContextDef,
    pub execution: ExecutionDef,
    pub outcomes: Vec<OutcomeDef>,
    #[serde(default)]
    pub lifecycle: Option<LifecycleDef>,
    #[serde(default)]
    pub retry: Option<RetryDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TriggerDef {
    #[serde(rename = "type")]
    pub trigger_type: TriggerType,
    #[serde(default)]
    pub filter: Option<TriggerFilter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TriggerType {
    GithubIssue,
    GithubPr,
    Manual,
    PostMerge,
    Schedule,
    Webhook,
}

impl TriggerType {
    pub const ALL: &[TriggerType] = &[
        Self::GithubIssue,
        Self::GithubPr,
        Self::Manual,
        Self::PostMerge,
        Self::Schedule,
        Self::Webhook,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GithubIssue => "github-issue",
            Self::GithubPr => "github-pr",
            Self::Manual => "manual",
            Self::PostMerge => "post-merge",
            Self::Schedule => "schedule",
            Self::Webhook => "webhook",
        }
    }
}

impl std::fmt::Display for TriggerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Type-specific filter fields for trigger matching.
/// Stored as a flat string map — schema depends on trigger type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TriggerFilter(pub std::collections::HashMap<String, serde_yaml::Value>);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ContextDef {
    pub template: String,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExecutionDef {
    pub skill: String,
    #[serde(default)]
    pub max_turns: Option<u32>,
    #[serde(default)]
    pub max_budget_usd: Option<f64>,
    #[serde(default)]
    pub timeout_minutes: Option<u32>,
    #[serde(default)]
    pub allowed_directories: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OutcomeDef {
    pub name: String,
    pub detect: OutcomeDetector,
    #[serde(default)]
    pub path: Option<String>,
    pub next: OutcomeAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutcomeDetector {
    FileExists,
    PrCreated,
    CiPassing,
    RunFailed,
    Custom,
}

impl OutcomeDetector {
    pub const ALL: &[OutcomeDetector] = &[
        Self::FileExists,
        Self::PrCreated,
        Self::CiPassing,
        Self::RunFailed,
        Self::Custom,
    ];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FileExists => "file-exists",
            Self::PrCreated => "pr-created",
            Self::CiPassing => "ci-passing",
            Self::RunFailed => "run-failed",
            Self::Custom => "custom",
        }
    }
}

impl std::fmt::Display for OutcomeDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutcomeAction {
    Complete,
    Retry,
    Review,
    CustomState,
}

impl OutcomeAction {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Retry => "retry",
            Self::Review => "review",
            Self::CustomState => "custom-state",
        }
    }
}

impl std::fmt::Display for OutcomeAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct LifecycleDef {
    #[serde(default)]
    pub dispatched: Option<String>,
    #[serde(default)]
    pub complete: Option<String>,
    #[serde(default)]
    pub failed: Option<String>,
    #[serde(default)]
    pub retrying: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RetryDef {
    pub max_attempts: u32,
    pub backoff: BackoffStrategy,
    pub base_delay_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackoffStrategy {
    Exponential,
    Fixed,
}

impl BackoffStrategy {
    pub const ALL: &[BackoffStrategy] = &[Self::Exponential, Self::Fixed];

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exponential => "exponential",
            Self::Fixed => "fixed",
        }
    }
}

impl std::fmt::Display for BackoffStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
