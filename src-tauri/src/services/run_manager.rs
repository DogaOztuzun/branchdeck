use crate::error::AppError;
use crate::models::run::{LaunchOptions, RunInfo, RunStatus, SidecarRequest, SidecarResponse};
use crate::models::task::TaskStatus;
use log::{debug, error, info, warn};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::Mutex;

pub struct RunManager {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    active_run: Option<RunInfo>,
    sidecar_path: PathBuf,
}

impl RunManager {
    #[must_use]
    pub fn new(sidecar_path: PathBuf) -> Self {
        Self {
            process: None,
            stdin: None,
            active_run: None,
            sidecar_path,
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
        if self.process.is_some() {
            debug!("Sidecar already running");
            return Ok(());
        }

        info!("Spawning sidecar at {}", self.sidecar_path.display());

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

        info!("Sidecar spawned successfully");
        Ok(())
    }

    /// Get the current run status.
    #[must_use]
    pub fn get_status(&self) -> Option<RunInfo> {
        debug!("Getting run status");
        self.active_run.clone()
    }

    /// Update the active run from a sidecar response.
    pub fn handle_response<R: tauri::Runtime>(
        &mut self,
        response: &SidecarResponse,
        app_handle: &tauri::AppHandle<R>,
    ) {
        match response {
            SidecarResponse::SessionStarted { session_id } => {
                if let Some(ref mut run) = self.active_run {
                    run.session_id = Some(session_id.clone());
                    run.status = RunStatus::Running;
                    info!("Run session started: {session_id}");
                    update_task_status(&run.task_path, TaskStatus::Running);
                    if let Err(e) = app_handle.emit("run:status_changed", &*run) {
                        error!("Failed to emit run:status_changed: {e}");
                    }
                }
            }
            SidecarResponse::RunStep { .. }
            | SidecarResponse::AssistantText { .. }
            | SidecarResponse::ToolCall { .. } => {
                if let Err(e) = app_handle.emit("run:step", response) {
                    error!("Failed to emit run:step: {e}");
                }
            }
            SidecarResponse::RunComplete { cost_usd, .. } => {
                if let Some(ref mut run) = self.active_run {
                    run.status = RunStatus::Succeeded;
                    if let Some(cost) = cost_usd {
                        run.cost_usd = *cost;
                    }
                    info!("Run completed successfully, cost: ${:.4}", run.cost_usd);
                    update_task_status(&run.task_path, TaskStatus::Succeeded);
                    if let Err(e) = app_handle.emit("run:status_changed", &*run) {
                        error!("Failed to emit run:status_changed: {e}");
                    }
                }
                self.active_run = None;
            }
            SidecarResponse::RunError {
                error: err_msg,
                status,
                cost_usd,
                ..
            } => {
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
                    error!("Run failed: {err_msg}");
                    update_task_status(&run.task_path, task_status);
                    if let Err(e) = app_handle.emit("run:status_changed", &*run) {
                        error!("Failed to emit run:status_changed: {e}");
                    }
                }
                self.active_run = None;
            }
        }
    }

    /// Mark the active run as failed (used when sidecar crashes).
    pub fn mark_run_failed<R: tauri::Runtime>(&mut self, app_handle: &tauri::AppHandle<R>) {
        if let Some(ref mut run) = self.active_run {
            run.status = RunStatus::Failed;
            warn!("Marking active run as failed due to sidecar crash");
            update_task_status(&run.task_path, TaskStatus::Failed);
            if let Err(e) = app_handle.emit("run:status_changed", &*run) {
                error!("Failed to emit run:status_changed: {e}");
            }
        }
        self.active_run = None;
        self.process = None;
        self.stdin = None;
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
    let run_info = RunInfo {
        session_id: None,
        task_path: task_path.to_owned(),
        status: RunStatus::Starting,
        started_at: now,
        cost_usd: 0.0,
    };

    manager.active_run = Some(run_info.clone());

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

/// Update the status field in a task.md file's YAML frontmatter.
///
/// Uses simple string replacement within the frontmatter section.
/// Logs errors but does not propagate them — task status on disk is
/// best-effort and must not break the run state machine.
fn update_task_status(task_path: &str, new_status: TaskStatus) {
    let status_str = match new_status {
        TaskStatus::Created => "created",
        TaskStatus::Running => "running",
        TaskStatus::Blocked => "blocked",
        TaskStatus::Succeeded => "succeeded",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    };

    let content = match std::fs::read_to_string(task_path) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to read task file for status update {task_path}: {e}");
            return;
        }
    };

    // Find the frontmatter section (between first and second ---)
    // and replace the status field within it.
    let Some(updated) = replace_frontmatter_status(&content, status_str) else {
        error!("Failed to locate status field in frontmatter of {task_path}");
        return;
    };

    if let Err(e) = std::fs::write(task_path, updated) {
        error!("Failed to write updated task status to {task_path}: {e}");
    } else {
        debug!("Updated task status to {status_str} in {task_path}");
    }
}

/// Replace the `status: <value>` line in YAML frontmatter.
/// Returns `None` if the frontmatter or status field cannot be found.
fn replace_frontmatter_status(content: &str, new_status: &str) -> Option<String> {
    // Frontmatter is delimited by `---\n` at start and `\n---\n` later
    let rest = content.strip_prefix("---\n")?;
    let end_idx = rest.find("\n---\n").or_else(|| rest.find("\n---"))?;
    let frontmatter = &rest[..end_idx];

    // Find and replace the status line
    let mut found = false;
    let new_fm: String = frontmatter
        .lines()
        .map(|line| {
            if line.starts_with("status:") {
                found = true;
                format!("status: {new_status}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !found {
        return None;
    }

    Some(format!("---\n{new_fm}{}", &rest[end_idx..]))
}

/// Type alias for the managed state.
pub type RunManagerState = Arc<Mutex<RunManager>>;

/// Create the initial `RunManager` managed state.
#[must_use]
pub fn create_run_manager_state(sidecar_path: PathBuf) -> RunManagerState {
    Arc::new(Mutex::new(RunManager::new(sidecar_path)))
}
