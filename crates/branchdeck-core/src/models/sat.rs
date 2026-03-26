use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A parsed SAT persona definition loaded from YAML files in `sat/personas/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatPersona {
    pub name: String,
    pub description: String,
    pub frustration_threshold: FrustrationThreshold,
    pub technical_level: TechnicalLevel,
    #[serde(default)]
    pub satisfaction_criteria: Vec<String>,
    #[serde(default)]
    pub behaviors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrustrationThreshold {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for FrustrationThreshold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => f.write_str("low"),
            Self::Medium => f.write_str("medium"),
            Self::High => f.write_str("high"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TechnicalLevel {
    None,
    Beginner,
    Intermediate,
    Expert,
}

impl std::fmt::Display for TechnicalLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Beginner => f.write_str("beginner"),
            Self::Intermediate => f.write_str("intermediate"),
            Self::Expert => f.write_str("expert"),
        }
    }
}

/// YAML frontmatter of a SAT scenario markdown file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatScenarioMeta {
    pub id: String,
    pub title: String,
    pub persona: String,
    #[serde(default = "default_priority")]
    pub priority: ScenarioPriority,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub generated_from: Option<String>,
}

fn default_priority() -> ScenarioPriority {
    ScenarioPriority::Medium
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ScenarioPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => f.write_str("low"),
            Self::Medium => f.write_str("medium"),
            Self::High => f.write_str("high"),
            Self::Critical => f.write_str("critical"),
        }
    }
}

/// A fully parsed SAT scenario: frontmatter metadata + markdown body sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatScenario {
    pub meta: SatScenarioMeta,
    pub context: String,
    pub steps: Vec<String>,
    pub expected_satisfaction: Vec<String>,
    #[serde(default)]
    pub edge_cases: Vec<String>,
}

/// Machine-readable manifest produced alongside generated scenario files.
/// Written to `sat/scenarios/manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatManifest {
    pub generated_at: String,
    pub persona_count: usize,
    pub scenario_count: usize,
    pub personas: Vec<SatManifestPersona>,
    pub scenarios: Vec<SatManifestEntry>,
}

/// Summary of a persona in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatManifestPersona {
    pub name: String,
    pub file: String,
}

/// Summary of a generated scenario in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatManifestEntry {
    pub id: String,
    pub title: String,
    pub persona: String,
    pub priority: ScenarioPriority,
    pub file: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Configuration for the scenario generation service.
#[derive(Debug, Clone)]
pub struct SatGenerationConfig {
    /// Root directory of the project (where sat/ lives).
    pub project_root: std::path::PathBuf,
    /// Path to personas directory (default: `sat/personas/`).
    pub personas_dir: std::path::PathBuf,
    /// Path to scenarios output directory (default: `sat/scenarios/`).
    pub scenarios_dir: std::path::PathBuf,
    /// Paths to project doc files to read for scenario generation.
    pub doc_paths: Vec<std::path::PathBuf>,
}

impl SatGenerationConfig {
    /// Create a config with standard defaults for a project root.
    #[must_use]
    pub fn new(project_root: std::path::PathBuf) -> Self {
        let personas_dir = project_root.join("sat/personas");
        let scenarios_dir = project_root.join("sat/scenarios");

        // Default doc paths — checked at runtime for existence
        let doc_paths = vec![
            project_root.join("docs/prd.md"),
            project_root.join("docs/PRD.md"),
            project_root.join("README.md"),
            project_root.join("docs/mvp-brief.md"),
        ];

        Self {
            project_root,
            personas_dir,
            scenarios_dir,
            doc_paths,
        }
    }
}

// ===========================================================================
// Execution types (Story 3.2)
// ===========================================================================

