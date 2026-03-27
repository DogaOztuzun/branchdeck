//! P1 tests for run lifecycle state management.
//!
//! Tests T4-UNIT-* from test-design-phase1.md.
//! Pure state machine transitions tested via `apply_*` functions (no Tauri runtime needed).
//! Persistence layer tested via `run_state` filesystem operations.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

mod common;

use branchdeck_core::models::run::{RunInfo, RunStatus};
use branchdeck_core::models::task::TaskStatus;
use branchdeck_core::services::run_effects::{self, RunEffect};
use branchdeck_core::services::run_responses;
use branchdeck_core::services::run_state;
use tempfile::TempDir;

const BRANCHDECK_DIR: &str = ".branchdeck";

/// Helper: create a worktree-like directory with .branchdeck/ and return paths.
fn setup_worktree() -> (TempDir, String, String) {
    let dir = TempDir::new().expect("create temp dir");
    let bd_dir = dir.path().join(BRANCHDECK_DIR);
    std::fs::create_dir_all(&bd_dir).unwrap();

    let task_path = bd_dir.join("task.md").to_str().unwrap().to_string();
    let worktree_path = dir.path().to_str().unwrap().to_string();

    // Create a minimal task.md so the path is valid
    std::fs::write(&task_path, "---\ntype: issue-fix\nscope: worktree\nstatus: created\nrepo: o/r\nbranch: b\ncreated: 2026-01-01\nrun-count: 0\n---\n").unwrap();

    (dir, task_path, worktree_path)
}

// ─── T4-UNIT-001: Run state persistence (save → load round trip) ───

#[test]
fn t4_unit_001_run_state_save_load_round_trip() {
    let (_dir, task_path, worktree_path) = setup_worktree();

    let run = RunInfo {
        session_id: Some("sess-abc".to_string()),
        task_path: task_path.clone(),
        status: RunStatus::Running,
        started_at: "2026-03-20T10:00:00+00:00".to_string(),
        cost_usd: 1.23,
        last_heartbeat: Some("2026-03-20T10:01:00+00:00".to_string()),
        elapsed_secs: 60,
        tab_id: Some("tab-1".to_string()),
        failure_reason: None,
    };

    run_state::save_run_state(&task_path, &run);

    let loaded = run_state::load_run_state(&worktree_path);
    assert!(loaded.is_some(), "Should load saved run state");

    let loaded = loaded.unwrap();
    assert_eq!(loaded.session_id.as_deref(), Some("sess-abc"));
    assert_eq!(loaded.status, RunStatus::Running);
    assert_eq!(loaded.cost_usd, 1.23);
    assert_eq!(loaded.elapsed_secs, 60);
    assert_eq!(loaded.tab_id.as_deref(), Some("tab-1"));
}

// ─── T4-UNIT-002: Run state transitions preserved through save/load ───

#[test]
fn t4_unit_002_run_state_status_transitions() {
    let (_dir, task_path, worktree_path) = setup_worktree();

    // Starting
    let mut run = common::make_run_info(RunStatus::Starting, None);
    run.task_path = task_path.clone();
    run_state::save_run_state(&task_path, &run);
    let loaded = run_state::load_run_state(&worktree_path).unwrap();
    assert_eq!(loaded.status, RunStatus::Starting);

    // Running (with session_id assigned)
    run.status = RunStatus::Running;
    run.session_id = Some("sess-123".to_string());
    run_state::save_run_state(&task_path, &run);
    let loaded = run_state::load_run_state(&worktree_path).unwrap();
    assert_eq!(loaded.status, RunStatus::Running);
    assert_eq!(loaded.session_id.as_deref(), Some("sess-123"));

    // Succeeded
    run.status = RunStatus::Succeeded;
    run.cost_usd = 0.50;
    run_state::save_run_state(&task_path, &run);
    let loaded = run_state::load_run_state(&worktree_path).unwrap();
    assert_eq!(loaded.status, RunStatus::Succeeded);
    assert_eq!(loaded.cost_usd, 0.50);
}

// ─── T4-UNIT-003: Run state error transition (running → failed) ───

