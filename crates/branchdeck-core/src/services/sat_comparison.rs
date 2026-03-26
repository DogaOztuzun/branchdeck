//! SAT before/after score comparison service (Story 4.2).
//!
//! Compares satisfaction scores from a pre-fix baseline run against a
//! post-merge re-score run to produce concrete evidence of improvement.
//!
//! Architecture:
//! - Pure functions: `compute_comparison`, `compute_scenario_comparison`,
//!   `aggregate_persona_deltas` — no I/O, fully testable
//! - I/O functions: `read_score_result`, `write_comparison` — thin wrappers

use std::path::{Path, PathBuf};

use log::{debug, error, info};

use crate::error::AppError;
use crate::models::sat::{
    DimensionDeltas, PersonaScoreDelta, SatScoreResult, ScenarioComparison, ScoreComparison,
};

// ---------------------------------------------------------------------------
// Pure comparison logic
// ---------------------------------------------------------------------------

/// Convert a u32 score (0-100) to i32 for delta computation.
/// Scores are clamped to 0-100, so this is always safe.
#[allow(clippy::cast_possible_wrap)]
fn score_as_i32(score: u32) -> i32 {
    score as i32
}

/// Compute per-dimension deltas between before and after scenario scores.
#[must_use]
pub fn compute_dimension_deltas(
    before: &crate::models::sat::SatScoreDimensions,
    after: &crate::models::sat::SatScoreDimensions,
) -> DimensionDeltas {
    DimensionDeltas {
        functionality: score_as_i32(after.functionality) - score_as_i32(before.functionality),
        usability: score_as_i32(after.usability) - score_as_i32(before.usability),
        error_handling: score_as_i32(after.error_handling) - score_as_i32(before.error_handling),
        performance: score_as_i32(after.performance) - score_as_i32(before.performance),
    }
}

/// Compute a per-scenario comparison between before and after scores.
///
/// Only produces a comparison for scenarios present in both runs.
#[must_use]
pub fn compute_scenario_comparisons(
    before: &SatScoreResult,
    after: &SatScoreResult,
) -> Vec<ScenarioComparison> {
    let mut comparisons = Vec::new();

    for after_score in &after.scenario_scores {
        if let Some(before_score) = before
            .scenario_scores
            .iter()
            .find(|s| s.scenario_id == after_score.scenario_id)
        {
            let delta = score_as_i32(after_score.score) - score_as_i32(before_score.score);
            let dimension_deltas =
                compute_dimension_deltas(&before_score.dimensions, &after_score.dimensions);

            comparisons.push(ScenarioComparison {
                scenario_id: after_score.scenario_id.clone(),
                persona: after_score.persona.clone(),
                before_score: before_score.score,
                after_score: after_score.score,
                delta,
                dimension_deltas,
            });
        }
    }

    comparisons
}

/// Aggregate per-persona score deltas from scenario comparisons.
///
/// Groups scenarios by persona and averages the before/after scores
/// for each persona.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_precision_loss)]
pub fn aggregate_persona_deltas(comparisons: &[ScenarioComparison]) -> Vec<PersonaScoreDelta> {
    use std::collections::BTreeMap;

    // Group by persona: (sum_before, sum_after, count)
    let mut groups: BTreeMap<String, (u64, u64, u64)> = BTreeMap::new();
    for comp in comparisons {
        let entry = groups.entry(comp.persona.clone()).or_insert((0, 0, 0));
        entry.0 += u64::from(comp.before_score);
        entry.1 += u64::from(comp.after_score);
        entry.2 += 1;
    }

    groups
        .into_iter()
        .map(|(persona, (sum_before, sum_after, count))| {
            let before = (sum_before as f64 / count as f64).round() as u32;
            let after = (sum_after as f64 / count as f64).round() as u32;
            PersonaScoreDelta {
                persona,
                before,
                after,
                delta: score_as_i32(after) - score_as_i32(before),
            }
        })
        .collect()
}

