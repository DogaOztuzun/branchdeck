use std::collections::HashMap;
use std::sync::Arc;

use log::{debug, error, info};
use tokio::time::{interval, Duration};

use crate::models::agent::{now_ms, Event};
use crate::models::github::IssueSummary;
use crate::services::event_bus::EventBus;
use crate::services::github;

const POLL_INTERVAL_SECS: u64 = 30;

/// Default label that triggers the implement-issue workflow.
const DEFAULT_ISSUE_LABEL: &str = "agent:implement";

/// Start the issue poller as a background tokio task.
/// Polls GitHub issues for all managed repos on an interval,
/// publishes `IssueDetected` events when new labeled issues appear.
pub fn start_issue_poller(event_bus: Arc<EventBus>, repo_paths: Vec<String>) {
    tokio::spawn(async move {
        poll_loop(&event_bus, &repo_paths).await;
    });
    info!("Issue poller started (interval={POLL_INTERVAL_SECS}s, label={DEFAULT_ISSUE_LABEL:?})");
}

async fn poll_loop(event_bus: &EventBus, repo_paths: &[String]) {
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_state: HashMap<String, Vec<IssueSummary>> = HashMap::new();

    loop {
        ticker.tick().await;

        if repo_paths.is_empty() {
            debug!("Issue poller: no repos configured, skipping");
            continue;
        }

        match github::list_all_issues_with_label(repo_paths, DEFAULT_ISSUE_LABEL).await {
            Ok(issues) => {
                let grouped = group_by_repo(&issues);
                publish_changes(event_bus, &grouped, &mut last_state);
            }
            Err(e) => {
                error!("Issue poller failed: {e}");
            }
        }
    }
}

fn group_by_repo(issues: &[IssueSummary]) -> HashMap<String, Vec<IssueSummary>> {
    let mut grouped: HashMap<String, Vec<IssueSummary>> = HashMap::new();
    for issue in issues {
        grouped
            .entry(issue.repo_name.clone())
            .or_default()
            .push(issue.clone());
    }
    grouped
}

fn publish_changes(
    event_bus: &EventBus,
    current: &HashMap<String, Vec<IssueSummary>>,
    last_state: &mut HashMap<String, Vec<IssueSummary>>,
) {
    for (repo, issues) in current {
        if issues.is_empty() {
            continue;
        }

        let changed = match last_state.get(repo) {
            Some(prev) => has_changes(prev, issues),
            None => true,
        };

        if changed {
            info!(
                "Issue poller: issue changes detected for {repo} ({} total)",
                issues.len()
            );
        }

        // Always publish when there are open issues — the orchestrator's
        // claimed/completed sets handle dedup. This ensures unclaimed issues
        // (e.g., after a failed dispatch) are retried on the next poll.
        let receivers = event_bus.publish(Event::IssueDetected {
            repo: repo.clone(),
            issues: issues.clone(),
            ts: now_ms(),
        });
        if receivers == 0 {
            debug!("Issue poller: no subscribers received event for {repo}");
        }
    }

    last_state.clone_from(current);
}

/// Check if the issue set changed (different numbers or count).
fn has_changes(prev: &[IssueSummary], current: &[IssueSummary]) -> bool {
    if prev.len() != current.len() {
        return true;
    }
    for issue in current {
        if !prev.iter().any(|p| p.number == issue.number) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_issue(number: u64, repo: &str) -> IssueSummary {
        IssueSummary {
            number,
            title: format!("Issue #{number}"),
            body: None,
            labels: vec!["agent:implement".to_string()],
            author: "test".to_string(),
            repo_name: repo.to_string(),
            created_at: None,
            url: String::new(),
        }
    }

    #[test]
    fn test_has_changes_empty_prev() {
        let prev = vec![];
        let current = vec![make_issue(1, "r")];
        assert!(has_changes(&prev, &current));
    }

    #[test]
    fn test_has_changes_same() {
        let prev = vec![make_issue(1, "r")];
        let current = vec![make_issue(1, "r")];
        assert!(!has_changes(&prev, &current));
    }

    #[test]
    fn test_has_changes_new_added() {
        let prev = vec![make_issue(1, "r")];
        let current = vec![make_issue(1, "r"), make_issue(2, "r")];
        assert!(has_changes(&prev, &current));
    }

    #[test]
    fn test_has_changes_detects_removal() {
        let prev = vec![make_issue(1, "r"), make_issue(2, "r")];
        let current = vec![make_issue(1, "r")];
        assert!(has_changes(&prev, &current));
    }

    #[test]
    fn test_group_by_repo() {
        let issues = vec![
            make_issue(1, "repo-a"),
            make_issue(2, "repo-b"),
            make_issue(3, "repo-a"),
        ];
        let grouped = group_by_repo(&issues);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped["repo-a"].len(), 2);
        assert_eq!(grouped["repo-b"].len(), 1);
    }
}
