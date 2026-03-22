use log::{debug, error, info};
use tauri::Emitter;

use crate::models::agent::EpochMs;
use crate::models::github::PrSummary;
use crate::models::orchestrator::{
    is_pr_eligible, pr_key, LifecycleEvent, LifecycleStatus, Orchestrator, OrchestratorEffect,
    RetryEntry, RunningEntry, SessionOutcome,
};

// --- Retry backoff constants ---

/// Continuation retry (fix incomplete, agent exited cleanly): fast retry.
const CONTINUATION_DELAY_MS: u64 = 1_000;
/// Base delay for failure retries (exponential backoff).
const FAILURE_BASE_MS: u64 = 10_000;
/// Maximum retry delay cap.
const FAILURE_MAX_MS: u64 = 300_000;
/// Maximum retry attempts before giving up.
const MAX_RETRIES: u32 = 5;

// --- Pure state machine functions ---
// All take &mut Orchestrator + inputs, return Vec<OrchestratorEffect>.
// No I/O, no Tauri, no async. Fully unit testable.

/// Handle incoming PR status changes. Filter eligible PRs, enforce
/// concurrency gate, dispatch sessions for available slots.
pub fn apply_pr_event(
    state: &mut Orchestrator,
    repo: &str,
    prs: &[PrSummary],
    now: EpochMs,
) -> Vec<OrchestratorEffect> {
    if !state.config.enabled && !state.config.auto_analyze {
        return Vec::new();
    }

    let mut effects = Vec::new();
    let available_slots = state
        .config
        .max_concurrent
        .saturating_sub(u32::try_from(state.running.len()).unwrap_or(u32::MAX));

    if available_slots == 0 {
        debug!("Orchestrator: no available slots, skipping PR event");
        return effects;
    }

    let mut dispatched = 0u32;

    for pr in prs {
        if dispatched >= available_slots {
            break;
        }

        let key = pr_key(&pr.repo_name, pr.number);

        // Skip if already claimed, running, completed, or queued for retry
        if state.claimed.contains(&key)
            || state.running.contains_key(&key)
            || state.completed.contains(&key)
            || state.retry_queue.contains_key(&key)
        {
            continue;
        }

        if !is_pr_eligible(pr, &state.config) {
            continue;
        }

        // Claim and dispatch
        state.claimed.insert(key.clone());

        // Build absolute worktree path using repo_paths mapping
        let repo_base = state
            .repo_paths
            .get(&pr.repo_name)
            .cloned()
            .unwrap_or_else(|| {
                // Fallback: try using repo from the event
                error!(
                    "No repo_path mapping for {}; worktree path will be relative",
                    pr.repo_name
                );
                String::new()
            });
        let sanitized_branch = pr.branch.replace("..", "").replace('/', "-");
        let worktree_path = if repo_base.is_empty() {
            format!(".worktrees/{repo}/{sanitized_branch}")
        } else {
            format!("{repo_base}/.worktrees/{repo}/{sanitized_branch}")
        };

        effects.push(OrchestratorEffect::DispatchSession {
            pr_key: key.clone(),
            worktree_path: worktree_path.clone(),
            pr_context: crate::models::orchestrator::PrContext {
                repo: pr.repo_name.clone(),
                number: pr.number,
                branch: pr.branch.clone(),
                base_branch: if pr.base_branch.is_empty() {
                    "main".to_string()
                } else {
                    pr.base_branch.clone()
                },
            },
            attempt: 1,
        });

        effects.push(OrchestratorEffect::EmitLifecycleEvent {
            event: LifecycleEvent {
                pr_key: key.clone(),
                worktree_path,
                status: LifecycleStatus::Running,
                attempt: 1,
                started_at: now,
            },
        });

        dispatched += 1;
    }

    effects
}

