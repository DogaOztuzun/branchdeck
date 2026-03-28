use crate::error::AppError;
use crate::models::run::{
    LaunchOptions, PendingPermission, RunInfo, RunStatus, SidecarRequest, SidecarResponse,
};
use crate::models::task::TaskStatus;
use crate::services::{run_effects, run_responses, run_stale, run_state, task};
use crate::traits::{self, EventEmitter};
use log::{debug, error, info, warn};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
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
    #[allow(clippy::cast_possible_wrap)]
    let secs = (epoch_ms / 1000) as i64;
    #[allow(clippy::cast_possible_truncation)]
    let nanos = ((epoch_ms % 1000) * 1_000_000) as u32;
    chrono::DateTime::from_timestamp(secs, nanos)
        .unwrap_or_default()
        .to_rfc3339()
}

/// Maximum number of consecutive launch failures before a queued run is dropped.
const MAX_QUEUE_FAILURES: u32 = 3;

/// A queued run waiting for a slot to open.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedRun {
    /// Pre-assigned `run_id` so callers can track the queued run before it launches.
    pub run_id: String,
    pub task_path: String,
    pub worktree_path: String,
    pub options: LaunchOptions,
    /// Number of consecutive launch failures. Dropped after `MAX_QUEUE_FAILURES`.
    #[serde(skip)]
    pub failure_count: u32,
}

/// Per-run state: bundles the sidecar process, `RunInfo`, and timing data.
struct ActiveRun {
    process: Option<Child>,
    stdin: Option<ChildStdin>,
    info: RunInfo,
    started_at_epoch_ms: u64,
    last_activity_ms: u64,
    pending_permissions: HashMap<String, PendingPermission>,
}

/// Status of the run queue.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueStatus {
    pub active: Vec<RunSummaryBrief>,
    pub queued: Vec<QueuedRun>,
    pub completed: u32,
    pub failed: u32,
    pub max_concurrent: u32,
}

/// Brief summary of an active run for queue status reporting.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummaryBrief {
    pub run_id: String,
    pub task_path: String,
    pub status: RunStatus,
}

pub struct RunManager {
    /// Active runs keyed by `run_id`.
    runs: HashMap<String, ActiveRun>,
    /// Pending queue for runs waiting for a slot.
    pending: VecDeque<QueuedRun>,
    /// Maximum number of concurrent runs.
    max_concurrent: u32,
    sidecar_path: PathBuf,
    event_bus: Arc<crate::services::event_bus::EventBus>,
    emitter: Arc<dyn EventEmitter>,
    hook_port: u16,
    /// Counts for queue progress tracking.
    queue_completed: u32,
    queue_failed: u32,
    /// Set when queue is cancelled to prevent race with `try_advance`.
    queue_cancelled: bool,
}

impl RunManager {
    #[must_use]
    pub fn new(
        sidecar_path: PathBuf,
        event_bus: Arc<crate::services::event_bus::EventBus>,
        emitter: Arc<dyn EventEmitter>,
        hook_port: u16,
        max_concurrent: u32,
    ) -> Self {
        Self {
            runs: HashMap::new(),
            pending: VecDeque::new(),
            max_concurrent: max_concurrent.max(1),
            sidecar_path,
            event_bus,
            emitter,
            hook_port,
            queue_completed: 0,
            queue_failed: 0,
            queue_cancelled: false,
        }
    }

    /// Check how many slots are available for new runs.
    #[must_use]
    fn available_slots(&self) -> u32 {
        #[allow(clippy::cast_possible_truncation)]
        let active = self.runs.len() as u32;
        self.max_concurrent.saturating_sub(active)
    }

    /// Spawn a sidecar process for a specific run.
    fn spawn_sidecar(
        &self,
        run_id: &str,
        _state: RunManagerState,
    ) -> Result<(Child, ChildStdin, BufReader<tokio::process::ChildStdout>), AppError> {
        info!(
            "Spawning sidecar for run {run_id} at {}",
            self.sidecar_path.display()
        );
        let start = std::time::Instant::now();

        let mut child = tokio::process::Command::new("node")
            .arg(&self.sidecar_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| {
                error!("Failed to spawn sidecar for run {run_id}: {e}");
                AppError::SidecarError(format!("Failed to spawn node process: {e}"))
            })?;

        let child_stdout = child.stdout.take().ok_or_else(|| {
            error!("Sidecar stdout not available for run {run_id}");
            AppError::SidecarError("Sidecar stdout not available".to_owned())
        })?;

        let child_stdin = child.stdin.take().ok_or_else(|| {
            error!("Sidecar stdin not available for run {run_id}");
            AppError::SidecarError("Sidecar stdin not available".to_owned())
        })?;

        let reader = BufReader::new(child_stdout);

        info!("Sidecar for run {run_id} spawned in {:?}", start.elapsed());
        Ok((child, child_stdin, reader))
    }

