use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A parsed workflow definition: YAML frontmatter + markdown prompt body.
/// Loaded from `WORKFLOW.md` files in `.branchdeck/workflows/` or global config.
#[derive(Debug, Clone)]
pub struct WorkflowDef {
    pub config: WorkflowConfig,
    pub prompt: String,
}

/// YAML frontmatter of a workflow definition.
/// Symphony-compatible base fields + Branchdeck extensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkflowConfig {
    // === Branchdeck identity ===
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,

    // === Symphony-compatible base ===
    pub tracker: TrackerDef,
    #[serde(default)]
    pub polling: Option<PollingDef>,
    #[serde(default)]
    pub workspace: Option<WorkspaceDef>,
    #[serde(default)]
    pub hooks: Option<HooksDef>,
    #[serde(default)]
    pub agent: Option<AgentDef>,

    // === Branchdeck extensions ===
    #[serde(default)]
    pub outcomes: Vec<OutcomeDef>,
    #[serde(default)]
    pub lifecycle: Option<LifecycleDef>,
    #[serde(default)]
    pub retry: Option<RetryDef>,
}

// === Tracker (Symphony: tracker) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TrackerDef {
    pub kind: TrackerKind,
    #[serde(default)]
    pub filter: Option<HashMap<String, serde_json::Value>>,
    // Symphony Linear-specific fields (optional, for compatibility)
    #[serde(default)]
    pub project_slug: Option<String>,
    #[serde(default)]
    pub active_states: Option<Vec<String>>,
    #[serde(default)]
    pub terminal_states: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrackerKind {
    GithubIssue,
    GithubPr,
    Linear,
    Manual,
    PostMerge,
    Schedule,
    Webhook,
}

impl TrackerKind {
    pub const ALL: &[TrackerKind] = &[
        Self::GithubIssue,
        Self::GithubPr,
        Self::Linear,
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
            Self::Linear => "linear",
            Self::Manual => "manual",
            Self::PostMerge => "post-merge",
            Self::Schedule => "schedule",
            Self::Webhook => "webhook",
        }
    }
}

impl std::fmt::Display for TrackerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// === Polling (Symphony: polling) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PollingDef {
    #[serde(default = "default_polling_interval")]
    pub interval_ms: u64,
}

fn default_polling_interval() -> u64 {
    30_000
}

// === Workspace (Symphony: workspace) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorkspaceDef {
    #[serde(default)]
    pub root: Option<String>,
}

// === Hooks (Symphony: hooks) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HooksDef {
    #[serde(default)]
    pub after_create: Option<String>,
    #[serde(default)]
    pub before_run: Option<String>,
    #[serde(default)]
    pub after_run: Option<String>,
    #[serde(default)]
    pub before_remove: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

// === Agent (Symphony: agent + our extensions) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgentDef {
    #[serde(default)]
    pub max_concurrent_agents: Option<u32>,
    #[serde(default)]
    pub max_turns: Option<u32>,
    #[serde(default)]
    pub max_budget_usd: Option<f64>,
    #[serde(default)]
    pub timeout_minutes: Option<u32>,
    #[serde(default)]
    pub allowed_directories: Option<Vec<String>>,
}

// === Outcomes (Branchdeck extension) ===

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
    pub const ALL: &[OutcomeAction] =
        &[Self::Complete, Self::Retry, Self::Review, Self::CustomState];

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

// === Lifecycle (Branchdeck extension) ===

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

// === Retry (Branchdeck extension) ===

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

// === Trigger Events (incoming events that match to workflows) ===

/// An incoming event that may trigger a workflow.
/// Carries the trigger type and context data for filter matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TriggerEvent {
    pub kind: TrackerKind,
    pub context: TriggerContext,
}

/// Context data carried by a trigger event.
/// Each variant corresponds to a `TrackerKind`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum TriggerContext {
    GithubIssue {
        repo: String,
        number: u64,
        title: String,
        labels: Vec<String>,
    },
    GithubPr {
        repo: String,
        number: u64,
        branch: String,
        base_branch: String,
        ci_status: Option<String>,
        review_decision: Option<String>,
    },
    Manual {
        workflow_name: String,
        #[serde(default)]
        params: HashMap<String, serde_json::Value>,
    },
    PostMerge {
        repo: String,
        pr_number: u64,
        branch: String,
    },
}

// === Dispatch types (effects produced by trigger matching + dispatch) ===

/// Effects produced by workflow dispatch (pure function output).
/// Executed by the imperative shell (`RunManager`, filesystem, events).
#[derive(Debug, Clone)]
pub enum DispatchEffect {
    CreateWorktree {
        repo_path: String,
        branch: String,
        worktree_path: String,
    },
    WriteContext {
        worktree_path: String,
        context_file: String,
        content: String,
    },
    DeploySkill {
        worktree_path: String,
        skill_content: String,
    },
    EnqueueRun {
        worktree_path: String,
        task_path: String,
        max_budget_usd: Option<f64>,
        allowed_directories: Vec<String>,
    },
    EmitWorkflowEvent {
        workflow_name: String,
        status: String,
        detail: String,
    },
    LogNoMatch {
        event_kind: TrackerKind,
        detail: String,
    },
}

/// Result of matching a trigger event to a workflow and preparing dispatch.
#[derive(Debug, Clone)]
pub struct DispatchPlan {
    pub workflow_name: String,
    pub effects: Vec<DispatchEffect>,
}