/// Handle session end. Route based on outcome:
/// - `AnalysisWritten` → emit `review_ready`
/// - `FixCompleted` → mark completed, release claim
/// - `FixIncomplete` → schedule continuation retry
/// - `NoOutput` → schedule failure retry with backoff
pub fn apply_session_end(
    state: &mut Orchestrator,
    pr_key: &str,
    outcome: SessionOutcome,
    now: EpochMs,
) -> Vec<OrchestratorEffect> {
    let Some(entry) = state.running.remove(pr_key) else {
        debug!("Orchestrator: session end for unknown PR {pr_key}");
        return Vec::new();
    };

    let mut effects = Vec::new();

    match outcome {
        SessionOutcome::AnalysisWritten => {
            state.review_ready.insert(
                pr_key.to_string(),
                crate::models::orchestrator::ReviewReadyEntry {
                    pr_key: pr_key.to_string(),
                    worktree_path: entry.worktree_path.clone(),
                    attempt: entry.attempt,
                    started_at: entry.started_at,
                    stale: false,
                },
            );

            effects.push(OrchestratorEffect::EmitLifecycleEvent {
                event: LifecycleEvent {
                    pr_key: pr_key.to_string(),
                    worktree_path: entry.worktree_path,
                    status: LifecycleStatus::ReviewReady,
                    attempt: entry.attempt,
                    started_at: entry.started_at,
                },
            });
        }
        SessionOutcome::FixCompleted => {
            state.claimed.remove(pr_key);
            state.completed.insert(pr_key.to_string());

            effects.push(OrchestratorEffect::EmitLifecycleEvent {
                event: LifecycleEvent {
                    pr_key: pr_key.to_string(),
                    worktree_path: entry.worktree_path,
                    status: LifecycleStatus::Completed,
                    attempt: entry.attempt,
                    started_at: entry.started_at,
                },
            });
        }
        SessionOutcome::FixIncomplete | SessionOutcome::NoOutput => {
            if entry.attempt >= MAX_RETRIES {
                // Max retries exceeded — mark as completed (failed)
                state.claimed.remove(pr_key);
                state.completed.insert(pr_key.to_string());
                error!("PR {pr_key} exceeded max retries ({MAX_RETRIES}), giving up");

                effects.push(OrchestratorEffect::EmitLifecycleEvent {
                    event: LifecycleEvent {
                        pr_key: pr_key.to_string(),
                        worktree_path: entry.worktree_path,
                        status: LifecycleStatus::Completed,
                        attempt: entry.attempt,
                        started_at: entry.started_at,
                    },
                });
            } else {
                let (delay_ms, error_msg) = match outcome {
                    SessionOutcome::FixIncomplete => (CONTINUATION_DELAY_MS, None),
                    _ => (
                        retry_backoff(entry.attempt),
                        Some("No output from agent session".to_string()),
                    ),
                };
                let worktree_path = entry.worktree_path.clone();

                effects.push(OrchestratorEffect::ScheduleRetry {
                    pr_key: pr_key.to_string(),
                    worktree_path: worktree_path.clone(),
                    attempt: entry.attempt + 1,
                    delay_ms,
                    error: error_msg.clone(),
                });

                state.retry_queue.insert(
                    pr_key.to_string(),
                    RetryEntry {
                        pr_key: pr_key.to_string(),
                        attempt: entry.attempt + 1,
                        due_at_ms: now + delay_ms,
                        error: error_msg,
                        worktree_path: worktree_path.clone(),
                    },
                );

                effects.push(OrchestratorEffect::EmitLifecycleEvent {
                    event: LifecycleEvent {
                        pr_key: pr_key.to_string(),
                        worktree_path: entry.worktree_path,
                        status: LifecycleStatus::Retrying,
                        attempt: entry.attempt + 1,
                        started_at: entry.started_at,
                    },
                });
            }
        }
    }

    effects
}

