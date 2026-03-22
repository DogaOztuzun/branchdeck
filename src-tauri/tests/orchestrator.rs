#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use branchdeck_lib::models::github::PrSummary;
use branchdeck_lib::models::orchestrator::{
    Orchestrator, OrchestratorConfig, OrchestratorEffect, SessionOutcome,
};
use branchdeck_lib::services::orchestrator::{
    apply_pr_event, apply_reconciliation, apply_relaunch, apply_retry_due, apply_session_end,
    apply_skip, retry_backoff,
};

fn make_config(enabled: bool, max_concurrent: u32) -> OrchestratorConfig {
    OrchestratorConfig {
        enabled,
        max_concurrent,
        auto_analyze: enabled,
        ..OrchestratorConfig::default()
    }
}

fn make_pr(number: u64, branch: &str, ci_status: Option<&str>) -> PrSummary {
    PrSummary {
        number,
        title: format!("PR #{number}"),
        branch: branch.to_string(),
        url: format!("https://github.com/test/repo/pull/{number}"),
        ci_status: ci_status.map(String::from),
        review_decision: None,
        repo_name: "test/repo".to_string(),
        author: "alice".to_string(),
        additions: None,
        deletions: None,
        changed_files: None,
        created_at: None,
    }
}

fn make_orchestrator(enabled: bool, max_concurrent: u32) -> Orchestrator {
    Orchestrator::new(make_config(enabled, max_concurrent))
}

// --- apply_pr_event tests ---

#[test]
fn pr_event_eligible_pr_dispatches() {
    let mut state = make_orchestrator(true, 1);
    let prs = vec![make_pr(1, "fix/bug", Some("FAILURE"))];

    let effects = apply_pr_event(&mut state, "test/repo", &prs, 1000);

    assert!(effects.iter().any(|e| matches!(
        e,
        OrchestratorEffect::DispatchSession { pr_key, .. } if pr_key == "test/repo#1"
    )));
    assert!(state.claimed.contains("test/repo#1"));
}

#[test]
fn pr_event_already_claimed_skips() {
    let mut state = make_orchestrator(true, 1);
    state.claimed.insert("test/repo#1".to_string());
    let prs = vec![make_pr(1, "fix/bug", Some("FAILURE"))];

    let effects = apply_pr_event(&mut state, "test/repo", &prs, 1000);

    assert!(effects.is_empty());
}

#[test]
fn pr_event_excluded_branch_skips() {
    let mut state = make_orchestrator(true, 1);
    let prs = vec![make_pr(1, "main", Some("FAILURE"))];

    let effects = apply_pr_event(&mut state, "test/repo", &prs, 1000);

    assert!(effects.is_empty());
}

#[test]
fn pr_event_passing_ci_skips() {
    let mut state = make_orchestrator(true, 1);
    let prs = vec![make_pr(1, "fix/bug", Some("SUCCESS"))];

    let effects = apply_pr_event(&mut state, "test/repo", &prs, 1000);

    assert!(effects.is_empty());
}

#[test]
fn pr_event_concurrency_gate() {
    let mut state = make_orchestrator(true, 1);
    let prs = vec![
        make_pr(1, "fix/a", Some("FAILURE")),
        make_pr(2, "fix/b", Some("FAILURE")),
        make_pr(3, "fix/c", Some("FAILURE")),
    ];

    let effects = apply_pr_event(&mut state, "test/repo", &prs, 1000);

    let dispatch_count = effects
        .iter()
        .filter(|e| matches!(e, OrchestratorEffect::DispatchSession { .. }))
        .count();
    assert_eq!(
        dispatch_count, 1,
        "should only dispatch 1 with max_concurrent=1"
    );
}

#[test]
fn pr_event_disabled_orchestrator_skips() {
    let mut state = make_orchestrator(false, 1);
    let prs = vec![make_pr(1, "fix/bug", Some("FAILURE"))];

    let effects = apply_pr_event(&mut state, "test/repo", &prs, 1000);

    assert!(effects.is_empty());
}

