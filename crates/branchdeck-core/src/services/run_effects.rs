use crate::models::agent::{self, Event};
use crate::models::run::{PendingPermission, RunInfo, RunStatus};
use crate::models::task::TaskStatus;
use crate::services::{event_bus::EventBus, run_state, task};
use crate::traits::{self, EventEmitter};
use log::{error, info};

/// Side effects produced by pure state transition functions.
///
/// Each variant is a unit of work to be executed by `execute_effects`.
/// The executor is intentionally thin — one line per arm, no logic.
#[derive(Debug, Clone)]
pub enum RunEffect {
    UpdateTaskStatus(String, TaskStatus),
    SaveRunState(String, RunInfo),
    DeleteRunState(String),
    CaptureArtifacts {
        task_path: String,
        status: String,
        started_at: u64,
    },
    EmitStatusChanged(RunInfo),
    EmitPermissionRequest(PendingPermission),
    PublishRunComplete {
        run: RunInfo,
        status: String,
    },
    MarkWorktreeForCleanup {
        path: String,
    },
}

/// Execute a list of side effects. Each arm is one line — no logic here.
pub fn execute_effects(effects: Vec<RunEffect>, emitter: &dyn EventEmitter, event_bus: &EventBus) {
    for effect in effects {
        match effect {
            RunEffect::UpdateTaskStatus(ref path, status) => {
                task::update_task_status(path, status);
            }
            RunEffect::SaveRunState(ref path, ref info) => {
                run_state::save_run_state(path, info);
            }
            RunEffect::DeleteRunState(ref path) => {
                run_state::delete_run_state(path);
            }
            RunEffect::CaptureArtifacts {
                ref task_path,
                ref status,
                started_at,
            } => {
                task::capture_run_artifacts(task_path, status, started_at);
            }
            RunEffect::EmitStatusChanged(ref info) => {
                if let Err(e) = traits::emit(emitter, "run:status_changed", info) {
                    error!("Failed to emit run:status_changed: {e}");
                }
            }
            RunEffect::EmitPermissionRequest(ref pending) => {
                if let Err(e) = traits::emit(emitter, "run:permission_request", pending) {
                    error!("Failed to emit run:permission_request: {e}");
                }
            }
            RunEffect::PublishRunComplete {
                ref run,
                ref status,
            } => {
                let _ = event_bus.publish(Event::RunComplete {
                    session_id: run.session_id.clone().unwrap_or_default(),
                    tab_id: run.tab_id.clone().unwrap_or_default(),
                    status: status.clone(),
                    cost_usd: run.cost_usd,
                    elapsed_secs: run.elapsed_secs,
                    ts: agent::now_ms(),
                });
            }
            RunEffect::MarkWorktreeForCleanup { ref path } => {
                let marker = std::path::Path::new(path).join(".branchdeck-cleanup");
                if let Err(e) = crate::util::write_atomic(&marker, b"pending-cleanup") {
                    error!("Failed to write cleanup marker at {}: {e}", marker.display());
                } else {
                    info!("Marked worktree for cleanup: {path}");
                }
            }
        }
    }
}

// ── Pure state transition functions ──
// These mutate RunInfo and return effects as data. No I/O, no framework deps.

/// Map a sidecar status string to (`RunStatus`, `TaskStatus`).
/// Unknown statuses default to Failed.
#[must_use]
pub fn map_sidecar_status(status: &str) -> (RunStatus, TaskStatus) {
    match status {
        "succeeded" => (RunStatus::Succeeded, TaskStatus::Succeeded),
        "cancelled" => (RunStatus::Cancelled, TaskStatus::Cancelled),
        _ => (RunStatus::Failed, TaskStatus::Failed),
    }
}

/// Pure: apply session started transition.
/// Sets `session_id` and status to Running.
pub fn apply_session_started(run: &mut RunInfo, session_id: &str) -> Vec<RunEffect> {
    run.session_id = Some(session_id.to_owned());
    run.status = RunStatus::Running;
    vec![
        RunEffect::UpdateTaskStatus(run.task_path.clone(), TaskStatus::Running),
        RunEffect::SaveRunState(run.task_path.clone(), run.clone()),
        RunEffect::EmitStatusChanged(run.clone()),
    ]
}