    /// Get all active run infos with computed elapsed times.
    #[must_use]
    pub fn get_all_runs(&self) -> Vec<RunInfo> {
        let now = now_epoch_ms();
        self.runs
            .values()
            .map(|active| {
                let mut run = active.info.clone();
                if active.started_at_epoch_ms > 0 {
                    run.elapsed_secs = (now.saturating_sub(active.started_at_epoch_ms)) / 1000;
                }
                if active.last_activity_ms > 0 {
                    run.last_heartbeat = Some(epoch_ms_to_rfc3339(active.last_activity_ms));
                }
                run
            })
            .collect()
    }

    /// Get a specific run by `run_id`. Checks active runs and pending queue.
    #[must_use]
    pub fn get_run(&self, run_id: &str) -> Option<RunInfo> {
        // Check active runs first
        if let Some(active) = self.runs.get(run_id) {
            let mut run = active.info.clone();
            let now = now_epoch_ms();
            if active.started_at_epoch_ms > 0 {
                run.elapsed_secs = (now.saturating_sub(active.started_at_epoch_ms)) / 1000;
            }
            if active.last_activity_ms > 0 {
                run.last_heartbeat = Some(epoch_ms_to_rfc3339(active.last_activity_ms));
            }
            return Some(run);
        }
        // Check pending queue
        self.pending
            .iter()
            .find(|q| q.run_id == run_id)
            .map(|q| RunInfo {
                run_id: q.run_id.clone(),
                session_id: None,
                task_path: q.task_path.clone(),
                status: RunStatus::Created,
                started_at: String::new(),
                cost_usd: 0.0,
                last_heartbeat: None,
                elapsed_secs: 0,
                tab_id: None,
                failure_reason: None,
                max_budget_usd: q.options.max_budget_usd,
                worktree_path: Some(q.worktree_path.clone()),
            })
    }

    /// Get the first active run status (backwards compat for single-run callers).
    #[must_use]
    pub fn get_status(&self) -> Option<RunInfo> {
        self.runs.keys().next().and_then(|k| self.get_run(k))
    }

    /// Check all active runs for staleness, permission timeouts, and cost budgets.
    /// Returns `true` if any runs were removed (slots freed), signalling the caller
    /// should call `try_advance`.
    pub async fn check_stale(&mut self) -> bool {
        let now = now_epoch_ms();
        let mut stale_runs = Vec::new();

        for (run_id, active) in &self.runs {
            // Skip terminal runs (they're about to be removed by the stdout reader)
            if matches!(
                active.info.status,
                RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled
            ) {
                continue;
            }
            if run_stale::check_run_stale(active.last_activity_ms, now) {
                stale_runs.push(run_id.clone());
            }
        }

        let had_stale = !stale_runs.is_empty();
        for run_id in stale_runs {
            self.mark_run_failed_by_id(&run_id, "heartbeat-stalled");
        }

        // Check per-run cost budgets
        self.check_cost_budgets();

        // Check permission timeouts for all active runs
        let run_ids: Vec<String> = self.runs.keys().cloned().collect();
        for run_id in run_ids {
            if let Some(active) = self.runs.get_mut(&run_id) {
                run_stale::check_permission_timeout(
                    &mut active.pending_permissions,
                    &mut Some(&mut active.info),
                    active.stdin.as_mut(),
                    self.emitter.as_ref(),
                )
                .await;
            }
        }

        had_stale
    }

