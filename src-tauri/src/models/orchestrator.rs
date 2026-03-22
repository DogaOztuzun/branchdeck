use serde::{Deserialize, Serialize};

use super::agent::EpochMs;
use super::github::PrSummary;

// --- IPC-facing types (sent to frontend) → camelCase ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningEntry {
    pub pr_key: String,
    pub worktree_path: String,
    pub tab_id: String,
    pub started_at: EpochMs,
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetryEntry {
    pub pr_key: String,
    pub attempt: u32,
    pub due_at_ms: EpochMs,
    pub error: Option<String>,
    pub worktree_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum LifecycleStatus {
    Running,
    ReviewReady,
    Approved,
    Fixing,
    Completed,
    Retrying,
    Stale,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEvent {
    pub pr_key: String,
    pub worktree_path: String,
    pub status: LifecycleStatus,
    pub attempt: u32,
    pub started_at: EpochMs,
}

// --- Agent-facing types (written to JSON files) → snake_case ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PrContext {
    pub repo: String,
    pub number: u64,
    pub branch: String,
    #[serde(default)]
    pub base_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlanStep {
    pub description: String,
    pub file: String,
    pub change_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FailureInfo {
    pub check_name: String,
    pub error_summary: String,
    pub root_cause: String,
    pub fix_approach: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReviewInfo {
    pub reviewer: String,
    pub comment: String,
    pub proposed_response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ApprovedPlan {
    pub plan_steps: Vec<PlanStep>,
    pub affected_files: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AnalysisPlan {
    pub pr: PrContext,
    pub confidence: String,
    pub failures: Vec<FailureInfo>,
    pub reviews: Vec<ReviewInfo>,
    pub plan_steps: Vec<PlanStep>,
    pub affected_files: Vec<String>,
    pub reasoning: String,
    #[serde(default)]
    pub approved: bool,
    #[serde(default)]
    pub approved_plan: Option<ApprovedPlan>,
    #[serde(default)]
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewReadyEntry {
    pub pr_key: String,
    pub worktree_path: String,
    pub attempt: u32,
    pub started_at: EpochMs,
    pub stale: bool,
}

// --- Internal types (orchestrator state machine) ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionOutcome {
    AnalysisWritten,
    FixCompleted,
    FixIncomplete,
    NoOutput,
}

#[derive(Debug, Clone)]
pub enum OrchestratorEffect {
    DispatchSession {
        pr_key: String,
        worktree_path: String,
        pr_context: PrContext,
        attempt: u32,
    },
    StopSession {
        pr_key: String,
        tab_id: String,
        worktree_path: String,
    },
    ScheduleRetry {
        pr_key: String,
        worktree_path: String,
        attempt: u32,
        delay_ms: u64,
        error: Option<String>,
    },
    CancelRetry {
        pr_key: String,
    },
    EmitLifecycleEvent {
        event: LifecycleEvent,
    },
    CleanupMetadata {
        worktree_path: String,
    },
}

// --- Orchestrator config (internal only, no serde for Phase A) ---

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub enabled: bool,
    pub max_concurrent: u32,
    pub auto_analyze: bool,
    pub auto_fix: bool,
    pub filter_authors: Vec<String>,
    pub filter_branches_exclude: Vec<String>,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_concurrent: 1,
            auto_analyze: true,
            auto_fix: false,
            filter_authors: Vec::new(),
            filter_branches_exclude: vec!["main".to_string(), "master".to_string()],
        }
    }
}

// --- Orchestrator state ---

pub struct Orchestrator {
    pub config: OrchestratorConfig,
    pub running: std::collections::HashMap<String, RunningEntry>,
    pub claimed: std::collections::HashSet<String>,
    pub retry_queue: std::collections::HashMap<String, RetryEntry>,
    pub completed: std::collections::HashSet<String>,
    /// PRs awaiting human review (analysis written, not yet approved)
    pub review_ready: std::collections::HashMap<String, ReviewReadyEntry>,
    /// Maps "owner/repo" → filesystem path (e.g. "/home/user/projects/repo")
    pub repo_paths: std::collections::HashMap<String, String>,
    /// Active retry timer handles — abort on cancel to prevent stale `RetryDue` events
    pub retry_timers: std::collections::HashMap<String, tokio::task::JoinHandle<()>>,
}

impl std::fmt::Debug for Orchestrator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Orchestrator")
            .field("config", &self.config)
            .field("running", &self.running.len())
            .field("claimed", &self.claimed.len())
            .field("retry_queue", &self.retry_queue.len())
            .field("completed", &self.completed.len())
            .field("review_ready", &self.review_ready.len())
            .field("repo_paths", &self.repo_paths.len())
            .finish_non_exhaustive()
    }
}

impl Orchestrator {
    #[must_use]
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            config,
            running: std::collections::HashMap::new(),
            claimed: std::collections::HashSet::new(),
            retry_queue: std::collections::HashMap::new(),
            completed: std::collections::HashSet::new(),
            review_ready: std::collections::HashMap::new(),
            repo_paths: std::collections::HashMap::new(),
            retry_timers: std::collections::HashMap::new(),
        }
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new(OrchestratorConfig::default())
    }
}

/// Construct a canonical PR key from repo name and PR number.
#[must_use]
pub fn pr_key(repo: &str, number: u64) -> String {
    format!("{repo}#{number}")
}

/// Check if a PR is eligible for orchestration based on config filters.
#[must_use]
pub fn is_pr_eligible(pr: &PrSummary, config: &OrchestratorConfig) -> bool {
    // Skip excluded branches
    if config
        .filter_branches_exclude
        .iter()
        .any(|b| b == &pr.branch)
    {
        return false;
    }

    // Skip non-matching authors if filter is set
    if !config.filter_authors.is_empty() && !config.filter_authors.iter().any(|a| a == &pr.author) {
        return false;
    }

    // Only consider PRs with failing CI
    matches!(pr.ci_status.as_deref(), Some("FAILURE" | "ERROR"))
}