/// Handle user-initiated relaunch (after approving analysis).
/// Cancel any pending retry, dispatch a fix session.
pub fn apply_relaunch(
    state: &mut Orchestrator,
    pr_key: &str,
    worktree_path: &str,
    now: EpochMs,
) -> Vec<OrchestratorEffect> {
    let mut effects = Vec::new();

    // Guard: skip if already running (prevent duplicate sessions)
    if state.running.contains_key(pr_key) {
        debug!("Orchestrator: relaunch skipped, {pr_key} already running");
        return effects;
    }

    // Cancel any pending retry to prevent duplicate dispatch
    let retry_attempt = state.retry_queue.remove(pr_key).map(|r| r.attempt);
    if retry_attempt.is_some() {
        effects.push(OrchestratorEffect::CancelRetry {
            pr_key: pr_key.to_string(),
        });
    }

    // Preserve attempt from review_ready or retry (don't reset to 1)
    let prev_attempt = retry_attempt
        .or_else(|| state.review_ready.get(pr_key).map(|r| r.attempt))
        .unwrap_or(0);
    let next_attempt = prev_attempt + 1;

    // Remove from review_ready (user approved)
    state.review_ready.remove(pr_key);

    // Ensure claimed
    state.claimed.insert(pr_key.to_string());

    // Parse pr_key to reconstruct minimal PrContext
    let pr_context = parse_pr_key(pr_key);

    effects.push(OrchestratorEffect::DispatchSession {
        pr_key: pr_key.to_string(),
        worktree_path: worktree_path.to_string(),
        pr_context,
        attempt: next_attempt,
    });

    effects.push(OrchestratorEffect::EmitLifecycleEvent {
        event: LifecycleEvent {
            pr_key: pr_key.to_string(),
            worktree_path: worktree_path.to_string(),
            status: LifecycleStatus::Fixing,
            attempt: next_attempt,
            started_at: now,
        },
    });

    effects
}

/// Reconcile orchestrator state with current PR states.
/// Stop sessions for merged/closed PRs, flag stale for CI-now-passing.
pub fn apply_reconciliation(
    state: &mut Orchestrator,
    current_prs: &[PrSummary],
) -> Vec<OrchestratorEffect> {
    let mut effects = Vec::new();

    // Build set of currently open PR keys
    let open_keys: std::collections::HashSet<String> = current_prs
        .iter()
        .map(|pr| pr_key(&pr.repo_name, pr.number))
        .collect();

    // Find running sessions whose PRs are no longer open (merged/closed)
    let stale_keys: Vec<String> = state
        .running
        .keys()
        .filter(|k| !open_keys.contains(k.as_str()))
        .cloned()
        .collect();

    for key in &stale_keys {
        if let Some(entry) = state.running.remove(key) {
            effects.push(OrchestratorEffect::StopSession {
                pr_key: key.clone(),
                tab_id: entry.tab_id.clone(),
                worktree_path: entry.worktree_path.clone(),
            });
            effects.push(OrchestratorEffect::CleanupMetadata {
                worktree_path: entry.worktree_path,
            });
        }
        state.claimed.remove(key);
        state.retry_queue.remove(key);
        state.review_ready.remove(key);
    }

    // Clean up retries for closed PRs
    let stale_retries: Vec<String> = state
        .retry_queue
        .keys()
        .filter(|k| !open_keys.contains(k.as_str()))
        .cloned()
        .collect();

    for key in &stale_retries {
        state.retry_queue.remove(key);
        state.claimed.remove(key);
        effects.push(OrchestratorEffect::CancelRetry {
            pr_key: key.clone(),
        });
    }

    // Detect CI-now-passing for review_ready PRs (AC10: stale detection)
    for pr in current_prs {
        let key = pr_key(&pr.repo_name, pr.number);
        if let Some(entry) = state.review_ready.get_mut(&key) {
            let ci_passing = matches!(pr.ci_status.as_deref(), Some("SUCCESS" | "NEUTRAL") | None);
            if ci_passing && !entry.stale {
                entry.stale = true;
                effects.push(OrchestratorEffect::EmitLifecycleEvent {
                    event: LifecycleEvent {
                        pr_key: key,
                        worktree_path: entry.worktree_path.clone(),
                        status: LifecycleStatus::Stale,
                        attempt: entry.attempt,
                        started_at: entry.started_at,
                    },
                });
            }
        }
    }

    effects
}

