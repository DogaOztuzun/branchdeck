//! Integration tests for `RunManager` queue logic.
//!
//! Tests the pure queue state management via public methods:
//! `get_queue_status`, `cancel_queue`, `record_queue_completion`.
//! Does NOT test sidecar spawning, stdout reading, or Tauri runtime integration.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use branchdeck_core::services::event_bus::EventBus;
use branchdeck_core::services::run_manager::{QueuedRun, RunManager};
use std::path::PathBuf;
use std::sync::Arc;

mod common;

/// Helper: create a `RunManager` with no real sidecar.
fn make_run_manager() -> RunManager {
    let event_bus = Arc::new(EventBus::new());
    RunManager::new(
        PathBuf::from("/nonexistent"),
        event_bus,
        common::test_emitter(),
        13370,
        1,
    )
}

// ─── Empty queue initially ───

#[test]
fn queue_status_empty_initially() {
    let rm = make_run_manager();
    let status = rm.get_queue_status();

    assert!(status.active.is_empty(), "No active run initially");
    assert!(status.queued.is_empty(), "Queue should be empty initially");
    assert_eq!(status.completed, 0);
    assert_eq!(status.failed, 0);
}

// ─── record_queue_completion increments completed ───

#[test]
fn record_completion_increments_completed() {
    let mut rm = make_run_manager();

    rm.record_queue_completion(true);
    rm.record_queue_completion(true);

    let status = rm.get_queue_status();
    assert_eq!(status.completed, 2);
    assert_eq!(status.failed, 0);
}

// ─── record_queue_completion increments failed ───

#[test]
fn record_completion_increments_failed() {
    let mut rm = make_run_manager();

    rm.record_queue_completion(false);

    let status = rm.get_queue_status();
    assert_eq!(status.completed, 0);
    assert_eq!(status.failed, 1);
}

// ─── Mixed completions tracked correctly ───

#[test]
fn record_completion_mixed() {
    let mut rm = make_run_manager();

    rm.record_queue_completion(true);
    rm.record_queue_completion(false);
    rm.record_queue_completion(true);
    rm.record_queue_completion(false);
    rm.record_queue_completion(true);

    let status = rm.get_queue_status();
    assert_eq!(status.completed, 3);
    assert_eq!(status.failed, 2);
}

// ─── cancel_queue clears queue and resets counts ───

#[test]
fn cancel_queue_clears_everything() {
    let mut rm = make_run_manager();

    // Record some completions first
    rm.record_queue_completion(true);
    rm.record_queue_completion(false);

    rm.cancel_queue();

    let status = rm.get_queue_status();
    assert!(
        status.queued.is_empty(),
        "Queue should be empty after cancel"
    );
    assert_eq!(status.completed, 0, "Completed count should reset");
    assert_eq!(status.failed, 0, "Failed count should reset");
}

// ─── QueuedRun serialization ───

#[test]
fn queued_run_serializes_to_camel_case() {
    let qr = QueuedRun {
        run_id: "run-test-1".to_string(),
        task_path: "/wt/.branchdeck/task.md".to_string(),
        worktree_path: "/wt".to_string(),
        options: branchdeck_core::models::run::LaunchOptions {
            max_turns: None,
            max_budget_usd: None,
            permission_mode: None,
            allowed_directories: Vec::new(),
        },
        failure_count: 0,
    };

    let json = serde_json::to_string(&qr).unwrap();
    assert!(json.contains("taskPath"), "Should use camelCase: {json}");
    assert!(
        json.contains("worktreePath"),
        "Should use camelCase: {json}"
    );
}

// ─── QueueStatus serialization ───

#[test]
fn queue_status_serializes_correctly() {
    let rm = make_run_manager();
    let status = rm.get_queue_status();

    let json = serde_json::to_string(&status).unwrap();
    // Verify camelCase field names
    assert!(json.contains("\"active\""), "Should have active field");
    assert!(json.contains("\"queued\""), "Should have queued field");
    assert!(
        json.contains("\"completed\""),
        "Should have completed field"
    );
    assert!(json.contains("\"failed\""), "Should have failed field");
}
