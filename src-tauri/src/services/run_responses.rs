use crate::models::agent::{self, Event};
use crate::models::run::{PendingPermission, RunInfo, RunStatus, SidecarResponse};
use crate::models::task::TaskStatus;
use crate::services::{event_bus::EventBus, run_state, task};
use log::{error, info, warn};
use tauri::Emitter;

use super::run_manager::now_epoch_ms;

/// Check if a response's `session_id` matches the active run's `session_id`.
/// Returns `true` if they match or if either is `None` (not yet assigned).
/// Returns `false` (mismatch) only when both are `Some` and differ.
pub fn session_matches(active_run: Option<&RunInfo>, response_session_id: Option<&String>) -> bool {
    if let (Some(active_sid), Some(resp_sid)) = (
        active_run.and_then(|r| r.session_id.as_ref()),
        response_session_id,
    ) {
        if active_sid != resp_sid {
            return false;
        }
    }
    true
}

/// Handle a `SessionStarted` response from the sidecar.
pub fn handle_session_started<R: tauri::Runtime>(
    active_run: &mut Option<RunInfo>,
    session_id: &String,
    app_handle: &tauri::AppHandle<R>,
) {
    if let Some(ref mut run) = active_run {
        run.session_id = Some(session_id.clone());
        run.status = RunStatus::Running;
        info!("Run session started: {session_id}");
        task::update_task_status(&run.task_path, TaskStatus::Running);
        run_state::save_run_state(&run.task_path, run);
        if let Err(e) = app_handle.emit("run:status_changed", &*run) {
            error!("Failed to emit run:status_changed: {e}");
        }
    }
}

/// Handle a `RunStep`, `AssistantText`, or `ToolCall` response from the sidecar.
pub fn handle_run_step<R: tauri::Runtime>(
    response: &SidecarResponse,
    app_handle: &tauri::AppHandle<R>,
) {
    if let Err(e) = app_handle.emit("run:step", response) {
        error!("Failed to emit run:step: {e}");
    }
}

/// Handle a `RunComplete` response from the sidecar.
pub fn handle_run_complete<R: tauri::Runtime>(
    active_run: &mut Option<RunInfo>,
    started_at_epoch_ms: &mut u64,
    last_activity_ms: &mut u64,
    pending_permissions: &mut std::collections::HashMap<String, PendingPermission>,
    cost_usd: Option<&f64>,
    app_handle: &tauri::AppHandle<R>,
    event_bus: &EventBus,
) {
    if let Some(ref mut run) = active_run {
        run.status = RunStatus::Succeeded;
        if let Some(cost) = cost_usd {
            run.cost_usd = *cost;
        }
        if *started_at_epoch_ms > 0 {
            run.elapsed_secs = (now_epoch_ms().saturating_sub(*started_at_epoch_ms)) / 1000;
        }

        // Emit RunComplete event for KnowledgeService BEFORE clearing active_run
        emit_run_complete_event(event_bus, run, "succeeded");

        info!("Run completed successfully, cost: ${:.4}", run.cost_usd);
        task::capture_run_artifacts(&run.task_path, "succeeded", *started_at_epoch_ms);
        task::update_task_status(&run.task_path, TaskStatus::Succeeded);
        run_state::delete_run_state(&run.task_path);
        if let Err(e) = app_handle.emit("run:status_changed", &*run) {
            error!("Failed to emit run:status_changed: {e}");
        }
    }
    *active_run = None;
    *last_activity_ms = 0;
    *started_at_epoch_ms = 0;
    pending_permissions.clear();
}

/// Handle a `PermissionRequest` response from the sidecar.
pub fn handle_permission_request<R: tauri::Runtime>(
    active_run: &mut Option<RunInfo>,
    pending_permissions: &mut std::collections::HashMap<String, PendingPermission>,
    tool: Option<&String>,
    command: Option<&String>,
    tool_use_id: &str,
    app_handle: &tauri::AppHandle<R>,
) {
    if active_run.is_none() {
        warn!("Ignoring permission request: no active run");
        return;
    }
    info!("Permission requested for tool {tool:?}, command: {command:?}, id: {tool_use_id}");
    let pending = PendingPermission {
        tool: tool.cloned(),
        command: command.cloned(),
        tool_use_id: tool_use_id.to_owned(),
        requested_at: now_epoch_ms(),
    };
    pending_permissions.insert(tool_use_id.to_owned(), pending.clone());
    if let Some(ref mut run) = active_run {
        run.status = RunStatus::Blocked;
        run_state::save_run_state(&run.task_path, run);
        if let Err(e) = app_handle.emit("run:permission_request", &pending) {
            error!("Failed to emit run:permission_request: {e}");
        }
        if let Err(e) = app_handle.emit("run:status_changed", &*run) {
            error!("Failed to emit run:status_changed: {e}");
        }
    }
}

/// Handle a `RunError` response from the sidecar.
#[allow(clippy::too_many_arguments)]
pub fn handle_run_error<R: tauri::Runtime>(
    active_run: &mut Option<RunInfo>,
    started_at_epoch_ms: &mut u64,
    last_activity_ms: &mut u64,
    pending_permissions: &mut std::collections::HashMap<String, PendingPermission>,
    err_msg: &str,
    status: &str,
    cost_usd: Option<&f64>,
    app_handle: &tauri::AppHandle<R>,
    event_bus: &EventBus,
) {
    if let Some(ref mut run) = active_run {
        let (run_status, task_status) = if status == "cancelled" {
            (RunStatus::Cancelled, TaskStatus::Cancelled)
        } else {
            (RunStatus::Failed, TaskStatus::Failed)
        };
        run.status = run_status;
        if let Some(cost) = cost_usd {
            run.cost_usd = *cost;
        }
        if *started_at_epoch_ms > 0 {
            run.elapsed_secs = (now_epoch_ms().saturating_sub(*started_at_epoch_ms)) / 1000;
        }

        // Emit RunComplete event for KnowledgeService BEFORE clearing active_run
        emit_run_complete_event(event_bus, run, status);

        error!("Run failed: {err_msg}");
        task::capture_run_artifacts(&run.task_path, status, *started_at_epoch_ms);
        task::update_task_status(&run.task_path, task_status);
        // Save (but do not delete) run.json so session_id is
        // available for a subsequent resume_run.
        run_state::save_run_state(&run.task_path, run);
        if let Err(e) = app_handle.emit("run:status_changed", &*run) {
            error!("Failed to emit run:status_changed: {e}");
        }
    }
    *active_run = None;
    *last_activity_ms = 0;
    *started_at_epoch_ms = 0;
    pending_permissions.clear();
}

/// Emit a `RunComplete` event via the `EventBus` for `KnowledgeService` consumption.
/// Public variant for use from `RunManager` (`mark_run_failed_with_reason`).
pub fn emit_run_complete_event_pub(event_bus: &EventBus, run: &RunInfo, status: &str) {
    emit_run_complete_event(event_bus, run, status);
}

fn emit_run_complete_event(event_bus: &EventBus, run: &RunInfo, status: &str) {
    let _ = event_bus.publish(Event::RunComplete {
        session_id: run.session_id.clone().unwrap_or_default(),
        tab_id: run.tab_id.clone().unwrap_or_default(),
        status: status.to_string(),
        cost_usd: run.cost_usd,
        elapsed_secs: run.elapsed_secs,
        ts: agent::now_ms(),
    });
}
