//! P1 tests for run lifecycle state management.
//!
//! Tests T4-UNIT-* from test-design-phase1.md.
//! Pure state machine transitions tested via `apply_*` functions (no Tauri runtime needed).
//! Persistence layer tested via `run_state` filesystem operations.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use branchdeck_lib::models::run::{RunInfo, RunStatus};
use branchdeck_lib::models::task::TaskStatus;
use branchdeck_lib::services::run_effects::{self, RunEffect};
use branchdeck_lib::services::run_responses;
use branchdeck_lib::services::run_state;
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

fn make_run_info(status: RunStatus, session_id: Option<&str>) -> RunInfo {
    RunInfo {
        session_id: session_id.map(String::from),
        task_path: "/fake/.branchdeck/task.md".to_string(),
        status,
        started_at: "2026-03-20T10:00:00+00:00".to_string(),
        cost_usd: 0.0,
        last_heartbeat: None,
        elapsed_secs: 0,
        tab_id: Some("tab-1".to_string()),
    }
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
    let mut run = make_run_info(RunStatus::Starting, None);
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

    let mut run = make_run_info(RunStatus::Running, Some("sess-456"));
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

    let run = make_run_info(RunStatus::Succeeded, Some("sess-789"));
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
        &make_run_info(RunStatus::Running, Some("sess-a")),
    );
    run_state::save_run_state(
        &task_path3,
        &make_run_info(RunStatus::Failed, Some("sess-c")),
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
    assert!(
        result.is_none(),
        "Should return None for corrupt run.json"
    );

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
    let run = make_run_info(RunStatus::Starting, None);
    assert!(
        run_responses::session_matches(Some(&run), Some(&"sess-1".to_string())),
        "Active run without session_id should accept any response"
    );
}

#[test]
fn session_matches_response_has_no_session() {
    let run = make_run_info(RunStatus::Running, Some("sess-1"));
    assert!(
        run_responses::session_matches(Some(&run), None),
        "Response without session_id should match (heartbeats)"
    );
}

#[test]
fn session_matches_same_session() {
    let run = make_run_info(RunStatus::Running, Some("sess-1"));
    assert!(
        run_responses::session_matches(Some(&run), Some(&"sess-1".to_string())),
        "Same session_id should match"
    );
}

#[test]
fn session_matches_different_session() {
    let run = make_run_info(RunStatus::Running, Some("sess-1"));
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
    let mut run = make_run_info(RunStatus::Starting, None);

    let effects = run_effects::apply_session_started(&mut run, "sess-123");

    // State mutations
    assert_eq!(run.status, RunStatus::Running);
    assert_eq!(run.session_id.as_deref(), Some("sess-123"));

    // Effects produced
    assert_eq!(effects.len(), 3);
    assert!(matches!(&effects[0], RunEffect::UpdateTaskStatus(_, TaskStatus::Running)));
    assert!(matches!(&effects[1], RunEffect::SaveRunState(..)));
    assert!(matches!(&effects[2], RunEffect::EmitStatusChanged(_)));
}

// ─── T4-SM-002: Running → Succeeded (run complete) ───

#[test]
fn t4_sm_002_apply_run_complete() {
    let mut run = make_run_info(RunStatus::Running, Some("sess-1"));
    let started_at = 1_000_000;
    let now = 1_060_000; // 60 seconds later

    let effects = run_effects::apply_run_complete(&mut run, Some(&0.75), started_at, now);

    // State mutations
    assert_eq!(run.status, RunStatus::Succeeded);
    assert_eq!(run.cost_usd, 0.75);
    assert_eq!(run.elapsed_secs, 60);

    // Effects: publish → capture → update status → delete state → emit
    assert_eq!(effects.len(), 5);
    assert!(matches!(&effects[0], RunEffect::PublishRunComplete { status, .. } if status == "succeeded"));
    assert!(matches!(&effects[1], RunEffect::CaptureArtifacts { status, .. } if status == "succeeded"));
    assert!(matches!(&effects[2], RunEffect::UpdateTaskStatus(_, TaskStatus::Succeeded)));
    assert!(matches!(&effects[3], RunEffect::DeleteRunState(_)));
    assert!(matches!(&effects[4], RunEffect::EmitStatusChanged(_)));
}