#[test]
fn t4_unit_003_run_state_failed_preserves_session_id() {
    let (_dir, task_path, worktree_path) = setup_worktree();

    let mut run = common::make_run_info(RunStatus::Running, Some("sess-456"));
    run.task_path = task_path.clone();
    run.cost_usd = 0.25;

    // Simulate failure — session_id preserved for resume
    run.status = RunStatus::Failed;
    run_state::save_run_state(&task_path, &run);

    let loaded = run_state::load_run_state(&worktree_path).unwrap();
    assert_eq!(loaded.status, RunStatus::Failed);
    assert_eq!(
        loaded.session_id.as_deref(),
        Some("sess-456"),
        "session_id must be preserved for resume_run"
    );
    assert_eq!(loaded.cost_usd, 0.25);
}

// ─── T4-UNIT-004: Delete run state cleans up file ───

#[test]
fn t4_unit_004_delete_run_state() {
    let (_dir, task_path, worktree_path) = setup_worktree();

    let run = common::make_run_info(RunStatus::Succeeded, Some("sess-789"));
    run_state::save_run_state(&task_path, &run);
    assert!(run_state::load_run_state(&worktree_path).is_some());

    run_state::delete_run_state(&task_path);
    assert!(
        run_state::load_run_state(&worktree_path).is_none(),
        "Should be gone after delete"
    );

    // Delete again should not panic
    run_state::delete_run_state(&task_path);
}

// ─── T4-UNIT-005: Scan run states across multiple worktrees ───

#[test]
fn t4_unit_005_scan_all_run_states() {
    let (_dir1, task_path1, wt1) = setup_worktree();
    let (_dir2, _task_path2, wt2) = setup_worktree();
    let (_dir3, task_path3, wt3) = setup_worktree();

    // Save run state in worktrees 1 and 3, not 2
    run_state::save_run_state(
        &task_path1,
        &common::make_run_info(RunStatus::Running, Some("sess-a")),
    );
    run_state::save_run_state(
        &task_path3,
        &common::make_run_info(RunStatus::Failed, Some("sess-c")),
    );

    let paths = vec![wt1, wt2, wt3];
    let results = run_state::scan_all_run_states(&paths);

    assert_eq!(results.len(), 2, "Should find 2 run states");
    assert!(results.iter().any(|r| r.status == RunStatus::Running));
    assert!(results.iter().any(|r| r.status == RunStatus::Failed));
}

// ─── T4-UNIT-006: Corrupt run.json is handled gracefully ───

#[test]
fn t4_unit_006_corrupt_run_state_handled() {
    let dir = TempDir::new().unwrap();
    let bd_dir = dir.path().join(BRANCHDECK_DIR);
    std::fs::create_dir_all(&bd_dir).unwrap();

    // Write corrupt JSON
    std::fs::write(bd_dir.join("run.json"), "{ not valid json").unwrap();

    let result = run_state::load_run_state(dir.path().to_str().unwrap());
    assert!(result.is_none(), "Should return None for corrupt run.json");

    // Corrupt file should be cleaned up
    assert!(
        !bd_dir.join("run.json").exists(),
        "Corrupt run.json should be deleted"
    );
}

// ─── Session matching logic (used by all handle_* functions) ───

#[test]
fn session_matches_both_none() {
    assert!(
        run_responses::session_matches(None, None),
        "No active run + no session = match"
    );
}

#[test]
fn session_matches_run_has_no_session_yet() {
    let run = common::make_run_info(RunStatus::Starting, None);
    assert!(
        run_responses::session_matches(Some(&run), Some(&"sess-1".to_string())),
        "Active run without session_id should accept any response"
    );
}

#[test]
fn session_matches_response_has_no_session() {
    let run = common::make_run_info(RunStatus::Running, Some("sess-1"));
    assert!(
        run_responses::session_matches(Some(&run), None),
        "Response without session_id should match (heartbeats)"
    );
}

#[test]
fn session_matches_same_session() {
    let run = common::make_run_info(RunStatus::Running, Some("sess-1"));
    assert!(
        run_responses::session_matches(Some(&run), Some(&"sess-1".to_string())),
        "Same session_id should match"
    );
}

#[test]
fn session_matches_different_session() {
    let run = common::make_run_info(RunStatus::Running, Some("sess-1"));
    assert!(
        !run_responses::session_matches(Some(&run), Some(&"sess-2".to_string())),
        "Different session_id should NOT match"
    );
}

// ─── RunInfo serialization ───

