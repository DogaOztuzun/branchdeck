use log::{debug, error, info};

use crate::models::workflow::{
    BackoffStrategy, OutcomeAction, OutcomeDef, OutcomeDetector, RetryDef, WorkflowDef,
};

/// Input data for outcome detection — what we know about the current run state.
#[derive(Debug, Clone)]
pub struct OutcomeInput {
    pub worktree_path: String,
    pub workflow_name: String,
    pub run_status: RunOutcomeStatus,
    pub pr_created: bool,
    pub ci_status: Option<String>,
}

/// Status of the run as reported by `RunManager`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunOutcomeStatus {
    Running,
    Succeeded,
    Failed,
}

/// Result of checking outcomes for a workflow run.
#[derive(Debug, Clone)]
pub struct OutcomeResult {
    pub matched_outcome: Option<MatchedOutcome>,
    pub effects: Vec<LifecycleEffect>,
}

/// A matched outcome from the workflow definition.
#[derive(Debug, Clone)]
pub struct MatchedOutcome {
    pub name: String,
    pub detector: OutcomeDetector,
    pub action: OutcomeAction,
}

/// Effects produced by lifecycle transitions (pure function output).
#[derive(Debug, Clone)]
pub enum LifecycleEffect {
    EmitLifecycleEvent {
        workflow_name: String,
        status: String,
        detail: String,
    },
    ScheduleRetry {
        workflow_name: String,
        worktree_path: String,
        attempt: u32,
        delay_ms: u64,
    },
    MarkComplete {
        workflow_name: String,
        worktree_path: String,
    },
    MarkFailed {
        workflow_name: String,
        worktree_path: String,
        reason: String,
    },
    TransitionToReview {
        workflow_name: String,
        worktree_path: String,
    },
    /// Circuit breaker tripped — SAT fix-verify cycle reached its iteration limit.
    /// The triage view should notify the user that autonomous fixing has stopped.
    CircuitBreakerTripped {
        repo: String,
        iteration: u32,
        max_iterations: u32,
        reason: String,
    },
}

/// Check all outcomes in order and return the first match.
/// Pure function: checks conditions against input data, no I/O.
#[must_use]
pub fn check_outcomes(def: &WorkflowDef, input: &OutcomeInput) -> OutcomeResult {
    for outcome in &def.config.outcomes {
        if detector_matches(outcome, input) {
            debug!(
                "Outcome '{}' matched for workflow '{}'",
                outcome.name, input.workflow_name
            );
            let matched = MatchedOutcome {
                name: outcome.name.clone(),
                detector: outcome.detect,
                action: outcome.next,
            };
            return OutcomeResult {
                matched_outcome: Some(matched),
                effects: Vec::new(),
            };
        }
    }

    OutcomeResult {
        matched_outcome: None,
        effects: Vec::new(),
    }
}

/// Check if a single outcome detector matches the current input.
fn detector_matches(outcome: &OutcomeDef, input: &OutcomeInput) -> bool {
    match outcome.detect {
        OutcomeDetector::FileExists => {
            let Some(path) = &outcome.path else {
                return false;
            };
            let full_path = format!("{}/{path}", input.worktree_path);
            std::path::Path::new(&full_path).exists()
        }
        OutcomeDetector::PrCreated => input.pr_created,
        OutcomeDetector::CiPassing => input
            .ci_status
            .as_deref()
            .is_some_and(|s| s == "SUCCESS" || s == "PASSING"),
        OutcomeDetector::RunFailed => input.run_status == RunOutcomeStatus::Failed,
        OutcomeDetector::Custom => false,
    }
}