/// Configuration for a SAT execution run.
#[derive(Debug, Clone)]
pub struct SatRunConfig {
    /// Root directory of the project (where `sat/` lives).
    pub project_root: PathBuf,
    /// Path to scenarios directory (default: `sat/scenarios/`).
    pub scenarios_dir: PathBuf,
    /// Path to runs output directory (default: `sat/runs/`).
    pub runs_dir: PathBuf,
    /// Path to the `tauri-driver` binary.
    pub tauri_driver_path: PathBuf,
    /// Path to the built application binary.
    pub app_binary_path: PathBuf,
    /// `WebDriver` host (default: `127.0.0.1`).
    pub webdriver_host: String,
    /// `WebDriver` port (default: `4444`).
    pub webdriver_port: u16,
    /// Maximum consecutive step failures before aborting a scenario.
    pub max_consecutive_failures: u32,
    /// Per-step timeout in milliseconds.
    pub step_timeout_ms: u64,
}

impl SatRunConfig {
    /// Create a config with standard defaults for a project root.
    #[must_use]
    pub fn new(project_root: PathBuf) -> Self {
        let scenarios_dir = project_root.join("sat/scenarios");
        let runs_dir = project_root.join("sat/runs");

        // Standard tauri-driver location (installed via `cargo install tauri-driver`)
        let tauri_driver_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".cargo/bin/tauri-driver");

        // Debug binary — production builds would override this
        let app_binary_path = project_root.join("src-tauri/target/debug/branchdeck");

        Self {
            project_root,
            scenarios_dir,
            runs_dir,
            tauri_driver_path,
            app_binary_path,
            webdriver_host: "127.0.0.1".to_string(),
            webdriver_port: 4444,
            max_consecutive_failures: 3,
            step_timeout_ms: 30_000,
        }
    }
}

/// Category for classifying execution failures.
/// `Runner` failures are infrastructure issues (`WebDriver`, `tauri-driver`),
/// not application bugs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCategory {
    /// Application-level failure (bug in the app under test).
    App,
    /// Runner/infrastructure failure (`WebDriver`, `tauri-driver`, connectivity).
    Runner,
    /// Step interpretation failure (could not translate NL step to action).
    Interpretation,
}

impl std::fmt::Display for FailureCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::App => f.write_str("app"),
            Self::Runner => f.write_str("runner"),
            Self::Interpretation => f.write_str("interpretation"),
        }
    }
}

/// Result of executing a single scenario step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatStepResult {
    /// 1-based step number.
    pub step_number: u32,
    /// Original step text from the scenario.
    pub step_text: String,
    /// Outcome of the step.
    pub status: StepStatus,
    /// Description of the action taken by the runner.
    pub action_taken: String,
    /// Relative path to screenshot captured before the step.
    #[serde(default)]
    pub before_screenshot: Option<String>,
    /// Relative path to screenshot captured after the step.
    #[serde(default)]
    pub after_screenshot: Option<String>,
    /// Summary of the page state after the step.
    #[serde(default)]
    pub page_summary: Option<String>,
    /// Reason for failure, if any.
    #[serde(default)]
    pub failure_reason: Option<String>,
    /// Failure category for classification.
    #[serde(default)]
    pub failure_category: Option<FailureCategory>,
    /// Step execution duration in milliseconds.
    pub duration_ms: u64,
    /// ISO 8601 timestamp when the step started.
    pub started_at: String,
}

/// Outcome of a single step execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    /// Step executed successfully.
    Pass,
    /// Step execution failed.
    Fail,
    /// Step was skipped (e.g., after scenario abort).
    Skip,
}

impl std::fmt::Display for StepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => f.write_str("pass"),
            Self::Fail => f.write_str("fail"),
            Self::Skip => f.write_str("skip"),
        }
    }
}