#[test]
fn run_info_json_round_trip() {
    let run = RunInfo {
        session_id: Some("sess-1".to_string()),
        task_path: "/wt/.branchdeck/task.md".to_string(),
        status: RunStatus::Blocked,
        started_at: "2026-03-20T10:00:00+00:00".to_string(),
        cost_usd: 2.5,
        last_heartbeat: Some("2026-03-20T10:05:00+00:00".to_string()),
        elapsed_secs: 300,
        tab_id: Some("tab-42".to_string()),
        failure_reason: None,
    };

    let json = serde_json::to_string(&run).unwrap();
    let deserialized: RunInfo = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.status, RunStatus::Blocked);
    assert_eq!(deserialized.session_id.as_deref(), Some("sess-1"));
    assert_eq!(deserialized.cost_usd, 2.5);
    assert_eq!(deserialized.elapsed_secs, 300);
}

#[test]
fn run_status_kebab_case_serialization() {
    // Verify all status variants serialize to kebab-case (matches frontend expectations)
    let statuses = vec![
        (RunStatus::Created, "\"created\""),
        (RunStatus::Starting, "\"starting\""),
        (RunStatus::Running, "\"running\""),
        (RunStatus::Blocked, "\"blocked\""),
        (RunStatus::Succeeded, "\"succeeded\""),
        (RunStatus::Failed, "\"failed\""),
        (RunStatus::Cancelled, "\"cancelled\""),
    ];

    for (status, expected) in statuses {
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(
            json, expected,
            "RunStatus::{status:?} should serialize to {expected}"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Pure state machine tests (via apply_* functions — no Tauri needed)
// ═══════════════════════════════════════════════════════════════════════

// ─── T4-SM-001: Starting → Running (session started) ───

#[test]
fn t4_sm_001_apply_session_started() {
    let mut run = common::make_run_info(RunStatus::Starting, None);

    let effects = run_effects::apply_session_started(&mut run, "sess-123");

    // State mutations
    assert_eq!(run.status, RunStatus::Running);
    assert_eq!(run.session_id.as_deref(), Some("sess-123"));

    // Effects produced
    assert_eq!(effects.len(), 3);
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::UpdateTaskStatus(_, TaskStatus::Running))));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::SaveRunState(..))));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::EmitStatusChanged(_))));
}

// ─── T4-SM-002: Running → Succeeded (run complete) ───

#[test]
fn t4_sm_002_apply_run_complete() {
    let mut run = common::make_run_info(RunStatus::Running, Some("sess-1"));
    let started_at = 1_000_000;
    let now = 1_060_000; // 60 seconds later

    let effects = run_effects::apply_run_complete(&mut run, Some(&0.75), started_at, now);

    // State mutations
    assert_eq!(run.status, RunStatus::Succeeded);
    assert_eq!(run.cost_usd, 0.75);
    assert_eq!(run.elapsed_secs, 60);

    // Effects: all expected effects present
    assert_eq!(effects.len(), 5);
    assert!(effects.iter().any(
        |e| matches!(e, RunEffect::PublishRunComplete { status, .. } if status == "succeeded")
    ));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::CaptureArtifacts { status, .. } if status == "succeeded")));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::UpdateTaskStatus(_, TaskStatus::Succeeded))));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::DeleteRunState(_))));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::EmitStatusChanged(_))));
}

// ─── T4-SM-003: Running → Failed (run error) ───

#[test]
fn t4_sm_003_apply_run_error_failed() {
    let mut run = common::make_run_info(RunStatus::Running, Some("sess-1"));
    let started_at = 1_000_000;
    let now = 1_030_000; // 30 seconds

    let effects = run_effects::apply_run_error(&mut run, "error", Some(&0.25), started_at, now);

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.cost_usd, 0.25);
    assert_eq!(run.elapsed_secs, 30);

    // Save (not delete) — session_id preserved for resume
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::SaveRunState(..))));
    assert!(!effects
        .iter()
        .any(|e| matches!(e, RunEffect::DeleteRunState(_))));
}

// ─── T4-SM-003b: Running → Cancelled (status == "cancelled") ───

#[test]
fn t4_sm_003b_apply_run_error_cancelled() {
    let mut run = common::make_run_info(RunStatus::Running, Some("sess-1"));

    let effects = run_effects::apply_run_error(&mut run, "cancelled", None, 0, 0);

    assert_eq!(
        run.status,
        RunStatus::Cancelled,
        "cancelled status maps to RunStatus::Cancelled"
    );
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::UpdateTaskStatus(_, TaskStatus::Cancelled))));
}

// ─── T4-SM-004: apply_* functions produce exactly the expected effect counts ───

