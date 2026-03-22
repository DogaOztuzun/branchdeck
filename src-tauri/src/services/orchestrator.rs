use log::debug;

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
        .saturating_sub(state.running.len() as u32);

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
/// - `AnalysisWritten` → emit review_ready
/// - `FixCompleted` → mark completed, release claim
/// - `FixIncomplete` → schedule continuation retry
/// - `NoOutput` → schedule failure retry with backoff
pub fn apply_session_end(
    state: &mut Orchestrator,
    pr_key: &str,
    outcome: SessionOutcome,
    now: EpochMs,
) -> Vec<OrchestratorEffect> {
    let entry = match state.running.remove(pr_key) {
        Some(e) => e,
        None => {
            debug!("Orchestrator: session end for unknown PR {pr_key}");
            return Vec::new();
        }
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
    let retry = match state.retry_queue.remove(pr_key) {
        Some(r) => r,
        None => return Vec::new(),
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

/// Calculate retry backoff: min(FAILURE_BASE_MS * 2^(attempt-1), FAILURE_MAX_MS)
#[must_use]
pub fn retry_backoff(attempt: u32) -> u64 {
    let delay = FAILURE_BASE_MS.saturating_mul(1u64 << attempt.saturating_sub(1));
    delay.min(FAILURE_MAX_MS)
}

/// Parse a pr_key ("owner/repo#42") into a minimal PrContext.
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

/// Look up running entry by tab_id (for session end matching).
#[must_use]
pub fn find_pr_key_by_tab_id(state: &Orchestrator, tab_id: &str) -> Option<String> {
    state
        .running
        .values()
        .find(|e| e.tab_id == tab_id)
        .map(|e| e.pr_key.clone())
}