/// Trajectory data for a single scenario execution.
/// Written to `sat/runs/run-{timestamp}/trajectory-{scenario_id}.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatTrajectory {
    /// Scenario ID from the scenario metadata.
    pub scenario_id: String,
    /// Relative path to the scenario file.
    pub scenario_file: String,
    /// ISO 8601 timestamp when execution started.
    pub started_at: String,
    /// ISO 8601 timestamp when execution completed.
    pub completed_at: String,
    /// Overall execution status.
    pub status: TrajectoryStatus,
    /// Per-step results.
    pub steps: Vec<SatStepResult>,
    /// Performance metrics.
    pub performance: SatPerformance,
}

/// Overall status of a trajectory (scenario execution).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrajectoryStatus {
    /// All steps completed (some may have failed).
    Completed,
    /// Execution was aborted due to consecutive failures.
    Aborted,
    /// Runner infrastructure failure prevented execution.
    RunnerFailure,
}

impl std::fmt::Display for TrajectoryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Completed => f.write_str("completed"),
            Self::Aborted => f.write_str("aborted"),
            Self::RunnerFailure => f.write_str("runner_failure"),
        }
    }
}

/// Performance metrics for a trajectory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatPerformance {
    /// Total duration in milliseconds.
    pub total_duration_ms: u64,
    /// Per-step durations in milliseconds.
    pub step_durations_ms: Vec<u64>,
}

/// Result of a full SAT execution run (may contain multiple scenario trajectories).
/// Written to `sat/runs/run-{timestamp}/run-result.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatRunResult {
    /// Unique run identifier (timestamp-based).
    pub run_id: String,
    /// ISO 8601 timestamp when the run started.
    pub started_at: String,
    /// ISO 8601 timestamp when the run completed.
    pub completed_at: String,
    /// Path to the run output directory.
    pub run_dir: String,
    /// Total scenarios attempted.
    pub scenarios_total: usize,
    /// Scenarios that completed (all steps ran).
    pub scenarios_completed: usize,
    /// Scenarios that were aborted.
    pub scenarios_aborted: usize,
    /// Scenarios that failed due to runner issues.
    pub scenarios_runner_failed: usize,
    /// Per-scenario trajectories.
    pub trajectories: Vec<SatTrajectory>,
}

// ===========================================================================
// Scoring types (Story 3.3)
// ===========================================================================

/// Configuration for the SAT scoring service.
#[derive(Debug, Clone)]
pub struct SatScoreConfig {
    /// Root directory of the project (where `sat/` lives).
    pub project_root: PathBuf,
    /// Path to runs directory (default: `sat/runs/`).
    pub runs_dir: PathBuf,
    /// Path to personas directory (default: `sat/personas/`).
    pub personas_dir: PathBuf,
    /// Path to learnings file (default: `sat/learnings.yaml`).
    pub learnings_path: PathBuf,
    /// Budget constraints for LLM scoring.
    pub budget: ScoringBudget,
}

impl SatScoreConfig {
    /// Create a config with standard defaults for a project root.
    #[must_use]
    pub fn new(project_root: PathBuf) -> Self {
        let runs_dir = project_root.join("sat/runs");
        let personas_dir = project_root.join("sat/personas");
        let learnings_path = project_root.join("sat/learnings.yaml");
        Self {
            project_root,
            runs_dir,
            personas_dir,
            learnings_path,
            budget: ScoringBudget::default(),
        }
    }
}

/// Budget constraints for LLM-based scoring.
///
/// Tracks token usage and cost to enforce the $5-15 budget cap
/// for 10-20 scenarios (NFR6).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ScoringBudget {
    /// Maximum cost in dollars for the entire scoring run.
    pub max_cost_dollars: f64,
    /// Cost per 1K input tokens (model-specific).
    pub input_cost_per_1k: f64,
    /// Cost per 1K output tokens (model-specific).
    pub output_cost_per_1k: f64,
    /// Accumulated input tokens so far.
    pub input_tokens_used: u64,
    /// Accumulated output tokens so far.
    pub output_tokens_used: u64,
}