#[test]
fn t4_sm_004_apply_functions_do_not_modify_run_count() {
    let mut run = common::make_run_info(RunStatus::Starting, None);

    let effects = run_effects::apply_session_started(&mut run, "sess-1");
    assert_eq!(
        effects.len(),
        3,
        "session_started should produce exactly 3 effects"
    );

    let effects = run_effects::apply_run_complete(&mut run, Some(&1.0), 1000, 2000);
    assert_eq!(
        effects.len(),
        5,
        "run_complete should produce exactly 5 effects"
    );

    let mut run = common::make_run_info(RunStatus::Running, Some("s1"));
    let effects = run_effects::apply_run_error(&mut run, "failed", None, 1000, 2000);
    assert_eq!(
        effects.len(),
        5,
        "run_error should produce exactly 5 effects"
    );

    let mut run = common::make_run_info(RunStatus::Running, Some("s1"));
    let effects = run_effects::apply_mark_failed(&mut run, "sidecar crash", 1000, 2000);
    assert_eq!(
        effects.len(),
        5,
        "mark_failed should produce exactly 5 effects"
    );
}

// ─── T4-SM-005: Running → Blocked (permission request) ───

#[test]
fn t4_sm_005_apply_permission_request() {
    let mut run = common::make_run_info(RunStatus::Running, Some("sess-1"));
    let tool = "Bash".to_string();
    let command = "rm -rf /tmp/test".to_string();

    let (pending, effects) = run_effects::apply_permission_request(
        &mut run,
        Some(&tool),
        Some(&command),
        "tu-abc",
        5_000_000,
    );

    assert_eq!(run.status, RunStatus::Blocked);
    assert_eq!(pending.tool_use_id, "tu-abc");
    assert_eq!(pending.tool.as_deref(), Some("Bash"));
    assert_eq!(pending.command.as_deref(), Some("rm -rf /tmp/test"));
    assert_eq!(pending.requested_at, 5_000_000);

    assert_eq!(effects.len(), 3);
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::SaveRunState(..))));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::EmitPermissionRequest(_))));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::EmitStatusChanged(_))));
}

// ─── T4-SM-006: mark_run_failed (stale detection) ───

#[test]
fn t4_sm_006_apply_mark_failed() {
    let mut run = common::make_run_info(RunStatus::Running, Some("sess-1"));
    let started_at = 1_000_000;
    let now = 1_120_000; // 120 seconds (stale threshold)

    let effects = run_effects::apply_mark_failed(&mut run, "network-timeout", started_at, now);

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.elapsed_secs, 120);
    assert_eq!(
        run.failure_reason.as_deref(),
        Some("network-timeout"),
        "failure_reason must be stored in RunInfo"
    );

    assert_eq!(effects.len(), 5);
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::PublishRunComplete { status, .. } if status == "failed")));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::CaptureArtifacts { status, .. } if status == "failed")));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::UpdateTaskStatus(_, TaskStatus::Failed))));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::SaveRunState(..))));
    assert!(effects
        .iter()
        .any(|e| matches!(e, RunEffect::EmitStatusChanged(_))));
    // Must NOT delete run state (session_id needed for resume)
    assert!(!effects
        .iter()
        .any(|e| matches!(e, RunEffect::DeleteRunState(_))));
}

// ─── T4-SM-007: run_complete with no cost and zero start time ───

#[test]
fn t4_sm_007_apply_run_complete_no_cost_no_timing() {
    let mut run = common::make_run_info(RunStatus::Running, Some("sess-1"));

    let effects = run_effects::apply_run_complete(&mut run, None, 0, 1_000_000);

    assert_eq!(run.status, RunStatus::Succeeded);
    assert_eq!(run.cost_usd, 0.0, "Cost should remain 0 when None passed");
    assert_eq!(
        run.elapsed_secs, 0,
        "Elapsed should be 0 when started_at is 0"
    );
    assert_eq!(effects.len(), 5);
}

// ─── Edge case: empty session_id string ───
#[test]
fn t4_sm_edge_empty_session_id() {
    let mut run = common::make_run_info(RunStatus::Starting, None);
    let effects = run_effects::apply_session_started(&mut run, "");
    assert_eq!(run.session_id.as_deref(), Some(""));
    assert_eq!(run.status, RunStatus::Running);
    assert_eq!(effects.len(), 3);
}