    /// Update the active run from a sidecar response, keyed by `run_id`.
    pub fn handle_response(&mut self, run_id: &str, response: &SidecarResponse) {
        let Some(active) = self.runs.get_mut(run_id) else {
            warn!("Ignoring response for unknown run {run_id}");
            return;
        };

        active.last_activity_ms = now_epoch_ms();
        let emitter = self.emitter.as_ref();

        match response {
            SidecarResponse::Heartbeat { session_id } => {
                if !run_responses::session_matches(Some(&active.info), session_id.as_ref()) {
                    warn!("Ignoring heartbeat with mismatched session_id for run {run_id}");
                    return;
                }
                debug!("Heartbeat received for run {run_id}");
            }
            SidecarResponse::SessionStarted { session_id } => {
                run_responses::handle_session_started(
                    &mut Some(&mut active.info),
                    session_id,
                    emitter,
                    &self.event_bus,
                );
            }
            SidecarResponse::RunStep { session_id, .. }
            | SidecarResponse::AssistantText { session_id, .. }
            | SidecarResponse::ToolCall { session_id, .. } => {
                if !run_responses::session_matches(Some(&active.info), session_id.as_ref()) {
                    warn!("Ignoring run step with mismatched session_id for run {run_id}");
                    return;
                }
                run_responses::handle_run_step(response, emitter);
            }
            SidecarResponse::RunComplete {
                cost_usd,
                session_id,
                ..
            } => {
                if !run_responses::session_matches(Some(&active.info), session_id.as_ref()) {
                    warn!("Ignoring run complete with mismatched session_id for run {run_id}");
                    return;
                }
                run_responses::handle_run_complete(
                    &mut Some(&mut active.info),
                    &mut active.started_at_epoch_ms,
                    &mut active.last_activity_ms,
                    &mut active.pending_permissions,
                    cost_usd.as_ref(),
                    emitter,
                    &self.event_bus,
                );
            }
            SidecarResponse::PermissionRequest {
                tool,
                command,
                tool_use_id,
                session_id,
            } => {
                if !run_responses::session_matches(Some(&active.info), session_id.as_ref()) {
                    warn!(
                        "Ignoring permission request with mismatched session_id for run {run_id}"
                    );
                    return;
                }
                run_responses::handle_permission_request(
                    &mut Some(&mut active.info),
                    &mut active.pending_permissions,
                    tool.as_ref(),
                    command.as_ref(),
                    tool_use_id,
                    emitter,
                    &self.event_bus,
                );
            }
            SidecarResponse::RunError {
                error: err_msg,
                status,
                cost_usd,
                session_id,
            } => {
                if !run_responses::session_matches(Some(&active.info), session_id.as_ref()) {
                    warn!("Ignoring run error with mismatched session_id for run {run_id}");
                    return;
                }
                run_responses::handle_run_error(
                    &mut Some(&mut active.info),
                    &mut active.started_at_epoch_ms,
                    &mut active.last_activity_ms,
                    &mut active.pending_permissions,
                    err_msg,
                    status,
                    cost_usd.as_ref(),
                    emitter,
                    &self.event_bus,
                );
            }
        }
    }

    /// Check if a run reached terminal state after handling a response.
    #[must_use]
    pub fn is_run_terminal(&self, run_id: &str) -> bool {
        self.runs.get(run_id).is_none_or(|a| {
            matches!(
                a.info.status,
                RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled
            )
        })
    }

    /// Remove a terminal run from the active map. Returns the final `RunInfo`.
    pub fn remove_terminal_run(&mut self, run_id: &str) -> Option<RunInfo> {
        if self.is_run_terminal(run_id) {
            self.runs.remove(run_id).map(|a| a.info)
        } else {
            None
        }
    }

    /// Mark a specific run as failed by `run_id`.
    pub fn mark_run_failed_by_id(&mut self, run_id: &str, reason: &str) {
        if let Some(active) = self.runs.get_mut(run_id) {
            let now = now_epoch_ms();
            warn!("Marking run {run_id} as failed: {reason}");
            let effects = run_effects::apply_mark_failed(
                &mut active.info,
                reason,
                active.started_at_epoch_ms,
                now,
            );
            run_effects::execute_effects(effects, self.emitter.as_ref(), &self.event_bus);

            // Kill the sidecar process
            if let Some(ref mut child) = active.process {
                if let Err(e) = child.start_kill() {
                    error!("Failed to kill sidecar process for run {run_id}: {e}");
                }
            }
        }
        // Remove the run after marking failed
        self.runs.remove(run_id);
    }

    /// Mark a specific run as failed (sidecar crash).
    pub fn mark_run_failed(&mut self, run_id: &str) {
        self.mark_run_failed_by_id(run_id, "sidecar crash");
    }

    /// Shut down the run manager during app exit.
    pub fn shutdown(&mut self) {
        if self.runs.is_empty() {
            debug!("Shutdown: no active runs to clean up");
            return;
        }

        let run_ids: Vec<String> = self.runs.keys().cloned().collect();
        let count = run_ids.len();
        for run_id in run_ids {
            if let Some(active) = self.runs.get_mut(&run_id) {
                if let Some(ref mut child) = active.process {
                    info!("Shutdown: killing sidecar for run {run_id}");
                    if let Err(e) = child.start_kill() {
                        error!("Shutdown: failed to kill sidecar for run {run_id}: {e}");
                    }
                }
            }
            self.mark_run_failed_by_id(&run_id, "daemon shutdown");
        }
        info!("Shutdown: cleaned up {count} active runs");
    }

    /// Get the current queue status.
    #[must_use]
    pub fn get_queue_status(&self) -> QueueStatus {
        QueueStatus {
            active: self
                .runs
                .values()
                .map(|a| RunSummaryBrief {
                    run_id: a.info.run_id.clone(),
                    task_path: a.info.task_path.clone(),
                    status: a.info.status,
                })
                .collect(),
            queued: self.pending.iter().cloned().collect(),
            completed: self.queue_completed,
            failed: self.queue_failed,
            max_concurrent: self.max_concurrent,
        }
    }

