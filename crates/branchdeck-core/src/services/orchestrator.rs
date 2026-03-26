use std::path::Path;
use std::sync::Arc;

use log::{debug, error, info};

use crate::models::agent::EpochMs;
use crate::models::github::{IssueSummary, PrSummary};
use crate::models::orchestrator::{
    is_pr_eligible, issue_key, pr_key, LifecycleEvent, LifecycleStatus, Orchestrator,
    OrchestratorEffect, RetryEntry, RunningEntry, SessionOutcome,
};
use crate::models::workflow::{TrackerKind, TriggerContext, TriggerEvent};
use crate::traits::{self, EventEmitter};

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
    _repo: &str,
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

        // Use workflow registry for trigger matching if available, fallback to hardcoded eligibility
        let eligible = if let Some(registry) = &state.registry {
            let trigger = TriggerEvent {
                kind: TrackerKind::GithubPr,
                context: TriggerContext::GithubPr {
                    repo: pr.repo_name.clone(),
                    number: pr.number,
                    branch: pr.branch.clone(),
                    base_branch: pr.base_branch.clone(),
                    ci_status: pr.ci_status.clone(),
                    review_decision: pr.review_decision.clone(),
                },
            };
            !registry.match_workflows(&trigger).is_empty()
        } else {
            is_pr_eligible(pr, &state.config)
        };
        if !eligible {
            continue;
        }

        // Claim and dispatch
        state.claimed.insert(key.clone());

        // Build absolute worktree path using repo_paths mapping
        let Some(repo_base) = state.repo_paths.get(&pr.repo_name).cloned() else {
            error!(
                "No repo_path mapping for {}; skipping dispatch for {key}",
                pr.repo_name
            );
            state.claimed.remove(&key);
            continue;
        };
        let worktree_path = build_worktree_path(&repo_base, &pr.branch);
        if worktree_path.is_empty() {
            error!(
                "Cannot build worktree path for PR {key} (branch: {})",
                pr.branch
            );
            state.claimed.remove(&key);
            continue;
        }

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
                session_id: None, // populated by executor after dispatch
            },
        });

        dispatched += 1;
    }

    effects
}

/// Handle incoming issue events. Convert to `TriggerEvent` for workflow matching.
/// Returns trigger events for issues not already claimed.
/// The actual dispatch is handled by the workflow engine in the executor.
#[must_use]
pub fn apply_issue_event(
    state: &mut Orchestrator,
    _repo: &str,
    issues: &[IssueSummary],
) -> Vec<TriggerEvent> {
    if !state.config.enabled {
        return Vec::new();
    }

    let mut triggers = Vec::new();

    for issue in issues {
        let key = issue_key(&issue.repo_name, issue.number);

        // Skip if already claimed or completed
        if state.claimed.contains(&key) || state.completed.contains(&key) {
            debug!("Orchestrator: issue {key} already tracked, skipping");
            continue;
        }

        // Claim to prevent re-triggering
        state.claimed.insert(key.clone());

        info!(
            "Orchestrator: issue {key} detected, creating trigger event for {:?}",
            issue.title
        );

        triggers.push(TriggerEvent {
            kind: TrackerKind::GithubIssue,
            context: TriggerContext::GithubIssue {
                repo: issue.repo_name.clone(),
                number: issue.number,
                title: issue.title.clone(),
                body: issue.body.clone(),
                labels: issue.labels.clone(),
            },
        });
    }

    triggers
}

