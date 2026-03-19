use crate::error::AppError;
use crate::models::run::{
    LaunchOptions, PendingPermission, RunInfo, RunStatus, SidecarRequest, SidecarResponse,
};
use crate::models::task::TaskStatus;
use crate::services::{run_responses, run_stale, run_state, task};
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::Mutex;

/// Get the current time as epoch milliseconds.
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn now_epoch_ms() -> u64 {
    // Truncation from u128 won't occur before year ~584 million
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Convert epoch milliseconds to an RFC 3339 string.
fn epoch_ms_to_rfc3339(epoch_ms: u64) -> String {
    let secs = (epoch_ms / 1000).cast_signed();
    // Nanos from millisecond remainder always fits in u32
    #[allow(clippy::cast_possible_truncation)]
    let nanos = ((epoch_ms % 1000) * 1_000_000) as u32;
    chrono::DateTime::from_timestamp(secs, nanos)
        .unwrap_or_default()
        .to_rfc3339()
}

pub struct RunManager {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    active_run: Option<RunInfo>,
    sidecar_path: PathBuf,
    /// Epoch milliseconds of the last heartbeat or activity from the sidecar.
    last_activity_ms: u64,
    /// Epoch milliseconds when the current run started.
    started_at_epoch_ms: u64,
    /// Pending permission requests awaiting user decisions, keyed by tool_use_id.
    pending_permissions: std::collections::HashMap<String, PendingPermission>,
}

impl RunManager {
    #[must_use]
    pub fn new(sidecar_path: PathBuf) -> Self {
        Self {
            process: None,
            stdin: None,
            active_run: None,
            sidecar_path,
            last_activity_ms: 0,
            started_at_epoch_ms: 0,
            pending_permissions: std::collections::HashMap::new(),
        }
    }

    /// Spawn the sidecar process if not already running.
    ///
    /// # Errors
    ///
    /// Returns `SidecarError` if the Node.js process cannot be spawned.
    fn ensure_sidecar<R: tauri::Runtime>(
        &mut self,
        app_handle: tauri::AppHandle<R>,
        state: RunManagerState,
    ) -> Result<(), AppError> {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    warn!("Sidecar process exited with {status}, will respawn");
                    self.process = None;
                    self.stdin = None;
                }
                Ok(None) => {
                    debug!("Sidecar already running");
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to check sidecar process status: {e}, will respawn");
                    self.process = None;
                    self.stdin = None;
                }
            }
        }

        info!("Spawning sidecar at {}", self.sidecar_path.display());
        let start = std::time::Instant::now();

        let mut child = tokio::process::Command::new("node")
            .arg(&self.sidecar_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| {
                error!("Failed to spawn sidecar: {e}");
                AppError::SidecarError(format!("Failed to spawn node process: {e}"))
            })?;

        let child_stdout = child.stdout.take().ok_or_else(|| {
            error!("Sidecar stdout not available");
            AppError::SidecarError("Sidecar stdout not available".to_owned())
        })?;

        let child_stdin = child.stdin.take().ok_or_else(|| {
            error!("Sidecar stdin not available");
            AppError::SidecarError("Sidecar stdin not available".to_owned())
        })?;

        self.process = Some(child);
        self.stdin = Some(child_stdin);

        // Spawn reader task for stdout
        let reader = BufReader::new(child_stdout);
        start_stdout_reader(state, app_handle, reader);

        info!("Sidecar spawned in {:?}", start.elapsed());
        Ok(())
    }

    /// Get the current run status with computed elapsed time.
    #[must_use]
    pub fn get_status(&self) -> Option<RunInfo> {
        debug!("Getting run status");
        self.active_run.clone().map(|mut run| {
            if self.started_at_epoch_ms > 0 {
                let now = now_epoch_ms();
                run.elapsed_secs = (now.saturating_sub(self.started_at_epoch_ms)) / 1000;
            }
            if self.last_activity_ms > 0 {
                run.last_heartbeat = Some(epoch_ms_to_rfc3339(self.last_activity_ms));
            }
            run
        })
    }

    /// Record activity (heartbeat or any sidecar response).
    fn update_activity(&mut self) {
        self.last_activity_ms = now_epoch_ms();
    }

    /// Check if the active run is stale (no activity for the stale threshold).
    /// If stale, marks the run as failed with a "stalled" reason.
    /// Also checks for permission request timeouts.
    pub async fn check_stale<R: tauri::Runtime>(&mut self, app_handle: &tauri::AppHandle<R>) {
        if self.active_run.is_none() {
            return;
        }

        if run_stale::check_run_stale(self.last_activity_ms) {
            self.mark_run_failed_with_reason(app_handle, "stalled: no heartbeat for 120s");
            return;
        }

        run_stale::check_permission_timeout(
            &mut self.pending_permissions,
            &mut self.active_run,
            self.stdin.as_mut(),
            app_handle,
        )
        .await;
    }

    /// Update the active run from a sidecar response.
    /// Dispatches to handler functions in `run_responses`.
    pub fn handle_response<R: tauri::Runtime>(
        &mut self,
        response: &SidecarResponse,
        app_handle: &tauri::AppHandle<R>,
    ) {
        // Update activity timestamp on every response (heartbeat or real)
        self.update_activity();

        match response {
            SidecarResponse::Heartbeat { session_id } => {
                if !run_responses::session_matches(self.active_run.as_ref(), session_id.as_ref()) {
                    warn!("Ignoring heartbeat with mismatched session_id: {session_id:?}");
                    return;
                }
                debug!("Heartbeat received");
            }
            SidecarResponse::SessionStarted { session_id } => {
                run_responses::handle_session_started(&mut self.active_run, session_id, app_handle);
            }
            SidecarResponse::RunStep { session_id, .. }
            | SidecarResponse::AssistantText { session_id, .. }
            | SidecarResponse::ToolCall { session_id, .. } => {
                if !run_responses::session_matches(self.active_run.as_ref(), session_id.as_ref()) {
                    warn!("Ignoring run step with mismatched session_id: {session_id:?}");
                    return;
                }
                run_responses::handle_run_step(response, app_handle);
            }
            SidecarResponse::RunComplete {
                cost_usd,
                session_id,
                ..
            } => {
                if !run_responses::session_matches(self.active_run.as_ref(), session_id.as_ref()) {
                    warn!("Ignoring run complete with mismatched session_id: {session_id:?}");
                    return;
                }
                run_responses::handle_run_complete(
                    &mut self.active_run,
                    &mut self.started_at_epoch_ms,
                    &mut self.last_activity_ms,
                    &mut self.pending_permissions,
                    cost_usd.as_ref(),
                    app_handle,
                );
            }
            SidecarResponse::PermissionRequest {
                tool,
                command,
                tool_use_id,
                session_id,
            } => {
                if !run_responses::session_matches(self.active_run.as_ref(), session_id.as_ref()) {
                    warn!("Ignoring permission request with mismatched session_id: {session_id:?}");
                    return;
                }
                run_responses::handle_permission_request(
                    &mut self.active_run,
                    &mut self.pending_permissions,
                    tool.as_ref(),
                    command.as_ref(),
                    tool_use_id,
                    app_handle,
                );
            }
            SidecarResponse::RunError {
                error: err_msg,
                status,
                cost_usd,
                session_id,
            } => {
                if !run_responses::session_matches(self.active_run.as_ref(), session_id.as_ref()) {
                    warn!("Ignoring run error with mismatched session_id: {session_id:?}");
                    return;
                }
                run_responses::handle_run_error(
                    &mut self.active_run,
                    &mut self.started_at_epoch_ms,
                    &mut self.last_activity_ms,
                    &mut self.pending_permissions,
                    err_msg,
                    status,
                    cost_usd.as_ref(),
                    app_handle,
                );
            }
        }
    }

    /// Mark the active run as failed (used when sidecar crashes).
    pub fn mark_run_failed<R: tauri::Runtime>(&mut self, app_handle: &tauri::AppHandle<R>) {
        self.mark_run_failed_with_reason(app_handle, "sidecar crash");
    }

    /// Mark the active run as failed with a specific reason.
    pub fn mark_run_failed_with_reason<R: tauri::Runtime>(
        &mut self,
        app_handle: &tauri::AppHandle<R>,
        reason: &str,
    ) {
        if let Some(ref mut run) = self.active_run {
            run.status = RunStatus::Failed;
            if self.started_at_epoch_ms > 0 {
                run.elapsed_secs = (now_epoch_ms().saturating_sub(self.started_at_epoch_ms)) / 1000;
            }
            warn!("Marking active run as failed: {reason}");
            task::update_task_status(&run.task_path, TaskStatus::Failed);
            // Save (but do not delete) run.json so session_id is
            // available for a subsequent resume_run.
            run_state::save_run_state(&run.task_path, run);
            if let Err(e) = app_handle.emit("run:status_changed", &*run) {
                error!("Failed to emit run:status_changed: {e}");
            }
        }
        self.active_run = None;
        self.process = None;
        self.stdin = None;
        self.last_activity_ms = 0;
        self.started_at_epoch_ms = 0;
        self.pending_permissions.clear();
    }

    /// Shut down the run manager during app exit.
    ///
    /// If there is an active run, kills the sidecar child process,
    /// marks the run as failed, updates task.md, and cleans up run.json.
    pub fn shutdown<R: tauri::Runtime>(&mut self, app_handle: &tauri::AppHandle<R>) {
        if self.active_run.is_none() {
            debug!("Shutdown: no active run to clean up");
            return;
        }

        // Kill the sidecar child process if it's running
        if let Some(ref mut child) = self.process {
            info!("Shutdown: killing sidecar child process");
            if let Err(e) = child.start_kill() {
                error!("Shutdown: failed to kill sidecar process: {e}");
            }
        }

        // Mark run as failed (also saves + deletes run.json)
        self.mark_run_failed(app_handle);
        info!("Shutdown: cleaned up active run");
    }

    /// Cancel the active run.
    ///
    /// # Errors
    ///
    /// Returns `RunError` if no run is active.
    /// Returns `SidecarError` if the cancel command cannot be sent.
    pub async fn cancel_run(&mut self) -> Result<(), AppError> {
        let session_id = self
            .active_run
            .as_ref()
            .map(|r| r.session_id.clone())
            .ok_or_else(|| {
                error!("Cannot cancel: no active run");
                AppError::RunError("No active run to cancel".to_owned())
            })?;

        let request = SidecarRequest::CancelRun { session_id };
        self.send_request(&request).await?;

        info!("Sent cancel request for active run");
        Ok(())
    }

    /// Respond to a pending permission request.
    ///
    /// # Errors
    ///
    /// Returns `RunError` if no matching permission is pending.
    /// Returns `SidecarError` if the response cannot be sent.
    pub async fn respond_to_permission(
        &mut self,
        tool_use_id: &str,
        decision: &str,
        reason: Option<&str>,
    ) -> Result<(), AppError> {
        let pending = self.pending_permissions.remove(tool_use_id).ok_or_else(|| {
            error!("No pending permission for tool_use_id: {tool_use_id}");
            AppError::RunError("No matching pending permission request".to_owned())
        })?;

        info!(
            "Responding to permission for tool {:?}: {decision}",
            pending.tool
        );

        let response_msg = crate::models::run::PermissionResponseMsg::PermissionResponse {
            tool_use_id: tool_use_id.to_owned(),
            decision: decision.to_owned(),
            reason: reason.map(str::to_owned),
        };

        self.send_permission_response(&response_msg).await?;

        if let Some(ref mut run) = self.active_run {
            run.status = RunStatus::Running;
            run_state::save_run_state(&run.task_path, run);
        }

        Ok(())
    }

    /// Send a permission response JSON message to the sidecar via stdin.
    async fn send_permission_response(
        &mut self,
        response: &crate::models::run::PermissionResponseMsg,
    ) -> Result<(), AppError> {
        let stdin = self.stdin.as_mut().ok_or_else(|| {
            error!("Sidecar stdin not available for permission response");
            AppError::SidecarError("Sidecar not running".to_owned())
        })?;

        let mut json = serde_json::to_string(response).map_err(|e| {
            error!("Failed to serialize permission response: {e}");
            AppError::SidecarError(format!("JSON serialization error: {e}"))
        })?;
        json.push('\n');

        stdin.write_all(json.as_bytes()).await.map_err(|e| {
            error!("Failed to write permission response to sidecar stdin: {e}");
            AppError::SidecarError(format!("Failed to write to sidecar: {e}"))
        })?;

        stdin.flush().await.map_err(|e| {
            error!("Failed to flush sidecar stdin: {e}");
            AppError::SidecarError(format!("Failed to flush sidecar stdin: {e}"))
        })?;

        debug!("Sent permission response to sidecar");
        Ok(())
    }

    /// Send a JSON request to the sidecar via stdin.
    async fn send_request(&mut self, request: &SidecarRequest) -> Result<(), AppError> {
        let stdin = self.stdin.as_mut().ok_or_else(|| {
            error!("Sidecar stdin not available");
            AppError::SidecarError("Sidecar not running".to_owned())
        })?;

        let mut json = serde_json::to_string(request).map_err(|e| {
            error!("Failed to serialize sidecar request: {e}");
            AppError::SidecarError(format!("JSON serialization error: {e}"))
        })?;
        json.push('\n');

        stdin.write_all(json.as_bytes()).await.map_err(|e| {
            error!("Failed to write to sidecar stdin: {e}");
            AppError::SidecarError(format!("Failed to write to sidecar: {e}"))
        })?;

        stdin.flush().await.map_err(|e| {
            error!("Failed to flush sidecar stdin: {e}");
            AppError::SidecarError(format!("Failed to flush sidecar stdin: {e}"))
        })?;

        debug!("Sent request to sidecar");
        Ok(())
    }
}

