//! SAT regression detection and loop continuation service (Story 4.3).
//!
//! Detects regressions introduced by fixes: scenarios that previously had
//! higher scores now score lower after a merged PR. Creates new GitHub issues
//! for regression findings and determines whether the cycle is verified or
//! needs to continue (the ratchet effect).
//!
//! Architecture:
//! - Pure functions: `detect_regressions`, `determine_outcome`,
//!   `build_regression_report`, `build_regression_issue_title`,
//!   `build_regression_issue_body`, `build_regression_labels`
//! - I/O functions: `write_regression_report`,
//!   `create_regression_issues`, `verify_cycle`

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use log::{debug, error, info};

use crate::error::AppError;
use crate::models::sat::{
    PostMergeRescoreContext, RegressionFinding, RegressionReport, ScoreComparison,
    VerificationOutcome,
};
use crate::services::sat_issues::IssueCreator;

// ---------------------------------------------------------------------------
// Pure regression detection
// ---------------------------------------------------------------------------

/// Detect regressions from a score comparison.
///
/// A regression is any scenario where `delta < 0` — the score dropped
/// after the fix was merged. Returns one `RegressionFinding` per regressed
/// scenario.
#[must_use]
#[allow(clippy::cast_sign_loss)]
pub fn detect_regressions(
    comparison: &ScoreComparison,
    merged_pr_number: u64,
    repo: &str,
) -> Vec<RegressionFinding> {
    comparison
        .scenario_comparisons
        .iter()
        .filter(|sc| sc.delta < 0)
        .map(|sc| RegressionFinding {
            scenario_id: sc.scenario_id.clone(),
            persona: sc.persona.clone(),
            before_score: sc.before_score,
            after_score: sc.after_score,
            regression_magnitude: sc.delta.unsigned_abs(),
            dimension_deltas: sc.dimension_deltas.clone(),
            suspected_pr_number: merged_pr_number,
            repo: repo.to_string(),
        })
        .collect()
}

/// Determine the verification outcome from a score comparison.
///
/// - `Verified`: no regressions and at least some improvement (or all unchanged)
/// - `Regressed`: all changes are regressions (no improvements)
/// - `Mixed`: some scenarios improved, some regressed
#[must_use]
pub fn determine_outcome(comparison: &ScoreComparison) -> VerificationOutcome {
    if comparison.regressed_count == 0 {
        VerificationOutcome::Verified
    } else if comparison.improved_count == 0 {
        VerificationOutcome::Regressed
    } else {
        VerificationOutcome::Mixed
    }
}

/// Build a full regression report from a score comparison and context.
#[must_use]
pub fn build_regression_report(
    comparison: &ScoreComparison,
    context: &PostMergeRescoreContext,
    generated_at: &str,
) -> RegressionReport {
    let regressions = detect_regressions(comparison, context.merged_pr_number, &context.repo);
    let outcome = determine_outcome(comparison);

    RegressionReport {
        after_run_id: comparison.after_run_id.clone(),
        before_run_id: comparison.before_run_id.clone(),
        generated_at: generated_at.to_string(),
        merged_pr_number: context.merged_pr_number,
        repo: context.repo.clone(),
        outcome,
        overall_delta: comparison.overall_delta,
        improved_count: comparison.improved_count,
        regressed_count: comparison.regressed_count,
        unchanged_count: comparison.unchanged_count,
        regressions,
    }
}

// ---------------------------------------------------------------------------
// Regression issue body building (pure)
// ---------------------------------------------------------------------------

/// Build the GitHub issue title for a regression finding.
#[must_use]
pub fn build_regression_issue_title(finding: &RegressionFinding) -> String {
    format!(
        "[SAT/Regression] {} regressed by {} points after PR #{}",
        finding.scenario_id, finding.regression_magnitude, finding.suspected_pr_number
    )
}

