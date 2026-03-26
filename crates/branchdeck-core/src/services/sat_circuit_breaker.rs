//! SAT circuit breaker service (Story 4.4).
//!
//! Limits autonomous fix-verify iterations to prevent infinite loops.
//! Tracks cycle state, makes continue/stop decisions, persists cycle-level
//! learnings, and computes classification accuracy across cycles.
//!
//! Architecture:
//! - Pure functions: `check_circuit_breaker`, `build_cycle_learning`,
//!   `compute_classification_accuracy`, `increment_cycle_state`
//! - I/O functions: `write_cycle_learning`

use log::{debug, error, info};
use std::path::Path;

use crate::error::AppError;
use crate::models::sat::{
    CircuitBreakerConfig, CircuitBreakerDecision, CircuitBreakerState, ClassificationAccuracy,
    SatCycleLearning, SatLearningsFile, ScoreComparison, VerificationOutcome,
};
use crate::services::workflow_lifecycle::LifecycleEffect;

// ---------------------------------------------------------------------------
// Circuit breaker decisions (pure)
// ---------------------------------------------------------------------------

/// Check whether the circuit breaker allows the cycle to continue.
///
/// Compares the current iteration against the configured maximum.
/// Returns `Continue` if under the limit, `Tripped` if at or over.
///
/// This is the core decision function — no I/O, no side effects.
#[must_use]
pub fn check_circuit_breaker(
    state: &CircuitBreakerState,
    config: &CircuitBreakerConfig,
) -> CircuitBreakerDecision {
    if state.cycle_iteration >= config.max_iterations {
        debug!(
            "Circuit breaker tripped: iteration {} >= max {} for repo {}",
            state.cycle_iteration, config.max_iterations, state.repo
        );
        CircuitBreakerDecision::Tripped {
            iteration: state.cycle_iteration,
            max: config.max_iterations,
            reason: format!(
                "SAT fix-verify cycle reached iteration limit ({}/{}) for {}. \
                 Autonomous fixing has stopped. Please review manually.",
                state.cycle_iteration, config.max_iterations, state.repo
            ),
        }
    } else {
        debug!(
            "Circuit breaker allows continue: iteration {}/{} for repo {}",
            state.cycle_iteration, config.max_iterations, state.repo
        );
        CircuitBreakerDecision::Continue {
            iteration: state.cycle_iteration,
            max: config.max_iterations,
        }
    }
}

/// Increment the cycle state, returning a new state with `cycle_iteration + 1`.
#[must_use]
pub fn increment_cycle_state(state: &CircuitBreakerState) -> CircuitBreakerState {
    CircuitBreakerState {
        cycle_iteration: state.cycle_iteration + 1,
        cycle_max: state.cycle_max,
        repo: state.repo.clone(),
        original_issue_number: state.original_issue_number,
    }
}

/// Build a `LifecycleEffect` for a tripped circuit breaker.
///
/// This effect notifies the triage view that the cycle has been stopped.
#[must_use]
pub fn build_tripped_effect(decision: &CircuitBreakerDecision) -> Option<LifecycleEffect> {
    match decision {
        CircuitBreakerDecision::Tripped {
            iteration,
            max,
            reason,
        } => Some(LifecycleEffect::CircuitBreakerTripped {
            repo: String::new(), // Caller fills in from context
            iteration: *iteration,
            max_iterations: *max,
            reason: reason.clone(),
        }),
        CircuitBreakerDecision::Continue { .. } => None,
    }
}

/// Build a `LifecycleEffect` for a tripped circuit breaker with repo context.
#[must_use]
pub fn build_tripped_effect_for_repo(
    decision: &CircuitBreakerDecision,
    repo: &str,
) -> Option<LifecycleEffect> {
    match decision {
        CircuitBreakerDecision::Tripped {
            iteration,
            max,
            reason,
        } => Some(LifecycleEffect::CircuitBreakerTripped {
            repo: repo.to_string(),
            iteration: *iteration,
            max_iterations: *max,
            reason: reason.clone(),
        }),
        CircuitBreakerDecision::Continue { .. } => None,
    }
}

// ---------------------------------------------------------------------------
// Cycle learnings (pure)
// ---------------------------------------------------------------------------