/// Launch a run for the given task.
///
/// This is a standalone function (not a method) because it needs both the
/// `RunManagerState` arc (to pass to the stdout reader) and mutable access
/// to the inner `RunManager`.
///
/// # Errors
///
/// Returns `RunError` if a run is already active.
/// Returns `TaskNotFound` if the task file does not exist.
/// Returns `SidecarError` if the sidecar cannot be spawned or written to.
pub async fn launch_run<R: tauri::Runtime>(
    state: RunManagerState,
    app_handle: tauri::AppHandle<R>,
    task_path: &str,
    worktree_path: &str,
    options: LaunchOptions,
) -> Result<RunInfo, AppError> {
    let mut manager = state.lock().await;

    if manager.active_run.is_some() {
        error!("Cannot launch run: a run is already active");
        return Err(AppError::RunError("A run is already active".to_owned()));
    }

    let task_file = Path::new(task_path);
    if !task_file.exists() {
        error!("Task file not found: {task_path}");
        return Err(AppError::TaskNotFound(task_path.to_owned()));
    }

    // Validate that task.md parses correctly before sending to sidecar
    let content = std::fs::read_to_string(task_file).map_err(|e| {
        error!("Failed to read task file {task_path}: {e}");
        AppError::Io(e)
    })?;
    crate::services::task::parse_task_md(&content, task_path)?;

    manager.ensure_sidecar(app_handle.clone(), Arc::clone(&state))?;

    let tab_id = uuid::Uuid::new_v4().to_string();
    let request = SidecarRequest::LaunchRun {
        task_path: task_path.to_owned(),
        worktree: worktree_path.to_owned(),
        options,
        hook_port: crate::HOOK_RECEIVER_PORT,
        tab_id: tab_id.clone(),
    };

    manager.send_request(&request).await?;

    let now = chrono::Utc::now().to_rfc3339();
    let now_ms = now_epoch_ms();
    let run_info = RunInfo {
        session_id: None,
        task_path: task_path.to_owned(),
        status: RunStatus::Starting,
        started_at: now,
        cost_usd: 0.0,
        last_heartbeat: None,
        elapsed_secs: 0,
        tab_id: Some(tab_id),
    };

    task::increment_run_count(task_path);
    manager.started_at_epoch_ms = now_ms;
    manager.last_activity_ms = now_ms;
    manager.active_run = Some(run_info.clone());
    run_state::save_run_state(task_path, &run_info);

    info!("Launched run for task {task_path}");

    if let Err(e) = app_handle.emit("run:status_changed", &run_info) {
        error!("Failed to emit run:status_changed event: {e}");
    }

    Ok(run_info)
}