/// Build the full GitHub issue body for a regression finding.
///
/// Includes: which scenario regressed, by how much, per-dimension breakdown,
/// and a link to the suspected cause PR.
#[must_use]
pub fn build_regression_issue_body(
    finding: &RegressionFinding,
    after_run_id: &str,
    before_run_id: &str,
    fingerprint: &str,
) -> String {
    let mut body = String::new();

    // Header
    let _ = writeln!(body, "## SAT Regression Finding");
    let _ = writeln!(body);
    let _ = writeln!(
        body,
        "A previously-passing scenario now scores **lower** after a fix was merged."
    );
    let _ = writeln!(
        body,
        "This regression was automatically detected by the SAT ratchet system."
    );
    let _ = writeln!(body);

    // Metadata table
    let _ = writeln!(body, "| Field | Value |");
    let _ = writeln!(body, "|:------|:------|");
    let _ = writeln!(body, "| **Scenario** | `{}` |", finding.scenario_id);
    let _ = writeln!(body, "| **Persona** | {} |", finding.persona);
    let _ = writeln!(body, "| **Score Before** | {}/100 |", finding.before_score);
    let _ = writeln!(body, "| **Score After** | {}/100 |", finding.after_score);
    let _ = writeln!(
        body,
        "| **Regression** | -{} points |",
        finding.regression_magnitude
    );
    let _ = writeln!(
        body,
        "| **Suspected Cause** | #{} |",
        finding.suspected_pr_number
    );
    let _ = writeln!(body, "| **Before Run** | `{before_run_id}` |");
    let _ = writeln!(body, "| **After Run** | `{after_run_id}` |");
    let _ = writeln!(body);

    // Dimension breakdown
    let _ = writeln!(body, "## Dimension Breakdown");
    let _ = writeln!(body);
    let _ = writeln!(body, "| Dimension | Delta |");
    let _ = writeln!(body, "|:----------|------:|");
    let _ = writeln!(
        body,
        "| Functionality | {:+} |",
        finding.dimension_deltas.functionality
    );
    let _ = writeln!(
        body,
        "| Usability | {:+} |",
        finding.dimension_deltas.usability
    );
    let _ = writeln!(
        body,
        "| Error Handling | {:+} |",
        finding.dimension_deltas.error_handling
    );
    let _ = writeln!(
        body,
        "| Performance | {:+} |",
        finding.dimension_deltas.performance
    );
    let _ = writeln!(body);

    // Context
    let _ = writeln!(body, "## Context");
    let _ = writeln!(body);
    let _ = writeln!(
        body,
        "This scenario scored **{}/100** before PR #{} was merged, and now scores **{}/100**.",
        finding.before_score, finding.suspected_pr_number, finding.after_score
    );
    let _ = writeln!(
        body,
        "The fix in PR #{} likely introduced a side effect that degraded this scenario.",
        finding.suspected_pr_number
    );
    let _ = writeln!(body);

    // Fingerprint for dedup
    let _ = writeln!(body, "---");
    let _ = writeln!(body, "<!-- sat-fingerprint:{fingerprint} -->");

    body
}

/// Build the labels for a regression issue.
///
/// Includes `agent:implement` for automatic pickup (loop continues),
/// `sat:regression` for categorization, and links to the suspected PR.
#[must_use]
pub fn build_regression_labels(finding: &RegressionFinding) -> Vec<String> {
    vec![
        "agent:implement".to_string(),
        "sat:regression".to_string(),
        format!("regression-from:pr-{}", finding.suspected_pr_number),
    ]
}

// ---------------------------------------------------------------------------
// I/O: write regression report
// ---------------------------------------------------------------------------

/// Write a `RegressionReport` as JSON to `regression-report.json` in the run directory.
///
/// # Errors
/// Returns `AppError::Sat` if serialization or file write fails.
pub fn write_regression_report(
    report: &RegressionReport,
    run_dir: &Path,
) -> Result<PathBuf, AppError> {
    let path = run_dir.join("regression-report.json");
    let json = serde_json::to_string_pretty(report).map_err(|e| {
        error!("Failed to serialize regression report: {e}");
        AppError::Sat(format!("regression report serialization error: {e}"))
    })?;
    crate::util::write_atomic(&path, json.as_bytes()).map_err(|e| {
        error!(
            "Failed to write regression report to {}: {e}",
            path.display()
        );
        AppError::Sat(format!("failed to write regression report: {e}"))
    })?;
    info!("Wrote regression report to {}", path.display());
    Ok(path)
}