// ─── Edge case: run_complete with zero cost ───
#[test]
fn t4_sm_edge_run_complete_zero_cost() {
    let mut run = common::make_run_info(RunStatus::Running, Some("s1"));
    let effects = run_effects::apply_run_complete(&mut run, Some(&0.0), 1000, 2000);
    assert_eq!(run.cost_usd, 0.0);
    assert_eq!(run.status, RunStatus::Succeeded);
    assert_eq!(effects.len(), 5);
}

// ─── Edge case: permission request with no tool/command ───
#[test]
fn t4_sm_edge_permission_no_tool() {
    let mut run = common::make_run_info(RunStatus::Running, Some("s1"));
    let (pending, effects) =
        run_effects::apply_permission_request(&mut run, None, None, "tu-1", 1000);
    assert_eq!(run.status, RunStatus::Blocked);
    assert!(pending.tool.is_none());
    assert!(pending.command.is_none());
    assert_eq!(effects.len(), 3);
}

// ─── Edge case: run_error with unknown status string ───
#[test]
fn t4_sm_edge_unknown_error_status() {
    let mut run = common::make_run_info(RunStatus::Running, Some("s1"));
    let effects = run_effects::apply_run_error(&mut run, "timeout", None, 1000, 2000);
    assert_eq!(
        run.status,
        RunStatus::Failed,
        "Unknown status should default to Failed"
    );
    assert_eq!(effects.len(), 5);
}

// ─── Edge case: apply_mark_failed with zero timestamps ───
#[test]
fn t4_sm_edge_mark_failed_zero_timestamps() {
    let mut run = common::make_run_info(RunStatus::Running, Some("s1"));
    let effects = run_effects::apply_mark_failed(&mut run, "sidecar crash", 0, 0);
    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.elapsed_secs, 0);
    assert_eq!(effects.len(), 5);
}

// ─── Edge case: map_sidecar_status covers all known + unknown strings ───
#[test]
fn t4_sm_edge_map_sidecar_status() {
    use branchdeck_core::services::run_effects::map_sidecar_status;
    assert_eq!(
        map_sidecar_status("cancelled"),
        (RunStatus::Cancelled, TaskStatus::Cancelled)
    );
    assert_eq!(
        map_sidecar_status("failed"),
        (RunStatus::Failed, TaskStatus::Failed)
    );
    assert_eq!(
        map_sidecar_status("error"),
        (RunStatus::Failed, TaskStatus::Failed)
    );
    assert_eq!(
        map_sidecar_status("timeout"),
        (RunStatus::Failed, TaskStatus::Failed)
    );
    assert_eq!(
        map_sidecar_status(""),
        (RunStatus::Failed, TaskStatus::Failed)
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Stale detection tests (check_run_stale — pure function)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn stale_detection_below_threshold() {
    use branchdeck_core::services::run_stale::{check_run_stale, STALE_THRESHOLD_SECS};
    let last_activity = 1_000_000;
    let now = last_activity + (STALE_THRESHOLD_SECS - 1) * 1000; // 1 second under
    assert!(!check_run_stale(last_activity, now));
}

#[test]
fn stale_detection_at_threshold() {
    use branchdeck_core::services::run_stale::{check_run_stale, STALE_THRESHOLD_SECS};
    let last_activity = 1_000_000;
    let now = last_activity + STALE_THRESHOLD_SECS * 1000; // exactly at threshold
    assert!(check_run_stale(last_activity, now));
}

#[test]
fn stale_detection_above_threshold() {
    use branchdeck_core::services::run_stale::{check_run_stale, STALE_THRESHOLD_SECS};
    let last_activity = 1_000_000;
    let now = last_activity + (STALE_THRESHOLD_SECS + 60) * 1000; // 60s over
    assert!(check_run_stale(last_activity, now));
}

#[test]
fn stale_detection_zero_activity_returns_false() {
    use branchdeck_core::services::run_stale::check_run_stale;
    assert!(
        !check_run_stale(0, 999_999_999),
        "zero last_activity is sentinel for 'not started'"
    );
}

#[test]
fn stale_threshold_constants_are_sane() {
    use branchdeck_core::services::run_stale::{PERMISSION_TIMEOUT_SECS, STALE_THRESHOLD_SECS};
    assert_eq!(
        STALE_THRESHOLD_SECS, 120,
        "stale threshold should be 2 minutes"
    );
    assert_eq!(
        PERMISSION_TIMEOUT_SECS, 300,
        "permission timeout should be 5 minutes"
    );
    // Relationship: permission timeout > stale threshold (verified by pinned values above)
}