/// Retry a failed or cancelled run by launching a fresh session.
///
/// Verifies no active run exists, the task is in a terminal-failed state,
/// resets task status to running, increments run count, then launches.
///
/// # Errors
///
/// Returns `RunError` if a run is already active or the task is not failed/cancelled.
/// Returns `TaskNotFound` if the task file does not exist.
/// Returns `SidecarError` if the sidecar cannot be spawned or written to.
pub async fn retry_run<R: tauri::Runtime>(
    state: RunManagerState,
    app_handle: tauri::AppHandle<R>,
    task_path: &str,
    worktree_path: &str,
) -> Result<RunInfo, AppError> {
    // Validate task exists and is in a retryable state
    let task_info = task::get_task(worktree_path)?;
    match task_info.frontmatter.status {
        TaskStatus::Failed | TaskStatus::Cancelled => {}
        other => {
            error!("Cannot retry task with status {other:?}: {task_path}");
            return Err(AppError::RunError(format!(
                "Cannot retry: task status is {other:?}, expected failed or cancelled"
            )));
        }
    }

    // Reset task status and increment run count
    task::update_task_status(task_path, TaskStatus::Running);
    task::increment_run_count(task_path);

    // Launch a fresh run (handles active-run check, sidecar spawn, etc.)
    let options = LaunchOptions {
        max_turns: None,
        max_budget_usd: None,
    };
    launch_run(state, app_handle, task_path, worktree_path, options).await
}