/// Build a cycle-level learning entry from a score comparison and verification outcome.
///
/// Captures the aggregate results of this fix-verify iteration: how many issues
/// were found, fixed, verified, and how the score changed.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn build_cycle_learning(
    run_id: &str,
    merged_pr_number: u64,
    repo: &str,
    cycle_iteration: u32,
    comparison: &ScoreComparison,
    outcome: VerificationOutcome,
    false_positives: usize,
    recorded_at: &str,
) -> SatCycleLearning {
    // issues_found = total scenarios compared
    let issues_found = comparison.scenario_comparisons.len();
    // issues_fixed = scenarios that improved
    let issues_fixed = comparison.improved_count;
    // issues_verified = scenarios that improved or stayed the same (no regression)
    let issues_verified = comparison.improved_count + comparison.unchanged_count;

    SatCycleLearning {
        recorded_at: recorded_at.to_string(),
        run_id: run_id.to_string(),
        merged_pr_number,
        repo: repo.to_string(),
        cycle_iteration,
        issues_found,
        issues_fixed,
        issues_verified,
        false_positives,
        score_before: comparison.overall_before,
        score_after: comparison.overall_after,
        outcome,
    }
}

// ---------------------------------------------------------------------------
// Classification accuracy (pure)
// ---------------------------------------------------------------------------

/// Compute classification accuracy from accumulated cycle learnings.
///
/// Accuracy = `true_positives / (true_positives + false_positives)`
///
/// Where:
/// - `true_positives` = sum of `issues_verified` across all cycles
///   (issues correctly identified as real app bugs that were then fixed)
/// - `false_positives` = sum of `false_positives` across all cycles
///   (findings misclassified as app bugs but were actually runner/scenario issues)
///
/// Returns `None` for accuracy if there are no cycles to compute from.
#[must_use]
pub fn compute_classification_accuracy(learnings: &SatLearningsFile) -> ClassificationAccuracy {
    if learnings.cycle_learnings.is_empty() {
        return ClassificationAccuracy {
            total_classifications: 0,
            true_positives: 0,
            false_positives: 0,
            accuracy: None,
            cycles_counted: 0,
        };
    }

    let mut total_classifications: usize = 0;
    let mut true_positives: usize = 0;
    let mut false_positives: usize = 0;

    for cycle in &learnings.cycle_learnings {
        total_classifications += cycle.issues_found;
        true_positives += cycle.issues_verified;
        false_positives += cycle.false_positives;
    }

    let denominator = true_positives + false_positives;
    let accuracy = if denominator > 0 {
        #[allow(clippy::cast_precision_loss)]
        Some(true_positives as f64 / denominator as f64)
    } else {
        None
    };

    ClassificationAccuracy {
        total_classifications,
        true_positives,
        false_positives,
        accuracy,
        cycles_counted: learnings.cycle_learnings.len(),
    }
}

// ---------------------------------------------------------------------------
// Cycle learning persistence (I/O)
// ---------------------------------------------------------------------------

