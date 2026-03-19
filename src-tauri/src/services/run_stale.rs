use crate::models::run::{PendingPermission, PermissionResponseMsg, RunInfo, RunStatus};
use log::{error, warn};
use tauri::Emitter;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;

use super::run_manager::now_epoch_ms;

/// Stale threshold: if no heartbeat or activity for this many seconds, mark run failed.
pub const STALE_THRESHOLD_SECS: u64 = 120;

/// Permission timeout: auto-deny if no response within this many seconds.
pub const PERMISSION_TIMEOUT_SECS: u64 = 300;

/// Check if the active run is stale (no activity for `STALE_THRESHOLD_SECS`).
/// Returns `true` if the run was marked as stale and should be failed.
pub fn check_run_stale(last_activity_ms: u64) -> bool {
    if last_activity_ms == 0 {
        return false;
    }

    let now = now_epoch_ms();
    let elapsed_secs = (now.saturating_sub(last_activity_ms)) / 1000;

    if elapsed_secs >= STALE_THRESHOLD_SECS {
        warn!(
            "Stale run detected: no activity for {elapsed_secs}s (threshold: {STALE_THRESHOLD_SECS}s)"
        );
        return true;
    }

    false
}

/// Check if a pending permission request has timed out.
/// If timed out, sends an auto-deny to the sidecar, clears the pending permission,
/// and resets the run status to Running.
pub async fn check_permission_timeout<R: tauri::Runtime>(
    pending_permission: &mut Option<PendingPermission>,
    active_run: &mut Option<RunInfo>,
    stdin: Option<&mut ChildStdin>,
    app_handle: &tauri::AppHandle<R>,
) {
    let Some(perm) = pending_permission.as_ref() else {
        return;
    };

    let now = now_epoch_ms();
    let perm_elapsed = (now.saturating_sub(perm.requested_at)) / 1000;
    if perm_elapsed < PERMISSION_TIMEOUT_SECS {
        return;
    }

    warn!("Permission request timed out for tool {:?}", perm.tool);
    let tool_use_id = perm.tool_use_id.clone();

    // Auto-deny the timed-out permission
    let deny_msg = PermissionResponseMsg::PermissionResponse {
        tool_use_id,
        decision: "deny".to_owned(),
        reason: Some("Timed out after 5 minutes".to_owned()),
    };
    if let Some(stdin) = stdin {
        if let Ok(json) = serde_json::to_string(&deny_msg) {
            let bytes = format!("{json}\n");
            if let Err(e) = stdin.write_all(bytes.as_bytes()).await {
                error!("Failed to send auto-deny to sidecar: {e}");
            }
        }
    }
    *pending_permission = None;
    if let Some(ref mut run) = active_run {
        run.status = RunStatus::Running;
        if let Err(e) = app_handle.emit("run:status_changed", &*run) {
            error!("Failed to emit run:status_changed after permission timeout: {e}");
        }
    }
}
