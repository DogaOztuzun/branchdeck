use std::collections::HashMap;
use std::sync::Arc;

use log::{debug, error, info};
use tokio::time::{interval, Duration};

use crate::models::agent::{now_ms, Event};
use crate::models::github::PrSummary;
use crate::services::event_bus::EventBus;
use crate::services::github;

const POLL_INTERVAL_SECS: u64 = 30;

/// Start the PR poller as a background tokio task.
/// Polls GitHub PRs for all managed repos on an interval,
/// publishes `PrStatusChanged` events when state changes.
pub fn start_pr_poller(event_bus: Arc<EventBus>, repo_paths: Vec<String>) {
    tauri::async_runtime::spawn(async move {
        poll_loop(&event_bus, &repo_paths).await;
    });
    info!("PR poller started (interval={}s)", POLL_INTERVAL_SECS);
}

async fn poll_loop(event_bus: &EventBus, repo_paths: &[String]) {
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));
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
            let _ = event_bus.publish(Event::PrStatusChanged {
                repo: repo.clone(),
                prs: prs.clone(),
                ts: now_ms(),
            });
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
        let _ = event_bus.publish(Event::PrStatusChanged {
            repo: repo.clone(),
            prs: Vec::new(),
            ts: now_ms(),
        });
    }

    // Update last known state
    *last_state = current.clone();
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
                {
                    return true;
                }
            }
        }
    }

    false
}
