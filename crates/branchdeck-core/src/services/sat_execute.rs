//! SAT scenario execution service.
//!
//! Orchestrates WebDriver-based scenario execution against a running Tauri app.
//! Reads scenarios from `sat/scenarios/`, executes them via tauri-driver,
//! captures step results with screenshots, and writes trajectory data.
//!
//! Architecture:
//! - Pure functions for trajectory building, failure classification, run directory setup
//! - I/O functions for file writes and process management
//! - The actual `WebDriver` interaction happens in the TypeScript runner
//!   (`sat/scripts/run-scenario.ts`); this service orchestrates the runner process

use std::path::{Path, PathBuf};

use log::{debug, error, info, warn};

use crate::error::AppError;
use crate::models::sat::{
    FailureCategory, SatPerformance, SatRunConfig, SatRunResult, SatScenario, SatStepResult,
    SatTrajectory, StepStatus, TrajectoryStatus,
};
use crate::services::sat_generate;

// ---------------------------------------------------------------------------
// Run directory management
// ---------------------------------------------------------------------------

/// Generate a run ID from a timestamp string (e.g., `"run-20260326T120000"`).
#[must_use]
pub fn make_run_id(now_iso: &str) -> String {
    // Sanitize the ISO timestamp for use as a directory name
    let sanitized: String = now_iso
        .chars()
        .filter(|c| *c != ':' && *c != '-' && *c != '+')
        .take(15)
        .collect();
    format!("run-{sanitized}")
}

/// Compute the run output directory path.
#[must_use]
pub fn run_dir_path(runs_dir: &Path, run_id: &str) -> PathBuf {
    runs_dir.join(run_id)
}