/// Handle a retry becoming due. Re-dispatch the session.
pub fn apply_retry_due(
    state: &mut Orchestrator,
    pr_key: &str,
    worktree_path: &str,
    now: EpochMs,
) -> Vec<OrchestratorEffect> {
    let Some(retry) = state.retry_queue.remove(pr_key) else {
        return Vec::new();
    };

    // Guard: respect concurrency limit
    let running_count = u32::try_from(state.running.len()).unwrap_or(u32::MAX);
    if running_count >= state.config.max_concurrent {
        // Put it back — will be retried next time
        state.retry_queue.insert(pr_key.to_string(), retry);
        debug!("Orchestrator: retry for {pr_key} deferred, at concurrency limit");
        return Vec::new();
    }

    let mut effects = Vec::new();
    let pr_context = parse_pr_key(pr_key);

    effects.push(OrchestratorEffect::DispatchSession {
        pr_key: pr_key.to_string(),
        worktree_path: worktree_path.to_string(),
        pr_context,
        attempt: retry.attempt,
    });

    effects.push(OrchestratorEffect::EmitLifecycleEvent {
        event: LifecycleEvent {
            pr_key: pr_key.to_string(),
            worktree_path: worktree_path.to_string(),
            status: LifecycleStatus::Running,
            attempt: retry.attempt,
            started_at: now,
        },
    });

    effects
}

/// Record a running session. Called by the effect executor after dispatch.
pub fn record_running(
    state: &mut Orchestrator,
    pr_key: &str,
    worktree_path: &str,
    tab_id: &str,
    now: EpochMs,
    attempt: u32,
) {
    state.running.insert(
        pr_key.to_string(),
        RunningEntry {
            pr_key: pr_key.to_string(),
            worktree_path: worktree_path.to_string(),
            tab_id: tab_id.to_string(),
            started_at: now,
            attempt,
        },
    );
}

/// Skip a PR — remove from all tracking, no retry.
pub fn apply_skip(state: &mut Orchestrator, pr_key: &str) -> Vec<OrchestratorEffect> {
    let mut effects = Vec::new();

    if let Some(entry) = state.running.remove(pr_key) {
        effects.push(OrchestratorEffect::StopSession {
            pr_key: pr_key.to_string(),
            tab_id: entry.tab_id,
            worktree_path: entry.worktree_path,
        });
    }

    state.claimed.remove(pr_key);
    state.retry_queue.remove(pr_key);
    state.review_ready.remove(pr_key);

    effects
}

// --- Helpers ---

/// Calculate retry backoff: `min(FAILURE_BASE_MS` * 2^(attempt-1), `FAILURE_MAX_MS`)
#[must_use]
pub fn retry_backoff(attempt: u32) -> u64 {
    let delay = FAILURE_BASE_MS.saturating_mul(1u64 << attempt.saturating_sub(1));
    delay.min(FAILURE_MAX_MS)
}

/// Parse a `pr_key` ("owner/repo#42") into a minimal `PrContext`.
/// Logs an error and returns a best-effort result for malformed keys.
fn parse_pr_key(key: &str) -> crate::models::orchestrator::PrContext {
    let Some((repo, number_str)) = key.rsplit_once('#') else {
        error!("Malformed pr_key (no # separator): {key}");
        return crate::models::orchestrator::PrContext {
            repo: key.to_string(),
            number: 0,
            branch: String::new(),
            base_branch: String::new(),
        };
    };
    let number = number_str.parse().unwrap_or_else(|_| {
        error!("Malformed PR number in pr_key: {key}");
        0
    });
    crate::models::orchestrator::PrContext {
        repo: repo.to_string(),
        number,
        branch: String::new(),
        base_branch: String::new(),
    }
}

/// Look up running entry by `tab_id` (for session end matching).
#[must_use]
pub fn find_pr_key_by_tab_id(state: &Orchestrator, tab_id: &str) -> Option<String> {
    state
        .running
        .values()
        .find(|e| e.tab_id == tab_id)
        .map(|e| e.pr_key.clone())
}

// --- Impure functions (file I/O, Tauri interaction) ---

/// Determine session outcome by reading analysis.json from the worktree.
#[derive(serde::Deserialize)]
struct AnalysisCheck {
    #[serde(default)]
    approved: bool,
    #[serde(default)]
    resolved: bool,
}

#[must_use]
pub fn determine_session_outcome(worktree_path: &str) -> SessionOutcome {
    let analysis_path = format!("{worktree_path}/.branchdeck/analysis.json");
    let Ok(content) = std::fs::read_to_string(&analysis_path) else {
        return SessionOutcome::NoOutput;
    };

    match serde_json::from_str::<AnalysisCheck>(&content) {
        Ok(a) if a.resolved => SessionOutcome::FixCompleted,
        Ok(a) if a.approved => SessionOutcome::FixIncomplete,
        Ok(_) => SessionOutcome::AnalysisWritten,
        Err(e) => {
            error!("Failed to parse analysis.json at {analysis_path}: {e}");
            SessionOutcome::NoOutput
        }
    }
}

