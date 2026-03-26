//! Merge poller service.
//!
//! Polls GitHub for recently merged PRs on an interval and publishes
//! `PrMerged` events to the event bus. The orchestrator's `apply_merge_event`
//! handler converts these into `PostMerge` trigger events for workflow dispatch
//! (e.g., SAT re-score after an implement-issue PR merges).
//!
//! Architecture:
//! - Background tokio task, same pattern as `pr_poller` and `issue_poller`
//! - Tracks already-seen merged PR keys to avoid duplicate events
//! - Only publishes events for PRs not previously seen as merged

use std::collections::HashSet;
use std::sync::Arc;

use log::{debug, error, info};
use tokio::time::{interval, Duration};

use crate::models::agent::{now_ms, Event};
use crate::models::github::MergedPrInfo;
use crate::services::event_bus::EventBus;
use crate::services::github;

const POLL_INTERVAL_SECS: u64 = 60;

/// Start the merge poller as a background tokio task.
/// Polls GitHub for recently merged PRs across all managed repos,
/// publishes `PrMerged` events for newly detected merges.
pub fn start_merge_poller(event_bus: Arc<EventBus>, repo_paths: Vec<String>) {
    tokio::spawn(async move {
        poll_loop(&event_bus, &repo_paths).await;
    });
    info!("Merge poller started (interval={POLL_INTERVAL_SECS}s)");
}

async fn poll_loop(event_bus: &EventBus, repo_paths: &[String]) {
    let mut ticker = interval(Duration::from_secs(POLL_INTERVAL_SECS));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut seen_merged: HashSet<String> = HashSet::new();
    let mut first_tick = true;

    loop {
        ticker.tick().await;

        if repo_paths.is_empty() {
            debug!("Merge poller: no repos configured, skipping");
            continue;
        }

        match github::list_all_recently_merged_prs(repo_paths).await {
            Ok(merged_prs) => {
                if first_tick {
                    // Pre-seed: mark all currently merged PRs as seen without publishing events.
                    // This prevents cold-start from triggering re-scores for already-processed merges.
                    for pr in &merged_prs {
                        seen_merged.insert(format!("{}#{}", pr.repo_name, pr.number));
                    }
                    info!(
                        "Merge poller: pre-seeded {} existing merged PRs",
                        seen_merged.len()
                    );
                    first_tick = false;
                } else {
                    publish_new_merges(event_bus, &merged_prs, &mut seen_merged);
                    // Note: seen_merged grows monotonically within a session. The API
                    // returns at most 20 PRs per repo, so new entries are bounded per tick.
                    // Across restarts, seen_merged resets (pre-seed handles that).
                    // Orchestrator.merged_prs is the authoritative dedup layer.
                }
            }
            Err(e) => {
                error!("Merge poller failed: {e}");
            }
        }
    }
}

/// Publish `PrMerged` events for PRs not yet seen as merged.
fn publish_new_merges(
    event_bus: &EventBus,
    merged_prs: &[MergedPrInfo],
    seen: &mut HashSet<String>,
) {
    for pr in merged_prs {
        let key = format!("{}#{}", pr.repo_name, pr.number);

        if seen.contains(&key) {
            continue;
        }

        seen.insert(key.clone());

        info!(
            "Merge poller: new merge detected — {} ({:?})",
            key, pr.branch
        );

        let receivers = event_bus.publish(Event::PrMerged {
            repo: pr.repo_name.clone(),
            pr_number: pr.number,
            branch: pr.branch.clone(),
            base_branch: pr.base_branch.clone(),
            ts: now_ms(),
        });

        if receivers == 0 {
            debug!("Merge poller: no subscribers received PrMerged event for {key}");
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn make_merged_pr(number: u64, repo: &str, branch: &str) -> MergedPrInfo {
        MergedPrInfo {
            number,
            title: format!("PR #{number}"),
            branch: branch.to_string(),
            base_branch: "main".to_string(),
            repo_name: repo.to_string(),
            merged_at: "2026-03-26T12:00:00Z".to_string(),
        }
    }

    #[test]
    fn publish_new_merges_skips_seen() {
        let bus = EventBus::new();
        let mut seen = HashSet::new();
        seen.insert("owner/repo#1".to_string());

        let prs = vec![
            make_merged_pr(1, "owner/repo", "fix/old"),
            make_merged_pr(2, "owner/repo", "fix/new"),
        ];

        publish_new_merges(&bus, &prs, &mut seen);

        // PR #1 was already seen, only #2 should be added
        assert!(seen.contains("owner/repo#2"));
        assert_eq!(seen.len(), 2);
    }

    #[test]
    fn publish_new_merges_adds_to_seen() {
        let bus = EventBus::new();
        let mut seen = HashSet::new();

        let prs = vec![make_merged_pr(5, "test/repo", "feat/thing")];
        publish_new_merges(&bus, &prs, &mut seen);

        assert!(seen.contains("test/repo#5"));
    }

    #[test]
    fn publish_new_merges_empty_input() {
        let bus = EventBus::new();
        let mut seen = HashSet::new();

        publish_new_merges(&bus, &[], &mut seen);
        assert!(seen.is_empty());
    }
}