// --- apply_session_end tests ---

#[test]
fn session_end_analysis_written_emits_review_ready() {
    let mut state = make_orchestrator(true, 1);
    state.running.insert(
        "test/repo#1".to_string(),
        branchdeck_lib::models::orchestrator::RunningEntry {
            pr_key: "test/repo#1".to_string(),
            worktree_path: "/tmp/wt".to_string(),
            tab_id: "tab-1".to_string(),
            started_at: 1000,
            attempt: 1,
        },
    );

    let effects = apply_session_end(
        &mut state,
        "test/repo#1",
        SessionOutcome::AnalysisWritten,
        2000,
    );

    assert!(effects.iter().any(|e| matches!(
        e,
        OrchestratorEffect::EmitLifecycleEvent { event }
            if event.status == branchdeck_lib::models::orchestrator::LifecycleStatus::ReviewReady
    )));
    assert!(!state.running.contains_key("test/repo#1"));
}

#[test]
fn session_end_fix_completed_marks_done() {
    let mut state = make_orchestrator(true, 1);
    state.claimed.insert("test/repo#1".to_string());
    state.running.insert(
        "test/repo#1".to_string(),
        branchdeck_lib::models::orchestrator::RunningEntry {
            pr_key: "test/repo#1".to_string(),
            worktree_path: "/tmp/wt".to_string(),
            tab_id: "tab-1".to_string(),
            started_at: 1000,
            attempt: 1,
        },
    );

    let effects = apply_session_end(
        &mut state,
        "test/repo#1",
        SessionOutcome::FixCompleted,
        2000,
    );

    assert!(!state.claimed.contains("test/repo#1"));
    assert!(state.completed.contains("test/repo#1"));
    assert!(effects.iter().any(|e| matches!(
        e,
        OrchestratorEffect::EmitLifecycleEvent { event }
            if event.status == branchdeck_lib::models::orchestrator::LifecycleStatus::Completed
    )));
}

#[test]
fn session_end_fix_incomplete_schedules_retry() {
    let mut state = make_orchestrator(true, 1);
    state.running.insert(
        "test/repo#1".to_string(),
        branchdeck_lib::models::orchestrator::RunningEntry {
            pr_key: "test/repo#1".to_string(),
            worktree_path: "/tmp/wt".to_string(),
            tab_id: "tab-1".to_string(),
            started_at: 1000,
            attempt: 1,
        },
    );

    let effects = apply_session_end(
        &mut state,
        "test/repo#1",
        SessionOutcome::FixIncomplete,
        2000,
    );

    assert!(effects
        .iter()
        .any(|e| matches!(e, OrchestratorEffect::ScheduleRetry { delay_ms: 1000, .. })));
    assert!(state.retry_queue.contains_key("test/repo#1"));
}

#[test]
fn session_end_no_output_schedules_backoff_retry() {
    let mut state = make_orchestrator(true, 1);
    state.running.insert(
        "test/repo#1".to_string(),
        branchdeck_lib::models::orchestrator::RunningEntry {
            pr_key: "test/repo#1".to_string(),
            worktree_path: "/tmp/wt".to_string(),
            tab_id: "tab-1".to_string(),
            started_at: 1000,
            attempt: 1,
        },
    );

    let effects = apply_session_end(&mut state, "test/repo#1", SessionOutcome::NoOutput, 2000);

    assert!(effects.iter().any(|e| matches!(
        e,
        OrchestratorEffect::ScheduleRetry {
            delay_ms: 10000,
            ..
        }
    )));
}

// --- apply_relaunch tests ---

#[test]
fn relaunch_dispatches_fix_session() {
    let mut state = make_orchestrator(true, 1);
    state.claimed.insert("test/repo#1".to_string());

    let effects = apply_relaunch(&mut state, "test/repo#1", "/tmp/wt", 3000);

    assert!(effects.iter().any(|e| matches!(
        e,
        OrchestratorEffect::DispatchSession { pr_key, .. } if pr_key == "test/repo#1"
    )));
    assert!(effects.iter().any(|e| matches!(
        e,
        OrchestratorEffect::EmitLifecycleEvent { event }
            if event.status == branchdeck_lib::models::orchestrator::LifecycleStatus::Fixing
    )));
}