/// Apply outcome action to produce lifecycle effects.
/// Handles retry scheduling, completion, and review transitions.
#[must_use]
pub fn apply_outcome(
    def: &WorkflowDef,
    outcome: &MatchedOutcome,
    input: &OutcomeInput,
    attempt: u32,
) -> Vec<LifecycleEffect> {
    let mut effects = Vec::new();

    match outcome.action {
        OutcomeAction::Complete => {
            let status_name = def
                .config
                .lifecycle
                .as_ref()
                .and_then(|l| l.complete.clone())
                .unwrap_or_else(|| "complete".to_string());

            info!(
                "Workflow '{}' completed via outcome '{}'",
                input.workflow_name, outcome.name
            );

            effects.push(LifecycleEffect::EmitLifecycleEvent {
                workflow_name: input.workflow_name.clone(),
                status: status_name,
                detail: format!(
                    "Completed via outcome '{}' ({})",
                    outcome.name, outcome.detector
                ),
            });
            effects.push(LifecycleEffect::MarkComplete {
                workflow_name: input.workflow_name.clone(),
                worktree_path: input.worktree_path.clone(),
            });
        }
        OutcomeAction::Retry => {
            let retry_effects = apply_retry(def, input, attempt);
            effects.extend(retry_effects);
        }
        OutcomeAction::Review => {
            info!(
                "Workflow '{}' needs review via outcome '{}'",
                input.workflow_name, outcome.name
            );

            effects.push(LifecycleEffect::EmitLifecycleEvent {
                workflow_name: input.workflow_name.clone(),
                status: "review".to_string(),
                detail: format!(
                    "Review needed via outcome '{}' ({})",
                    outcome.name, outcome.detector
                ),
            });
            effects.push(LifecycleEffect::TransitionToReview {
                workflow_name: input.workflow_name.clone(),
                worktree_path: input.worktree_path.clone(),
            });
        }
        OutcomeAction::CustomState => {
            effects.push(LifecycleEffect::EmitLifecycleEvent {
                workflow_name: input.workflow_name.clone(),
                status: outcome.name.clone(),
                detail: format!("Custom state '{}' via {}", outcome.name, outcome.detector),
            });
        }
    }

    effects
}

/// Apply retry logic per the workflow's retry policy.
fn apply_retry(def: &WorkflowDef, input: &OutcomeInput, attempt: u32) -> Vec<LifecycleEffect> {
    let mut effects = Vec::new();

    let Some(retry) = &def.config.retry else {
        error!(
            "Workflow '{}' outcome requests retry but no retry policy defined",
            input.workflow_name
        );
        effects.push(LifecycleEffect::MarkFailed {
            workflow_name: input.workflow_name.clone(),
            worktree_path: input.worktree_path.clone(),
            reason: "Retry requested but no retry policy defined".to_string(),
        });
        return effects;
    };

    if attempt >= retry.max_attempts {
        let status_name = def
            .config
            .lifecycle
            .as_ref()
            .and_then(|l| l.failed.clone())
            .unwrap_or_else(|| "failed".to_string());

        error!(
            "Workflow '{}' exceeded max retries ({}/{})",
            input.workflow_name, attempt, retry.max_attempts
        );

        effects.push(LifecycleEffect::EmitLifecycleEvent {
            workflow_name: input.workflow_name.clone(),
            status: status_name,
            detail: format!(
                "Max retries exhausted ({attempt}/{max})",
                max = retry.max_attempts
            ),
        });
        effects.push(LifecycleEffect::MarkFailed {
            workflow_name: input.workflow_name.clone(),
            worktree_path: input.worktree_path.clone(),
            reason: format!(
                "Max retries exhausted ({attempt}/{max})",
                max = retry.max_attempts
            ),
        });
    } else {
        let delay_ms = compute_backoff_delay(retry, attempt);
        let status_name = def
            .config
            .lifecycle
            .as_ref()
            .and_then(|l| l.retrying.clone())
            .unwrap_or_else(|| "retrying".to_string());

        info!(
            "Workflow '{}' scheduling retry {}/{} (delay: {delay_ms}ms)",
            input.workflow_name,
            attempt + 1,
            retry.max_attempts
        );

        effects.push(LifecycleEffect::EmitLifecycleEvent {
            workflow_name: input.workflow_name.clone(),
            status: status_name,
            detail: format!(
                "Retry {next}/{max} in {delay_ms}ms",
                next = attempt + 1,
                max = retry.max_attempts
            ),
        });
        effects.push(LifecycleEffect::ScheduleRetry {
            workflow_name: input.workflow_name.clone(),
            worktree_path: input.worktree_path.clone(),
            attempt: attempt + 1,
            delay_ms,
        });
    }

    effects
}