impl Default for ScoringBudget {
    fn default() -> Self {
        Self {
            max_cost_dollars: 15.0,
            // Claude Sonnet 3.5 pricing as reasonable defaults
            input_cost_per_1k: 0.003,
            output_cost_per_1k: 0.015,
            input_tokens_used: 0,
            output_tokens_used: 0,
        }
    }
}

/// Confidence level for a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for ConfidenceLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => f.write_str("high"),
            Self::Medium => f.write_str("medium"),
            Self::Low => f.write_str("low"),
        }
    }
}

/// Category for a scored finding.
///
/// Extends `FailureCategory` with an additional `Scenario` variant
/// to distinguish bad tests from real bugs and runner artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingCategory {
    /// Real application bug.
    App,
    /// Runner/infrastructure artifact (`WebDriver`, `tauri-driver`).
    Runner,
    /// Bad test scenario (unreliable or poorly defined steps).
    Scenario,
}

impl std::fmt::Display for FindingCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::App => f.write_str("app"),
            Self::Runner => f.write_str("runner"),
            Self::Scenario => f.write_str("scenario"),
        }
    }
}

/// A single finding from SAT scoring — an issue discovered during evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatFinding {
    /// Which scenario produced this finding.
    pub scenario_id: String,
    /// Step number where the issue was observed (0 = overall scenario).
    pub step_number: u32,
    /// Human-readable summary of the issue.
    pub summary: String,
    /// Detailed description of what went wrong.
    pub detail: String,
    /// Classification: app bug, runner artifact, or bad scenario.
    pub category: FindingCategory,
    /// How confident the LLM judge is in this classification.
    pub confidence: ConfidenceLevel,
    /// Evidence references (screenshot paths, step text, etc.).
    #[serde(default)]
    pub evidence: Vec<String>,
    /// Suggested severity (1 = critical, 5 = cosmetic).
    pub severity: u8,
}

/// Per-scenario satisfaction score produced by LLM-as-judge evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatScenarioScore {
    /// Scenario ID.
    pub scenario_id: String,
    /// Persona name associated with the scenario.
    pub persona: String,
    /// Overall satisfaction score (0-100).
    pub score: u32,
    /// Breakdown of the score by dimension.
    pub dimensions: SatScoreDimensions,
    /// LLM's reasoning for the score.
    pub reasoning: String,
    /// Findings (issues) discovered in this scenario.
    pub findings: Vec<SatFinding>,
}

/// Score dimensions — different aspects of user satisfaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatScoreDimensions {
    /// Did the feature work as expected? (0-100)
    pub functionality: u32,
    /// Was the experience smooth and responsive? (0-100)
    pub usability: u32,
    /// Was feedback clear and errors recoverable? (0-100)
    pub error_handling: u32,
    /// Did performance meet expectations? (0-100)
    pub performance: u32,
}

/// Token usage for a single LLM scoring call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// Full scoring result for a SAT run.
/// Written atomically to `sat/runs/run-{id}/scores.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatScoreResult {
    /// Run ID that was scored.
    pub run_id: String,
    /// ISO 8601 timestamp when scoring started.
    pub scored_at: String,
    /// Per-scenario scores.
    pub scenario_scores: Vec<SatScenarioScore>,
    /// Aggregate score across all scenarios (weighted average).
    pub aggregate_score: u32,
    /// All findings across all scenarios.
    pub all_findings: Vec<SatFinding>,
    /// Summary counts by finding category.
    pub finding_counts: FindingCounts,
    /// Total token usage for the scoring run.
    pub token_usage: TokenUsage,
    /// Estimated cost in dollars.
    pub estimated_cost_dollars: f64,
}

/// Summary counts of findings by category.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FindingCounts {
    pub app: usize,
    pub runner: usize,
    pub scenario: usize,
    pub total: usize,
}

