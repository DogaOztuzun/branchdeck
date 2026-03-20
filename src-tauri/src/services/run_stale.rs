use crate::models::run::{PendingPermission, PermissionResponseMsg, RunInfo, RunStatus};
use crate::services::run_state;
use log::{error, warn};
use std::collections::HashMap;
use tauri::Emitter;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;

use super::run_manager::now_epoch_ms;

/// Stale threshold: if no heartbeat or activity for this many seconds, mark run failed.
pub const STALE_THRESHOLD_SECS: u64 = 120;

/// Permission timeout: auto-deny if no response within this many seconds.
pub const PERMISSION_TIMEOUT_SECS: u64 = 300;

/// Check if the active run is stale (no activity for `STALE_THRESHOLD_SECS`).
/// Returns `true` if the run should be marked as failed.
///
/// Takes `now_ms` as a parameter for testability (no internal clock dependency).
#[must_use]
pub fn check_run_stale(last_activity_ms: u64, now_ms: u64) -> bool {
    if last_activity_ms == 0 {
        return false;
    }

    let elapsed_secs = (now_ms.saturating_sub(last_activity_ms)) / 1000;

    if elapsed_secs >= STALE_THRESHOLD_SECS {
        warn!(
            "Stale run detected: no activity for {elapsed_secs}s (threshold: {STALE_THRESHOLD_SECS}s)"
        );
        return true;
    }

    false
}

/// Check if any pending permission requests have timed out.
/// If timed out, sends an auto-deny to the sidecar, removes from the map,
/// and resets the run status to Running if no permissions remain.
#[allow(clippy::implicit_hasher)]
pub async fn check_permission_timeout<R: tauri::Runtime>(
    pending_permissions: &mut HashMap<String, PendingPermission>,
    active_run: &mut Option<RunInfo>,
    mut stdin: Option<&mut ChildStdin>,
    app_handle: &tauri::AppHandle<R>,
) {
    if pending_permissions.is_empty() {
        return;
    }

    let now = now_epoch_ms();
    let timed_out: Vec<String> = pending_permissions
        .iter()
        .filter(|(_, perm)| {
            (now.saturating_sub(perm.requested_at)) / 1000 >= PERMISSION_TIMEOUT_SECS
        })
        .map(|(id, _)| id.clone())
        .collect();

    for tool_use_id in &timed_out {
        if let Some(perm) = pending_permissions.remove(tool_use_id) {
            warn!("Permission request timed out for tool {:?}", perm.tool);

            let deny_msg = PermissionResponseMsg::PermissionResponse {
                tool_use_id: tool_use_id.clone(),
                decision: "deny".to_owned(),
                reason: Some("Timed out after 5 minutes".to_owned()),
            };
            if let Some(ref mut stdin) = stdin {
                if let Ok(json) = serde_json::to_string(&deny_msg) {
                    let bytes = format!("{json}\n");
                    if let Err(e) = stdin.write_all(bytes.as_bytes()).await {
                        error!("Failed to send auto-deny to sidecar: {e}");
                    }
                    let _ = stdin.flush().await;
                }
            }
        }
    }

    // If no permissions remain, restore run status to Running
    if !timed_out.is_empty() && pending_permissions.is_empty() {
        if let Some(ref mut run) = active_run {
            run.status = RunStatus::Running;
            run_state::save_run_state(&run.task_path, run);
            if let Err(e) = app_handle.emit("run:status_changed", &*run) {
                error!("Failed to emit run:status_changed after permission timeout: {e}");
            }
        }
    }
}
