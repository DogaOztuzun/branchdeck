use crate::error::AppError;
use crate::models::run::{LaunchOptions, RunInfo, RunStatus, SidecarRequest, SidecarResponse};
use crate::models::task::TaskStatus;
use crate::services::{run_state, task};
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::Mutex;

/// Get the current time as epoch milliseconds.
#[allow(clippy::cast_possible_truncation)]
fn now_epoch_ms() -> u64 {
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

/// Stale threshold: if no heartbeat or activity for this many seconds, mark run failed.
const STALE_THRESHOLD_SECS: u64 = 120;

pub struct RunManager {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    active_run: Option<RunInfo>,
    sidecar_path: PathBuf,
    /// Epoch milliseconds of the last heartbeat or activity from the sidecar.
    last_activity_ms: u64,
    /// Epoch milliseconds when the current run started.
    started_at_epoch_ms: u64,
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

    /// Check if the active run is stale (no activity for `STALE_THRESHOLD_SECS`).
    /// If stale, marks the run as failed with a "stalled" reason.
    pub fn check_stale<R: tauri::Runtime>(&mut self, app_handle: &tauri::AppHandle<R>) {
        if self.active_run.is_none() {
            return;
        }

        if self.last_activity_ms == 0 {
            return;
        }

        let now = now_epoch_ms();
        let elapsed_secs = (now.saturating_sub(self.last_activity_ms)) / 1000;

        if elapsed_secs >= STALE_THRESHOLD_SECS {
            warn!(
                "Stale run detected: no activity for {elapsed_secs}s (threshold: {STALE_THRESHOLD_SECS}s)"
            );
            self.mark_run_failed_with_reason(app_handle, "stalled: no heartbeat for 120s");
        }
    }

    /// Check if a response's `session_id` matches the active run's `session_id`.
    /// Returns `true` if they match or if either is `None` (not yet assigned).
    /// Returns `false` (mismatch) only when both are `Some` and differ.
    fn session_matches(&self, response_session_id: Option<&String>) -> bool {
        if let (Some(active_sid), Some(resp_sid)) = (
            self.active_run.as_ref().and_then(|r| r.session_id.as_ref()),
            response_session_id,
        ) {
            if active_sid != resp_sid {
                return false;
            }
        }
        true
    }

    /// Update the active run from a sidecar response.
    pub fn handle_response<R: tauri::Runtime>(
        &mut self,
        response: &SidecarResponse,
        app_handle: &tauri::AppHandle<R>,
    ) {
        // Update activity timestamp on every response (heartbeat or real)
        self.update_activity();

        match response {
            SidecarResponse::Heartbeat { session_id } => {
                if !self.session_matches(session_id.as_ref()) {
                    warn!("Ignoring heartbeat with mismatched session_id: {session_id:?}");
                    return;
                }
                debug!("Heartbeat received");
            }
            SidecarResponse::SessionStarted { session_id } => {
                if let Some(ref mut run) = self.active_run {
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
            SidecarResponse::RunStep { session_id, .. }
            | SidecarResponse::AssistantText { session_id, .. }
            | SidecarResponse::ToolCall { session_id, .. } => {
                if !self.session_matches(session_id.as_ref()) {
                    warn!("Ignoring run step with mismatched session_id: {session_id:?}");
                    return;
                }
                if let Err(e) = app_handle.emit("run:step", response) {
                    error!("Failed to emit run:step: {e}");
                }
            }
            SidecarResponse::RunComplete {
                cost_usd,
                session_id,
                ..
            } => {
                if !self.session_matches(session_id.as_ref()) {
                    warn!("Ignoring run complete with mismatched session_id: {session_id:?}");
                    return;
                }
                if let Some(ref mut run) = self.active_run {
                    run.status = RunStatus::Succeeded;
                    if let Some(cost) = cost_usd {
                        run.cost_usd = *cost;
                    }
                    if self.started_at_epoch_ms > 0 {
                        run.elapsed_secs =
                            (now_epoch_ms().saturating_sub(self.started_at_epoch_ms)) / 1000;
                    }
                    info!("Run completed successfully, cost: ${:.4}", run.cost_usd);
                    task::update_task_status(&run.task_path, TaskStatus::Succeeded);
                    run_state::save_run_state(&run.task_path, run);
                    run_state::delete_run_state(&run.task_path);
                    if let Err(e) = app_handle.emit("run:status_changed", &*run) {
                        error!("Failed to emit run:status_changed: {e}");
                    }
                }
                self.active_run = None;
                self.last_activity_ms = 0;
                self.started_at_epoch_ms = 0;
            }
            SidecarResponse::RunError {
                error: err_msg,
                status,
                cost_usd,
                session_id,
            } => {
                if !self.session_matches(session_id.as_ref()) {
                    warn!("Ignoring run error with mismatched session_id: {session_id:?}");
                    return;
                }
                if let Some(ref mut run) = self.active_run {
                    let (run_status, task_status) = if status == "cancelled" {
                        (RunStatus::Cancelled, TaskStatus::Cancelled)
                    } else {
                        (RunStatus::Failed, TaskStatus::Failed)
                    };
                    run.status = run_status;
                    if let Some(cost) = cost_usd {
                        run.cost_usd = *cost;
                    }
                    if self.started_at_epoch_ms > 0 {
                        run.elapsed_secs =
                            (now_epoch_ms().saturating_sub(self.started_at_epoch_ms)) / 1000;
                    }
                    error!("Run failed: {err_msg}");
                    task::update_task_status(&run.task_path, task_status);
                    run_state::save_run_state(&run.task_path, run);
                    run_state::delete_run_state(&run.task_path);
                    if let Err(e) = app_handle.emit("run:status_changed", &*run) {
                        error!("Failed to emit run:status_changed: {e}");
                    }
                }
                self.active_run = None;
                self.last_activity_ms = 0;
                self.started_at_epoch_ms = 0;
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
            run_state::save_run_state(&run.task_path, run);
            run_state::delete_run_state(&run.task_path);
            if let Err(e) = app_handle.emit("run:status_changed", &*run) {
                error!("Failed to emit run:status_changed: {e}");
            }
        }
        self.active_run = None;
        self.process = None;
        self.stdin = None;
        self.last_activity_ms = 0;
        self.started_at_epoch_ms = 0;
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

    let request = SidecarRequest::LaunchRun {
        task_path: task_path.to_owned(),
        worktree: worktree_path.to_owned(),
        options,
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
    };

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