/// Handle a PR merge detection. Produces `TriggerEvent::PostMerge` for each
/// newly-merged PR that was previously tracked (claimed/completed).
///
/// The merge poller calls this when it detects `merged_at` on a PR. The function:
/// 1. Skips PRs already processed as merged (idempotent)
/// 2. Records the PR key in `merged_prs` to prevent duplicate triggers
/// 3. Returns `PostMerge` trigger events for workflow dispatch
///
/// The actual re-score dispatch is handled by `workflow_dispatch::plan_dispatch`
/// matching a `post-merge` tracker workflow.
#[must_use]
pub fn apply_merge_event(
    state: &mut Orchestrator,
    repo: &str,
    pr_number: u64,
    branch: &str,
) -> Vec<TriggerEvent> {
    if !state.config.enabled {
        return Vec::new();
    }

    let key = pr_key(repo, pr_number);

    // Skip if already processed as merged
    if state.merged_prs.contains(&key) {
        debug!("Orchestrator: merge for {key} already processed, skipping");
        return Vec::new();
    }

    // Record to prevent duplicate triggers
    state.merged_prs.insert(key.clone());

    // Clean up any stale orchestrator state for this PR
    state.running.remove(&key);
    state.claimed.remove(&key);
    state.retry_queue.remove(&key);
    state.review_ready.remove(&key);

    info!(
        "Orchestrator: PR {key} merged, creating PostMerge trigger for re-score"
    );

    vec![TriggerEvent {
        kind: TrackerKind::PostMerge,
        context: TriggerContext::PostMerge {
            repo: repo.to_string(),
            pr_number,
            branch: branch.to_string(),
        },
    }]
}

/// Handle session end. Route based on outcome:
/// - `AnalysisWritten` → emit `review_ready`
/// - `FixCompleted` → mark completed, release claim
/// - `FixIncomplete` → schedule continuation retry
/// - `NoOutput` → schedule failure retry with backoff
#[allow(clippy::too_many_lines)]
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
                    branch: entry.branch.clone(),
                    base_branch: entry.base_branch.clone(),
                },
            );

            effects.push(OrchestratorEffect::EmitLifecycleEvent {
                event: LifecycleEvent {
                    pr_key: pr_key.to_string(),
                    worktree_path: entry.worktree_path,
                    status: LifecycleStatus::ReviewReady,
                    attempt: entry.attempt,
                    started_at: entry.started_at,
                    session_id: None,
                },
            });
        }
        SessionOutcome::FixCompleted => {
            state.claimed.remove(pr_key);
            state.completed.insert(pr_key.to_string());

            let event = LifecycleEvent {
                pr_key: pr_key.to_string(),
                worktree_path: entry.worktree_path,
                status: LifecycleStatus::Completed,
                attempt: entry.attempt,
                started_at: entry.started_at,
                session_id: None,
            };
            info!("PR {pr_key} completed (fix succeeded), tracking in completed_lifecycles");
            state
                .completed_lifecycles
                .insert(pr_key.to_string(), event.clone());

            effects.push(OrchestratorEffect::EmitLifecycleEvent { event });
        }
        SessionOutcome::FixIncomplete | SessionOutcome::NoOutput => {
            if entry.attempt >= MAX_RETRIES {
                // Max retries exceeded — mark as completed (failed)
                state.claimed.remove(pr_key);
                state.completed.insert(pr_key.to_string());
                error!("PR {pr_key} exceeded max retries ({MAX_RETRIES}), giving up");

                let event = LifecycleEvent {
                    pr_key: pr_key.to_string(),
                    worktree_path: entry.worktree_path,
                    status: LifecycleStatus::Failed,
                    attempt: entry.attempt,
                    started_at: entry.started_at,
                    session_id: None,
                };
                state
                    .completed_lifecycles
                    .insert(pr_key.to_string(), event.clone());

                effects.push(OrchestratorEffect::EmitLifecycleEvent { event });
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
                        branch: entry.branch.clone(),
                        base_branch: entry.base_branch.clone(),
                    },
                );

                effects.push(OrchestratorEffect::EmitLifecycleEvent {
                    event: LifecycleEvent {
                        pr_key: pr_key.to_string(),
                        worktree_path: entry.worktree_path,
                        status: LifecycleStatus::Retrying,
                        attempt: entry.attempt + 1,
                        started_at: entry.started_at,
                        session_id: None,
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

    // Guard: respect concurrency limit
    let running_count = u32::try_from(state.running.len()).unwrap_or(u32::MAX);
    if running_count >= state.config.max_concurrent {
        debug!("Orchestrator: relaunch deferred for {pr_key}, at concurrency limit");
        return effects;
    }

    // Cancel any pending retry to prevent duplicate dispatch
    let retry_entry = state.retry_queue.remove(pr_key);
    if retry_entry.is_some() {
        effects.push(OrchestratorEffect::CancelRetry {
            pr_key: pr_key.to_string(),
        });
    }

    // Recover branch info + attempt from retry or review_ready
    let review_entry = state.review_ready.remove(pr_key);
    let (prev_attempt, branch, base_branch) = if let Some(ref r) = retry_entry {
        (r.attempt, r.branch.clone(), r.base_branch.clone())
    } else if let Some(ref r) = review_entry {
        (r.attempt, r.branch.clone(), r.base_branch.clone())
    } else {
        (0, String::new(), String::new())
    };
    let next_attempt = prev_attempt + 1;

    // Ensure claimed
    state.claimed.insert(pr_key.to_string());

    // Build PrContext from stored data instead of parsing pr_key
    let mut pr_context = parse_pr_key(pr_key);
    if !branch.is_empty() {
        pr_context.branch = branch;
    }
    if !base_branch.is_empty() {
        pr_context.base_branch = base_branch;
    }

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
            session_id: None, // populated by executor after dispatch
        },
    });

    effects
}

