#![allow(dead_code)]

use branchdeck_lib::models::run::{RunInfo, RunStatus};

/// Canonical task.md YAML content — single source of truth for all test files.
pub fn valid_task_md() -> String {
    base_task_md("created", 0, Some(42))
}

/// Build a task.md string with configurable status, run-count, and optional PR.
pub fn base_task_md(status: &str, run_count: u32, pr: Option<u64>) -> String {
    let pr_line = pr.map_or(String::new(), |n| format!("\npr: {n}"));
    format!(
        "\
---
type: issue-fix
scope: worktree
status: {status}
repo: owner/repo
branch: fix/bug-123{pr_line}
created: 2026-03-20T10:00:00+00:00
run-count: {run_count}
---
"
    )
}

/// Build a task.md with body content.
pub fn task_md_with_body(status: &str, run_count: u32, pr: Option<u64>, body: &str) -> String {
    let pr_line = pr.map_or(String::new(), |n| format!("\npr: {n}"));
    format!(
        "\
---
type: issue-fix
scope: worktree
status: {status}
repo: owner/repo
branch: fix/bug-123{pr_line}
created: 2026-03-20T10:00:00+00:00
run-count: {run_count}
---
{body}"
    )
}

/// Build a `PrSummary` for orchestrator tests.
pub fn make_pr_status(number: u64, failing: bool) -> branchdeck_lib::models::github::PrSummary {
    branchdeck_lib::models::github::PrSummary {
        number,
        title: format!("PR #{number}"),
        branch: format!("fix/pr-{number}"),
        url: format!("https://github.com/test/repo/pull/{number}"),
        ci_status: Some(if failing { "FAILURE" } else { "SUCCESS" }.to_string()),
        review_decision: None,
        repo_name: "test/repo".to_string(),
        author: "alice".to_string(),
        additions: None,
        deletions: None,
        changed_files: None,
        created_at: None,
    }
}

/// Build a `RunningEntry` for orchestrator tests.
pub fn make_running_entry(
    pr_key: &str,
) -> branchdeck_lib::models::orchestrator::RunningEntry {
    branchdeck_lib::models::orchestrator::RunningEntry {
        pr_key: pr_key.to_string(),
        worktree_path: format!("/tmp/wt/{pr_key}"),
        tab_id: format!("tab-{pr_key}"),
        started_at: 1000,
        attempt: 1,
    }
}

/// Build a `RunInfo` for testing. No filesystem needed.
pub fn make_run_info(status: RunStatus, session_id: Option<&str>) -> RunInfo {
    RunInfo {
        session_id: session_id.map(String::from),
        task_path: "/fake/.branchdeck/task.md".to_string(),
        status,
        started_at: "2026-03-20T10:00:00+00:00".to_string(),
        cost_usd: 0.0,
        last_heartbeat: None,
        elapsed_secs: 0,
        tab_id: Some("tab-1".to_string()),
    }
}