/// A learning entry for `sat/learnings.yaml` — accumulated knowledge from scoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatLearning {
    /// ISO 8601 timestamp when the learning was recorded.
    pub recorded_at: String,
    /// Run ID that produced this learning.
    pub run_id: String,
    /// Scenario ID (if per-scenario).
    #[serde(default)]
    pub scenario_id: Option<String>,
    /// Category of the finding.
    pub category: FindingCategory,
    /// Confidence level.
    pub confidence: ConfidenceLevel,
    /// Human-readable summary.
    pub summary: String,
}

/// A cycle-level learning entry for `sat/learnings.yaml`.
///
/// Captures the aggregate results of a full fix-verify cycle, including
/// issues found/fixed/verified, score changes, and false positives.
/// Used to compute classification accuracy across cycles (FR27, NFR24).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatCycleLearning {
    /// ISO 8601 timestamp when the cycle learning was recorded.
    pub recorded_at: String,
    /// Run ID of the post-merge re-score.
    pub run_id: String,
    /// PR number that was merged (the fix).
    pub merged_pr_number: u64,
    /// Repository in "owner/repo" format.
    pub repo: String,
    /// Cycle iteration number (1-based).
    pub cycle_iteration: u32,
    /// Number of issues found in the original SAT run.
    pub issues_found: usize,
    /// Number of issues that were fixed (score improved). Used as true positives
    /// for classification accuracy: fixed = correctly identified as real app bugs.
    pub issues_fixed: usize,
    /// Number of false positives detected (runner/scenario misclassified as app).
    pub false_positives: usize,
    /// Aggregate score before the fix.
    pub score_before: u32,
    /// Aggregate score after the fix.
    pub score_after: u32,
    /// Verification outcome of this cycle.
    pub outcome: VerificationOutcome,
}

/// Top-level structure of `sat/learnings.yaml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatLearningsFile {
    #[serde(default)]
    pub learnings: Vec<SatLearning>,
    /// Cycle-level learnings from fix-verify cycles (Story 4.4).
    #[serde(default)]
    pub cycle_learnings: Vec<SatCycleLearning>,
}

// ===========================================================================
// Circuit breaker types (Story 4.4)
// ===========================================================================

/// Configuration for the circuit breaker that limits autonomous fix iterations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CircuitBreakerConfig {
    /// Maximum number of fix-verify iterations per cycle.
    /// Default: 3 (FR25).
    pub max_iterations: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self { max_iterations: 3 }
    }
}

/// Tracked state of the circuit breaker for a given cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CircuitBreakerState {
    /// Current iteration number (1-based, incremented each time a fix cycle runs).
    pub cycle_iteration: u32,
    /// Maximum iterations allowed (from config).
    pub cycle_max: u32,
    /// Repository in "owner/repo" format.
    pub repo: String,
    /// The original issue number that started this cycle.
    pub original_issue_number: Option<u64>,
}

/// Decision from the circuit breaker: continue or stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitBreakerDecision {
    /// Iteration is within limits — cycle may continue.
    Continue {
        /// Current iteration (after increment).
        iteration: u32,
        /// Maximum allowed.
        max: u32,
    },
    /// Iteration limit reached — cycle must stop.
    Tripped {
        /// The iteration that would have been next.
        iteration: u32,
        /// Maximum allowed.
        max: u32,
        /// Human-readable reason.
        reason: String,
    },
}

/// Classification accuracy metrics computed from cycle learnings.
///
/// Tracks how well the SAT LLM judge classifies findings over time (NFR24).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ClassificationAccuracy {
    /// Total classifications (`issues_found` across all cycles).
    pub total_classifications: usize,
    /// True positives: issues that were found AND verified as fixed.
    pub true_positives: usize,
    /// False positives: findings that were misclassified (runner/scenario as app).
    pub false_positives: usize,
    /// Accuracy: `true_positives / (true_positives + false_positives)`.
    /// `None` if no data is available.
    pub accuracy: Option<f64>,
    /// Number of cycles included in this calculation.
    pub cycles_counted: usize,
}