/// Default PR shepherd skill content.
const DEFAULT_SKILL: &str = include_str!("../../../.claude/skills/pr-shepherd/SKILL.md");

/// Deploy the PR shepherd skill file into a worktree if not already present.
fn deploy_skill_file(worktree_path: &str) {
    let skill_dir = format!("{worktree_path}/.claude/skills/pr-shepherd");
    let skill_path = format!("{skill_dir}/SKILL.md");

    if std::path::Path::new(&skill_path).exists() {
        debug!("Skill file already exists at {skill_path}");
        return;
    }

    if let Err(e) = std::fs::create_dir_all(&skill_dir) {
        error!("Failed to create skill directory {skill_dir}: {e}");
        return;
    }

    if let Err(e) = std::fs::write(&skill_path, DEFAULT_SKILL) {
        error!("Failed to write skill file {skill_path}: {e}");
        return;
    }

    info!("Deployed pr-shepherd skill to {skill_path}");
}

/// Write pr-context.json into the worktree's .branchdeck directory.
fn write_pr_context(worktree_path: &str, pr_context: &crate::models::orchestrator::PrContext) {
    let dir = format!("{worktree_path}/.branchdeck");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        error!("Failed to create .branchdeck dir: {e}");
        return;
    }

    let path = format!("{dir}/pr-context.json");
    match serde_json::to_string_pretty(pr_context) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                error!("Failed to write pr-context.json: {e}");
            }
        }
        Err(e) => error!("Failed to serialize pr-context: {e}"),
    }
}

/// Create a minimal task.md for the orchestrator session.
fn create_orchestrator_task(
    worktree_path: &str,
    _pr_key: &str,
    pr_context: &crate::models::orchestrator::PrContext,
) -> String {
    let dir = format!("{worktree_path}/.branchdeck");
    let _ = std::fs::create_dir_all(&dir);
    let task_path = format!("{dir}/task.md");
    let now = chrono::Utc::now().to_rfc3339();

    let content = format!(
        "---\ntype: pr-shepherd\nscope: worktree\nstatus: created\n\
         repo: {repo}\nbranch: {branch}\npr: {number}\n\
         created: {now}\nrun-count: 0\n---\n\n\
         You are shepherding GitHub PR #{number} in {repo} (branch: {branch}).\n\n\
         Follow the pr-shepherd skill at .claude/skills/pr-shepherd/SKILL.md exactly.\n\
         Read .branchdeck/pr-context.json for PR details (repo, number, branch, base_branch).\n\n\
         CRITICAL: You MUST write .branchdeck/analysis.json with your findings.\n\
         Check if .branchdeck/analysis.json already exists to determine your phase:\n\
         - No file → Analyze the PR (check CI, reviews, codebase) and write analysis.json\n\
         - File with approved: true → Execute the approved fix plan\n\
         - File with approved: false → Do nothing, end session\n",
        repo = pr_context.repo,
        branch = pr_context.branch,
        number = pr_context.number,
    );

    if let Err(e) = std::fs::write(&task_path, &content) {
        error!("Failed to write orchestrator task: {e}");
    }

    task_path
}