    /// Cancel the queue — cancels all active runs and clears pending items.
    pub fn cancel_queue(&mut self) {
        let cleared = self.pending.len();
        self.pending.clear();
        self.queue_completed = 0;
        self.queue_failed = 0;
        self.queue_cancelled = true;

        let run_ids: Vec<String> = self.runs.keys().cloned().collect();
        let active_count = run_ids.len();
        for run_id in run_ids {
            if let Err(e) = self.cancel_run_by_id(&run_id) {
                error!("Failed to cancel run {run_id} during queue cancel: {e}");
            }
        }

        info!("Cancelled queue: cleared {cleared} pending items, cancelled {active_count} active runs");
    }

    /// Remove a queued run by worktree path. Returns true if found and removed.
    pub fn remove_queued_by_worktree(&mut self, worktree_path: &str) -> bool {
        let before = self.pending.len();
        self.pending.retain(|r| r.worktree_path != worktree_path);
        let removed = before - self.pending.len();
        if removed > 0 {
            info!("Removed {removed} queued run(s) for worktree {worktree_path}");
        }
        removed > 0
    }

    /// Record a queue run completion (for progress tracking).
    pub fn record_queue_completion(&mut self, succeeded: bool) {
        if succeeded {
            self.queue_completed += 1;
        } else {
            self.queue_failed += 1;
        }
    }

    /// Cancel a specific run by `run_id`.
    ///
    /// # Errors
    ///
    /// Returns `RunError` if no active run matches the given ID.
    pub fn cancel_run_by_id(&mut self, run_id: &str) -> Result<RunInfo, AppError> {
        let active = self.runs.get_mut(run_id).ok_or_else(|| {
            error!("Cannot cancel: no active run with id {run_id}");
            AppError::RunError(format!("No active run with id {run_id}"))
        })?;

        // Kill sidecar child process immediately
        if let Some(ref mut child) = active.process {
            info!("Killing sidecar process for run {run_id}");
            if let Err(e) = child.start_kill() {
                error!("Failed to kill sidecar process during cancel of run {run_id}: {e}");
            }
        }

        // Apply cancellation effects
        let now = now_epoch_ms();
        let effects = run_effects::apply_cancel(&mut active.info, active.started_at_epoch_ms, now);
        run_effects::execute_effects(effects, self.emitter.as_ref(), &self.event_bus);
        info!(
            "Cancelled run {run_id} for task {} after {}s",
            active.info.task_path, active.info.elapsed_secs
        );

        // Mark worktree for cleanup
        if let Some(ref wt_path) = active.info.worktree_path {
            let cleanup_effects = vec![run_effects::RunEffect::MarkWorktreeForCleanup {
                path: wt_path.clone(),
            }];
            run_effects::execute_effects(cleanup_effects, self.emitter.as_ref(), &self.event_bus);
        }

        let cancelled_run = active.info.clone();
        self.runs.remove(run_id);
        Ok(cancelled_run)
    }

    /// Respond to a pending permission request for a specific run.
    ///
    /// # Errors
    ///
    /// Returns `RunError` if no active run or no pending permission matches.
    /// Returns `SidecarError` if the response cannot be sent.
    pub async fn respond_to_permission(
        &mut self,
        run_id: &str,
        tool_use_id: &str,
        decision: &str,
        reason: Option<&str>,
    ) -> Result<(), AppError> {
        let active = self.runs.get_mut(run_id).ok_or_else(|| {
            error!("No active run with id {run_id} for permission response");
            AppError::RunError(format!("No active run with id {run_id}"))
        })?;

        let pending = active
            .pending_permissions
            .remove(tool_use_id)
            .ok_or_else(|| {
                error!("No pending permission for tool_use_id: {tool_use_id} in run {run_id}");
                AppError::RunError("No matching pending permission request".to_owned())
            })?;

        info!(
            "Responding to permission for tool {:?} in run {run_id}: {decision}",
            pending.tool
        );

        let response_msg = crate::models::run::PermissionResponseMsg::PermissionResponse {
            tool_use_id: tool_use_id.to_owned(),
            decision: decision.to_owned(),
            reason: reason.map(str::to_owned),
        };

        send_to_stdin(active.stdin.as_mut(), &response_msg).await?;

        // Only restore Running status if the run is still in a non-terminal state
        if matches!(
            active.info.status,
            RunStatus::Blocked | RunStatus::Starting | RunStatus::Running
        ) {
            active.info.status = RunStatus::Running;
            run_state::save_run_state(&active.info.task_path, &active.info);
        }

        Ok(())
    }