// ===========================================================================
// Issue creation types (Story 3.4)
// ===========================================================================

/// Configuration for SAT issue creation.
#[derive(Debug, Clone)]
pub struct SatIssueConfig {
    /// Root directory of the project (where `sat/` lives).
    pub project_root: PathBuf,
    /// Path to runs directory (default: `sat/runs/`).
    pub runs_dir: PathBuf,
    /// Path to the git repository (for resolving owner/repo).
    pub repo_path: PathBuf,
    /// Minimum severity threshold (1 = critical, 2 = high). Findings with
    /// severity <= this value are eligible for issue creation.
    pub max_severity: u8,
    /// Only create issues for findings in these categories.
    pub allowed_categories: Vec<FindingCategory>,
    /// Only create issues for findings with these confidence levels.
    pub allowed_confidences: Vec<ConfidenceLevel>,
}

impl SatIssueConfig {
    /// Create a config with standard defaults for a project root.
    #[must_use]
    pub fn new(project_root: PathBuf) -> Self {
        let runs_dir = project_root.join("sat/runs");
        let repo_path = project_root.clone();
        Self {
            project_root,
            runs_dir,
            repo_path,
            max_severity: 2, // critical (1) and high (2)
            allowed_categories: vec![FindingCategory::App],
            allowed_confidences: vec![ConfidenceLevel::High],
        }
    }
}

/// Outcome of creating a single GitHub issue from a finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum IssueCreationOutcome {
    /// Issue was created successfully.
    Created {
        issue_number: u64,
        issue_url: String,
    },
    /// Issue was skipped because a duplicate fingerprint already exists.
    SkippedDuplicate { fingerprint: String },
    /// Issue creation failed.
    Failed { reason: String },
}

/// Result of attempting to create an issue for a single finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatIssueEntry {
    /// The finding's scenario ID.
    pub scenario_id: String,
    /// Persona name from the scored finding.
    pub persona: String,
    /// Idempotent fingerprint (SHA-256 of `scenario_id` + persona + `run_id`).
    pub fingerprint: String,
    /// Finding summary.
    pub summary: String,
    /// Outcome of the creation attempt.
    pub outcome: IssueCreationOutcome,
}

/// Full result of SAT issue creation for a run.
/// Written to `sat/runs/run-{id}/issues.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatIssueResult {
    /// Run ID that was processed.
    pub run_id: String,
    /// ISO 8601 timestamp when issue creation started.
    pub created_at: String,
    /// Per-finding issue creation entries.
    pub entries: Vec<SatIssueEntry>,
    /// Count of issues actually created.
    pub created_count: usize,
    /// Count of duplicates skipped.
    pub skipped_count: usize,
    /// Count of failures.
    pub failed_count: usize,
}

// ===========================================================================
// Pipeline types (Story 3.5)
// ===========================================================================

/// Stage in the SAT pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SatPipelineStage {
    /// Generate scenarios and build manifest.
    Generate,
    /// Execute scenarios via `WebDriver`.
    Execute,
    /// Score results with LLM-as-judge.
    Score,
    /// Create GitHub issues from findings.
    CreateIssues,
}

impl std::fmt::Display for SatPipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Generate => f.write_str("generate"),
            Self::Execute => f.write_str("execute"),
            Self::Score => f.write_str("score"),
            Self::CreateIssues => f.write_str("create-issues"),
        }
    }
}

/// Status of a SAT pipeline run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum SatPipelineStatus {
    /// Pipeline is currently running a stage.
    Running { stage: SatPipelineStage },
    /// Pipeline completed all stages successfully.
    Completed,
    /// Pipeline failed at a specific stage.
    Failed {
        stage: SatPipelineStage,
        error: String,
    },
}

impl std::fmt::Display for SatPipelineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running { stage } => write!(f, "running ({stage})"),
            Self::Completed => f.write_str("completed"),
            Self::Failed { stage, error } => write!(f, "failed at {stage}: {error}"),
        }
    }
}