/// Execute the `DispatchSession` effect: create worktree, deploy files, enqueue run.
async fn execute_dispatch<R: tauri::Runtime>(
    key: &str,
    worktree_path: &str,
    pr_context: &crate::models::orchestrator::PrContext,
    attempt: u32,
    orchestrator: &OrchestratorState,
    run_manager: &crate::services::run_manager::RunManagerState,
    app_handle: &tauri::AppHandle<R>,
) {
    use crate::models::agent::now_ms;

    // Create worktree if it doesn't exist
    if !std::path::Path::new(worktree_path).exists() {
        if let Some(repo_base) = worktree_path
            .find("/.worktrees/")
            .map(|idx| &worktree_path[..idx])
        {
            let base = if pr_context.base_branch.is_empty() {
                None
            } else {
                Some(pr_context.base_branch.as_str())
            };
            match crate::services::git::create_worktree(
                std::path::Path::new(repo_base),
                &pr_context.branch,
                Some(&pr_context.branch),
                base,
            ) {
                Ok(wt) => info!("Created worktree at {}", wt.path.display()),
                Err(e) => {
                    error!("Failed to create worktree for {key}: {e}");
                    orchestrator.lock().await.claimed.remove(key);
                    return;
                }
            }
        } else {
            error!("Cannot derive repo path from worktree_path: {worktree_path}");
            orchestrator.lock().await.claimed.remove(key);
            return;
        }
    }

    write_pr_context(worktree_path, pr_context);
    deploy_skill_file(worktree_path);
    let task_path = create_orchestrator_task(worktree_path, key, pr_context);

    let launch_options = crate::models::run::LaunchOptions {
        max_turns: None,
        max_budget_usd: None,
        permission_mode: Some("bypassPermissions".to_string()),
    };
    match crate::services::run_manager::enqueue_run(
        std::sync::Arc::clone(run_manager),
        app_handle.clone(),
        &task_path,
        worktree_path,
        launch_options,
    )
    .await
    {
        Ok(_status) => {
            let tab_id = {
                let rm = run_manager.lock().await;
                rm.get_status()
                    .and_then(|r| r.tab_id)
                    .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
            };
            let mut orch = orchestrator.lock().await;
            record_running(&mut orch, key, worktree_path, &tab_id, now_ms(), attempt);
            info!("Dispatched orchestrator session for {key} (tab={tab_id})");
        }
        Err(e) => {
            error!("Failed to dispatch session for {key}: {e}");
            orchestrator.lock().await.claimed.remove(key);
        }
    }
}

/// Execute orchestrator effects. Thin executor — dispatches to services.
#[allow(clippy::too_many_lines)]
pub async fn execute_effects<R: tauri::Runtime>(
    effects: Vec<OrchestratorEffect>,
    orchestrator: &OrchestratorState,
    run_manager: &crate::services::run_manager::RunManagerState,
    app_handle: &tauri::AppHandle<R>,
    event_bus: Option<&std::sync::Arc<crate::services::event_bus::EventBus>>,
) {
    for effect in effects {
        match effect {
            OrchestratorEffect::DispatchSession {
                pr_key: key,
                worktree_path,
                pr_context,
                attempt,
            } => {
                execute_dispatch(
                    &key,
                    &worktree_path,
                    &pr_context,
                    attempt,
                    orchestrator,
                    run_manager,
                    app_handle,
                )
                .await;
            }
            OrchestratorEffect::StopSession {
                pr_key: key,
                tab_id,
                worktree_path: wt_path,
            } => {
                info!("Stopping session for {key} (tab={tab_id})");
                let mut rm = run_manager.lock().await;
                // Cancel if the active run matches this tab_id
                if let Some(active) = rm.get_status() {
                    if active.tab_id.as_deref() == Some(&tab_id) {
                        if let Err(e) = rm.cancel_run().await {
                            error!("Failed to cancel run for {key}: {e}");
                        }
                    }
                }
                // Also remove from queue if not yet active
                rm.remove_queued_by_worktree(&wt_path);
            }
            OrchestratorEffect::ScheduleRetry {
                pr_key: key,
                worktree_path: wt_path,
                delay_ms,
                ..
            } => {
                debug!("Scheduling retry for {key} in {delay_ms}ms");
                if let Some(bus) = event_bus.cloned() {
                    let timer_key = key.clone();
                    let handle = tokio::spawn(async move {
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        let _ = bus.publish(crate::models::agent::Event::RetryDue {
                            pr_key: key.clone(),
                            worktree_path: wt_path,
                        });
                        debug!("Retry timer fired for {key}");
                    });
                    // Store handle so CancelRetry can abort it
                    let mut orch = orchestrator.lock().await;
                    orch.retry_timers.insert(timer_key, handle);
                } else {
                    error!("No EventBus available for retry timer for {key}");
                }
            }
            OrchestratorEffect::CancelRetry { pr_key: key } => {
                let mut orch = orchestrator.lock().await;
                if let Some(handle) = orch.retry_timers.remove(&key) {
                    handle.abort();
                    debug!("Aborted retry timer for {key}");
                } else {
                    debug!("Cancelled retry for {key} (no active timer)");
                }
            }
            OrchestratorEffect::EmitLifecycleEvent { event } => {
                if let Err(e) = app_handle.emit("lifecycle:updated", &event) {
                    error!("Failed to emit lifecycle:updated: {e}");
                }
            }
            OrchestratorEffect::CleanupMetadata { worktree_path } => {
                let analysis = format!("{worktree_path}/.branchdeck/analysis.json");
                let context = format!("{worktree_path}/.branchdeck/pr-context.json");
                let _ = std::fs::remove_file(&analysis);
                let _ = std::fs::remove_file(&context);
                debug!("Cleaned up metadata at {worktree_path}");
            }
        }
    }
}

