use crate::models::agent::{self, Event};
use crate::models::run::{PendingPermission, RunInfo, RunStatus};
use crate::models::task::TaskStatus;
use crate::services::{event_bus::EventBus, run_state, task};
use crate::traits::{self, EventEmitter};
use log::error;

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

/// Pure: apply mark-run-failed transition (sidecar crash or stale detection).
///
/// Unlike `apply_run_error`, this does NOT capture cost because it's called
/// when the sidecar is unresponsive — cost data is unavailable.
pub fn apply_mark_failed(
    run: &mut RunInfo,
    started_at_epoch_ms: u64,
    now_ms: u64,
) -> Vec<RunEffect> {
    run.status = RunStatus::Failed;
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