/// Configuration for a full SAT pipeline cycle.
#[derive(Debug, Clone)]
pub struct SatPipelineConfig {
    /// Root directory of the project (where `sat/` lives).
    pub project_root: PathBuf,
    /// Optional scenario ID filter — only execute these scenarios.
    #[allow(dead_code)]
    pub scenario_filter: Vec<String>,
    /// Maximum budget in USD for the entire cycle.
    pub max_budget_usd: f64,
}

impl SatPipelineConfig {
    /// Create a config with standard defaults for a project root.
    #[must_use]
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            project_root,
            scenario_filter: Vec::new(),
            max_budget_usd: 15.0,
        }
    }
}

/// Per-stage timing record in the pipeline result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatStageResult {
    /// Which stage this covers.
    pub stage: SatPipelineStage,
    /// Whether the stage succeeded.
    pub success: bool,
    /// Duration of this stage in milliseconds.
    pub duration_ms: u64,
    /// Error message if the stage failed.
    #[serde(default)]
    pub error: Option<String>,
}

/// Result of a complete SAT pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatPipelineResult {
    /// Overall status of the pipeline.
    pub status: SatPipelineStatus,
    /// Per-stage results (in order of execution).
    pub stages: Vec<SatStageResult>,
    /// Total duration in milliseconds.
    pub total_duration_ms: u64,
    /// Run ID produced by the execute stage (if reached).
    #[serde(default)]
    pub run_id: Option<String>,
    /// Aggregate satisfaction score (if scoring completed).
    #[serde(default)]
    pub aggregate_score: Option<u32>,
    /// Number of issues created (if issue creation completed).
    #[serde(default)]
    pub issues_created: Option<usize>,
}

// ===========================================================================
// Post-merge re-score context (Story 4.1)
// ===========================================================================

/// Context for a post-merge re-score run.
/// Links the re-score back to the original issue/PR for traceability.
/// Written to the worktree as `.branchdeck/rescore-context.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PostMergeRescoreContext {
    /// Repository in "owner/repo" format.
    pub repo: String,
    /// PR number that was merged (the implement-issue PR).
    pub merged_pr_number: u64,
    /// Branch name of the merged PR.
    pub merged_branch: String,
    /// Scenario IDs to re-run (from the original SAT run that found the issues).
    /// Empty means re-run all scenarios.
    #[serde(default)]
    pub scenario_filter: Vec<String>,
    /// Original issue number that the merged PR was fixing (if traceable).
    #[serde(default)]
    pub original_issue_number: Option<u64>,
    /// Run ID of the original SAT run that found the issue (if known).
    #[serde(default)]
    pub original_run_id: Option<String>,
}

/// Result of detecting a post-merge trigger.
/// Produced by the pure `apply_merge_event` function.
#[derive(Debug, Clone)]
pub struct PostMergeTrigger {
    /// PR key (e.g., "owner/repo#42").
    pub pr_key: String,
    /// Context for the re-score run.
    pub rescore_context: PostMergeRescoreContext,
}

// ===========================================================================
// Score comparison types (Story 4.2)
// ===========================================================================

/// Per-persona score delta showing before/after change.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PersonaScoreDelta {
    /// Persona name (e.g., "confused-newcomer").
    pub persona: String,
    /// Score before the fix (from the original SAT run).
    pub before: u32,
    /// Score after the fix (from the post-merge re-score).
    pub after: u32,
    /// Signed delta (after - before). Positive = improvement.
    pub delta: i32,
}

/// Per-scenario comparison of before/after scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ScenarioComparison {
    /// Scenario ID.
    pub scenario_id: String,
    /// Persona associated with the scenario.
    pub persona: String,
    /// Score before the fix.
    pub before_score: u32,
    /// Score after the fix.
    pub after_score: u32,
    /// Signed delta (after - before). Positive = improvement.
    pub delta: i32,
    /// Per-dimension deltas.
    pub dimension_deltas: DimensionDeltas,
}