/// Append a cycle learning to the learnings file.
///
/// Loads the existing file, appends the new cycle learning, and writes
/// atomically (write to temp + rename via `write_atomic`).
///
/// # Errors
/// Returns `AppError::Sat` if the file cannot be read or written.
pub fn write_cycle_learning(
    learnings_path: &Path,
    learning: &SatCycleLearning,
) -> Result<(), AppError> {
    let mut file = crate::services::sat_score::load_learnings(learnings_path)?;

    file.cycle_learnings.push(learning.clone());

    let yaml = serde_yaml::to_string(&file).map_err(|e| {
        error!("Failed to serialize learnings with cycle entry: {e}");
        AppError::Sat(format!("learnings serialization error: {e}"))
    })?;

    crate::util::write_atomic(learnings_path, yaml.as_bytes()).map_err(|e| {
        error!(
            "Failed to write learnings to {}: {e}",
            learnings_path.display()
        );
        AppError::Sat(format!("failed to write learnings: {e}"))
    })?;

    info!(
        "Wrote cycle learning for PR #{} (iteration {}) to {}",
        learning.merged_pr_number,
        learning.cycle_iteration,
        learnings_path.display()
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::sat::{
        ConfidenceLevel, DimensionDeltas, FindingCategory, SatLearning, ScenarioComparison,
        ScoreComparison,
    };

    fn make_config(max: u32) -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            max_iterations: max,
        }
    }

    fn make_state(iteration: u32, max: u32) -> CircuitBreakerState {
        CircuitBreakerState {
            cycle_iteration: iteration,
            cycle_max: max,
            repo: "owner/repo".into(),
            original_issue_number: Some(42),
        }
    }

    fn make_comparison(
        improved: usize,
        regressed: usize,
        unchanged: usize,
        overall_before: u32,
        overall_after: u32,
    ) -> ScoreComparison {
        let mut scenarios = Vec::new();
        for i in 0..improved {
            scenarios.push(ScenarioComparison {
                scenario_id: format!("improved-{i}"),
                persona: "newcomer".into(),
                before_score: 40,
                after_score: 80,
                delta: 40,
                dimension_deltas: DimensionDeltas {
                    functionality: 40,
                    usability: 0,
                    error_handling: 0,
                    performance: 0,
                },
            });
        }
        for i in 0..regressed {
            scenarios.push(ScenarioComparison {
                scenario_id: format!("regressed-{i}"),
                persona: "power-user".into(),
                before_score: 80,
                after_score: 50,
                delta: -30,
                dimension_deltas: DimensionDeltas {
                    functionality: -30,
                    usability: 0,
                    error_handling: 0,
                    performance: 0,
                },
            });
        }
        for i in 0..unchanged {
            scenarios.push(ScenarioComparison {
                scenario_id: format!("unchanged-{i}"),
                persona: "newcomer".into(),
                before_score: 70,
                after_score: 70,
                delta: 0,
                dimension_deltas: DimensionDeltas {
                    functionality: 0,
                    usability: 0,
                    error_handling: 0,
                    performance: 0,
                },
            });
        }

        #[allow(clippy::cast_possible_wrap)]
        let overall_delta = overall_after as i32 - overall_before as i32;

        ScoreComparison {
            before_run_id: "run-before".into(),
            after_run_id: "run-after".into(),
            compared_at: "2026-03-26T12:00:00Z".into(),
            scenario_comparisons: scenarios,
            persona_deltas: Vec::new(),
            overall_before,
            overall_after,
            overall_delta,
            improved_count: improved,
            regressed_count: regressed,
            unchanged_count: unchanged,
        }
    }

    // -- Circuit breaker decisions -------------------------------------------

    #[test]
    fn circuit_breaker_allows_first_iteration() {
        let state = make_state(1, 3);
        let config = make_config(3);
        let decision = check_circuit_breaker(&state, &config);
        assert!(matches!(
            decision,
            CircuitBreakerDecision::Continue {
                iteration: 1,
                max: 3
            }
        ));
    }

    #[test]
    fn circuit_breaker_allows_second_iteration() {
        let state = make_state(2, 3);
        let config = make_config(3);
        let decision = check_circuit_breaker(&state, &config);
        assert!(matches!(
            decision,
            CircuitBreakerDecision::Continue {
                iteration: 2,
                max: 3
            }
        ));
    }

    #[test]
    fn circuit_breaker_trips_at_max() {
        let state = make_state(3, 3);
        let config = make_config(3);
        let decision = check_circuit_breaker(&state, &config);
        assert!(matches!(
            decision,
            CircuitBreakerDecision::Tripped {
                iteration: 3,
                max: 3,
                ..
            }
        ));
    }

    #[test]
    fn circuit_breaker_trips_over_max() {
        let state = make_state(5, 3);
        let config = make_config(3);
        let decision = check_circuit_breaker(&state, &config);
        assert!(matches!(decision, CircuitBreakerDecision::Tripped { .. }));
    }

    #[test]
    fn circuit_breaker_default_config_is_3() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.max_iterations, 3);
    }

    #[test]
    fn increment_cycle_state_bumps_iteration() {
        let state = make_state(1, 3);
        let next = increment_cycle_state(&state);
        assert_eq!(next.cycle_iteration, 2);
        assert_eq!(next.cycle_max, 3);
        assert_eq!(next.repo, "owner/repo");
    }

    // -- Tripped effect ------------------------------------------------------

    #[test]
    fn tripped_effect_some_when_tripped() {
        let decision = CircuitBreakerDecision::Tripped {
            iteration: 3,
            max: 3,
            reason: "limit reached".into(),
        };
        let effect = build_tripped_effect_for_repo(&decision, "owner/repo");
        assert!(effect.is_some());
        if let Some(LifecycleEffect::CircuitBreakerTripped {
            repo,
            iteration,
            max_iterations,
            ..
        }) = effect
        {
            assert_eq!(repo, "owner/repo");
            assert_eq!(iteration, 3);
            assert_eq!(max_iterations, 3);
        } else {
            panic!("Expected CircuitBreakerTripped effect");
        }
    }

    #[test]
    fn tripped_effect_none_when_continue() {
        let decision = CircuitBreakerDecision::Continue {
            iteration: 1,
            max: 3,
        };
        assert!(build_tripped_effect_for_repo(&decision, "owner/repo").is_none());
    }

    // -- Cycle learnings (pure) ----------------------------------------------

    #[test]
    fn build_cycle_learning_captures_counts() {
        let comparison = make_comparison(3, 1, 2, 50, 65);
        let learning = build_cycle_learning(
            "run-after",
            42,
            "owner/repo",
            1,
            &comparison,
            VerificationOutcome::Mixed,
            2,
            "2026-03-26T12:00:00Z",
        );

        assert_eq!(learning.run_id, "run-after");
        assert_eq!(learning.merged_pr_number, 42);
        assert_eq!(learning.repo, "owner/repo");
        assert_eq!(learning.cycle_iteration, 1);
        assert_eq!(learning.issues_found, 6); // 3+1+2 scenarios compared
        assert_eq!(learning.issues_fixed, 3); // improved_count
        assert_eq!(learning.issues_verified, 5); // improved + unchanged
        assert_eq!(learning.false_positives, 2);
        assert_eq!(learning.score_before, 50);
        assert_eq!(learning.score_after, 65);
        assert_eq!(learning.outcome, VerificationOutcome::Mixed);
    }

    // -- Classification accuracy (pure) --------------------------------------

    #[test]
    fn accuracy_empty_when_no_cycles() {
        let file = SatLearningsFile::default();
        let acc = compute_classification_accuracy(&file);
        assert_eq!(acc.total_classifications, 0);
        assert_eq!(acc.true_positives, 0);
        assert_eq!(acc.false_positives, 0);
        assert!(acc.accuracy.is_none());
        assert_eq!(acc.cycles_counted, 0);
    }

    #[test]
    fn accuracy_computed_from_single_cycle() {
        let file = SatLearningsFile {
            learnings: Vec::new(),
            cycle_learnings: vec![SatCycleLearning {
                recorded_at: "2026-03-26T12:00:00Z".into(),
                run_id: "run-1".into(),
                merged_pr_number: 42,
                repo: "owner/repo".into(),
                cycle_iteration: 1,
                issues_found: 10,
                issues_fixed: 7,
                issues_verified: 8,
                false_positives: 2,
                score_before: 40,
                score_after: 75,
                outcome: VerificationOutcome::Verified,
            }],
        };

        let acc = compute_classification_accuracy(&file);
        assert_eq!(acc.total_classifications, 10);
        assert_eq!(acc.true_positives, 8);
        assert_eq!(acc.false_positives, 2);
        assert_eq!(acc.cycles_counted, 1);
        // accuracy = 8 / (8+2) = 0.8
        let a = acc.accuracy.unwrap();
        assert!((a - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn accuracy_aggregated_across_multiple_cycles() {
        let file = SatLearningsFile {
            learnings: Vec::new(),
            cycle_learnings: vec![
                SatCycleLearning {
                    recorded_at: "2026-03-26T12:00:00Z".into(),
                    run_id: "run-1".into(),
                    merged_pr_number: 42,
                    repo: "owner/repo".into(),
                    cycle_iteration: 1,
                    issues_found: 10,
                    issues_fixed: 7,
                    issues_verified: 8,
                    false_positives: 1,
                    score_before: 40,
                    score_after: 75,
                    outcome: VerificationOutcome::Verified,
                },
                SatCycleLearning {
                    recorded_at: "2026-03-27T12:00:00Z".into(),
                    run_id: "run-2".into(),
                    merged_pr_number: 50,
                    repo: "owner/repo".into(),
                    cycle_iteration: 2,
                    issues_found: 5,
                    issues_fixed: 3,
                    issues_verified: 4,
                    false_positives: 1,
                    score_before: 75,
                    score_after: 85,
                    outcome: VerificationOutcome::Mixed,
                },
            ],
        };

        let acc = compute_classification_accuracy(&file);
        assert_eq!(acc.total_classifications, 15); // 10 + 5
        assert_eq!(acc.true_positives, 12); // 8 + 4
        assert_eq!(acc.false_positives, 2); // 1 + 1
        assert_eq!(acc.cycles_counted, 2);
        // accuracy = 12 / (12+2) = 12/14 ≈ 0.857
        let a = acc.accuracy.unwrap();
        assert!((a - 12.0 / 14.0).abs() < f64::EPSILON);
    }

    #[test]
    fn accuracy_none_when_zero_tp_and_zero_fp() {
        let file = SatLearningsFile {
            learnings: Vec::new(),
            cycle_learnings: vec![SatCycleLearning {
                recorded_at: "2026-03-26T12:00:00Z".into(),
                run_id: "run-1".into(),
                merged_pr_number: 42,
                repo: "owner/repo".into(),
                cycle_iteration: 1,
                issues_found: 5,
                issues_fixed: 0,
                issues_verified: 0,
                false_positives: 0,
                score_before: 40,
                score_after: 40,
                outcome: VerificationOutcome::Regressed,
            }],
        };

        let acc = compute_classification_accuracy(&file);
        assert_eq!(acc.total_classifications, 5);
        // Denominator is 0, so accuracy is None
        assert!(acc.accuracy.is_none());
        assert_eq!(acc.cycles_counted, 1);
    }

    // -- Learnings persistence (I/O) -----------------------------------------

    #[test]
    fn write_cycle_learning_round_trip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("sat").join("learnings.yaml");

        let learning = SatCycleLearning {
            recorded_at: "2026-03-26T12:00:00Z".into(),
            run_id: "run-1".into(),
            merged_pr_number: 42,
            repo: "owner/repo".into(),
            cycle_iteration: 1,
            issues_found: 10,
            issues_fixed: 7,
            issues_verified: 8,
            false_positives: 2,
            score_before: 40,
            score_after: 75,
            outcome: VerificationOutcome::Verified,
        };

        write_cycle_learning(&path, &learning).unwrap();

        // Read back
        let loaded = crate::services::sat_score::load_learnings(&path).unwrap();
        assert_eq!(loaded.cycle_learnings.len(), 1);
        assert_eq!(loaded.cycle_learnings[0].merged_pr_number, 42);
        assert_eq!(loaded.cycle_learnings[0].issues_found, 10);
        assert_eq!(loaded.cycle_learnings[0].issues_verified, 8);
        assert_eq!(loaded.cycle_learnings[0].false_positives, 2);
        assert_eq!(loaded.cycle_learnings[0].score_before, 40);
        assert_eq!(loaded.cycle_learnings[0].score_after, 75);
    }

    #[test]
    fn write_cycle_learning_appends_to_existing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("sat").join("learnings.yaml");

        let learning1 = SatCycleLearning {
            recorded_at: "2026-03-26T12:00:00Z".into(),
            run_id: "run-1".into(),
            merged_pr_number: 42,
            repo: "owner/repo".into(),
            cycle_iteration: 1,
            issues_found: 10,
            issues_fixed: 7,
            issues_verified: 8,
            false_positives: 2,
            score_before: 40,
            score_after: 75,
            outcome: VerificationOutcome::Verified,
        };

        let learning2 = SatCycleLearning {
            recorded_at: "2026-03-27T12:00:00Z".into(),
            run_id: "run-2".into(),
            merged_pr_number: 50,
            repo: "owner/repo".into(),
            cycle_iteration: 2,
            issues_found: 5,
            issues_fixed: 3,
            issues_verified: 4,
            false_positives: 1,
            score_before: 75,
            score_after: 85,
            outcome: VerificationOutcome::Mixed,
        };

        write_cycle_learning(&path, &learning1).unwrap();
        write_cycle_learning(&path, &learning2).unwrap();

        let loaded = crate::services::sat_score::load_learnings(&path).unwrap();
        assert_eq!(loaded.cycle_learnings.len(), 2);
        assert_eq!(loaded.cycle_learnings[0].run_id, "run-1");
        assert_eq!(loaded.cycle_learnings[1].run_id, "run-2");
    }

    #[test]
    fn learnings_yaml_preserves_both_learning_types() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("sat").join("learnings.yaml");

        // First write a regular learning
        let regular = vec![SatLearning {
            recorded_at: "2026-03-26T12:00:00Z".into(),
            run_id: "run-0".into(),
            scenario_id: Some("s1".into()),
            category: FindingCategory::App,
            confidence: ConfidenceLevel::High,
            summary: "Button does not respond".into(),
        }];

        let existing = SatLearningsFile::default();
        crate::services::sat_score::write_learnings(&path, &existing, &regular).unwrap();

        // Now add a cycle learning
        let cycle = SatCycleLearning {
            recorded_at: "2026-03-26T12:00:00Z".into(),
            run_id: "run-1".into(),
            merged_pr_number: 42,
            repo: "owner/repo".into(),
            cycle_iteration: 1,
            issues_found: 5,
            issues_fixed: 3,
            issues_verified: 4,
            false_positives: 1,
            score_before: 40,
            score_after: 75,
            outcome: VerificationOutcome::Verified,
        };

        write_cycle_learning(&path, &cycle).unwrap();

        // Both should be present
        let loaded = crate::services::sat_score::load_learnings(&path).unwrap();
        assert_eq!(loaded.learnings.len(), 1);
        assert_eq!(loaded.learnings[0].summary, "Button does not respond");
        assert_eq!(loaded.cycle_learnings.len(), 1);
        assert_eq!(loaded.cycle_learnings[0].merged_pr_number, 42);
    }
}