    /// Check per-run cost budgets and cancel runs that exceed their budget.
    fn check_cost_budgets(&mut self) {
        let over_budget: Vec<String> = self
            .runs
            .iter()
            .filter_map(|(run_id, active)| {
                // Skip terminal runs awaiting removal
                if matches!(
                    active.info.status,
                    RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled
                ) {
                    return None;
                }
                if let Some(budget) = active.info.max_budget_usd {
                    if active.info.cost_usd > budget {
                        return Some(run_id.clone());
                    }
                }
                None
            })
            .collect();

        for run_id in over_budget {
            warn!("Run {run_id} exceeded cost budget, cancelling");
            if let Err(e) = self.cancel_run_by_id(&run_id) {
                error!("Failed to cancel over-budget run {run_id}: {e}");
            }
            if let Err(e) = traits::emit(
                self.emitter.as_ref(),
                "run:cost_exceeded",
                &serde_json::json!({ "runId": run_id }),
            ) {
                error!("Failed to emit run:cost_exceeded for {run_id}: {e}");
            }
        }
    }
}

/// Send a serializable message to a sidecar's stdin.
async fn send_to_stdin<T: serde::Serialize>(
    stdin: Option<&mut ChildStdin>,
    message: &T,
) -> Result<(), AppError> {
    let stdin = stdin.ok_or_else(|| {
        error!("Sidecar stdin not available");
        AppError::SidecarError("Sidecar not running".to_owned())
    })?;

    let mut json = serde_json::to_string(message).map_err(|e| {
        error!("Failed to serialize message: {e}");
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

    debug!("Sent message to sidecar");
    Ok(())
}

/// Launch a run for the given task. If at capacity, enqueues to pending.
///
/// # Errors
///
/// Returns `TaskNotFound` if the task file does not exist.
/// Returns `SidecarError` if the sidecar cannot be spawned or written to.
#[allow(clippy::too_many_lines)]
pub async fn launch_run(
    state: RunManagerState,
    task_path: &str,
    worktree_path: &str,
    options: LaunchOptions,
) -> Result<RunInfo, AppError> {
    let mut manager = state.lock().await;
    let max_budget_usd = options.max_budget_usd;

    // Reject duplicate task_path in active runs or pending queue
    let is_active_dup = manager.runs.values().any(|a| a.info.task_path == task_path);
    let is_pending_dup = manager.pending.iter().any(|q| q.task_path == task_path);
    if is_active_dup || is_pending_dup {
        error!("Duplicate task_path rejected: {task_path}");
        return Err(AppError::RunError(format!(
            "A run for task {task_path} is already active or queued"
        )));
    }

    // Check if at capacity — if so, enqueue to pending
    if manager.available_slots() == 0 {
        let run_id = uuid::Uuid::new_v4().to_string();
        info!(
            "At capacity ({} runs), enqueuing task {task_path} as {run_id}",
            manager.max_concurrent
        );
        manager.pending.push_back(QueuedRun {
            run_id: run_id.clone(),
            task_path: task_path.to_owned(),
            worktree_path: worktree_path.to_owned(),
            options,
            failure_count: 0,
        });

        let queue_status = manager.get_queue_status();
        if let Err(e) = traits::emit(manager.emitter.as_ref(), "run:queue_status", &queue_status) {
            error!("Failed to emit queue status: {e}");
        }

        // Return a Created RunInfo with the pre-assigned run_id
        let run_info = RunInfo {
            run_id,
            session_id: None,
            task_path: task_path.to_owned(),
            status: RunStatus::Created,
            started_at: chrono::Utc::now().to_rfc3339(),
            cost_usd: 0.0,
            last_heartbeat: None,
            elapsed_secs: 0,
            tab_id: None,
            failure_reason: None,
            max_budget_usd,
            worktree_path: Some(worktree_path.to_owned()),
        };
        return Ok(run_info);
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

    let run_id = uuid::Uuid::new_v4().to_string();
    let (child, child_stdin, reader) = manager.spawn_sidecar(&run_id, Arc::clone(&state))?;

    let tab_id = uuid::Uuid::new_v4().to_string();
    let hook_port = manager.hook_port;
    let request = SidecarRequest::LaunchRun {
        task_path: task_path.to_owned(),
        worktree: worktree_path.to_owned(),
        options,
        hook_port,
        tab_id: tab_id.clone(),
    };

    let now = chrono::Utc::now().to_rfc3339();
    let now_ms = now_epoch_ms();
    let run_info = RunInfo {
        run_id: run_id.clone(),
        session_id: None,
        task_path: task_path.to_owned(),
        status: RunStatus::Starting,
        started_at: now,
        cost_usd: 0.0,
        last_heartbeat: None,
        elapsed_secs: 0,
        tab_id: Some(tab_id),
        failure_reason: None,
        max_budget_usd,
        worktree_path: Some(worktree_path.to_owned()),
    };

    let active = ActiveRun {
        process: Some(child),
        stdin: Some(child_stdin),
        info: run_info.clone(),
        started_at_epoch_ms: now_ms,
        last_activity_ms: now_ms,
        pending_permissions: HashMap::new(),
    };

    manager.runs.insert(run_id.clone(), active);

    // Send the launch request to the sidecar
    let stdin = manager.runs.get_mut(&run_id).and_then(|a| a.stdin.as_mut());
    if let Err(e) = send_to_stdin(stdin, &request).await {
        // Clean up if send failed
        manager.runs.remove(&run_id);
        return Err(e);
    }

    task::increment_run_count(task_path);
    run_state::save_run_state(task_path, &run_info);

    info!("Launched run {run_id} for task {task_path}");

    if let Err(e) = traits::emit(manager.emitter.as_ref(), "run:status_changed", &run_info) {
        error!("Failed to emit run:status_changed event: {e}");
    }

    // Start stdout reader for this specific run
    start_stdout_reader(Arc::clone(&state), run_id.clone(), reader);

    Ok(run_info)
}

/// Retry a failed or cancelled run by launching a fresh session.
///
/// # Errors
///
/// Returns `RunError` if the task is not in a retryable state.
/// Returns `TaskNotFound` if the task file does not exist.
pub async fn retry_run(
    state: RunManagerState,
    task_path: &str,
    worktree_path: &str,
) -> Result<RunInfo, AppError> {
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

    let options = LaunchOptions {
        max_turns: None,
        max_budget_usd: None,
        permission_mode: None,
        allowed_directories: Vec::new(),
    };
    launch_run(state, task_path, worktree_path, options).await
}

/// Resume a failed or cancelled run by continuing its previous SDK session.
///
/// # Errors
///
/// Returns `RunError` if the task is not resumable or no `session_id` exists.
/// Returns `TaskNotFound` if the task file does not exist.
pub async fn resume_run(
    state: RunManagerState,
    task_path: &str,
    worktree_path: &str,
) -> Result<RunInfo, AppError> {
    let mut manager = state.lock().await;

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

    // Load session_id from run.json
    let previous_run = run_state::load_run_state(worktree_path);
    let session_id = previous_run.and_then(|r| r.session_id).ok_or_else(|| {
        error!("Cannot resume: no session_id found for {task_path}");
        AppError::RunError("No session to resume — try retry instead".to_owned())
    })?;

    let task_file = Path::new(task_path);
    if !task_file.exists() {
        error!("Task file not found: {task_path}");
        return Err(AppError::TaskNotFound(task_path.to_owned()));
    }

    if manager.available_slots() == 0 {
        error!(
            "Cannot resume: at capacity ({} runs)",
            manager.max_concurrent
        );
        return Err(AppError::RunError(
            "At max concurrent capacity — cannot resume now".to_owned(),
        ));
    }

    let run_id = uuid::Uuid::new_v4().to_string();
    let (child, child_stdin, reader) = manager.spawn_sidecar(&run_id, Arc::clone(&state))?;

    let tab_id = uuid::Uuid::new_v4().to_string();
    let hook_port = manager.hook_port;
    let request = SidecarRequest::ResumeRun {
        task_path: task_path.to_owned(),
        worktree: worktree_path.to_owned(),
        session_id: session_id.clone(),
        options: LaunchOptions {
            max_turns: None,
            max_budget_usd: None,
            permission_mode: None,
            allowed_directories: Vec::new(),
        },
        hook_port,
        tab_id: tab_id.clone(),
    };

    let now = chrono::Utc::now().to_rfc3339();
    let now_ms = now_epoch_ms();
    let run_info = RunInfo {
        run_id: run_id.clone(),
        session_id: Some(session_id),
        task_path: task_path.to_owned(),
        status: RunStatus::Starting,
        started_at: now,
        cost_usd: 0.0,
        last_heartbeat: None,
        elapsed_secs: 0,
        tab_id: Some(tab_id),
        failure_reason: None,
        max_budget_usd: None,
        worktree_path: Some(worktree_path.to_owned()),
    };

    let active = ActiveRun {
        process: Some(child),
        stdin: Some(child_stdin),
        info: run_info.clone(),
        started_at_epoch_ms: now_ms,
        last_activity_ms: now_ms,
        pending_permissions: HashMap::new(),
    };

    manager.runs.insert(run_id.clone(), active);

    // Send resume request
    let stdin = manager.runs.get_mut(&run_id).and_then(|a| a.stdin.as_mut());
    if let Err(e) = send_to_stdin(stdin, &request).await {
        manager.runs.remove(&run_id);
        return Err(e);
    }

    task::update_task_status(task_path, TaskStatus::Running);
    task::increment_run_count(task_path);
    run_state::save_run_state(task_path, &run_info);

    info!("Resumed run {run_id} for task {task_path}");

    if let Err(e) = traits::emit(manager.emitter.as_ref(), "run:status_changed", &run_info) {
        error!("Failed to emit run:status_changed event: {e}");
    }

    start_stdout_reader(Arc::clone(&state), run_id.clone(), reader);

    Ok(run_info)
}

/// Spawn a tokio task that reads stdout lines from a sidecar for a specific `run_id`.
fn start_stdout_reader(
    state: RunManagerState,
    run_id: String,
    reader: BufReader<tokio::process::ChildStdout>,
) {
    tokio::spawn(async move {
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
                            let is_complete =
                                matches!(&response, SidecarResponse::RunComplete { .. });
                            let is_terminal = is_complete
                                || matches!(&response, SidecarResponse::RunError { .. });

                            let should_advance = {
                                let mut manager = state.lock().await;
                                manager.handle_response(&run_id, &response);

                                if is_terminal {
                                    // Remove terminal run and check queue
                                    manager.remove_terminal_run(&run_id);
                                    manager.record_queue_completion(is_complete);

                                    let queue_status = manager.get_queue_status();
                                    if let Err(e) = traits::emit(
                                        manager.emitter.as_ref(),
                                        "run:queue_status",
                                        &queue_status,
                                    ) {
                                        error!("Failed to emit queue status: {e}");
                                    }

                                    !manager.pending.is_empty()
                                        && manager.available_slots() > 0
                                        && !manager.queue_cancelled
                                } else {
                                    false
                                }
                            };

                            if should_advance {
                                if let Err(e) = try_advance(Arc::clone(&state)).await {
                                    error!("Queue advance failed: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse sidecar response for run {run_id}: {e} — line: {trimmed}");
                        }
                    }
                }
                Ok(None) => {
                    // Stdout closed — sidecar process has exited
                    warn!("Sidecar stdout closed for run {run_id}");
                    let should_advance = {
                        let mut manager = state.lock().await;
                        manager.mark_run_failed(&run_id);
                        manager.record_queue_completion(false);

                        let queue_status = manager.get_queue_status();
                        if let Err(e) = traits::emit(
                            manager.emitter.as_ref(),
                            "run:queue_status",
                            &queue_status,
                        ) {
                            error!("Failed to emit queue status: {e}");
                        }

                        !manager.pending.is_empty()
                            && manager.available_slots() > 0
                            && !manager.queue_cancelled
                    };
                    if should_advance {
                        if let Err(e) = try_advance(Arc::clone(&state)).await {
                            error!("Queue advance after sidecar death failed: {e}");
                        }
                    }
                    break;
                }
                Err(e) => {
                    error!("Error reading sidecar stdout for run {run_id}: {e}");
                    let should_advance = {
                        let mut manager = state.lock().await;
                        manager.mark_run_failed(&run_id);
                        manager.record_queue_completion(false);

                        let queue_status = manager.get_queue_status();
                        if let Err(e) = traits::emit(
                            manager.emitter.as_ref(),
                            "run:queue_status",
                            &queue_status,
                        ) {
                            error!("Failed to emit queue status: {e}");
                        }

                        !manager.pending.is_empty()
                            && manager.available_slots() > 0
                            && !manager.queue_cancelled
                    };
                    if should_advance {
                        if let Err(e) = try_advance(Arc::clone(&state)).await {
                            error!("Queue advance after read error failed: {e}");
                        }
                    }
                    break;
                }
            }
        }
    });
}

