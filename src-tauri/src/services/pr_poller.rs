use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use log::{debug, error, info};
use tauri::Emitter;
use tokio::time::{interval, Duration};

use crate::models::agent::{now_ms, Event};
use crate::models::github::PrSummary;
use crate::services::event_bus::EventBus;
use crate::services::github;

const POLL_INTERVAL_SECS: u64 = 30;

/// Shared state: all discovered PRs keyed by repo name.
pub type DiscoveredPrsState = Arc<RwLock<HashMap<String, Vec<PrSummary>>>>;

/// Start the PR poller as a background tokio task.
/// Polls GitHub PRs for all managed repos on an interval,
/// publishes `PrStatusChanged` events when state changes,
/// and emits `pr:discovered` Tauri events for the frontend.
pub fn start_pr_poller<R: tauri::Runtime + 'static>(
    event_bus: Arc<EventBus>,
    repo_paths: Vec<String>,
    discovered_prs: DiscoveredPrsState,
    app_handle: tauri::AppHandle<R>,
) {
    tauri::async_runtime::spawn(async move {
        poll_loop(&event_bus, &repo_paths, &discovered_prs, &app_handle).await;
    });
    info!("PR poller started (interval={POLL_INTERVAL_SECS}s)");
}

async fn poll_loop<R: tauri::Runtime>(
    event_bus: &EventBus,
    repo_paths: &[String],
    discovered_prs: &DiscoveredPrsState,
    app_handle: &tauri::AppHandle<R>,
) {
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_state: HashMap<String, Vec<PrSummary>> = HashMap::new();

    loop {
        ticker.tick().await;

        if repo_paths.is_empty() {
            debug!("PR poller: no repos configured, skipping");
            continue;
        }

        match github::list_all_open_prs(repo_paths, None).await {
            Ok(prs) => {
                let grouped = group_by_repo(&prs);
                publish_changes(event_bus, &grouped, &mut last_state);

                // Update shared state for frontend IPC queries
                if let Ok(mut state) = discovered_prs.write() {
                    state.clone_from(&grouped);
                }

                // Emit full snapshot to frontend
                let all_prs_flat: Vec<PrSummary> =
                    grouped.values().flat_map(|v| v.iter().cloned()).collect();
                info!("PR poller: emitting pr:discovered ({} PRs)", all_prs_flat.len());
                if let Err(e) = app_handle.emit("pr:discovered", &all_prs_flat) {
                    error!("Failed to emit pr:discovered: {e}");
                }
            }
            Err(e) => {
                error!("PR poller failed: {e}");
            }
        }
    }
}

fn group_by_repo(prs: &[PrSummary]) -> HashMap<String, Vec<PrSummary>> {
    let mut grouped: HashMap<String, Vec<PrSummary>> = HashMap::new();
    for pr in prs {
        grouped
            .entry(pr.repo_name.clone())
            .or_default()
            .push(pr.clone());
    }
    grouped
}

fn publish_changes(
    event_bus: &EventBus,
    current: &HashMap<String, Vec<PrSummary>>,
    last_state: &mut HashMap<String, Vec<PrSummary>>,
) {
    for (repo, prs) in current {
        let changed = match last_state.get(repo) {
            Some(prev) => has_changes(prev, prs),
            None => true,
        };

        if changed {
            debug!("PR poller: changes detected for {repo} ({} PRs)", prs.len());
            let receivers = event_bus.publish(Event::PrStatusChanged {
                repo: repo.clone(),
                prs: prs.clone(),
                ts: now_ms(),
            });
            if receivers == 0 {
                error!("PR poller: no subscribers received event for {repo}");
            }
        }
    }

    // Detect repos that no longer have open PRs
    let current_repos: std::collections::HashSet<&String> = current.keys().collect();
    let removed: Vec<String> = last_state
        .keys()
        .filter(|r| !current_repos.contains(r))
        .cloned()
        .collect();

    for repo in &removed {
        debug!("PR poller: no more open PRs for {repo}");
        let receivers = event_bus.publish(Event::PrStatusChanged {
            repo: repo.clone(),
            prs: Vec::new(),
            ts: now_ms(),
        });
        if receivers == 0 {
            error!("PR poller: no subscribers received removed-repo event for {repo}");
        }
    }

    // Update last known state
    last_state.clone_from(current);
}

fn has_changes(prev: &[PrSummary], current: &[PrSummary]) -> bool {
    if prev.len() != current.len() {
        return true;
    }

    for curr_pr in current {
        let prev_pr = prev.iter().find(|p| p.number == curr_pr.number);
        match prev_pr {
            None => return true,
            Some(p) => {
                if p.ci_status != curr_pr.ci_status
                    || p.review_decision != curr_pr.review_decision
                    || p.head_sha != curr_pr.head_sha
                {
                    return true;
                }
            }
        }
    }

    false
}