/// Create the run output directory structure.
///
/// Creates:
/// - `sat/runs/{run_id}/`
/// - `sat/runs/{run_id}/screenshots/`
///
/// # Errors
/// Returns `AppError::Sat` if directories cannot be created.
pub fn create_run_dirs(run_dir: &Path) -> Result<(), AppError> {
    let screenshots_dir = run_dir.join("screenshots");
    std::fs::create_dir_all(&screenshots_dir).map_err(|e| {
        error!(
            "Failed to create run directories at {}: {e}",
            run_dir.display()
        );
        AppError::Sat(format!(
            "failed to create run dir {}: {e}",
            run_dir.display()
        ))
    })?;
    debug!("Created run directory: {}", run_dir.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Failure classification (pure)
// ---------------------------------------------------------------------------

/// Classify a failure based on the error message and context.
///
/// `WebDriver` session failures (connection refused, session not created, timeout
/// connecting to driver) are classified as `Runner` — they indicate infrastructure
/// issues, not application bugs.
#[must_use]
pub fn classify_failure(error_msg: &str) -> FailureCategory {
    let lower = error_msg.to_lowercase();

    // Runner/infrastructure failures — WebDriver and tauri-driver issues
    let runner_patterns = [
        "connection refused",
        "session not created",
        "unable to create session",
        "webdriver",
        "tauri-driver",
        "econnrefused",
        "timeout",
        "no such session",
        "session deleted",
        "chrome not reachable",
        "unknown server-side error",
        "driver",
        "socket hang up",
        "fetch failed",
        "network error",
    ];

    for pattern in &runner_patterns {
        if lower.contains(pattern) {
            return FailureCategory::Runner;
        }
    }

    // Interpretation failures — step could not be translated to an action
    let interpretation_patterns = [
        "could not interpret",
        "unrecognized step",
        "could not parse",
        "element not found",
        "navigation target not found",
        "verification target not found",
    ];

    for pattern in &interpretation_patterns {
        if lower.contains(pattern) {
            return FailureCategory::Interpretation;
        }
    }

    // Default: application-level failure
    FailureCategory::App
}

// ---------------------------------------------------------------------------
// Trajectory building (pure)
// ---------------------------------------------------------------------------

/// Create a skipped step result (used when scenario is aborted).
#[must_use]
pub fn make_skipped_step(step_number: u32, step_text: &str, now_iso: &str) -> SatStepResult {
    SatStepResult {
        step_number,
        step_text: step_text.to_string(),
        status: StepStatus::Skip,
        action_taken: "Skipped — scenario aborted after consecutive failures".to_string(),
        before_screenshot: None,
        after_screenshot: None,
        page_summary: None,
        failure_reason: Some("Scenario aborted".to_string()),
        failure_category: None,
        duration_ms: 0,
        started_at: now_iso.to_string(),
    }
}

/// Build performance metrics from step results.
#[must_use]
pub fn build_performance(steps: &[SatStepResult]) -> SatPerformance {
    let step_durations_ms: Vec<u64> = steps.iter().map(|s| s.duration_ms).collect();
    let total_duration_ms: u64 = step_durations_ms.iter().sum();
    SatPerformance {
        total_duration_ms,
        step_durations_ms,
    }
}

/// Determine the trajectory status from step results and abort state.
#[must_use]
pub fn determine_trajectory_status(
    steps: &[SatStepResult],
    was_aborted: bool,
    runner_failed: bool,
) -> TrajectoryStatus {
    if runner_failed {
        return TrajectoryStatus::RunnerFailure;
    }
    if was_aborted {
        return TrajectoryStatus::Aborted;
    }
    // Even if some steps failed, if we ran them all, it's "completed"
    if steps.is_empty() {
        return TrajectoryStatus::RunnerFailure;
    }
    TrajectoryStatus::Completed
}

/// Build a trajectory from collected step results.
#[must_use]
pub fn build_trajectory(
    scenario_id: &str,
    scenario_file: &str,
    started_at: &str,
    completed_at: &str,
    steps: Vec<SatStepResult>,
    was_aborted: bool,
    runner_failed: bool,
) -> SatTrajectory {
    let status = determine_trajectory_status(&steps, was_aborted, runner_failed);
    let performance = build_performance(&steps);
    SatTrajectory {
        scenario_id: scenario_id.to_string(),
        scenario_file: scenario_file.to_string(),
        started_at: started_at.to_string(),
        completed_at: completed_at.to_string(),
        status,
        steps,
        performance,
    }
}

/// Build the overall run result from trajectories.
#[must_use]
pub fn build_run_result(
    run_id: &str,
    run_dir: &str,
    started_at: &str,
    completed_at: &str,
    trajectories: Vec<SatTrajectory>,
) -> SatRunResult {
    let scenarios_total = trajectories.len();
    let scenarios_completed = trajectories
        .iter()
        .filter(|t| t.status == TrajectoryStatus::Completed)
        .count();
    let scenarios_aborted = trajectories
        .iter()
        .filter(|t| t.status == TrajectoryStatus::Aborted)
        .count();
    let scenarios_runner_failed = trajectories
        .iter()
        .filter(|t| t.status == TrajectoryStatus::RunnerFailure)
        .count();

    SatRunResult {
        run_id: run_id.to_string(),
        started_at: started_at.to_string(),
        completed_at: completed_at.to_string(),
        run_dir: run_dir.to_string(),
        scenarios_total,
        scenarios_completed,
        scenarios_aborted,
        scenarios_runner_failed,
        trajectories,
    }
}

// ---------------------------------------------------------------------------
// Trajectory I/O
// ---------------------------------------------------------------------------

/// Write a trajectory JSON file to the run directory.
///
/// # Errors
/// Returns `AppError::Sat` if serialization or file write fails.
pub fn write_trajectory(trajectory: &SatTrajectory, run_dir: &Path) -> Result<PathBuf, AppError> {
    let filename = format!("trajectory-{}.json", trajectory.scenario_id);
    let path = run_dir.join(&filename);
    let json = serde_json::to_string_pretty(trajectory).map_err(|e| {
        error!("Failed to serialize trajectory: {e}");
        AppError::Sat(format!("trajectory serialization error: {e}"))
    })?;
    crate::util::write_atomic(&path, json.as_bytes()).map_err(|e| {
        error!("Failed to write trajectory to {}: {e}", path.display());
        AppError::Sat(format!("failed to write trajectory: {e}"))
    })?;
    debug!(
        "Wrote trajectory for {} to {}",
        trajectory.scenario_id,
        path.display()
    );
    Ok(path)
}

/// Write the run result JSON to the run directory.
///
/// # Errors
/// Returns `AppError::Sat` if serialization or file write fails.
pub fn write_run_result(result: &SatRunResult, run_dir: &Path) -> Result<PathBuf, AppError> {
    let path = run_dir.join("run-result.json");
    let json = serde_json::to_string_pretty(result).map_err(|e| {
        error!("Failed to serialize run result: {e}");
        AppError::Sat(format!("run result serialization error: {e}"))
    })?;
    crate::util::write_atomic(&path, json.as_bytes()).map_err(|e| {
        error!("Failed to write run result to {}: {e}", path.display());
        AppError::Sat(format!("failed to write run result: {e}"))
    })?;
    info!("Wrote run result to {}", path.display());
    Ok(path)
}

// ---------------------------------------------------------------------------
// WebDriver runner invocation
// ---------------------------------------------------------------------------

/// Build the environment variables for the `WebdriverIO` runner process.
#[must_use]
pub fn build_runner_env(
    config: &SatRunConfig,
    scenario_file: &Path,
    run_dir: &Path,
) -> Vec<(String, String)> {
    vec![
        (
            "SAT_SCENARIO_FILE".to_string(),
            scenario_file.to_string_lossy().to_string(),
        ),
        (
            "SAT_RUN_DIR".to_string(),
            run_dir.to_string_lossy().to_string(),
        ),
        (
            "TAURI_DRIVER_PATH".to_string(),
            config.tauri_driver_path.to_string_lossy().to_string(),
        ),
        (
            "APP_BINARY_PATH".to_string(),
            config.app_binary_path.to_string_lossy().to_string(),
        ),
        ("WEBDRIVER_HOST".to_string(), config.webdriver_host.clone()),
        (
            "WEBDRIVER_PORT".to_string(),
            config.webdriver_port.to_string(),
        ),
    ]
}

/// Run the `WebdriverIO` test runner for a single scenario.
///
/// Spawns `bunx wdio` with the SAT config, passing the scenario file path
/// and run directory via environment variables. The actual step execution
/// happens in `sat/scripts/run-scenario.ts`.
///
/// # Errors
/// Returns `AppError::Sat` if the process cannot be spawned or exits with error.
pub fn run_wdio_scenario(
    config: &SatRunConfig,
    scenario_file: &Path,
    run_dir: &Path,
) -> Result<SatTrajectory, AppError> {
    let wdio_config = config.project_root.join("sat/scripts/wdio.sat.conf.ts");
    if !wdio_config.exists() {
        return Err(AppError::Sat(format!(
            "WebdriverIO config not found: {}",
            wdio_config.display()
        )));
    }

    let env_vars = build_runner_env(config, scenario_file, run_dir);

    info!(
        "Running WDIO scenario: {} (run dir: {})",
        scenario_file.display(),
        run_dir.display()
    );

    let mut cmd = std::process::Command::new("bunx");
    cmd.arg("wdio")
        .arg("run")
        .arg(&wdio_config)
        .current_dir(&config.project_root);

    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    let output = cmd.output().map_err(|e| {
        error!("Failed to spawn WDIO runner: {e}");
        AppError::Sat(format!("failed to spawn WDIO runner: {e}"))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        warn!(
            "WDIO runner exited with status {}: stderr={}",
            output.status,
            stderr.chars().take(500).collect::<String>()
        );
    }

    debug!(
        "WDIO stdout (last 200 chars): {}",
        stdout
            .chars()
            .rev()
            .take(200)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>()
    );

    // The WDIO runner writes trajectory JSON to run_dir — read it back.
    // Extract scenario ID from the file to find the trajectory.
    let scenario = sat_generate::parse_scenario_file(scenario_file)?;
    let trajectory_path = run_dir.join(format!("trajectory-{}.json", scenario.meta.id));

    if trajectory_path.exists() {
        let content = std::fs::read_to_string(&trajectory_path).map_err(|e| {
            error!(
                "Failed to read trajectory from {}: {e}",
                trajectory_path.display()
            );
            AppError::Sat(format!("failed to read trajectory: {e}"))
        })?;
        let trajectory: SatTrajectory = serde_json::from_str(&content).map_err(|e| {
            error!("Failed to parse trajectory JSON: {e}");
            AppError::Sat(format!("trajectory parse error: {e}"))
        })?;
        Ok(trajectory)
    } else if !output.status.success() {
        // Runner failed entirely — classify as runner failure
        let now = chrono::Utc::now().to_rfc3339();
        let trajectory = build_trajectory(
            &scenario.meta.id,
            &scenario_file.to_string_lossy(),
            &now,
            &now,
            Vec::new(),
            false,
            true,
        );
        write_trajectory(&trajectory, run_dir)?;
        Ok(trajectory)
    } else {
        Err(AppError::Sat(format!(
            "WDIO runner succeeded but no trajectory file found at {}",
            trajectory_path.display()
        )))
    }
}

// ---------------------------------------------------------------------------
// Full execution pipeline
// ---------------------------------------------------------------------------

/// Load scenarios to execute, optionally filtered by IDs.
///
/// # Errors
/// Returns `AppError::Sat` if scenarios cannot be loaded.
pub fn load_execution_scenarios(
    scenarios_dir: &Path,
    filter_ids: &[String],
) -> Result<Vec<(PathBuf, SatScenario)>, AppError> {
    let all_scenarios = sat_generate::load_scenarios(scenarios_dir)?;

    if all_scenarios.is_empty() {
        return Err(AppError::Sat(
            "no scenarios found in sat/scenarios/ — run scenario generation first".into(),
        ));
    }

    let filtered: Vec<(PathBuf, SatScenario)> = all_scenarios
        .into_iter()
        .filter(|s| filter_ids.is_empty() || filter_ids.contains(&s.meta.id))
        .map(|s| {
            let path = scenarios_dir.join(format!("{}.md", s.meta.id));
            (path, s)
        })
        .collect();

    if filtered.is_empty() {
        return Err(AppError::Sat(format!(
            "no scenarios matched filter: {filter_ids:?}"
        )));
    }

    info!("Loaded {} scenarios for execution", filtered.len());
    Ok(filtered)
}

/// Execute all scenarios in a SAT run.
///
/// This is the top-level orchestration function:
/// 1. Load scenarios from `sat/scenarios/`
/// 2. Create run output directory `sat/runs/run-{timestamp}/`
/// 3. Execute each scenario via `WebdriverIO`
/// 4. Collect trajectories and write run result
///
/// # Errors
/// Returns `AppError::Sat` on fatal errors (can't create dirs, no scenarios).
/// Individual scenario failures are captured in trajectories, not propagated.
pub fn execute_run(config: &SatRunConfig, filter_ids: &[String]) -> Result<SatRunResult, AppError> {
    let now = chrono::Utc::now().to_rfc3339();
    let run_id = make_run_id(&now);
    let run_dir = run_dir_path(&config.runs_dir, &run_id);

    info!("Starting SAT execution run {run_id}");

    // 1. Create run directory
    create_run_dirs(&run_dir)?;

    // 2. Load scenarios
    let scenarios = load_execution_scenarios(&config.scenarios_dir, filter_ids)?;
    info!("Executing {} scenarios in run {run_id}", scenarios.len());

    // 3. Execute each scenario
    let mut trajectories = Vec::new();
    for (scenario_path, scenario) in &scenarios {
        info!(
            "Executing scenario: {} ({})",
            scenario.meta.id, scenario.meta.title
        );

        match run_wdio_scenario(config, scenario_path, &run_dir) {
            Ok(trajectory) => {
                info!(
                    "Scenario {} completed with status: {}",
                    scenario.meta.id, trajectory.status
                );
                trajectories.push(trajectory);
            }
            Err(e) => {
                error!("Scenario {} execution error: {e}", scenario.meta.id);
                // Create a runner-failure trajectory so the run result is complete
                let fail_now = chrono::Utc::now().to_rfc3339();
                let trajectory = build_trajectory(
                    &scenario.meta.id,
                    &scenario_path.to_string_lossy(),
                    &fail_now,
                    &fail_now,
                    Vec::new(),
                    false,
                    true,
                );
                if let Err(write_err) = write_trajectory(&trajectory, &run_dir) {
                    warn!(
                        "Failed to write failure trajectory for {}: {write_err}",
                        scenario.meta.id
                    );
                }
                trajectories.push(trajectory);
            }
        }
    }

    // 4. Build and write run result
    let completed_at = chrono::Utc::now().to_rfc3339();
    let run_result = build_run_result(
        &run_id,
        &run_dir.to_string_lossy(),
        &now,
        &completed_at,
        trajectories,
    );

    write_run_result(&run_result, &run_dir)?;

    info!(
        "SAT run {run_id} complete: {}/{} scenarios completed, {} aborted, {} runner failures",
        run_result.scenarios_completed,
        run_result.scenarios_total,
        run_result.scenarios_aborted,
        run_result.scenarios_runner_failed,
    );

    Ok(run_result)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::sat::{FailureCategory, StepStatus, TrajectoryStatus};

    // -- make_run_id ---------------------------------------------------------

    #[test]
    fn run_id_from_iso_timestamp() {
        let id = make_run_id("2026-03-26T12:00:00+00:00");
        assert!(id.starts_with("run-"));
        // Should not contain colons or dashes (sanitized)
        let suffix = &id[4..];
        assert!(!suffix.contains(':'));
        assert!(!suffix.contains('-'));
    }

    #[test]
    fn run_id_deterministic() {
        let a = make_run_id("2026-03-26T12:00:00Z");
        let b = make_run_id("2026-03-26T12:00:00Z");
        assert_eq!(a, b);
    }

    // -- classify_failure ----------------------------------------------------

    #[test]
    fn classify_runner_failures() {
        assert_eq!(
            classify_failure("connection refused at 127.0.0.1:4444"),
            FailureCategory::Runner
        );
        assert_eq!(
            classify_failure("Session not created: no matching capabilities"),
            FailureCategory::Runner
        );
        assert_eq!(
            classify_failure("tauri-driver exited unexpectedly"),
            FailureCategory::Runner
        );
        assert_eq!(classify_failure("ECONNREFUSED"), FailureCategory::Runner);
        assert_eq!(
            classify_failure("Request timeout after 30000ms"),
            FailureCategory::Runner
        );
    }

    #[test]
    fn classify_interpretation_failures() {
        assert_eq!(
            classify_failure("Could not interpret step: do something weird"),
            FailureCategory::Interpretation
        );
        assert_eq!(
            classify_failure("Element not found: magic button"),
            FailureCategory::Interpretation
        );
        assert_eq!(
            classify_failure("Unrecognized step pattern: xyz"),
            FailureCategory::Interpretation
        );
    }

    #[test]
    fn classify_app_failures() {
        assert_eq!(
            classify_failure("Expected text 'Success' but found 'Error'"),
            FailureCategory::App
        );
        assert_eq!(
            classify_failure("Button click caused unhandled exception in app"),
            FailureCategory::App
        );
    }

    // -- make_skipped_step ---------------------------------------------------

    #[test]
    fn skipped_step_has_correct_fields() {
        let step = make_skipped_step(5, "Click the button", "2026-03-26T00:00:00Z");
        assert_eq!(step.step_number, 5);
        assert_eq!(step.step_text, "Click the button");
        assert_eq!(step.status, StepStatus::Skip);
        assert_eq!(step.duration_ms, 0);
        assert!(step.failure_reason.is_some());
        assert!(step.before_screenshot.is_none());
    }

    // -- build_performance ---------------------------------------------------

    #[test]
    fn build_performance_sums_durations() {
        let steps = vec![
            SatStepResult {
                step_number: 1,
                step_text: "Step 1".into(),
                status: StepStatus::Pass,
                action_taken: "did stuff".into(),
                before_screenshot: None,
                after_screenshot: None,
                page_summary: None,
                failure_reason: None,
                failure_category: None,
                duration_ms: 100,
                started_at: "t1".into(),
            },
            SatStepResult {
                step_number: 2,
                step_text: "Step 2".into(),
                status: StepStatus::Fail,
                action_taken: "failed".into(),
                before_screenshot: None,
                after_screenshot: None,
                page_summary: None,
                failure_reason: Some("oops".into()),
                failure_category: Some(FailureCategory::App),
                duration_ms: 250,
                started_at: "t2".into(),
            },
        ];

        let perf = build_performance(&steps);
        assert_eq!(perf.total_duration_ms, 350);
        assert_eq!(perf.step_durations_ms, vec![100, 250]);
    }

    #[test]
    fn build_performance_empty_steps() {
        let perf = build_performance(&[]);
        assert_eq!(perf.total_duration_ms, 0);
        assert!(perf.step_durations_ms.is_empty());
    }

    // -- determine_trajectory_status -----------------------------------------

    #[test]
    fn status_runner_failure_takes_priority() {
        let steps = vec![make_skipped_step(1, "a", "t")];
        assert_eq!(
            determine_trajectory_status(&steps, true, true),
            TrajectoryStatus::RunnerFailure
        );
    }

    #[test]
    fn status_aborted_when_not_runner() {
        let steps = vec![make_skipped_step(1, "a", "t")];
        assert_eq!(
            determine_trajectory_status(&steps, true, false),
            TrajectoryStatus::Aborted
        );
    }

    #[test]
    fn status_completed_when_all_ran() {
        let steps = vec![make_skipped_step(1, "a", "t")];
        assert_eq!(
            determine_trajectory_status(&steps, false, false),
            TrajectoryStatus::Completed
        );
    }

    #[test]
    fn status_runner_failure_when_no_steps() {
        assert_eq!(
            determine_trajectory_status(&[], false, false),
            TrajectoryStatus::RunnerFailure
        );
    }

    // -- build_trajectory ----------------------------------------------------

    #[test]
    fn build_trajectory_basic() {
        let steps = vec![SatStepResult {
            step_number: 1,
            step_text: "Open app".into(),
            status: StepStatus::Pass,
            action_taken: "Opened".into(),
            before_screenshot: Some("s/1-before.png".into()),
            after_screenshot: Some("s/1-after.png".into()),
            page_summary: Some("Title: App".into()),
            failure_reason: None,
            failure_category: None,
            duration_ms: 200,
            started_at: "2026-03-26T00:00:00Z".into(),
        }];

        let traj = build_trajectory(
            "test-01",
            "scenarios/test-01.md",
            "2026-03-26T00:00:00Z",
            "2026-03-26T00:00:01Z",
            steps,
            false,
            false,
        );

        assert_eq!(traj.scenario_id, "test-01");
        assert_eq!(traj.status, TrajectoryStatus::Completed);
        assert_eq!(traj.steps.len(), 1);
        assert_eq!(traj.performance.total_duration_ms, 200);
    }

    // -- build_run_result ----------------------------------------------------

    #[test]
    fn build_run_result_counts() {
        let completed_traj = build_trajectory(
            "s1",
            "f1",
            "t1",
            "t2",
            vec![make_skipped_step(1, "a", "t")],
            false,
            false,
        );
        let aborted_traj = build_trajectory(
            "s2",
            "f2",
            "t1",
            "t2",
            vec![make_skipped_step(1, "b", "t")],
            true,
            false,
        );
        let runner_traj = build_trajectory("s3", "f3", "t1", "t2", Vec::new(), false, true);

        let result = build_run_result(
            "run-test",
            "/tmp/run-test",
            "t1",
            "t2",
            vec![completed_traj, aborted_traj, runner_traj],
        );

        assert_eq!(result.scenarios_total, 3);
        assert_eq!(result.scenarios_completed, 1);
        assert_eq!(result.scenarios_aborted, 1);
        assert_eq!(result.scenarios_runner_failed, 1);
    }

    // -- create_run_dirs (filesystem) ----------------------------------------

    #[test]
    fn create_run_dirs_creates_structure() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path().join("run-test");
        create_run_dirs(&run_dir).unwrap();

        assert!(run_dir.exists());
        assert!(run_dir.join("screenshots").exists());
    }

    // -- write_trajectory (filesystem) ---------------------------------------

    #[test]
    fn write_and_read_trajectory() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path().join("run-test");
        create_run_dirs(&run_dir).unwrap();

        let traj = build_trajectory(
            "test-scenario",
            "scenarios/test-scenario.md",
            "2026-03-26T00:00:00Z",
            "2026-03-26T00:00:05Z",
            vec![make_skipped_step(1, "Do thing", "2026-03-26T00:00:00Z")],
            false,
            false,
        );

        let path = write_trajectory(&traj, &run_dir).unwrap();
        assert!(path.exists());
        assert_eq!(path.file_name().unwrap(), "trajectory-test-scenario.json");

        // Read back and verify
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: SatTrajectory = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.scenario_id, "test-scenario");
        assert_eq!(parsed.steps.len(), 1);
    }

    // -- write_run_result (filesystem) ---------------------------------------

    #[test]
    fn write_and_read_run_result() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path().join("run-test");
        create_run_dirs(&run_dir).unwrap();

        let result = build_run_result(
            "run-test",
            &run_dir.to_string_lossy(),
            "2026-03-26T00:00:00Z",
            "2026-03-26T00:01:00Z",
            Vec::new(),
        );

        let path = write_run_result(&result, &run_dir).unwrap();
        assert!(path.exists());
        assert_eq!(path.file_name().unwrap(), "run-result.json");

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: SatRunResult = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.run_id, "run-test");
        assert_eq!(parsed.scenarios_total, 0);
    }

    // -- build_runner_env ----------------------------------------------------

    #[test]
    fn runner_env_contains_required_vars() {
        let config = SatRunConfig::new(PathBuf::from("/tmp/project"));
        let env = build_runner_env(
            &config,
            Path::new("/tmp/project/sat/scenarios/test.md"),
            Path::new("/tmp/project/sat/runs/run-001"),
        );

        let keys: Vec<&str> = env.iter().map(|(k, _)| k.as_str()).collect();
        assert!(keys.contains(&"SAT_SCENARIO_FILE"));
        assert!(keys.contains(&"SAT_RUN_DIR"));
        assert!(keys.contains(&"TAURI_DRIVER_PATH"));
        assert!(keys.contains(&"APP_BINARY_PATH"));
        assert!(keys.contains(&"WEBDRIVER_HOST"));
        assert!(keys.contains(&"WEBDRIVER_PORT"));
    }
}