/// Per-dimension before/after deltas.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DimensionDeltas {
    pub functionality: i32,
    pub usability: i32,
    pub error_handling: i32,
    pub performance: i32,
}

/// Full before/after score comparison for a SAT cycle.
/// Written to `sat/runs/run-{id}/comparison.json` (NFR23).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ScoreComparison {
    /// Run ID of the original (before) SAT run.
    pub before_run_id: String,
    /// Run ID of the post-merge (after) re-score run.
    pub after_run_id: String,
    /// ISO 8601 timestamp when comparison was computed.
    pub compared_at: String,
    /// Per-scenario comparisons (only scenarios present in both runs).
    pub scenario_comparisons: Vec<ScenarioComparison>,
    /// Per-persona aggregated deltas.
    pub persona_deltas: Vec<PersonaScoreDelta>,
    /// Overall score before the fix.
    pub overall_before: u32,
    /// Overall score after the fix.
    pub overall_after: u32,
    /// Overall signed delta.
    pub overall_delta: i32,
    /// Number of scenarios that improved (delta > 0).
    pub improved_count: usize,
    /// Number of scenarios that regressed (delta < 0).
    pub regressed_count: usize,
    /// Number of scenarios unchanged (delta == 0).
    pub unchanged_count: usize,
}

// ===========================================================================
// Regression detection types (Story 4.3)
// ===========================================================================

/// A single regression detected by comparing before/after scores.
///
/// A regression is a scenario that previously had a higher score
/// and now scores lower after a fix was merged.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RegressionFinding {
    /// Scenario ID that regressed.
    pub scenario_id: String,
    /// Persona associated with the scenario.
    pub persona: String,
    /// Score before the fix (from the original SAT run).
    pub before_score: u32,
    /// Score after the fix (from the post-merge re-score).
    pub after_score: u32,
    /// Magnitude of regression (always positive; before - after).
    pub regression_magnitude: u32,
    /// Per-dimension regression details (negative values = regression).
    pub dimension_deltas: DimensionDeltas,
    /// The PR number suspected of causing the regression.
    pub suspected_pr_number: u64,
    /// Repository in "owner/repo" format.
    pub repo: String,
}

/// Outcome of verifying a SAT cycle after a fix is merged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationOutcome {
    /// All scenarios improved or stayed the same. Cycle is done.
    Verified,
    /// Some previously-passing scenarios now score lower. New issues needed.
    Regressed,
    /// Mixed: some improved, some regressed. New issues for regressions.
    Mixed,
}

impl std::fmt::Display for VerificationOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Verified => f.write_str("verified"),
            Self::Regressed => f.write_str("regressed"),
            Self::Mixed => f.write_str("mixed"),
        }
    }
}

/// Full regression report for a SAT cycle verification.
/// Written to `sat/runs/run-{id}/regression-report.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RegressionReport {
    /// Run ID of the post-merge re-score run.
    pub after_run_id: String,
    /// Run ID of the original (before) SAT run.
    pub before_run_id: String,
    /// ISO 8601 timestamp when the report was generated.
    pub generated_at: String,
    /// PR number that was merged (the fix).
    pub merged_pr_number: u64,
    /// Repository in "owner/repo" format.
    pub repo: String,
    /// Verification outcome.
    pub outcome: VerificationOutcome,
    /// Overall score delta (positive = improvement).
    pub overall_delta: i32,
    /// Number of scenarios that improved.
    pub improved_count: usize,
    /// Number of scenarios that regressed.
    pub regressed_count: usize,
    /// Number of scenarios unchanged.
    pub unchanged_count: usize,
    /// Detailed regression findings (empty if outcome is `Verified`).
    pub regressions: Vec<RegressionFinding>,
}