/// Batch-launch multiple runs. Fills available slots, enqueues the rest.
///
/// # Errors
///
/// Returns `AppError` if the first run cannot be launched.
pub async fn batch_launch(
    state: RunManagerState,
    pairs: Vec<(String, String)>,
) -> Result<QueueStatus, AppError> {
    {
        let mut manager = state.lock().await;
        let cleared = manager.pending.len();
        manager.pending.clear();
        if cleared > 0 {
            info!("batch_launch: cleared {cleared} previously pending items");
            if let Err(e) = traits::emit(
                manager.emitter.as_ref(),
                "run:queue_cleared",
                &serde_json::json!({ "clearedCount": cleared }),
            ) {
                error!("Failed to emit run:queue_cleared: {e}");
            }
        }
        if manager.runs.is_empty() {
            manager.queue_completed = 0;
            manager.queue_failed = 0;
        }
        manager.queue_cancelled = false;

        // Enqueue all pairs as pending
        for (task_path, worktree_path) in &pairs {
            manager.pending.push_back(QueuedRun {
                run_id: uuid::Uuid::new_v4().to_string(),
                task_path: task_path.clone(),
                worktree_path: worktree_path.clone(),
                options: LaunchOptions {
                    max_turns: None,
                    max_budget_usd: None,
                    permission_mode: None,
                    allowed_directories: Vec::new(),
                },
                failure_count: 0,
            });
        }
    }

    // Fill available slots
    try_advance(Arc::clone(&state)).await?;

    let manager = state.lock().await;
    Ok(manager.get_queue_status())
}