#[test]
fn relaunch_cancels_pending_retry() {
    let mut state = make_orchestrator(true, 1);
    state.retry_queue.insert(
        "test/repo#1".to_string(),
        branchdeck_lib::models::orchestrator::RetryEntry {
            pr_key: "test/repo#1".to_string(),
            attempt: 2,
            due_at_ms: 5000,
            error: None,
        },
    );

    let effects = apply_relaunch(&mut state, "test/repo#1", "/tmp/wt", 3000);

    assert!(effects.iter().any(|e| matches!(
        e,
        OrchestratorEffect::CancelRetry { pr_key } if pr_key == "test/repo#1"
    )));
    assert!(!state.retry_queue.contains_key("test/repo#1"));
}

// --- apply_reconciliation tests ---

#[test]
fn reconciliation_stops_merged_pr() {
    let mut state = make_orchestrator(true, 1);
    state.claimed.insert("test/repo#1".to_string());
    state.running.insert(
        "test/repo#1".to_string(),
        branchdeck_lib::models::orchestrator::RunningEntry {
            pr_key: "test/repo#1".to_string(),
            worktree_path: "/tmp/wt".to_string(),
            tab_id: "tab-1".to_string(),
            started_at: 1000,
            attempt: 1,
        },
    );

    // Empty PR list = PR was merged/closed
    let effects = apply_reconciliation(&mut state, &[]);

    assert!(effects.iter().any(|e| matches!(
        e,
        OrchestratorEffect::StopSession { pr_key, .. } if pr_key == "test/repo#1"
    )));
    assert!(effects
        .iter()
        .any(|e| matches!(e, OrchestratorEffect::CleanupMetadata { .. })));
    assert!(!state.running.contains_key("test/repo#1"));
    assert!(!state.claimed.contains("test/repo#1"));
}

// --- apply_retry_due tests ---

#[test]
fn retry_due_redispatches() {
    let mut state = make_orchestrator(true, 1);
    state.retry_queue.insert(
        "test/repo#1".to_string(),
        branchdeck_lib::models::orchestrator::RetryEntry {
            pr_key: "test/repo#1".to_string(),
            attempt: 2,
            due_at_ms: 5000,
            error: None,
        },
    );

    let effects = apply_retry_due(&mut state, "test/repo#1", "/tmp/wt", 6000);

    assert!(effects.iter().any(|e| matches!(
        e,
        OrchestratorEffect::DispatchSession { pr_key, .. } if pr_key == "test/repo#1"
    )));
    assert!(!state.retry_queue.contains_key("test/repo#1"));
}

// --- apply_skip tests ---

#[test]
fn skip_removes_from_all_tracking() {
    let mut state = make_orchestrator(true, 1);
    state.claimed.insert("test/repo#1".to_string());
    state.running.insert(
        "test/repo#1".to_string(),
        branchdeck_lib::models::orchestrator::RunningEntry {
            pr_key: "test/repo#1".to_string(),
            worktree_path: "/tmp/wt".to_string(),
            tab_id: "tab-1".to_string(),
            started_at: 1000,
            attempt: 1,
        },
    );

    let effects = apply_skip(&mut state, "test/repo#1");

    assert!(effects
        .iter()
        .any(|e| matches!(e, OrchestratorEffect::StopSession { .. })));
    assert!(!state.claimed.contains("test/repo#1"));
    assert!(!state.running.contains_key("test/repo#1"));
}

// --- Retry backoff tests ---

#[test]
fn retry_backoff_formula() {
    assert_eq!(retry_backoff(1), 10_000);
    assert_eq!(retry_backoff(2), 20_000);
    assert_eq!(retry_backoff(3), 40_000);
    assert_eq!(retry_backoff(10), 300_000); // capped
    assert_eq!(retry_backoff(20), 300_000); // still capped
}