/// Resume a failed or cancelled run by continuing its previous SDK session.
///
/// Loads the `session_id` from the last known run state (run.json), then sends
/// a `ResumeRun` request to the sidecar so the SDK picks up where it left off.
///
/// # Errors
///
/// Returns `RunError` if a run is already active, the task is not failed/cancelled,
/// or no `session_id` is available to resume.
/// Returns `TaskNotFound` if the task file does not exist.
/// Returns `SidecarError` if the sidecar cannot be spawned or written to.
pub async fn resume_run<R: tauri::Runtime>(
    state: RunManagerState,
    app_handle: tauri::AppHandle<R>,
    task_path: &str,
    worktree_path: &str,
) -> Result<RunInfo, AppError> {
    // Validate task exists and is in a resumable state
    let task_info = task::get_task(worktree_path)?;
    match task_info.frontmatter.status {
        TaskStatus::Failed | TaskStatus::Cancelled => {}
        other => {
            error!("Cannot resume task with status {other:?}: {task_path}");
            return Err(AppError::RunError(format!(
                "Cannot resume: task status is {other:?}, expected failed or cancelled"
            )));
        }
    }

    // Load session_id from run.json (saved during previous run)
    let previous_run = run_state::load_run_state(worktree_path);
    let session_id = previous_run.and_then(|r| r.session_id).ok_or_else(|| {
        error!("Cannot resume: no session_id found for {task_path}");
        AppError::RunError("No session to resume — try retry instead".to_owned())
    })?;

    let mut manager = state.lock().await;

    if manager.active_run.is_some() {
        error!("Cannot resume run: a run is already active");
        return Err(AppError::RunError("A run is already active".to_owned()));
    }

    let task_file = Path::new(task_path);
    if !task_file.exists() {
        error!("Task file not found: {task_path}");
        return Err(AppError::TaskNotFound(task_path.to_owned()));
    }

    // Reset task status and increment run count
    task::update_task_status(task_path, TaskStatus::Running);
    task::increment_run_count(task_path);

    manager.ensure_sidecar(app_handle.clone(), Arc::clone(&state))?;

    let tab_id = uuid::Uuid::new_v4().to_string();
    let request = SidecarRequest::ResumeRun {
        task_path: task_path.to_owned(),
        worktree: worktree_path.to_owned(),
        session_id: session_id.clone(),
        options: LaunchOptions {
            max_turns: None,
            max_budget_usd: None,
        },
        hook_port: crate::HOOK_RECEIVER_PORT,
        tab_id: tab_id.clone(),
    };

    manager.send_request(&request).await?;

    let now = chrono::Utc::now().to_rfc3339();
    let now_ms = now_epoch_ms();
    let run_info = RunInfo {
        session_id: Some(session_id),
        task_path: task_path.to_owned(),
        status: RunStatus::Starting,
        started_at: now,
        cost_usd: 0.0,
        last_heartbeat: None,
        elapsed_secs: 0,
        tab_id: Some(tab_id),
    };

    manager.started_at_epoch_ms = now_ms;
    manager.last_activity_ms = now_ms;
    manager.active_run = Some(run_info.clone());
    run_state::save_run_state(task_path, &run_info);

    info!("Resumed run for task {task_path}");

    if let Err(e) = app_handle.emit("run:status_changed", &run_info) {
        error!("Failed to emit run:status_changed event: {e}");
    }

    Ok(run_info)
}