// ---------------------------------------------------------------------------
// Regression issue creation (I/O)
// ---------------------------------------------------------------------------

/// Create GitHub issues for all regression findings in a report.
///
/// Uses the same `IssueCreator` trait as `sat_issues` for consistency.
/// Each regression gets its own issue with `sat:regression` label and
/// `agent:implement` for automatic pickup (continuing the loop).
///
/// Returns the number of issues successfully created.
///
/// # Errors
/// Returns `AppError::Sat` on fatal errors. Individual issue creation
/// failures are logged but do not stop the process.
pub fn create_regression_issues(
    report: &RegressionReport,
    owner: &str,
    repo_name: &str,
    creator: &dyn IssueCreator,
) -> Result<usize, AppError> {
    if report.regressions.is_empty() {
        debug!("No regressions to create issues for");
        return Ok(0);
    }

    info!(
        "Creating {} regression issues for run {}",
        report.regressions.len(),
        report.after_run_id
    );

    let mut created_count = 0_usize;

    for finding in &report.regressions {
        let fingerprint = crate::services::sat_issues::generate_fingerprint(
            &finding.scenario_id,
            &finding.persona,
            &report.after_run_id,
        );

        // Check for duplicate
        match creator.issue_exists_with_fingerprint(owner, repo_name, &fingerprint) {
            Ok(true) => {
                debug!(
                    "Skipping duplicate regression issue for {} (fingerprint {fingerprint})",
                    finding.scenario_id
                );
                continue;
            }
            Ok(false) => { /* proceed */ }
            Err(e) => {
                debug!(
                    "Fingerprint check failed for {} — proceeding: {e}",
                    finding.scenario_id
                );
            }
        }

        let title = build_regression_issue_title(finding);
        let body = build_regression_issue_body(
            finding,
            &report.after_run_id,
            &report.before_run_id,
            &fingerprint,
        );
        let labels = build_regression_labels(finding);

        match creator.create_issue(owner, repo_name, &title, &body, &labels) {
            Ok((issue_number, _url)) => {
                info!(
                    "Created regression issue #{issue_number} for scenario {}",
                    finding.scenario_id
                );
                created_count += 1;
            }
            Err(e) => {
                error!(
                    "Failed to create regression issue for {}: {e}",
                    finding.scenario_id
                );
            }
        }
    }

    info!(
        "Regression issue creation complete: {created_count}/{} created",
        report.regressions.len()
    );

    Ok(created_count)
}

// ---------------------------------------------------------------------------
// Full cycle verification (I/O)
// ---------------------------------------------------------------------------