/// Pure: apply run complete transition.
/// Sets status to Succeeded, captures cost and elapsed time.
pub fn apply_run_complete(
    run: &mut RunInfo,
    cost_usd: Option<&f64>,
    started_at_epoch_ms: u64,
    now_ms: u64,
) -> Vec<RunEffect> {
    run.status = RunStatus::Succeeded;
    if let Some(cost) = cost_usd {
        run.cost_usd = *cost;
    }
    if started_at_epoch_ms > 0 {
        run.elapsed_secs = (now_ms.saturating_sub(started_at_epoch_ms)) / 1000;
    }
    vec![
        RunEffect::PublishRunComplete {
            run: run.clone(),
            status: "succeeded".to_owned(),
        },
        RunEffect::CaptureArtifacts {
            task_path: run.task_path.clone(),
            status: "succeeded".to_owned(),
            started_at: started_at_epoch_ms,
        },
        RunEffect::UpdateTaskStatus(run.task_path.clone(), TaskStatus::Succeeded),
        RunEffect::DeleteRunState(run.task_path.clone()),
        RunEffect::EmitStatusChanged(run.clone()),
    ]
}

/// Pure: apply run error transition.
/// Maps status string to RunStatus/TaskStatus, captures cost and elapsed time.
/// Saves (not deletes) run state so `session_id` is available for resume.
pub fn apply_run_error(
    run: &mut RunInfo,
    status: &str,
    cost_usd: Option<&f64>,
    started_at_epoch_ms: u64,
    now_ms: u64,
) -> Vec<RunEffect> {
    let (run_status, task_status) = map_sidecar_status(status);
    run.status = run_status;
    if let Some(cost) = cost_usd {
        run.cost_usd = *cost;
    }
    if started_at_epoch_ms > 0 {
        run.elapsed_secs = (now_ms.saturating_sub(started_at_epoch_ms)) / 1000;
    }
    vec![
        RunEffect::PublishRunComplete {
            run: run.clone(),
            status: status.to_owned(),
        },
        RunEffect::CaptureArtifacts {
            task_path: run.task_path.clone(),
            status: status.to_owned(),
            started_at: started_at_epoch_ms,
        },
        RunEffect::UpdateTaskStatus(run.task_path.clone(), task_status),
        RunEffect::SaveRunState(run.task_path.clone(), run.clone()),
        RunEffect::EmitStatusChanged(run.clone()),
    ]
}

/// Pure: apply permission request transition.
/// Sets status to Blocked, returns the pending permission.
pub fn apply_permission_request(
    run: &mut RunInfo,
    tool: Option<&String>,
    command: Option<&String>,
    tool_use_id: &str,
    requested_at: u64,
) -> (PendingPermission, Vec<RunEffect>) {
    run.status = RunStatus::Blocked;
    let pending = PendingPermission {
        tool: tool.cloned(),
        command: command.cloned(),
        tool_use_id: tool_use_id.to_owned(),
        requested_at,
    };
    let effects = vec![
        RunEffect::SaveRunState(run.task_path.clone(), run.clone()),
        RunEffect::EmitPermissionRequest(pending.clone()),
        RunEffect::EmitStatusChanged(run.clone()),
    ];
    (pending, effects)
}

/// Pure: apply immediate cancellation transition.
///
/// Sets status to Cancelled, captures elapsed time, and emits lifecycle events.
/// Cost data may be unavailable because the sidecar was killed, so it's best-effort.
pub fn apply_cancel(
    run: &mut RunInfo,
    started_at_epoch_ms: u64,
    now_ms: u64,
) -> Vec<RunEffect> {
    run.status = RunStatus::Cancelled;
    if started_at_epoch_ms > 0 {
        run.elapsed_secs = (now_ms.saturating_sub(started_at_epoch_ms)) / 1000;
    }
    vec![
        RunEffect::PublishRunComplete {
            run: run.clone(),
            status: "cancelled".to_owned(),
        },
        RunEffect::CaptureArtifacts {
            task_path: run.task_path.clone(),
            status: "cancelled".to_owned(),
            started_at: started_at_epoch_ms,
        },
        RunEffect::UpdateTaskStatus(run.task_path.clone(), TaskStatus::Cancelled),
        RunEffect::SaveRunState(run.task_path.clone(), run.clone()),
        RunEffect::EmitStatusChanged(run.clone()),
    ]
}