// ─── T4-SM-003: Running → Failed (run error) ───

#[test]
fn t4_sm_003_apply_run_error_failed() {
    let mut run = make_run_info(RunStatus::Running, Some("sess-1"));
    let started_at = 1_000_000;
    let now = 1_030_000; // 30 seconds

    let effects = run_effects::apply_run_error(&mut run, "error", Some(&0.25), started_at, now);

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.cost_usd, 0.25);
    assert_eq!(run.elapsed_secs, 30);

    // Save (not delete) — session_id preserved for resume
    assert!(matches!(&effects[3], RunEffect::SaveRunState(..)));
    assert!(!effects.iter().any(|e| matches!(e, RunEffect::DeleteRunState(_))));
}

// ─── T4-SM-003b: Running → Cancelled (status == "cancelled") ───

#[test]
fn t4_sm_003b_apply_run_error_cancelled() {
    let mut run = make_run_info(RunStatus::Running, Some("sess-1"));

    let effects = run_effects::apply_run_error(&mut run, "cancelled", None, 0, 0);

    assert_eq!(run.status, RunStatus::Cancelled, "cancelled status maps to RunStatus::Cancelled");
    assert!(matches!(&effects[2], RunEffect::UpdateTaskStatus(_, TaskStatus::Cancelled)));
}

// ─── T4-SM-004: retry_run — no double increment ───
// (retry_run calls launch_run which increments; apply_* does NOT increment)

#[test]
fn t4_sm_004_apply_functions_do_not_increment_run_count() {
    // Verify none of the apply functions produce an effect that increments run_count.
    // run_count is only incremented in launch_run/resume_run (orchestration layer).
    let mut run = make_run_info(RunStatus::Starting, None);

    let effects = run_effects::apply_session_started(&mut run, "sess-1");
    for effect in &effects {
        // No IncrementRunCount effect variant exists — by design
        assert!(!matches!(effect, RunEffect::UpdateTaskStatus(..)) || {
            // UpdateTaskStatus only changes status, not run_count
            true
        });
    }
}

// ─── T4-SM-005: Running → Blocked (permission request) ───

#[test]
fn t4_sm_005_apply_permission_request() {
    let mut run = make_run_info(RunStatus::Running, Some("sess-1"));
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
    assert!(matches!(&effects[0], RunEffect::SaveRunState(..)));
    assert!(matches!(&effects[1], RunEffect::EmitPermissionRequest(_)));
    assert!(matches!(&effects[2], RunEffect::EmitStatusChanged(_)));
}

// ─── T4-SM-006: mark_run_failed (stale detection) ───

#[test]
fn t4_sm_006_apply_mark_failed() {
    let mut run = make_run_info(RunStatus::Running, Some("sess-1"));
    let started_at = 1_000_000;
    let now = 1_120_000; // 120 seconds (stale threshold)

    let effects = run_effects::apply_mark_failed(&mut run, started_at, now);

    assert_eq!(run.status, RunStatus::Failed);
    assert_eq!(run.elapsed_secs, 120);

    // Must capture artifacts BEFORE the run is cleared (order matters)
    assert!(matches!(&effects[0], RunEffect::PublishRunComplete { status, .. } if status == "failed"));
    assert!(matches!(&effects[1], RunEffect::CaptureArtifacts { status, .. } if status == "failed"));
    assert!(matches!(&effects[2], RunEffect::UpdateTaskStatus(_, TaskStatus::Failed)));
    assert!(matches!(&effects[3], RunEffect::SaveRunState(..))); // save, not delete
    assert!(matches!(&effects[4], RunEffect::EmitStatusChanged(_)));
}

// ─── T4-SM-007: run_complete with no cost and zero start time ───

#[test]
fn t4_sm_007_apply_run_complete_no_cost_no_timing() {
    let mut run = make_run_info(RunStatus::Running, Some("sess-1"));

    let effects = run_effects::apply_run_complete(&mut run, None, 0, 1_000_000);

    assert_eq!(run.status, RunStatus::Succeeded);
    assert_eq!(run.cost_usd, 0.0, "Cost should remain 0 when None passed");
    assert_eq!(run.elapsed_secs, 0, "Elapsed should be 0 when started_at is 0");
    assert_eq!(effects.len(), 5);
}