/// Compute a full before/after score comparison.
///
/// This is the main pure function. Takes two `SatScoreResult` values
/// (before and after) plus a timestamp, and produces a `ScoreComparison`.
#[must_use]
pub fn compute_comparison(
    before: &SatScoreResult,
    after: &SatScoreResult,
    compared_at: &str,
) -> ScoreComparison {
    let scenario_comparisons = compute_scenario_comparisons(before, after);
    let persona_deltas = aggregate_persona_deltas(&scenario_comparisons);

    let improved_count = scenario_comparisons.iter().filter(|c| c.delta > 0).count();
    let regressed_count = scenario_comparisons.iter().filter(|c| c.delta < 0).count();
    let unchanged_count = scenario_comparisons.iter().filter(|c| c.delta == 0).count();

    let overall_delta =
        score_as_i32(after.aggregate_score) - score_as_i32(before.aggregate_score);

    ScoreComparison {
        before_run_id: before.run_id.clone(),
        after_run_id: after.run_id.clone(),
        compared_at: compared_at.to_string(),
        scenario_comparisons,
        persona_deltas,
        overall_before: before.aggregate_score,
        overall_after: after.aggregate_score,
        overall_delta,
        improved_count,
        regressed_count,
        unchanged_count,
    }
}

// ---------------------------------------------------------------------------
// I/O: read score results, write comparison
// ---------------------------------------------------------------------------

/// Read a `SatScoreResult` from `scores.json` in a run directory.
///
/// # Errors
/// Returns `AppError::Sat` if the file cannot be read or parsed.
pub fn read_score_result(run_dir: &Path) -> Result<SatScoreResult, AppError> {
    let path = run_dir.join("scores.json");
    let content = std::fs::read_to_string(&path).map_err(|e| {
        error!(
            "Failed to read score result from {}: {e}",
            path.display()
        );
        AppError::Sat(format!("failed to read scores: {e}"))
    })?;
    serde_json::from_str(&content).map_err(|e| {
        error!("Failed to parse score result JSON from {}: {e}", path.display());
        AppError::Sat(format!("score result parse error: {e}"))
    })
}

/// Resolve the run directory for a given run ID within the runs root.
#[must_use]
pub fn run_dir_for_id(runs_dir: &Path, run_id: &str) -> PathBuf {
    runs_dir.join(run_id)
}

/// Write a `ScoreComparison` as JSON to `comparison.json` in the after-run directory.
///
/// # Errors
/// Returns `AppError::Sat` if serialization or file write fails.
pub fn write_comparison(
    comparison: &ScoreComparison,
    run_dir: &Path,
) -> Result<PathBuf, AppError> {
    let path = run_dir.join("comparison.json");
    let json = serde_json::to_string_pretty(comparison).map_err(|e| {
        error!("Failed to serialize score comparison: {e}");
        AppError::Sat(format!("comparison serialization error: {e}"))
    })?;
    crate::util::write_atomic(&path, json.as_bytes()).map_err(|e| {
        error!("Failed to write comparison to {}: {e}", path.display());
        AppError::Sat(format!("failed to write comparison: {e}"))
    })?;
    info!("Wrote score comparison to {}", path.display());
    Ok(path)
}