/// Pure: apply mark-run-failed transition (sidecar crash or stale detection).
///
/// Unlike `apply_run_error`, this does NOT capture cost because it's called
/// when the sidecar is unresponsive — cost data is unavailable.
/// The `reason` is stored in `RunInfo.failure_reason` for downstream retry logic.
pub fn apply_mark_failed(
    run: &mut RunInfo,
    reason: &str,
    started_at_epoch_ms: u64,
    now_ms: u64,
) -> Vec<RunEffect> {
    run.status = RunStatus::Failed;
    run.failure_reason = Some(reason.to_owned());
    if started_at_epoch_ms > 0 {
        run.elapsed_secs = (now_ms.saturating_sub(started_at_epoch_ms)) / 1000;
    }
    vec![
        RunEffect::PublishRunComplete {
            run: run.clone(),
            status: "failed".to_owned(),
        },
        RunEffect::CaptureArtifacts {
            task_path: run.task_path.clone(),
            status: "failed".to_owned(),
            started_at: started_at_epoch_ms,
        },
        RunEffect::UpdateTaskStatus(run.task_path.clone(), TaskStatus::Failed),
        RunEffect::SaveRunState(run.task_path.clone(), run.clone()),
        RunEffect::EmitStatusChanged(run.clone()),
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn make_run(task_path: &str) -> RunInfo {
        RunInfo {
            session_id: Some("sess-1".to_owned()),
            task_path: task_path.to_owned(),
            status: RunStatus::Running,
            started_at: "2026-03-27T00:00:00Z".to_owned(),
            cost_usd: 0.05,
            last_heartbeat: None,
            elapsed_secs: 0,
            tab_id: Some("tab-1".to_owned()),
        }
    }

    #[test]
    fn cancel_sets_status_and_elapsed() {
        let mut run = make_run("/tmp/test/.branchdeck/task.md");
        let started = 1_000_000;
        let now = 1_030_000; // 30 seconds later

        let effects = apply_cancel(&mut run, started, now);

        assert_eq!(run.status, RunStatus::Cancelled);
        assert_eq!(run.elapsed_secs, 30);
        assert!(effects
            .iter()
            .any(|e| matches!(e, RunEffect::UpdateTaskStatus(_, TaskStatus::Cancelled))));
        assert!(effects.iter().any(
            |e| matches!(e, RunEffect::PublishRunComplete { status, .. } if status == "cancelled")
        ));
        assert!(effects
            .iter()
            .any(|e| matches!(e, RunEffect::EmitStatusChanged(_))));
        assert!(effects
            .iter()
            .any(|e| matches!(e, RunEffect::SaveRunState(..))));
    }

    #[test]
    fn cancel_with_zero_start_skips_elapsed() {
        let mut run = make_run("/tmp/test/.branchdeck/task.md");
        let effects = apply_cancel(&mut run, 0, 1_000_000);

        assert_eq!(run.status, RunStatus::Cancelled);
        assert_eq!(run.elapsed_secs, 0);
        assert!(!effects.is_empty());
    }

    #[test]
    fn map_sidecar_status_succeeded() {
        let (run_status, task_status) = map_sidecar_status("succeeded");
        assert_eq!(run_status, RunStatus::Succeeded);
        assert_eq!(task_status, TaskStatus::Succeeded);
    }

    #[test]
    fn complete_sets_succeeded_status() {
        let mut run = make_run("/tmp/test/.branchdeck/task.md");
        let effects = apply_run_complete(&mut run, Some(&0.12), 1_000_000, 1_060_000);

        assert_eq!(run.status, RunStatus::Succeeded);
        assert!((run.cost_usd - 0.12).abs() < f64::EPSILON);
        assert_eq!(run.elapsed_secs, 60);
        assert!(effects
            .iter()
            .any(|e| matches!(e, RunEffect::DeleteRunState(..))));
    }
}