/// Type alias for orchestrator managed state.
pub type OrchestratorState = std::sync::Arc<tokio::sync::Mutex<Orchestrator>>;

/// Create orchestrator managed state.
#[must_use]
pub fn create_orchestrator_state(
    config: crate::models::orchestrator::OrchestratorConfig,
) -> OrchestratorState {
    std::sync::Arc::new(tokio::sync::Mutex::new(Orchestrator::new(config)))
}

/// Start the orchestrator event loop as a background tokio task.
/// Subscribes to `EventBus` and routes events to pure state machine functions.
pub fn start_orchestrator<R: tauri::Runtime + 'static>(
    orchestrator: OrchestratorState,
    event_bus: std::sync::Arc<crate::services::event_bus::EventBus>,
    run_manager: crate::services::run_manager::RunManagerState,
    app_handle: tauri::AppHandle<R>,
) {
    tauri::async_runtime::spawn(async move {
        let mut rx = event_bus.subscribe();
        info!("Orchestrator event loop started");

        loop {
            match rx.recv().await {
                Ok(event) => {
                    handle_event(&event, &orchestrator, &run_manager, &app_handle, &event_bus)
                        .await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    error!("Orchestrator event loop lagged by {n} events");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!("Orchestrator event loop: EventBus closed, shutting down");
                    break;
                }
            }
        }
    });
}

/// Route a single event to the appropriate pure function, then execute effects.
async fn handle_event<R: tauri::Runtime>(
    event: &crate::models::agent::Event,
    orchestrator: &OrchestratorState,
    run_manager: &crate::services::run_manager::RunManagerState,
    app_handle: &tauri::AppHandle<R>,
    event_bus: &std::sync::Arc<crate::services::event_bus::EventBus>,
) {
    use crate::models::agent::{now_ms, Event};

    match event {
        Event::PrStatusChanged { repo, prs, .. } => {
            let effects = {
                let mut orch = orchestrator.lock().await;
                // apply_pr_event has its own enabled/auto_analyze guard
                let mut effects = apply_pr_event(&mut orch, repo, prs, now_ms());
                // Reconciliation always runs (cleans up merged/closed PRs)
                effects.extend(apply_reconciliation(&mut orch, prs));
                effects
            };
            execute_effects(
                effects,
                orchestrator,
                run_manager,
                app_handle,
                Some(event_bus),
            )
            .await;
        }
        Event::RunComplete { tab_id, .. } => {
            let (key, worktree) = {
                let orch = orchestrator.lock().await;
                match find_pr_key_by_tab_id(&orch, tab_id) {
                    Some(k) => {
                        let wt = orch
                            .running
                            .get(&k)
                            .map(|e| e.worktree_path.clone())
                            .unwrap_or_default();
                        (k, wt)
                    }
                    None => return, // Not an orchestrator-managed run
                }
            };

            let outcome = determine_session_outcome(&worktree);
            let effects = {
                let mut orch = orchestrator.lock().await;
                apply_session_end(&mut orch, &key, outcome, now_ms())
            };
            execute_effects(
                effects,
                orchestrator,
                run_manager,
                app_handle,
                Some(event_bus),
            )
            .await;
        }
        Event::RetryDue {
            pr_key: key,
            worktree_path,
        } => {
            let effects = {
                let mut orch = orchestrator.lock().await;
                apply_retry_due(&mut orch, key, worktree_path, now_ms())
            };
            execute_effects(
                effects,
                orchestrator,
                run_manager,
                app_handle,
                Some(event_bus),
            )
            .await;
        }
        _ => {} // Other events not handled by orchestrator
    }
}