/// Compute retry delay based on backoff strategy and attempt number.
#[must_use]
pub fn compute_backoff_delay(retry: &RetryDef, attempt: u32) -> u64 {
    match retry.backoff {
        BackoffStrategy::Fixed => retry.base_delay_ms,
        BackoffStrategy::Exponential => {
            let shift = attempt.min(63);
            retry.base_delay_ms.saturating_mul(1u64 << shift)
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::models::workflow::*;
    use crate::services::workflow::parse_workflow_md;

    fn make_workflow_with_outcomes() -> WorkflowDef {
        let md = "---\n\
                   name: test-workflow\n\
                   description: Test\n\
                   tracker:\n\
                   \x20 kind: manual\n\
                   outcomes:\n\
                   \x20 - name: output-written\n\
                   \x20   detect: file-exists\n\
                   \x20   path: output.json\n\
                   \x20   next: complete\n\
                   \x20 - name: pr-ready\n\
                   \x20   detect: pr-created\n\
                   \x20   next: review\n\
                   \x20 - name: agent-failed\n\
                   \x20   detect: run-failed\n\
                   \x20   next: retry\n\
                   retry:\n\
                   \x20 max_attempts: 3\n\
                   \x20 backoff: exponential\n\
                   \x20 base_delay_ms: 1000\n\
                   lifecycle:\n\
                   \x20 dispatched: Dispatched\n\
                   \x20 complete: Done\n\
                   \x20 failed: Failed\n\
                   \x20 retrying: Retrying\n\
                   ---\n\
                   Do the work.\n";
        parse_workflow_md(md).unwrap()
    }

    fn make_input(status: RunOutcomeStatus) -> OutcomeInput {
        OutcomeInput {
            worktree_path: "/nonexistent/path".to_string(),
            workflow_name: "test-workflow".to_string(),
            run_status: status,
            pr_created: false,
            ci_status: None,
        }
    }

    #[test]
    fn check_outcomes_file_exists_no_file() {
        let def = make_workflow_with_outcomes();
        let input = make_input(RunOutcomeStatus::Succeeded);
        let result = check_outcomes(&def, &input);
        // No file exists at /nonexistent/path/output.json, no PR, no failure
        assert!(result.matched_outcome.is_none());
    }

    #[test]
    fn check_outcomes_pr_created() {
        let def = make_workflow_with_outcomes();
        let mut input = make_input(RunOutcomeStatus::Succeeded);
        input.pr_created = true;

        let result = check_outcomes(&def, &input);
        let matched = result.matched_outcome.unwrap();
        assert_eq!(matched.name, "pr-ready");
        assert_eq!(matched.action, OutcomeAction::Review);
    }

    #[test]
    fn check_outcomes_run_failed() {
        let def = make_workflow_with_outcomes();
        let input = make_input(RunOutcomeStatus::Failed);

        let result = check_outcomes(&def, &input);
        let matched = result.matched_outcome.unwrap();
        assert_eq!(matched.name, "agent-failed");
        assert_eq!(matched.action, OutcomeAction::Retry);
    }

    #[test]
    fn check_outcomes_ci_passing() {
        let md = "---\n\
                   name: ci-check\n\
                   description: Test\n\
                   tracker:\n\
                   \x20 kind: github-pr\n\
                   outcomes:\n\
                   \x20 - name: ci-green\n\
                   \x20   detect: ci-passing\n\
                   \x20   next: complete\n\
                   ---\n\
                   Check CI.\n";
        let def = parse_workflow_md(md).unwrap();
        let input = OutcomeInput {
            worktree_path: "/tmp/wt".to_string(),
            workflow_name: "ci-check".to_string(),
            run_status: RunOutcomeStatus::Running,
            pr_created: false,
            ci_status: Some("SUCCESS".to_string()),
        };

        let result = check_outcomes(&def, &input);
        let matched = result.matched_outcome.unwrap();
        assert_eq!(matched.name, "ci-green");
        assert_eq!(matched.action, OutcomeAction::Complete);
    }

    #[test]
    fn check_outcomes_first_match_wins() {
        // Both pr-created and run-failed match: first (file-exists won't, then pr-created) wins
        let def = make_workflow_with_outcomes();
        let mut input = make_input(RunOutcomeStatus::Failed);
        input.pr_created = true;

        let result = check_outcomes(&def, &input);
        let matched = result.matched_outcome.unwrap();
        // pr-ready (index 1) comes before agent-failed (index 2) but file-exists (index 0) doesn't match
        assert_eq!(matched.name, "pr-ready");
    }

    #[test]
    fn apply_outcome_complete() {
        let def = make_workflow_with_outcomes();
        let input = make_input(RunOutcomeStatus::Succeeded);
        let outcome = MatchedOutcome {
            name: "output-written".to_string(),
            detector: OutcomeDetector::FileExists,
            action: OutcomeAction::Complete,
        };

        let effects = apply_outcome(&def, &outcome, &input, 1);
        assert!(effects.iter().any(|e| matches!(
            e,
            LifecycleEffect::EmitLifecycleEvent { status, .. } if status == "Done"
        )));
        assert!(effects
            .iter()
            .any(|e| matches!(e, LifecycleEffect::MarkComplete { .. })));
    }

    #[test]
    fn apply_outcome_retry_schedules_with_backoff() {
        let def = make_workflow_with_outcomes();
        let input = make_input(RunOutcomeStatus::Failed);
        let outcome = MatchedOutcome {
            name: "agent-failed".to_string(),
            detector: OutcomeDetector::RunFailed,
            action: OutcomeAction::Retry,
        };

        let effects = apply_outcome(&def, &outcome, &input, 1);
        assert!(effects.iter().any(|e| matches!(
            e,
            LifecycleEffect::ScheduleRetry {
                attempt: 2,
                delay_ms: 2000,
                ..
            }
        )));
        assert!(effects.iter().any(|e| matches!(
            e,
            LifecycleEffect::EmitLifecycleEvent { status, .. } if status == "Retrying"
        )));
    }

    #[test]
    fn apply_outcome_retry_exhausted() {
        let def = make_workflow_with_outcomes();
        let input = make_input(RunOutcomeStatus::Failed);
        let outcome = MatchedOutcome {
            name: "agent-failed".to_string(),
            detector: OutcomeDetector::RunFailed,
            action: OutcomeAction::Retry,
        };

        // attempt = 3 (== max_attempts), should fail
        let effects = apply_outcome(&def, &outcome, &input, 3);
        assert!(effects
            .iter()
            .any(|e| matches!(e, LifecycleEffect::MarkFailed { .. })));
        assert!(effects.iter().any(|e| matches!(
            e,
            LifecycleEffect::EmitLifecycleEvent { status, .. } if status == "Failed"
        )));
    }

    #[test]
    fn apply_outcome_review() {
        let def = make_workflow_with_outcomes();
        let input = make_input(RunOutcomeStatus::Succeeded);
        let outcome = MatchedOutcome {
            name: "pr-ready".to_string(),
            detector: OutcomeDetector::PrCreated,
            action: OutcomeAction::Review,
        };

        let effects = apply_outcome(&def, &outcome, &input, 1);
        assert!(effects
            .iter()
            .any(|e| matches!(e, LifecycleEffect::TransitionToReview { .. })));
    }

    #[test]
    fn backoff_fixed_constant() {
        let retry = RetryDef {
            max_attempts: 3,
            backoff: BackoffStrategy::Fixed,
            base_delay_ms: 5000,
        };
        assert_eq!(compute_backoff_delay(&retry, 0), 5000);
        assert_eq!(compute_backoff_delay(&retry, 1), 5000);
        assert_eq!(compute_backoff_delay(&retry, 5), 5000);
    }

    #[test]
    fn backoff_exponential_doubles() {
        let retry = RetryDef {
            max_attempts: 5,
            backoff: BackoffStrategy::Exponential,
            base_delay_ms: 1000,
        };
        assert_eq!(compute_backoff_delay(&retry, 0), 1000);
        assert_eq!(compute_backoff_delay(&retry, 1), 2000);
        assert_eq!(compute_backoff_delay(&retry, 2), 4000);
        assert_eq!(compute_backoff_delay(&retry, 3), 8000);
    }

    #[test]
    fn apply_outcome_retry_no_policy_fails() {
        let md = "---\n\
                   name: no-retry\n\
                   description: No retry policy\n\
                   tracker:\n\
                   \x20 kind: manual\n\
                   outcomes:\n\
                   \x20 - name: agent-failed\n\
                   \x20   detect: run-failed\n\
                   \x20   next: retry\n\
                   ---\n\
                   Work.\n";
        let def = parse_workflow_md(md).unwrap();
        let input = OutcomeInput {
            worktree_path: "/tmp/wt".to_string(),
            workflow_name: "no-retry".to_string(),
            run_status: RunOutcomeStatus::Failed,
            pr_created: false,
            ci_status: None,
        };
        let outcome = MatchedOutcome {
            name: "agent-failed".to_string(),
            detector: OutcomeDetector::RunFailed,
            action: OutcomeAction::Retry,
        };

        let effects = apply_outcome(&def, &outcome, &input, 1);
        assert!(effects
            .iter()
            .any(|e| matches!(e, LifecycleEffect::MarkFailed { .. })));
    }

    #[test]
    fn lifecycle_def_resolve_named_fields() {
        let lifecycle = LifecycleDef {
            dispatched: Some("Dispatching Agent".to_string()),
            complete: Some("All Done".to_string()),
            failed: Some("Agent Failed".to_string()),
            retrying: Some("Scheduling Retry".to_string()),
            custom_statuses: HashMap::new(),
        };
        assert_eq!(lifecycle.resolve_display_status("dispatched"), "Dispatching Agent");
        assert_eq!(lifecycle.resolve_display_status("running"), "Dispatching Agent");
        assert_eq!(lifecycle.resolve_display_status("complete"), "All Done");
        assert_eq!(lifecycle.resolve_display_status("completed"), "All Done");
        assert_eq!(lifecycle.resolve_display_status("failed"), "Agent Failed");
        assert_eq!(lifecycle.resolve_display_status("retrying"), "Scheduling Retry");
    }

    #[test]
    fn lifecycle_def_resolve_custom_statuses() {
        let mut custom = HashMap::new();
        custom.insert("analyzing".to_string(), "Analyzing Code".to_string());
        custom.insert("patching".to_string(), "Applying Patches".to_string());
        custom.insert("validating".to_string(), "Running Validation".to_string());

        let lifecycle = LifecycleDef {
            dispatched: None,
            complete: None,
            failed: None,
            retrying: None,
            custom_statuses: custom,
        };
        assert_eq!(lifecycle.resolve_display_status("analyzing"), "Analyzing Code");
        assert_eq!(lifecycle.resolve_display_status("patching"), "Applying Patches");
        assert_eq!(lifecycle.resolve_display_status("validating"), "Running Validation");
        // Unknown status falls back to raw key
        assert_eq!(lifecycle.resolve_display_status("unknown"), "unknown");
    }

    #[test]
    fn lifecycle_def_named_fields_override_custom() {
        let mut custom = HashMap::new();
        custom.insert("failed".to_string(), "Custom Failed".to_string());

        let lifecycle = LifecycleDef {
            dispatched: None,
            complete: None,
            failed: Some("Named Failed".to_string()),
            retrying: None,
            custom_statuses: custom,
        };
        // Named field takes precedence over custom_statuses
        assert_eq!(lifecycle.resolve_display_status("failed"), "Named Failed");
    }

    #[test]
    fn lifecycle_def_custom_statuses_parsed_from_yaml() {
        let md = "---\n\
                   name: custom-workflow\n\
                   description: Custom statuses test\n\
                   tracker:\n\
                   \x20 kind: manual\n\
                   lifecycle:\n\
                   \x20 dispatched: Launched\n\
                   \x20 complete: Verified\n\
                   \x20 custom_statuses:\n\
                   \x20   analyzing: Deep Analysis\n\
                   \x20   patching: Hot Patching\n\
                   ---\n\
                   Do custom work.\n";
        let def = parse_workflow_md(md).unwrap();
        let lifecycle = def.config.lifecycle.unwrap();
        assert_eq!(lifecycle.dispatched.as_deref(), Some("Launched"));
        assert_eq!(lifecycle.resolve_display_status("analyzing"), "Deep Analysis");
        assert_eq!(lifecycle.resolve_display_status("patching"), "Hot Patching");
        assert_eq!(lifecycle.resolve_display_status("running"), "Launched");
    }
}