/// Verify a SAT cycle after a fix is merged.
///
/// This is the top-level I/O function that:
/// 1. Builds a regression report from the score comparison and context
/// 2. Writes the report to disk
/// 3. Creates GitHub issues for any regressions (loop continuation)
/// 4. Returns the report with the verification outcome
///
/// # Errors
/// Returns `AppError::Sat` on fatal errors (can't write report, etc.).
pub fn verify_cycle(
    comparison: &ScoreComparison,
    context: &PostMergeRescoreContext,
    run_dir: &Path,
    owner: &str,
    repo_name: &str,
    creator: &dyn IssueCreator,
) -> Result<RegressionReport, AppError> {
    let generated_at = chrono::Utc::now().to_rfc3339();
    let report = build_regression_report(comparison, context, &generated_at);

    info!(
        "Cycle verification for PR #{}: outcome={}, {} regressions",
        context.merged_pr_number,
        report.outcome,
        report.regressions.len()
    );

    write_regression_report(&report, run_dir)?;

    match report.outcome {
        VerificationOutcome::Verified => {
            info!(
                "Cycle verified: all scenarios improved or unchanged after PR #{}",
                context.merged_pr_number
            );
        }
        VerificationOutcome::Regressed | VerificationOutcome::Mixed => {
            let issues_created = create_regression_issues(&report, owner, repo_name, creator)?;
            info!("Created {issues_created} regression issues to continue the loop");
        }
    }

    Ok(report)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::sat::{
        DimensionDeltas, PostMergeRescoreContext, ScenarioComparison, ScoreComparison,
    };

    fn make_comparison(scenarios: Vec<ScenarioComparison>, overall_delta: i32) -> ScoreComparison {
        let improved_count = scenarios.iter().filter(|s| s.delta > 0).count();
        let regressed_count = scenarios.iter().filter(|s| s.delta < 0).count();
        let unchanged_count = scenarios.iter().filter(|s| s.delta == 0).count();

        #[allow(clippy::cast_sign_loss)]
        let overall_after = if overall_delta >= 0 {
            50 + overall_delta as u32
        } else {
            50 - overall_delta.unsigned_abs()
        };

        ScoreComparison {
            before_run_id: "run-before".into(),
            after_run_id: "run-after".into(),
            compared_at: "2026-03-26T12:00:00Z".into(),
            scenario_comparisons: scenarios,
            persona_deltas: Vec::new(),
            overall_before: 50,
            overall_after,
            overall_delta,
            improved_count,
            regressed_count,
            unchanged_count,
        }
    }

    fn make_scenario(id: &str, persona: &str, before: u32, after: u32) -> ScenarioComparison {
        #[allow(clippy::cast_possible_wrap)]
        let delta = after as i32 - before as i32;
        ScenarioComparison {
            scenario_id: id.into(),
            persona: persona.into(),
            before_score: before,
            after_score: after,
            delta,
            dimension_deltas: DimensionDeltas {
                functionality: delta,
                usability: 0,
                error_handling: 0,
                performance: 0,
            },
        }
    }

    fn make_context(pr_number: u64) -> PostMergeRescoreContext {
        PostMergeRescoreContext {
            repo: "owner/repo".into(),
            merged_pr_number: pr_number,
            merged_branch: "fix/issue-42".into(),
            scenario_filter: Vec::new(),
            original_issue_number: Some(42),
            original_run_id: Some("run-before".into()),
        }
    }

    // -- detect_regressions ---------------------------------------------------

    #[test]
    fn detect_regressions_finds_negative_deltas() {
        let comparison = make_comparison(
            vec![
                make_scenario("s1", "newcomer", 80, 60),   // regression: -20
                make_scenario("s2", "power-user", 50, 85), // improvement: +35
                make_scenario("s3", "newcomer", 70, 40),   // regression: -30
            ],
            -5,
        );

        let regressions = detect_regressions(&comparison, 42, "owner/repo");

        assert_eq!(regressions.len(), 2);

        let r1 = &regressions[0];
        assert_eq!(r1.scenario_id, "s1");
        assert_eq!(r1.persona, "newcomer");
        assert_eq!(r1.before_score, 80);
        assert_eq!(r1.after_score, 60);
        assert_eq!(r1.regression_magnitude, 20);
        assert_eq!(r1.suspected_pr_number, 42);
        assert_eq!(r1.repo, "owner/repo");

        let r2 = &regressions[1];
        assert_eq!(r2.scenario_id, "s3");
        assert_eq!(r2.regression_magnitude, 30);
    }

    #[test]
    fn detect_regressions_empty_when_all_improved() {
        let comparison = make_comparison(
            vec![
                make_scenario("s1", "newcomer", 30, 70),
                make_scenario("s2", "power-user", 50, 85),
            ],
            30,
        );

        let regressions = detect_regressions(&comparison, 42, "owner/repo");
        assert!(regressions.is_empty());
    }

    #[test]
    fn detect_regressions_empty_when_unchanged() {
        let comparison = make_comparison(vec![make_scenario("s1", "newcomer", 75, 75)], 0);

        let regressions = detect_regressions(&comparison, 42, "owner/repo");
        assert!(regressions.is_empty());
    }

    // -- determine_outcome ----------------------------------------------------

    #[test]
    fn outcome_verified_when_no_regressions() {
        let comparison = make_comparison(
            vec![
                make_scenario("s1", "newcomer", 30, 70),
                make_scenario("s2", "power-user", 50, 85),
            ],
            30,
        );
        assert_eq!(
            determine_outcome(&comparison),
            VerificationOutcome::Verified
        );
    }

    #[test]
    fn outcome_verified_when_all_unchanged() {
        let comparison = make_comparison(vec![make_scenario("s1", "newcomer", 75, 75)], 0);
        assert_eq!(
            determine_outcome(&comparison),
            VerificationOutcome::Verified
        );
    }

    #[test]
    fn outcome_regressed_when_all_regressed() {
        let comparison = make_comparison(
            vec![
                make_scenario("s1", "newcomer", 80, 60),
                make_scenario("s2", "power-user", 70, 50),
            ],
            -20,
        );
        assert_eq!(
            determine_outcome(&comparison),
            VerificationOutcome::Regressed
        );
    }

    #[test]
    fn outcome_mixed_when_some_improved_some_regressed() {
        let comparison = make_comparison(
            vec![
                make_scenario("s1", "newcomer", 80, 60),   // regression
                make_scenario("s2", "power-user", 50, 85), // improvement
            ],
            5,
        );
        assert_eq!(determine_outcome(&comparison), VerificationOutcome::Mixed);
    }

    // -- build_regression_report -----------------------------------------------

    #[test]
    fn report_captures_all_fields() {
        let comparison = make_comparison(
            vec![
                make_scenario("s1", "newcomer", 80, 60),
                make_scenario("s2", "power-user", 50, 85),
                make_scenario("s3", "newcomer", 70, 70),
            ],
            5,
        );
        let context = make_context(42);

        let report = build_regression_report(&comparison, &context, "2026-03-26T12:00:00Z");

        assert_eq!(report.after_run_id, "run-after");
        assert_eq!(report.before_run_id, "run-before");
        assert_eq!(report.merged_pr_number, 42);
        assert_eq!(report.repo, "owner/repo");
        assert_eq!(report.outcome, VerificationOutcome::Mixed);
        assert_eq!(report.improved_count, 1);
        assert_eq!(report.regressed_count, 1);
        assert_eq!(report.unchanged_count, 1);
        assert_eq!(report.regressions.len(), 1);
        assert_eq!(report.regressions[0].scenario_id, "s1");
    }

    #[test]
    fn report_verified_has_empty_regressions() {
        let comparison = make_comparison(vec![make_scenario("s1", "newcomer", 30, 70)], 40);
        let context = make_context(42);

        let report = build_regression_report(&comparison, &context, "2026-03-26T12:00:00Z");

        assert_eq!(report.outcome, VerificationOutcome::Verified);
        assert!(report.regressions.is_empty());
    }

    // -- Issue title/body/labels -----------------------------------------------

    #[test]
    fn regression_issue_title_contains_key_info() {
        let finding = RegressionFinding {
            scenario_id: "newcomer-first-launch".into(),
            persona: "confused-newcomer".into(),
            before_score: 80,
            after_score: 55,
            regression_magnitude: 25,
            dimension_deltas: DimensionDeltas {
                functionality: -20,
                usability: -5,
                error_handling: 0,
                performance: 0,
            },
            suspected_pr_number: 42,
            repo: "owner/repo".into(),
        };

        let title = build_regression_issue_title(&finding);
        assert!(title.contains("[SAT/Regression]"));
        assert!(title.contains("newcomer-first-launch"));
        assert!(title.contains("25 points"));
        assert!(title.contains("PR #42"));
    }

    #[test]
    fn regression_issue_body_contains_required_fields() {
        let finding = RegressionFinding {
            scenario_id: "newcomer-first-launch".into(),
            persona: "confused-newcomer".into(),
            before_score: 80,
            after_score: 55,
            regression_magnitude: 25,
            dimension_deltas: DimensionDeltas {
                functionality: -20,
                usability: -5,
                error_handling: 0,
                performance: 0,
            },
            suspected_pr_number: 42,
            repo: "owner/repo".into(),
        };

        let body = build_regression_issue_body(&finding, "run-after", "run-before", "fp123");

        // Required fields from acceptance criteria
        assert!(body.contains("newcomer-first-launch")); // which scenario
        assert!(body.contains("80/100")); // before score
        assert!(body.contains("55/100")); // after score
        assert!(body.contains("-25 points")); // by how much
        assert!(body.contains("#42")); // suspected cause PR
        assert!(body.contains("confused-newcomer")); // persona
        assert!(body.contains("run-after")); // after run
        assert!(body.contains("run-before")); // before run
        assert!(body.contains("sat-fingerprint:fp123")); // dedup fingerprint

        // Dimension breakdown
        assert!(body.contains("Functionality"));
        assert!(body.contains("-20"));
        assert!(body.contains("Usability"));
        assert!(body.contains("-5"));
    }

    #[test]
    fn regression_labels_include_required() {
        let finding = RegressionFinding {
            scenario_id: "s1".into(),
            persona: "newcomer".into(),
            before_score: 80,
            after_score: 60,
            regression_magnitude: 20,
            dimension_deltas: DimensionDeltas {
                functionality: -20,
                usability: 0,
                error_handling: 0,
                performance: 0,
            },
            suspected_pr_number: 42,
            repo: "owner/repo".into(),
        };

        let labels = build_regression_labels(&finding);
        assert!(labels.contains(&"agent:implement".to_string()));
        assert!(labels.contains(&"sat:regression".to_string()));
        assert!(labels.contains(&"regression-from:pr-42".to_string()));
    }

    // -- I/O: write regression report -----------------------------------------

    #[test]
    fn write_and_read_regression_report_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path();

        let comparison = make_comparison(
            vec![
                make_scenario("s1", "newcomer", 80, 60),
                make_scenario("s2", "power-user", 50, 85),
            ],
            5,
        );
        let context = make_context(42);
        let report = build_regression_report(&comparison, &context, "2026-03-26T12:00:00Z");

        let path = write_regression_report(&report, run_dir).unwrap();
        assert!(path.exists());
        assert_eq!(path.file_name().unwrap(), "regression-report.json");

        // Read back and verify
        let raw_json = std::fs::read_to_string(&path).unwrap();
        let loaded: RegressionReport = serde_json::from_str(&raw_json).unwrap();
        assert_eq!(loaded.after_run_id, "run-after");
        assert_eq!(loaded.outcome, VerificationOutcome::Mixed);
        assert_eq!(loaded.regressions.len(), 1);
        assert_eq!(loaded.regressions[0].scenario_id, "s1");
        assert_eq!(loaded.regressions[0].regression_magnitude, 20);
    }

    // -- Full cycle verification with mock -------------------------------------

    struct MockCreator {
        existing_fingerprints: Vec<String>,
        created: std::cell::RefCell<Vec<(String, String, Vec<String>)>>,
    }

    impl MockCreator {
        fn new(existing_fingerprints: Vec<String>) -> Self {
            Self {
                existing_fingerprints,
                created: std::cell::RefCell::new(Vec::new()),
            }
        }
    }

    impl IssueCreator for MockCreator {
        fn create_issue(
            &self,
            _owner: &str,
            _repo: &str,
            title: &str,
            body: &str,
            labels: &[String],
        ) -> Result<(u64, String), AppError> {
            let num = self.created.borrow().len() as u64 + 100;
            self.created
                .borrow_mut()
                .push((title.to_string(), body.to_string(), labels.to_vec()));
            Ok((num, format!("https://github.com/test/repo/issues/{num}")))
        }

        fn issue_exists_with_fingerprint(
            &self,
            _owner: &str,
            _repo: &str,
            fingerprint: &str,
        ) -> Result<bool, AppError> {
            Ok(self
                .existing_fingerprints
                .contains(&fingerprint.to_string()))
        }
    }

    #[test]
    fn verify_cycle_verified_creates_no_issues() {
        let tmp = tempfile::TempDir::new().unwrap();
        let comparison = make_comparison(
            vec![
                make_scenario("s1", "newcomer", 30, 70),
                make_scenario("s2", "power-user", 50, 85),
            ],
            30,
        );
        let context = make_context(42);
        let creator = MockCreator::new(Vec::new());

        let report =
            verify_cycle(&comparison, &context, tmp.path(), "owner", "repo", &creator).unwrap();

        assert_eq!(report.outcome, VerificationOutcome::Verified);
        assert!(report.regressions.is_empty());
        assert!(creator.created.borrow().is_empty());

        // Report file should exist
        assert!(tmp.path().join("regression-report.json").exists());
    }

    #[test]
    fn verify_cycle_regressed_creates_issues() {
        let tmp = tempfile::TempDir::new().unwrap();
        let comparison = make_comparison(
            vec![
                make_scenario("s1", "newcomer", 80, 60),
                make_scenario("s2", "power-user", 70, 50),
            ],
            -20,
        );
        let context = make_context(42);
        let creator = MockCreator::new(Vec::new());

        let report =
            verify_cycle(&comparison, &context, tmp.path(), "owner", "repo", &creator).unwrap();

        assert_eq!(report.outcome, VerificationOutcome::Regressed);
        assert_eq!(report.regressions.len(), 2);
        assert_eq!(creator.created.borrow().len(), 2);

        // Verify the created issues have correct labels
        let (title, body, labels) = &creator.created.borrow()[0];
        assert!(title.contains("[SAT/Regression]"));
        assert!(body.contains("sat-fingerprint:"));
        assert!(labels.contains(&"agent:implement".to_string()));
        assert!(labels.contains(&"sat:regression".to_string()));
    }

    #[test]
    fn verify_cycle_mixed_creates_issues_for_regressions_only() {
        let tmp = tempfile::TempDir::new().unwrap();
        let comparison = make_comparison(
            vec![
                make_scenario("s1", "newcomer", 80, 60),   // regression
                make_scenario("s2", "power-user", 50, 85), // improvement
                make_scenario("s3", "newcomer", 70, 70),   // unchanged
            ],
            5,
        );
        let context = make_context(42);
        let creator = MockCreator::new(Vec::new());

        let report =
            verify_cycle(&comparison, &context, tmp.path(), "owner", "repo", &creator).unwrap();

        assert_eq!(report.outcome, VerificationOutcome::Mixed);
        // Only 1 regression issue should be created (s1)
        assert_eq!(creator.created.borrow().len(), 1);
        let (title, _, _) = &creator.created.borrow()[0];
        assert!(title.contains("s1"));
    }

    #[test]
    fn verify_cycle_skips_duplicate_regression_issues() {
        let tmp = tempfile::TempDir::new().unwrap();
        let comparison = make_comparison(vec![make_scenario("s1", "newcomer", 80, 60)], -20);
        let context = make_context(42);

        // Pre-populate the fingerprint for s1
        let fingerprint =
            crate::services::sat_issues::generate_fingerprint("s1", "newcomer", "run-after");
        let creator = MockCreator::new(vec![fingerprint]);

        let report =
            verify_cycle(&comparison, &context, tmp.path(), "owner", "repo", &creator).unwrap();

        assert_eq!(report.outcome, VerificationOutcome::Regressed);
        assert_eq!(report.regressions.len(), 1);
        // But no issues created (duplicate)
        assert!(creator.created.borrow().is_empty());
    }

    // -- Serialization --------------------------------------------------------

    #[test]
    fn regression_report_json_round_trip() {
        let report = RegressionReport {
            after_run_id: "run-after".into(),
            before_run_id: "run-before".into(),
            generated_at: "2026-03-26T12:00:00Z".into(),
            merged_pr_number: 42,
            repo: "owner/repo".into(),
            outcome: VerificationOutcome::Mixed,
            overall_delta: -5,
            improved_count: 1,
            regressed_count: 2,
            unchanged_count: 0,
            regressions: vec![RegressionFinding {
                scenario_id: "s1".into(),
                persona: "newcomer".into(),
                before_score: 80,
                after_score: 60,
                regression_magnitude: 20,
                dimension_deltas: DimensionDeltas {
                    functionality: -20,
                    usability: 0,
                    error_handling: 0,
                    performance: 0,
                },
                suspected_pr_number: 42,
                repo: "owner/repo".into(),
            }],
        };

        let json = serde_json::to_string_pretty(&report).unwrap();
        let parsed: RegressionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.outcome, VerificationOutcome::Mixed);
        assert_eq!(parsed.regressions.len(), 1);
        assert_eq!(parsed.regressions[0].regression_magnitude, 20);
    }

    #[test]
    fn verification_outcome_display() {
        assert_eq!(VerificationOutcome::Verified.to_string(), "verified");
        assert_eq!(VerificationOutcome::Regressed.to_string(), "regressed");
        assert_eq!(VerificationOutcome::Mixed.to_string(), "mixed");
    }
}