/// Compare scores between two SAT runs end-to-end.
///
/// Reads the score results from both run directories, computes the comparison,
/// and writes `comparison.json` to the after-run directory.
///
/// # Arguments
/// * `runs_dir` — Root directory containing run subdirectories (`sat/runs/`).
/// * `before_run_id` — Run ID of the original (pre-fix) SAT run.
/// * `after_run_id` — Run ID of the post-merge re-score run.
///
/// # Errors
/// Returns `AppError::Sat` if either score file cannot be read, or if writing fails.
pub fn compare_runs(
    runs_dir: &Path,
    before_run_id: &str,
    after_run_id: &str,
) -> Result<ScoreComparison, AppError> {
    let before_dir = run_dir_for_id(runs_dir, before_run_id);
    let after_dir = run_dir_for_id(runs_dir, after_run_id);

    info!(
        "Comparing SAT scores: {before_run_id} (before) vs {after_run_id} (after)"
    );

    let before = read_score_result(&before_dir)?;
    let after = read_score_result(&after_dir)?;

    debug!(
        "Before: {}/100 ({} scenarios), After: {}/100 ({} scenarios)",
        before.aggregate_score,
        before.scenario_scores.len(),
        after.aggregate_score,
        after.scenario_scores.len(),
    );

    let compared_at = chrono::Utc::now().to_rfc3339();
    let comparison = compute_comparison(&before, &after, &compared_at);

    write_comparison(&comparison, &after_dir)?;

    info!(
        "Score comparison complete: {}/100 -> {}/100 ({:+}) — {} improved, {} regressed, {} unchanged",
        comparison.overall_before,
        comparison.overall_after,
        comparison.overall_delta,
        comparison.improved_count,
        comparison.regressed_count,
        comparison.unchanged_count,
    );

    Ok(comparison)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::sat::{
        FindingCounts, SatScenarioScore, SatScoreDimensions, SatScoreResult, TokenUsage,
    };

    fn make_dimensions(func: u32, usab: u32, err: u32, perf: u32) -> SatScoreDimensions {
        SatScoreDimensions {
            functionality: func,
            usability: usab,
            error_handling: err,
            performance: perf,
        }
    }

    fn make_scenario_score(id: &str, persona: &str, score: u32, dims: SatScoreDimensions) -> SatScenarioScore {
        SatScenarioScore {
            scenario_id: id.to_string(),
            persona: persona.to_string(),
            score,
            dimensions: dims,
            reasoning: String::new(),
            findings: Vec::new(),
        }
    }

    fn make_score_result(run_id: &str, scores: Vec<SatScenarioScore>, aggregate: u32) -> SatScoreResult {
        SatScoreResult {
            run_id: run_id.to_string(),
            scored_at: "2026-03-26T00:00:00Z".to_string(),
            scenario_scores: scores,
            aggregate_score: aggregate,
            all_findings: Vec::new(),
            finding_counts: FindingCounts::default(),
            token_usage: TokenUsage { input_tokens: 0, output_tokens: 0 },
            estimated_cost_dollars: 0.0,
        }
    }

    // -- Dimension deltas -----------------------------------------------------

    #[test]
    fn dimension_deltas_positive_improvement() {
        let before = make_dimensions(40, 30, 20, 50);
        let after = make_dimensions(80, 70, 60, 90);
        let deltas = compute_dimension_deltas(&before, &after);
        assert_eq!(deltas.functionality, 40);
        assert_eq!(deltas.usability, 40);
        assert_eq!(deltas.error_handling, 40);
        assert_eq!(deltas.performance, 40);
    }

    #[test]
    fn dimension_deltas_regression() {
        let before = make_dimensions(80, 70, 60, 90);
        let after = make_dimensions(40, 30, 20, 50);
        let deltas = compute_dimension_deltas(&before, &after);
        assert_eq!(deltas.functionality, -40);
        assert_eq!(deltas.usability, -40);
        assert_eq!(deltas.error_handling, -40);
        assert_eq!(deltas.performance, -40);
    }

    // -- Scenario comparisons -------------------------------------------------

    #[test]
    fn scenario_comparisons_matched_only() {
        let before = make_score_result("run-before", vec![
            make_scenario_score("s1", "newcomer", 31, make_dimensions(30, 25, 20, 40)),
            make_scenario_score("s2", "power-user", 70, make_dimensions(80, 60, 70, 65)),
            make_scenario_score("s3", "newcomer", 50, make_dimensions(50, 50, 50, 50)),
        ], 50);

        let after = make_score_result("run-after", vec![
            make_scenario_score("s1", "newcomer", 74, make_dimensions(80, 70, 60, 80)),
            make_scenario_score("s2", "power-user", 85, make_dimensions(90, 80, 85, 80)),
            // s3 is not in the after run (not re-scored)
        ], 80);

        let comparisons = compute_scenario_comparisons(&before, &after);
        assert_eq!(comparisons.len(), 2);

        let s1 = &comparisons[0];
        assert_eq!(s1.scenario_id, "s1");
        assert_eq!(s1.persona, "newcomer");
        assert_eq!(s1.before_score, 31);
        assert_eq!(s1.after_score, 74);
        assert_eq!(s1.delta, 43);

        let s2 = &comparisons[1];
        assert_eq!(s2.scenario_id, "s2");
        assert_eq!(s2.delta, 15);
    }

    #[test]
    fn scenario_comparisons_no_overlap() {
        let before = make_score_result("run-before", vec![
            make_scenario_score("s1", "newcomer", 50, make_dimensions(50, 50, 50, 50)),
        ], 50);
        let after = make_score_result("run-after", vec![
            make_scenario_score("s99", "expert", 80, make_dimensions(80, 80, 80, 80)),
        ], 80);

        let comparisons = compute_scenario_comparisons(&before, &after);
        assert!(comparisons.is_empty());
    }

    // -- Persona deltas -------------------------------------------------------

    #[test]
    fn persona_deltas_aggregated() {
        let comparisons = vec![
            ScenarioComparison {
                scenario_id: "s1".into(),
                persona: "newcomer".into(),
                before_score: 30,
                after_score: 70,
                delta: 40,
                dimension_deltas: DimensionDeltas { functionality: 40, usability: 40, error_handling: 40, performance: 40 },
            },
            ScenarioComparison {
                scenario_id: "s2".into(),
                persona: "newcomer".into(),
                before_score: 40,
                after_score: 80,
                delta: 40,
                dimension_deltas: DimensionDeltas { functionality: 40, usability: 40, error_handling: 40, performance: 40 },
            },
            ScenarioComparison {
                scenario_id: "s3".into(),
                persona: "power-user".into(),
                before_score: 70,
                after_score: 85,
                delta: 15,
                dimension_deltas: DimensionDeltas { functionality: 15, usability: 15, error_handling: 15, performance: 15 },
            },
        ];

        let deltas = aggregate_persona_deltas(&comparisons);
        assert_eq!(deltas.len(), 2);

        // BTreeMap ensures sorted order
        let newcomer = &deltas[0];
        assert_eq!(newcomer.persona, "newcomer");
        assert_eq!(newcomer.before, 35); // (30+40)/2 = 35
        assert_eq!(newcomer.after, 75);  // (70+80)/2 = 75
        assert_eq!(newcomer.delta, 40);

        let power_user = &deltas[1];
        assert_eq!(power_user.persona, "power-user");
        assert_eq!(power_user.before, 70);
        assert_eq!(power_user.after, 85);
        assert_eq!(power_user.delta, 15);
    }

    #[test]
    fn persona_deltas_empty_comparisons() {
        let deltas = aggregate_persona_deltas(&[]);
        assert!(deltas.is_empty());
    }

    // -- Full comparison ------------------------------------------------------

    #[test]
    fn compute_comparison_full() {
        let before = make_score_result("run-001", vec![
            make_scenario_score("s1", "confused-newcomer", 31, make_dimensions(30, 25, 20, 40)),
            make_scenario_score("s2", "power-user", 70, make_dimensions(80, 60, 70, 65)),
        ], 50);

        let after = make_score_result("run-002", vec![
            make_scenario_score("s1", "confused-newcomer", 74, make_dimensions(80, 70, 60, 80)),
            make_scenario_score("s2", "power-user", 85, make_dimensions(90, 80, 85, 80)),
        ], 80);

        let comparison = compute_comparison(&before, &after, "2026-03-26T12:00:00Z");

        assert_eq!(comparison.before_run_id, "run-001");
        assert_eq!(comparison.after_run_id, "run-002");
        assert_eq!(comparison.overall_before, 50);
        assert_eq!(comparison.overall_after, 80);
        assert_eq!(comparison.overall_delta, 30);
        assert_eq!(comparison.improved_count, 2);
        assert_eq!(comparison.regressed_count, 0);
        assert_eq!(comparison.unchanged_count, 0);
        assert_eq!(comparison.scenario_comparisons.len(), 2);
        assert_eq!(comparison.persona_deltas.len(), 2);

        // Check the confused-newcomer delta specifically (from acceptance criteria)
        let newcomer = comparison.persona_deltas.iter()
            .find(|d| d.persona == "confused-newcomer")
            .expect("confused-newcomer persona delta should exist");
        assert_eq!(newcomer.before, 31);
        assert_eq!(newcomer.after, 74);
        assert_eq!(newcomer.delta, 43);
    }

    #[test]
    fn compute_comparison_with_regression() {
        let before = make_score_result("run-001", vec![
            make_scenario_score("s1", "user", 80, make_dimensions(80, 80, 80, 80)),
            make_scenario_score("s2", "user", 60, make_dimensions(60, 60, 60, 60)),
        ], 70);

        let after = make_score_result("run-002", vec![
            make_scenario_score("s1", "user", 60, make_dimensions(60, 60, 60, 60)),
            make_scenario_score("s2", "user", 90, make_dimensions(90, 90, 90, 90)),
        ], 75);

        let comparison = compute_comparison(&before, &after, "2026-03-26T12:00:00Z");

        assert_eq!(comparison.overall_delta, 5);
        assert_eq!(comparison.improved_count, 1);
        assert_eq!(comparison.regressed_count, 1);
        assert_eq!(comparison.unchanged_count, 0);
    }

    #[test]
    fn compute_comparison_unchanged() {
        let before = make_score_result("run-001", vec![
            make_scenario_score("s1", "user", 75, make_dimensions(75, 75, 75, 75)),
        ], 75);

        let after = make_score_result("run-002", vec![
            make_scenario_score("s1", "user", 75, make_dimensions(75, 75, 75, 75)),
        ], 75);

        let comparison = compute_comparison(&before, &after, "2026-03-26T12:00:00Z");

        assert_eq!(comparison.overall_delta, 0);
        assert_eq!(comparison.improved_count, 0);
        assert_eq!(comparison.regressed_count, 0);
        assert_eq!(comparison.unchanged_count, 1);
    }

    // -- I/O round-trip -------------------------------------------------------

    #[test]
    fn write_and_read_comparison_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path();

        let before = make_score_result("run-001", vec![
            make_scenario_score("s1", "newcomer", 31, make_dimensions(30, 25, 20, 40)),
        ], 31);
        let after = make_score_result("run-002", vec![
            make_scenario_score("s1", "newcomer", 74, make_dimensions(80, 70, 60, 80)),
        ], 74);

        let comparison = compute_comparison(&before, &after, "2026-03-26T12:00:00Z");
        let path = write_comparison(&comparison, run_dir).unwrap();

        assert!(path.exists());
        assert_eq!(path.file_name().unwrap(), "comparison.json");

        // Read back and verify it's valid JSON
        let content = std::fs::read_to_string(&path).unwrap();
        let loaded: ScoreComparison = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.before_run_id, "run-001");
        assert_eq!(loaded.after_run_id, "run-002");
        assert_eq!(loaded.overall_delta, 43);
        assert_eq!(loaded.scenario_comparisons.len(), 1);
        assert_eq!(loaded.persona_deltas.len(), 1);
    }

    // -- Serialization --------------------------------------------------------

    #[test]
    fn comparison_json_is_machine_readable() {
        let comparison = ScoreComparison {
            before_run_id: "run-001".into(),
            after_run_id: "run-002".into(),
            compared_at: "2026-03-26T12:00:00Z".into(),
            scenario_comparisons: vec![ScenarioComparison {
                scenario_id: "s1".into(),
                persona: "confused-newcomer".into(),
                before_score: 31,
                after_score: 74,
                delta: 43,
                dimension_deltas: DimensionDeltas {
                    functionality: 50,
                    usability: 45,
                    error_handling: 40,
                    performance: 40,
                },
            }],
            persona_deltas: vec![PersonaScoreDelta {
                persona: "confused-newcomer".into(),
                before: 31,
                after: 74,
                delta: 43,
            }],
            overall_before: 31,
            overall_after: 74,
            overall_delta: 43,
            improved_count: 1,
            regressed_count: 0,
            unchanged_count: 0,
        };

        let json = serde_json::to_string_pretty(&comparison).unwrap();

        // Verify all required fields are present in the JSON (NFR23)
        assert!(json.contains("\"before_run_id\""));
        assert!(json.contains("\"after_run_id\""));
        assert!(json.contains("\"scenario_comparisons\""));
        assert!(json.contains("\"persona_deltas\""));
        assert!(json.contains("\"overall_delta\""));
        assert!(json.contains("\"improved_count\""));
        assert!(json.contains("\"regressed_count\""));

        // Verify it can be round-tripped
        let parsed: ScoreComparison = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.overall_delta, 43);
    }
}