// --- Interleaved PR independence test ---

#[test]
fn interleaved_prs_are_independent() {
    let mut state = make_orchestrator(true, 3);
    let prs = vec![
        make_pr(1, "fix/a", Some("FAILURE")),
        make_pr(2, "fix/b", Some("FAILURE")),
        make_pr(3, "fix/c", Some("FAILURE")),
    ];

    let effects = apply_pr_event(&mut state, "test/repo", &prs, 1000);

    // All 3 should be dispatched (max_concurrent=3)
    let dispatch_count = effects
        .iter()
        .filter(|e| matches!(e, OrchestratorEffect::DispatchSession { .. }))
        .count();
    assert_eq!(dispatch_count, 3);

    // Simulate running entries
    for i in 1..=3u64 {
        let key = format!("test/repo#{i}");
        state.running.insert(
            key.clone(),
            branchdeck_lib::models::orchestrator::RunningEntry {
                pr_key: key,
                worktree_path: format!("/tmp/wt{i}"),
                tab_id: format!("tab-{i}"),
                started_at: 1000,
                attempt: 1,
            },
        );
    }

    // Complete PR#1, leave others running
    let effects = apply_session_end(
        &mut state,
        "test/repo#1",
        SessionOutcome::FixCompleted,
        2000,
    );
    assert!(state.completed.contains("test/repo#1"));
    assert!(state.running.contains_key("test/repo#2"));
    assert!(state.running.contains_key("test/repo#3"));
    assert!(!effects.is_empty());

    // Fail PR#2 with retry
    let effects = apply_session_end(&mut state, "test/repo#2", SessionOutcome::NoOutput, 3000);
    assert!(state.retry_queue.contains_key("test/repo#2"));
    assert!(state.running.contains_key("test/repo#3"));
    assert!(!effects.is_empty());
}

// --- determine_session_outcome tests ---

#[test]
fn determine_outcome_no_file() {
    let outcome =
        branchdeck_lib::services::orchestrator::determine_session_outcome("/nonexistent/path");
    assert_eq!(outcome, SessionOutcome::NoOutput);
}

#[test]
fn determine_outcome_analysis_written() {
    let dir = tempfile::tempdir().unwrap();
    let bd = dir.path().join(".branchdeck");
    std::fs::create_dir_all(&bd).unwrap();
    std::fs::write(
        bd.join("analysis.json"),
        r#"{"approved": false, "resolved": false}"#,
    )
    .unwrap();

    let outcome = branchdeck_lib::services::orchestrator::determine_session_outcome(
        &dir.path().display().to_string(),
    );
    assert_eq!(outcome, SessionOutcome::AnalysisWritten);
}

#[test]
fn determine_outcome_fix_completed() {
    let dir = tempfile::tempdir().unwrap();
    let bd = dir.path().join(".branchdeck");
    std::fs::create_dir_all(&bd).unwrap();
    std::fs::write(
        bd.join("analysis.json"),
        r#"{"approved": true, "resolved": true}"#,
    )
    .unwrap();

    let outcome = branchdeck_lib::services::orchestrator::determine_session_outcome(
        &dir.path().display().to_string(),
    );
    assert_eq!(outcome, SessionOutcome::FixCompleted);
}

#[test]
fn determine_outcome_fix_incomplete() {
    let dir = tempfile::tempdir().unwrap();
    let bd = dir.path().join(".branchdeck");
    std::fs::create_dir_all(&bd).unwrap();
    std::fs::write(
        bd.join("analysis.json"),
        r#"{"approved": true, "resolved": false}"#,
    )
    .unwrap();

    let outcome = branchdeck_lib::services::orchestrator::determine_session_outcome(
        &dir.path().display().to_string(),
    );
    assert_eq!(outcome, SessionOutcome::FixIncomplete);
}