/// Try to advance the queue by launching pending runs into available slots.
///
/// # Errors
///
/// Returns `AppError` if a pending run cannot be launched.
pub async fn try_advance(state: RunManagerState) -> Result<(), AppError> {
    loop {
        let next = {
            let mut manager = state.lock().await;
            if manager.queue_cancelled {
                info!("Queue was cancelled — not advancing");
                return Ok(());
            }
            if manager.available_slots() == 0 {
                return Ok(());
            }
            manager.pending.pop_front()
        };

        let Some(next) = next else {
            return Ok(());
        };

        info!("Queue advancing: launching run for {}", next.task_path);
        if let Err(e) = launch_run(
            Arc::clone(&state),
            &next.task_path,
            &next.worktree_path,
            next.options.clone(),
        )
        .await
        {
            let mut failed = next;
            failed.failure_count += 1;
            if failed.failure_count >= MAX_QUEUE_FAILURES {
                error!(
                    "Queue advance failed for {} ({} times) — dropping from queue: {e}",
                    failed.task_path, failed.failure_count
                );
                let mut manager = state.lock().await;
                manager.queue_failed += 1;
            } else {
                error!(
                    "Queue advance failed for {} (attempt {}/{}) — re-enqueuing: {e}",
                    failed.task_path, failed.failure_count, MAX_QUEUE_FAILURES
                );
                let mut manager = state.lock().await;
                manager.pending.push_front(failed);
            }
            return Err(e);
        }
    }
}

