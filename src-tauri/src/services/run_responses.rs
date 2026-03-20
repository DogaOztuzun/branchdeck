use crate::models::run::{PendingPermission, RunInfo, SidecarResponse};
use crate::services::event_bus::EventBus;
use crate::services::run_effects::{self, execute_effects};
use log::{error, info, warn};

use super::run_manager::now_epoch_ms;

/// Check if a response's `session_id` matches the active run's `session_id`.
/// Returns `true` if they match or if either is `None` (not yet assigned).
/// Returns `false` (mismatch) only when both are `Some` and differ.
#[must_use]
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
    event_bus: &EventBus,
) {
    if let Some(ref mut run) = active_run {
        info!("Run session started: {session_id}");
        let effects = run_effects::apply_session_started(run, session_id);
        execute_effects(effects, app_handle, event_bus);
    }
}

/// Handle a `RunStep`, `AssistantText`, or `ToolCall` response from the sidecar.
pub fn handle_run_step<R: tauri::Runtime>(
    response: &SidecarResponse,
    app_handle: &tauri::AppHandle<R>,
    event_bus: &EventBus,
) {
    execute_effects(
        vec![run_effects::RunEffect::EmitRunStep(response.clone())],
        app_handle,
        event_bus,
    );
}

/// Handle a `RunComplete` response from the sidecar.
#[allow(clippy::implicit_hasher)]
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
        let now = now_epoch_ms();
        let effects = run_effects::apply_run_complete(run, cost_usd, *started_at_epoch_ms, now);
        info!("Run completed successfully, cost: ${:.4}", run.cost_usd);
        execute_effects(effects, app_handle, event_bus);
    }
    if !pending_permissions.is_empty() {
        warn!(
            "Clearing {} unresolved permission requests during run cleanup",
            pending_permissions.len()
        );
    }
    *active_run = None;
    *last_activity_ms = 0;
    *started_at_epoch_ms = 0;
    pending_permissions.clear();
}

/// Handle a `PermissionRequest` response from the sidecar.
#[allow(clippy::implicit_hasher)]
pub fn handle_permission_request<R: tauri::Runtime>(
    active_run: &mut Option<RunInfo>,
    pending_permissions: &mut std::collections::HashMap<String, PendingPermission>,
    tool: Option<&String>,
    command: Option<&String>,
    tool_use_id: &str,
    app_handle: &tauri::AppHandle<R>,
    event_bus: &EventBus,
) {
    let Some(ref mut run) = active_run else {
        warn!("Ignoring permission request: no active run");
        return;
    };
    info!("Permission requested for tool {tool:?}, command: {command:?}, id: {tool_use_id}");
    let now = now_epoch_ms();
    let (pending, effects) =
        run_effects::apply_permission_request(run, tool, command, tool_use_id, now);
    pending_permissions.insert(tool_use_id.to_owned(), pending);
    execute_effects(effects, app_handle, event_bus);
}

/// Handle a `RunError` response from the sidecar.
#[allow(clippy::too_many_arguments, clippy::implicit_hasher)]
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
        let now = now_epoch_ms();
        error!("Run failed: {err_msg}");
        let effects =
            run_effects::apply_run_error(run, status, cost_usd, *started_at_epoch_ms, now);
        execute_effects(effects, app_handle, event_bus);
    }
    if !pending_permissions.is_empty() {
        warn!(
            "Clearing {} unresolved permission requests during run cleanup",
            pending_permissions.len()
        );
    }
    *active_run = None;
    *last_activity_ms = 0;
    *started_at_epoch_ms = 0;
    pending_permissions.clear();
}
