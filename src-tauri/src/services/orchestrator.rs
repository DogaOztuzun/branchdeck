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

        let worktree_path = format!(".worktrees/{repo}/{}", pr.branch);

        effects.push(OrchestratorEffect::DispatchSession {
            pr_key: key.clone(),
            worktree_path: worktree_path.clone(),
            pr_context: crate::models::orchestrator::PrContext {
                repo: pr.repo_name.clone(),
                number: pr.number,
                branch: pr.branch.clone(),
                base_branch: "main".to_string(),
            },
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
        SessionOutcome::FixIncomplete => {
            effects.push(OrchestratorEffect::ScheduleRetry {
                pr_key: pr_key.to_string(),
                attempt: entry.attempt + 1,
                delay_ms: CONTINUATION_DELAY_MS,
                error: None,
            });

            state.retry_queue.insert(
                pr_key.to_string(),
                RetryEntry {
                    pr_key: pr_key.to_string(),
                    attempt: entry.attempt + 1,
                    due_at_ms: now + CONTINUATION_DELAY_MS,
                    error: None,
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
        SessionOutcome::NoOutput => {
            let delay = retry_backoff(entry.attempt);

            effects.push(OrchestratorEffect::ScheduleRetry {
                pr_key: pr_key.to_string(),
                attempt: entry.attempt + 1,
                delay_ms: delay,
                error: Some("No output from agent session".to_string()),
            });

            state.retry_queue.insert(
                pr_key.to_string(),
                RetryEntry {
                    pr_key: pr_key.to_string(),
                    attempt: entry.attempt + 1,
                    due_at_ms: now + delay,
                    error: Some("No output from agent session".to_string()),
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

    // Cancel any pending retry to prevent duplicate dispatch
    if state.retry_queue.remove(pr_key).is_some() {
        effects.push(OrchestratorEffect::CancelRetry {
            pr_key: pr_key.to_string(),
        });
    }

    // Ensure claimed
    state.claimed.insert(pr_key.to_string());

    // Parse pr_key to reconstruct minimal PrContext
    let pr_context = parse_pr_key(pr_key);

    effects.push(OrchestratorEffect::DispatchSession {
        pr_key: pr_key.to_string(),
        worktree_path: worktree_path.to_string(),
        pr_context,
    });

    effects.push(OrchestratorEffect::EmitLifecycleEvent {
        event: LifecycleEvent {
            pr_key: pr_key.to_string(),
            worktree_path: worktree_path.to_string(),
            status: LifecycleStatus::Fixing,
            attempt: 1,
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
            });
            effects.push(OrchestratorEffect::CleanupMetadata {
                worktree_path: entry.worktree_path,
            });
        }
        state.claimed.remove(key);
        state.retry_queue.remove(key);
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

    let mut effects = Vec::new();
    let pr_context = parse_pr_key(pr_key);

    effects.push(OrchestratorEffect::DispatchSession {
        pr_key: pr_key.to_string(),
        worktree_path: worktree_path.to_string(),
        pr_context,
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
        });
    }

    state.claimed.remove(pr_key);
    state.retry_queue.remove(pr_key);

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
fn parse_pr_key(key: &str) -> crate::models::orchestrator::PrContext {
    let (repo, number_str) = key.rsplit_once('#').unwrap_or((key, "0"));
    let number = number_str.parse().unwrap_or(0);
    crate::models::orchestrator::PrContext {
        repo: repo.to_string(),
        number,
        branch: String::new(),
        base_branch: "main".to_string(),
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
    pr_key: &str,
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
         Shepherd PR {pr_key} using the pr-shepherd skill.\n\
         Read .branchdeck/pr-context.json for PR details.\n",
        repo = pr_context.repo,
        branch = pr_context.branch,
        number = pr_context.number,
    );

    if let Err(e) = std::fs::write(&task_path, &content) {
        error!("Failed to write orchestrator task: {e}");
    }

    task_path
}

/// Execute orchestrator effects. Thin executor — dispatches to services.
pub async fn execute_effects<R: tauri::Runtime>(
    effects: Vec<OrchestratorEffect>,
    orchestrator: &OrchestratorState,
    run_manager: &crate::services::run_manager::RunManagerState,
    app_handle: &tauri::AppHandle<R>,
) {
    use crate::models::agent::now_ms;

    for effect in effects {
        match effect {
            OrchestratorEffect::DispatchSession {
                pr_key: key,
                worktree_path,
                pr_context,
            } => {
                // Prepare worktree files
                write_pr_context(&worktree_path, &pr_context);
                deploy_skill_file(&worktree_path);
                let task_path = create_orchestrator_task(&worktree_path, &key, &pr_context);

                // Enqueue via RunManager with bypassPermissions for autonomous operation
                let launch_options = crate::models::run::LaunchOptions {
                    max_turns: None,
                    max_budget_usd: None,
                    permission_mode: Some("bypassPermissions".to_string()),
                };
                match crate::services::run_manager::enqueue_run(
                    std::sync::Arc::clone(run_manager),
                    app_handle.clone(),
                    &task_path,
                    &worktree_path,
                    launch_options,
                )
                .await
                {
                    Ok(_status) => {
                        // Read the actual tab_id from the RunManager's active run
                        let tab_id = {
                            let rm = run_manager.lock().await;
                            rm.get_status()
                                .and_then(|r| r.tab_id)
                                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
                        };
                        let mut orch = orchestrator.lock().await;
                        record_running(&mut orch, &key, &worktree_path, &tab_id, now_ms(), 1);
                        info!("Dispatched orchestrator session for {key} (tab={tab_id})");
                    }
                    Err(e) => {
                        error!("Failed to dispatch session for {key}: {e}");
                        // Release claim on failure
                        let mut orch = orchestrator.lock().await;
                        orch.claimed.remove(&key);
                    }
                }
            }
            OrchestratorEffect::StopSession { pr_key: key, .. } => {
                info!("Stopping session for {key}");
                // RunManager handles run termination via its own lifecycle
            }
            OrchestratorEffect::ScheduleRetry {
                pr_key: key,
                delay_ms,
                ..
            } => {
                debug!("Scheduled retry for {key} in {delay_ms}ms");
                // Retry timers handled by the event loop
            }
            OrchestratorEffect::CancelRetry { pr_key: key } => {
                debug!("Cancelled retry for {key}");
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
                    handle_event(&event, &orchestrator, &run_manager, &app_handle).await;
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
) {
    use crate::models::agent::{now_ms, Event};

    match event {
        Event::PrStatusChanged { repo, prs, .. } => {
            let effects = {
                let mut orch = orchestrator.lock().await;
                if !orch.config.enabled {
                    return;
                }
                let mut effects = apply_pr_event(&mut orch, repo, prs, now_ms());
                // Also reconcile on every PR update
                effects.extend(apply_reconciliation(&mut orch, prs));
                effects
            };
            execute_effects(effects, orchestrator, run_manager, app_handle).await;
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
            execute_effects(effects, orchestrator, run_manager, app_handle).await;
        }
        _ => {} // Other events not handled by orchestrator
    }
}