/// Enqueue a single run without clearing the existing queue.
///
/// # Errors
///
/// Returns `AppError` if the run cannot be launched.
pub async fn enqueue_run(
    state: RunManagerState,
    task_path: &str,
    worktree_path: &str,
    options: LaunchOptions,
) -> Result<QueueStatus, AppError> {
    {
        let mut manager = state.lock().await;
        // Reset cancelled flag so the new item can be drained
        manager.queue_cancelled = false;
        manager.pending.push_back(QueuedRun {
            run_id: uuid::Uuid::new_v4().to_string(),
            task_path: task_path.to_string(),
            worktree_path: worktree_path.to_string(),
            options,
            failure_count: 0,
        });
    }

    // Try to launch immediately if slots available
    try_advance(Arc::clone(&state)).await?;

    let manager = state.lock().await;
    Ok(manager.get_queue_status())
}

/// Cancel a run by ID. Searches active runs by `run_id`, `session_id`, or `task_path`.
///
/// # Errors
///
/// Returns `RunError` if no active run matches the given ID.
pub async fn force_cancel_run(state: RunManagerState, run_id: &str) -> Result<RunInfo, AppError> {
    let mut manager = state.lock().await;

    // Direct match by run_id
    if manager.runs.contains_key(run_id) {
        let result = manager.cancel_run_by_id(run_id);
        if result.is_ok() {
            // Try to advance queue after cancellation
            let should_advance = !manager.pending.is_empty() && manager.available_slots() > 0;
            drop(manager);
            if should_advance {
                if let Err(e) = try_advance(Arc::clone(&state)).await {
                    error!("Queue advance after cancel failed: {e}");
                }
            }
        }
        return result;
    }

    // Fallback: match by session_id or task_path
    let matching_id = manager
        .runs
        .iter()
        .find(|(_, a)| a.info.session_id.as_deref() == Some(run_id) || a.info.task_path == run_id)
        .map(|(k, _)| k.clone());

    if let Some(id) = matching_id {
        let result = manager.cancel_run_by_id(&id);
        if result.is_ok() {
            let should_advance = !manager.pending.is_empty() && manager.available_slots() > 0;
            drop(manager);
            if should_advance {
                if let Err(e) = try_advance(Arc::clone(&state)).await {
                    error!("Queue advance after cancel failed: {e}");
                }
            }
        }
        return result;
    }

    error!("No active run matching id {run_id}");
    Err(AppError::RunError(format!(
        "No active run with id {run_id}"
    )))
}

/// Type alias for the managed state.
pub type RunManagerState = Arc<Mutex<RunManager>>;

/// Create the initial `RunManager` managed state.
#[must_use]
pub fn create_run_manager_state(
    sidecar_path: PathBuf,
    event_bus: Arc<crate::services::event_bus::EventBus>,
    emitter: Arc<dyn EventEmitter>,
    hook_port: u16,
    max_concurrent: u32,
) -> RunManagerState {
    Arc::new(Mutex::new(RunManager::new(
        sidecar_path,
        event_bus,
        emitter,
        hook_port,
        max_concurrent,
    )))
}

/// Advance the queue after a run completes. Backwards-compatible wrapper.
///
/// # Errors
///
/// Returns `AppError` if the next run cannot be launched.
pub async fn advance_queue(state: RunManagerState, succeeded: bool) -> Result<(), AppError> {
    {
        let mut manager = state.lock().await;
        manager.record_queue_completion(succeeded);

        let queue_status = manager.get_queue_status();
        if let Err(e) = traits::emit(manager.emitter.as_ref(), "run:queue_status", &queue_status) {
            error!("Failed to emit queue status: {e}");
        }
    }

    try_advance(state).await
}
