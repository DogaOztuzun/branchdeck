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