/// Spawn a tokio task that reads stdout lines from the sidecar,
/// parses them, and calls `handle_response` / `mark_run_failed` directly.
fn start_stdout_reader<R: tauri::Runtime>(
    state: RunManagerState,
    app_handle: tauri::AppHandle<R>,
    reader: BufReader<tokio::process::ChildStdout>,
) {
    tauri::async_runtime::spawn(async move {
        let mut lines = reader.lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<SidecarResponse>(trimmed) {
                        Ok(response) => {
                            let mut manager = state.lock().await;
                            manager.handle_response(&response, &app_handle);
                        }
                        Err(e) => {
                            warn!("Failed to parse sidecar response: {e} — line: {trimmed}");
                        }
                    }
                }
                Ok(None) => {
                    // Stdout closed — sidecar process has exited
                    warn!("Sidecar stdout closed (process exited)");
                    let mut manager = state.lock().await;
                    manager.mark_run_failed(&app_handle);
                    break;
                }
                Err(e) => {
                    error!("Error reading sidecar stdout: {e}");
                    let mut manager = state.lock().await;
                    manager.mark_run_failed(&app_handle);
                    break;
                }
            }
        }
    });
}

/// Type alias for the managed state.
pub type RunManagerState = Arc<Mutex<RunManager>>;

/// Create the initial `RunManager` managed state.
#[must_use]
pub fn create_run_manager_state(sidecar_path: PathBuf) -> RunManagerState {
    Arc::new(Mutex::new(RunManager::new(sidecar_path)))
}