/// Reconcile orchestrator state with current PR states for a specific repo.
/// Stop sessions for merged/closed PRs, flag stale for CI-now-passing.
pub fn apply_reconciliation(
    state: &mut Orchestrator,
    repo: &str,
    current_prs: &[PrSummary],
) -> Vec<OrchestratorEffect> {
    let mut effects = Vec::new();
    let repo_prefix = format!("{repo}#");

    // Build set of currently open PR keys
    let open_keys: std::collections::HashSet<String> = current_prs
        .iter()
        .map(|pr| pr_key(&pr.repo_name, pr.number))
        .collect();

    // Find running sessions for THIS repo whose PRs are no longer open
    let stale_keys: Vec<String> = state
        .running
        .keys()
        .filter(|k| k.starts_with(&repo_prefix) && !open_keys.contains(k.as_str()))
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

    // Clean up retries for closed PRs (scoped to this repo)
    let stale_retries: Vec<String> = state
        .retry_queue
        .keys()
        .filter(|k| k.starts_with(&repo_prefix) && !open_keys.contains(k.as_str()))
        .cloned()
        .collect();

    for key in &stale_retries {
        state.retry_queue.remove(key);
        state.claimed.remove(key);
        effects.push(OrchestratorEffect::CancelRetry {
            pr_key: key.clone(),
        });
    }

    // Re-activate completed PRs that have new CI failures (AC-7)
    let completed_keys_for_repo: Vec<String> = state
        .completed
        .iter()
        .filter(|k| k.starts_with(&repo_prefix))
        .cloned()
        .collect();

    for key in &completed_keys_for_repo {
        // Check if this PR is in the current set with failing CI
        let should_reactivate = current_prs.iter().any(|pr| {
            pr_key(&pr.repo_name, pr.number) == *key
                && (matches!(pr.ci_status.as_deref(), Some("FAILURE" | "ERROR"))
                    || matches!(pr.review_decision.as_deref(), Some("CHANGES_REQUESTED")))
        });

        if should_reactivate {
            info!(
                "Reconciliation: re-activating completed PR {key} (new failure or review changes)"
            );
            state.completed.remove(key);
            state.completed_lifecycles.remove(key);
        } else {
            debug!("Reconciliation: completed PR {key} still passing, keeping in Done");
        }
    }

    // Detect CI-now-passing for review_ready PRs (AC10: stale detection)
    for pr in current_prs {
        let key = pr_key(&pr.repo_name, pr.number);
        if let Some(entry) = state.review_ready.get_mut(&key) {
            let ci_passing = matches!(pr.ci_status.as_deref(), Some("SUCCESS" | "NEUTRAL"));
            if ci_passing && !entry.stale {
                entry.stale = true;
                effects.push(OrchestratorEffect::EmitLifecycleEvent {
                    event: LifecycleEvent {
                        pr_key: key,
                        worktree_path: entry.worktree_path.clone(),
                        status: LifecycleStatus::Stale,
                        attempt: entry.attempt,
                        started_at: entry.started_at,
                        session_id: None,
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
        // Put it back and schedule a re-check after a short delay
        let wt = retry.worktree_path.clone();
        state.retry_queue.insert(pr_key.to_string(), retry);
        debug!("Orchestrator: retry for {pr_key} deferred, at concurrency limit");
        return vec![OrchestratorEffect::ScheduleRetry {
            pr_key: pr_key.to_string(),
            worktree_path: wt,
            attempt: 0, // not a real attempt, just a re-check timer
            delay_ms: CONTINUATION_DELAY_MS,
            error: None,
        }];
    }

    let mut effects = Vec::new();

    // Build PrContext from stored retry data instead of parsing pr_key
    let mut pr_context = parse_pr_key(pr_key);
    if !retry.branch.is_empty() {
        pr_context.branch = retry.branch;
    }
    if !retry.base_branch.is_empty() {
        pr_context.base_branch = retry.base_branch;
    }

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
            session_id: None, // populated by executor after dispatch
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
    pr_context: &crate::models::orchestrator::PrContext,
) {
    state.running.insert(
        pr_key.to_string(),
        RunningEntry {
            pr_key: pr_key.to_string(),
            worktree_path: worktree_path.to_string(),
            tab_id: tab_id.to_string(),
            started_at: now,
            attempt,
            branch: pr_context.branch.clone(),
            base_branch: pr_context.base_branch.clone(),
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
    state.completed.insert(pr_key.to_string());

    // Emit terminal lifecycle event so frontend clears the card
    let event = LifecycleEvent {
        pr_key: pr_key.to_string(),
        worktree_path: String::new(),
        status: LifecycleStatus::Completed,
        attempt: 0,
        started_at: 0,
        session_id: None,
    };
    info!("PR {pr_key} skipped, tracking in completed_lifecycles");
    state
        .completed_lifecycles
        .insert(pr_key.to_string(), event.clone());

    effects.push(OrchestratorEffect::EmitLifecycleEvent { event });

    effects
}

// --- Helpers ---

/// Build the worktree path for a branch, matching `git::create_worktree`'s logic.
/// Returns `repo_path.parent()/worktrees/{sanitized_branch}`.
#[must_use]
pub fn build_worktree_path(repo_path: &str, branch: &str) -> String {
    let sanitized = crate::services::git::sanitize_worktree_name(branch);
    if sanitized.is_empty() {
        return String::new();
    }
    let repo = std::path::Path::new(repo_path);
    let parent = repo.parent().unwrap_or(repo);
    parent
        .join("worktrees")
        .join(&sanitized)
        .display()
        .to_string()
}

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

// --- Impure functions (file I/O, event emission) ---

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
const DEFAULT_SKILL: &str = include_str!("../../../../.claude/skills/pr-shepherd/SKILL.md");

/// Deploy the PR shepherd skill file into a worktree if not already present.
fn deploy_skill_file(worktree_path: &str, skill_content: Option<&str>) {
    let content = skill_content.unwrap_or(DEFAULT_SKILL);
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

    if let Err(e) = crate::util::write_atomic(Path::new(&skill_path), content.as_bytes()) {
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
            if let Err(e) = crate::util::write_atomic(Path::new(&path), json.as_bytes()) {
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

    if let Err(e) = crate::util::write_atomic(Path::new(&task_path), content.as_bytes()) {
        error!("Failed to write orchestrator task: {e}");
    }

    task_path
}

/// Find the filesystem path where a branch is already checked out.
fn find_worktree_for_branch(
    repo_path: &std::path::Path,
    branch: &str,
) -> Option<std::path::PathBuf> {
    let repo = git2::Repository::open(repo_path).ok()?;

    // Check if the main repo itself has this branch checked out
    if let Ok(head) = repo.head() {
        if head.shorthand() == Some(branch) {
            return Some(repo_path.to_path_buf());
        }
    }

    // Check worktrees
    if let Ok(worktrees) = repo.worktrees() {
        for name in worktrees.iter().flatten() {
            if let Ok(wt) = repo.find_worktree(name) {
                let wt_path = wt.path();
                if let Ok(wt_repo) = git2::Repository::open(wt_path) {
                    if let Ok(head) = wt_repo.head() {
                        if head.shorthand() == Some(branch) {
                            return Some(wt_path.to_path_buf());
                        }
                    }
                }
            }
        }
    }

    None
}

/// Check if worktree exists with the correct branch. Returns true if a new worktree
/// needs to be created (either doesn't exist, wrong branch, or invalid repo).
fn worktree_needs_create(worktree_path: &str, expected_branch: &str) -> bool {
    let wt_path = std::path::Path::new(worktree_path);
    if !wt_path.exists() {
        return true;
    }

    let Ok(repo) = git2::Repository::open(wt_path) else {
        info!("Invalid git worktree at {worktree_path}, removing");
        let _ = std::fs::remove_dir_all(wt_path);
        return true;
    };

    let current_branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(String::from))
        .unwrap_or_default();

    if current_branch == expected_branch {
        debug!("Worktree at {worktree_path} already on correct branch");
        return false;
    }

    info!(
        "Worktree at {worktree_path} is on branch '{current_branch}', \
         need '{expected_branch}' — removing stale worktree"
    );

    // Remove stale worktree so we can recreate on the right branch
    if let Some(repo_base) = worktree_path
        .find("/worktrees/")
        .map(|idx| &worktree_path[..idx])
    {
        let name = wt_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        if let Err(e) =
            crate::services::git::remove_worktree(std::path::Path::new(repo_base), name, false)
        {
            error!("Failed to remove stale worktree: {e}");
            let _ = std::fs::remove_dir_all(wt_path);
            // Prune orphaned git worktree registration after fallback removal
            match std::process::Command::new("git")
                .args(["worktree", "prune"])
                .current_dir(repo_base)
                .output()
            {
                Ok(out) if out.status.success() => {
                    debug!("Pruned orphaned worktree registrations in {repo_base}");
                }
                Ok(out) => {
                    error!(
                        "git worktree prune failed: {}",
                        String::from_utf8_lossy(&out.stderr)
                    );
                }
                Err(e) => error!("Failed to run git worktree prune: {e}"),
            }
        }
    }

    true
}

/// Execute the `DispatchSession` effect: create worktree, deploy files, enqueue run.
#[allow(clippy::too_many_lines)]
async fn execute_dispatch(
    key: &str,
    worktree_path: &str,
    pr_context: &crate::models::orchestrator::PrContext,
    attempt: u32,
    orchestrator: &OrchestratorState,
    run_manager: &crate::services::run_manager::RunManagerState,
) {
    use crate::models::agent::now_ms;

    // The effective worktree path — may be overridden if we reuse an existing checkout
    let mut effective_path = worktree_path.to_string();

    // Ensure worktree exists and is on the correct branch
    if worktree_needs_create(worktree_path, &pr_context.branch) {
        // Look up repo filesystem path from orchestrator state
        let repo_fs_path = {
            let orch = orchestrator.lock().await;
            orch.repo_paths.get(&pr_context.repo).cloned()
        };
        let Some(repo_fs_path) = repo_fs_path else {
            error!(
                "No repo_path mapping for {} — cannot create worktree",
                pr_context.repo
            );
            orchestrator.lock().await.claimed.remove(key);
            return;
        };
        let base = if pr_context.base_branch.is_empty() {
            None
        } else {
            Some(pr_context.base_branch.as_str())
        };
        match crate::services::git::create_worktree(
            std::path::Path::new(&repo_fs_path),
            &pr_context.branch,
            Some(&pr_context.branch),
            base,
        ) {
            Ok(wt) => info!("Created worktree at {}", wt.path.display()),
            Err(e) => {
                let err_msg = format!("{e}");
                if err_msg.contains("already checked out") {
                    if let Some(existing_path) = find_worktree_for_branch(
                        std::path::Path::new(&repo_fs_path),
                        &pr_context.branch,
                    ) {
                        info!(
                            "Branch {} already checked out at {}, reusing",
                            pr_context.branch,
                            existing_path.display()
                        );
                        effective_path = existing_path.display().to_string();
                    } else {
                        error!(
                            "Branch {} already checked out but couldn't find where",
                            pr_context.branch
                        );
                        orchestrator.lock().await.claimed.remove(key);
                        return;
                    }
                } else {
                    error!("Failed to create worktree for {key}: {e}");
                    orchestrator.lock().await.claimed.remove(key);
                    return;
                }
            }
        }
    }

    let worktree_path = effective_path.as_str();
    write_pr_context(worktree_path, pr_context);

    // Look up the pr-shepherd workflow definition for skill content and config
    let repo_fs_path = {
        let orch = orchestrator.lock().await;
        orch.repo_paths.get(&pr_context.repo).cloned()
    };
    let workflow_def = repo_fs_path.as_ref().and_then(|rp| {
        let dirs = crate::services::workflow::default_search_dirs(rp);
        let registry = crate::services::workflow::WorkflowRegistry::scan(&dirs);
        registry.get_workflow("pr-shepherd").cloned()
    });

    let skill_content = workflow_def.as_ref().map(|def| def.prompt.as_str());
    deploy_skill_file(worktree_path, skill_content);
    let task_path = create_orchestrator_task(worktree_path, key, pr_context);

    let max_budget_usd = workflow_def
        .as_ref()
        .and_then(|def| def.config.agent.as_ref())
        .and_then(|a| a.max_budget_usd);
    let launch_options = crate::models::run::LaunchOptions {
        max_turns: None,
        max_budget_usd,
        permission_mode: Some("bypassPermissions".to_string()),
        allowed_directories: vec![worktree_path.to_string()],
    };
    match crate::services::run_manager::enqueue_run(
        Arc::clone(run_manager),
        &task_path,
        worktree_path,
        launch_options,
    )
    .await
    {
        Ok(_status) => {
            // Read tab_id from run.json saved by launch_run for THIS task
            let tab_id = crate::services::run_state::load_run_state(worktree_path)
                .and_then(|r| r.tab_id)
                .unwrap_or_else(|| {
                    error!("No tab_id in run.json for {key} — fallback to UUID");
                    uuid::Uuid::new_v4().to_string()
                });
            let mut orch = orchestrator.lock().await;
            record_running(
                &mut orch,
                key,
                worktree_path,
                &tab_id,
                now_ms(),
                attempt,
                pr_context,
            );
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
pub async fn execute_effects(
    effects: Vec<OrchestratorEffect>,
    orchestrator: &OrchestratorState,
    run_manager: &crate::services::run_manager::RunManagerState,
    emitter: &Arc<dyn EventEmitter>,
    event_bus: Option<&Arc<crate::services::event_bus::EventBus>>,
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
            OrchestratorEffect::EmitLifecycleEvent { mut event } => {
                // Populate session_id from running entry if not already set
                if event.session_id.is_none()
                    && matches!(
                        event.status,
                        LifecycleStatus::Running | LifecycleStatus::Fixing
                    )
                {
                    let orch = orchestrator.lock().await;
                    if let Some(entry) = orch.running.get(&event.pr_key) {
                        event.session_id = Some(entry.tab_id.clone());
                    }
                }
                if let Err(e) = traits::emit(emitter.as_ref(), "lifecycle:updated", &event) {
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
pub type OrchestratorState = Arc<tokio::sync::Mutex<Orchestrator>>;

/// Create orchestrator managed state.
#[must_use]
pub fn create_orchestrator_state(
    config: crate::models::orchestrator::OrchestratorConfig,
) -> OrchestratorState {
    Arc::new(tokio::sync::Mutex::new(Orchestrator::new(config)))
}

/// Load the workflow registry from all configured repo paths and cache it.
/// Call this after `repo_paths` have been populated.
pub fn load_registry(state: &mut Orchestrator) {
    let mut all_dirs = Vec::new();
    for repo_path in state.repo_paths.values() {
        all_dirs.extend(crate::services::workflow::default_search_dirs(repo_path));
    }
    let registry = crate::services::workflow::WorkflowRegistry::scan(&all_dirs);
    info!(
        "Orchestrator: loaded workflow registry with {} workflow(s)",
        registry.len()
    );
    state.registry = Some(registry);
}

/// Start the orchestrator event loop as a background tokio task.
/// Subscribes to `EventBus` and routes events to pure state machine functions.
pub fn start_orchestrator(
    orchestrator: OrchestratorState,
    event_bus: Arc<crate::services::event_bus::EventBus>,
    run_manager: crate::services::run_manager::RunManagerState,
    emitter: Arc<dyn EventEmitter>,
) {
    tokio::spawn(async move {
        let mut rx = event_bus.subscribe();
        info!("Orchestrator event loop started");

        loop {
            match rx.recv().await {
                Ok(event) => {
                    handle_event(&event, &orchestrator, &run_manager, &emitter, &event_bus).await;
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
#[allow(clippy::too_many_lines)]
async fn handle_event(
    event: &crate::models::agent::Event,
    orchestrator: &OrchestratorState,
    run_manager: &crate::services::run_manager::RunManagerState,
    emitter: &Arc<dyn EventEmitter>,
    event_bus: &Arc<crate::services::event_bus::EventBus>,
) {
    use crate::models::agent::{now_ms, Event};

    match event {
        Event::PrStatusChanged { repo, prs, .. } => {
            let effects = {
                let mut orch = orchestrator.lock().await;
                // apply_pr_event has its own enabled/auto_analyze guard
                let mut effects = apply_pr_event(&mut orch, repo, prs, now_ms());
                // Reconciliation always runs (cleans up merged/closed PRs), scoped to this repo
                effects.extend(apply_reconciliation(&mut orch, repo, prs));
                effects
            };
            execute_effects(effects, orchestrator, run_manager, emitter, Some(event_bus)).await;
        }
        Event::RunComplete { tab_id, status, .. } => {
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

            // Issue workflows (key contains #i) don't produce analysis.json.
            // Treat successful completion as FixCompleted, failure as NoOutput.
            let is_issue_workflow = key.contains("#i");
            let outcome = if is_issue_workflow {
                if status == "success" {
                    SessionOutcome::FixCompleted
                } else {
                    SessionOutcome::NoOutput
                }
            } else {
                determine_session_outcome(&worktree)
            };
            let effects = {
                let mut orch = orchestrator.lock().await;
                apply_session_end(&mut orch, &key, outcome, now_ms())
            };
            execute_effects(effects, orchestrator, run_manager, emitter, Some(event_bus)).await;
        }
        Event::RetryDue {
            pr_key: key,
            worktree_path,
        } => {
            let effects = {
                let mut orch = orchestrator.lock().await;
                // Clean up the fired timer handle to prevent unbounded accumulation
                orch.retry_timers.remove(key);
                apply_retry_due(&mut orch, key, worktree_path, now_ms())
            };
            execute_effects(effects, orchestrator, run_manager, emitter, Some(event_bus)).await;
        }
        Event::IssueDetected { repo, issues, .. } => {
            let trigger_events = {
                let mut orch = orchestrator.lock().await;
                apply_issue_event(&mut orch, repo, issues)
            };

            if trigger_events.is_empty() {
                return;
            }

            // Get repo path and cached registry for workflow dispatch
            let (repo_path, registry) = {
                let orch = orchestrator.lock().await;
                let rp = orch.repo_paths.get(repo).cloned();
                let reg = orch.registry.clone();
                (rp, reg)
            };

            let Some(repo_path) = repo_path else {
                error!("No repo_path mapping for {repo}; cannot dispatch issue workflows");
                return;
            };

            // Use cached registry, fall back to fresh scan
            let registry = registry.unwrap_or_else(|| {
                let dirs = crate::services::workflow::default_search_dirs(&repo_path);
                crate::services::workflow::WorkflowRegistry::scan(&dirs)
            });

            for trigger in &trigger_events {
                // Respect concurrency limit — unclaim remaining triggers on break
                {
                    let orch = orchestrator.lock().await;
                    let running_count = u32::try_from(orch.running.len()).unwrap_or(u32::MAX);
                    if running_count >= orch.config.max_concurrent {
                        debug!("Orchestrator: at concurrency limit, deferring issue dispatch");
                        // Unclaim all remaining triggers so they retry next poll
                        drop(orch);
                        let mut orch = orchestrator.lock().await;
                        for t in &trigger_events {
                            if let TriggerContext::GithubIssue {
                                repo: r, number, ..
                            } = &t.context
                            {
                                orch.claimed.remove(&issue_key(r, *number));
                            }
                        }
                        break;
                    }
                }
                let plan = crate::services::workflow_dispatch::plan_dispatch(
                    &registry, trigger, &repo_path,
                );
                if plan.workflow_name.is_empty() {
                    debug!(
                        "No workflow matched issue trigger for {repo}: {:?}",
                        trigger.context
                    );
                    // Unclaim so next poll retries (e.g. after user installs a matching workflow)
                    if let TriggerContext::GithubIssue {
                        repo: r, number, ..
                    } = &trigger.context
                    {
                        orchestrator
                            .lock()
                            .await
                            .claimed
                            .remove(&issue_key(r, *number));
                    }
                    continue;
                }

                info!(
                    "Issue trigger matched workflow {:?} for {repo}",
                    plan.workflow_name
                );

                // Execute the dispatch plan
                let result = crate::services::workflow_dispatch::execute_dispatch_plan(
                    &plan,
                    run_manager,
                    emitter,
                )
                .await;

                if let TriggerContext::GithubIssue {
                    repo: r, number, ..
                } = &trigger.context
                {
                    let key = issue_key(r, *number);
                    let issue_branch = format!("workflow/{}-issue-{}", plan.workflow_name, number);
                    if let Some((worktree_path, tab_id)) = result {
                        let now = crate::models::agent::now_ms();
                        let mut orch = orchestrator.lock().await;
                        orch.running.insert(
                            key.clone(),
                            RunningEntry {
                                pr_key: key.clone(),
                                worktree_path: worktree_path.clone(),
                                tab_id,
                                started_at: now,
                                attempt: 1,
                                branch: issue_branch,
                                base_branch: "main".to_string(),
                            },
                        );
                        drop(orch);

                        // Emit lifecycle event for issue dispatch
                        let event = LifecycleEvent {
                            pr_key: key.clone(),
                            worktree_path,
                            status: LifecycleStatus::Running,
                            attempt: 1,
                            started_at: now,
                            session_id: None,
                        };
                        if let Err(e) = traits::emit(emitter.as_ref(), "lifecycle:updated", &event)
                        {
                            error!("Failed to emit lifecycle:updated for issue {key}: {e}");
                        }
                    } else {
                        // Dispatch failed — unclaim so the issue can be retried next poll
                        error!("Dispatch failed for issue {key}, unclaiming");
                        let mut orch = orchestrator.lock().await;
                        orch.claimed.remove(&key);
                    }
                }
            }
        }
        _ => {} // Other events not handled by orchestrator
    }
}
