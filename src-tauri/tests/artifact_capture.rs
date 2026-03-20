//! P0 integration tests for artifact capture after run completion.
//!
//! Tests T2-INT-001 through T2-INT-006 from test-design-phase1.md.
//! Requires temp git repos via git2 + tempfile.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::doc_markdown, clippy::cast_possible_wrap, clippy::cast_sign_loss)]

mod common;

use branchdeck_lib::services::task;
use git2::{Repository, Signature};
use std::path::Path;
use tempfile::TempDir;

const TASK_DIR: &str = ".branchdeck";
const TASK_FILE: &str = "task.md";

/// Helper: create a temp dir with a git repo, .branchdeck/task.md, and an initial commit.
/// Returns (TempDir, task_path_string, epoch_ms that is 1 second AFTER the initial commit).
///
/// Git commit timestamps are second-precision, so we use a backdated initial commit
/// and return a started_at 1 second after it. Test commits made with `make_commit`
/// use the current time, so they'll be captured by `collect_recent_commits`.
fn setup_repo_with_task(task_content: &str) -> (TempDir, String, u64) {
    let dir = TempDir::new().expect("create temp dir");
    let repo = Repository::init(dir.path()).expect("init git repo");

    // Create .branchdeck/task.md
    let bd_dir = dir.path().join(TASK_DIR);
    std::fs::create_dir_all(&bd_dir).unwrap();
    let task_path = bd_dir.join(TASK_FILE);
    std::fs::write(&task_path, task_content).unwrap();

    // Create a dummy file and initial commit with a backdated timestamp
    let dummy = dir.path().join("README.md");
    std::fs::write(&dummy, "# Test repo\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    index.add_path(Path::new(".branchdeck/task.md")).unwrap();
    index.write().unwrap();

    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();

    // Backdate the initial commit by 10 seconds so it's clearly before started_at
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let past_time = git2::Time::new(now_secs - 10, 0);
    let sig = Signature::new("Test", "test@example.com", &past_time).unwrap();

    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .unwrap();

    // started_at is 5 seconds ago — after the initial commit but before new commits
    let started_at_ms = ((now_secs - 5) as u64) * 1000;

    (dir, task_path.to_str().unwrap().to_string(), started_at_ms)
}

/// Helper: make a commit in the repo with the given message.
fn make_commit(repo_path: &Path, filename: &str, message: &str) {
    let repo = Repository::open(repo_path).unwrap();

    // Write a file
    std::fs::write(repo_path.join(filename), message).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new(filename)).unwrap();
    index.write().unwrap();

    let tree_oid = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("Test", "test@example.com").unwrap();
    let head = repo.head().unwrap().peel_to_commit().unwrap();

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
        .unwrap();
}

fn base_task_md(status: &str, run_count: u32) -> String {
    common::task_md_with_body(status, run_count, Some(42), "\n## Instructions\n\nWork on the task.\n\n## Goal\n\nFix the bug.\n")
}

// ─── T2-INT-001: Capture artifacts after successful run with commits ───

#[test]
fn t2_int_001_capture_artifacts_with_commits() {
    let (dir, task_path, started_at) = setup_repo_with_task(&base_task_md("running", 1));

    // Make commits after the start time
    make_commit(dir.path(), "fix1.rs", "fix: first fix");
    make_commit(dir.path(), "fix2.rs", "fix: second fix");

    task::capture_run_artifacts(&task_path, "succeeded", started_at);

    let content = std::fs::read_to_string(&task_path).unwrap();

    assert!(
        content.contains("## Artifacts"),
        "Should have Artifacts section"
    );
    assert!(
        content.contains("### Run 1"),
        "Should have Run 1 header, got:\n{content}"
    );
    assert!(
        content.contains("succeeded"),
        "Should contain status 'succeeded'"
    );
    assert!(
        content.contains("**Branch:**"),
        "Should contain branch info"
    );
    assert!(
        content.contains("**HEAD:**"),
        "Should contain HEAD SHA"
    );
    assert!(
        content.contains("**PR:** #42"),
        "Should contain PR number"
    );
    assert!(
        content.contains("**Commits:** 2"),
        "Should show 2 commits, got:\n{content}"
    );
    assert!(
        content.contains("first fix"),
        "Should contain commit message"
    );
    assert!(
        content.contains("second fix"),
        "Should contain commit message"
    );
}

// ─── T2-INT-002: Capture artifacts after failed run (partial commits) ───

#[test]
fn t2_int_002_capture_artifacts_failed_run() {
    let (dir, task_path, started_at) = setup_repo_with_task(&base_task_md("running", 1));

    // Only one commit before failure
    make_commit(dir.path(), "partial.rs", "wip: partial work");

    task::capture_run_artifacts(&task_path, "failed", started_at);

    let content = std::fs::read_to_string(&task_path).unwrap();

    assert!(
        content.contains("## Artifacts"),
        "Should have Artifacts section"
    );
    assert!(
        content.contains("### Run 1 — failed"),
        "Should show failed status"
    );
    assert!(
        content.contains("**Commits:** 1"),
        "Should show 1 commit"
    );
}

// ─── T2-INT-003: Capture with no commits (cancelled immediately) ───

#[test]
fn t2_int_003_capture_artifacts_no_commits() {
    let (_dir, task_path, started_at) = setup_repo_with_task(&base_task_md("running", 1));

    // No commits made — started_at is after the initial commit
    task::capture_run_artifacts(&task_path, "cancelled", started_at);

    let content = std::fs::read_to_string(&task_path).unwrap();

    assert!(
        content.contains("## Artifacts"),
        "Should have Artifacts section"
    );
    assert!(
        content.contains("### Run 1 — cancelled"),
        "Should show cancelled status"
    );
    assert!(
        content.contains("**Commits:** none"),
        "Should show no commits"
    );
}

// ─── T2-INT-004: Second run appends to existing Artifacts section ───

#[test]
fn t2_int_004_second_run_appends_artifacts() {
    let task_content = format!(
        "{}\n## Artifacts\n\n### Run 1 — succeeded\n\n- **Branch:** `main`\n- **HEAD:** `abc1234`\n- **Commits:** none\n",
        base_task_md("running", 2)
    );
    let (dir, task_path, started_at) = setup_repo_with_task(&task_content);

    make_commit(dir.path(), "new.rs", "feat: new feature");

    task::capture_run_artifacts(&task_path, "succeeded", started_at);

    let content = std::fs::read_to_string(&task_path).unwrap();

    assert!(
        content.contains("### Run 1 — succeeded"),
        "Should preserve first run block"
    );
    assert!(
        content.contains("### Run 2 — succeeded"),
        "Should have second run block, got:\n{content}"
    );

    // Verify order: Run 1 before Run 2
    let run1_pos = content.find("### Run 1").unwrap();
    let run2_pos = content.find("### Run 2").unwrap();
    assert!(
        run1_pos < run2_pos,
        "Run 1 should appear before Run 2"
    );
}

// ─── T2-INT-005: Capture when worktree is missing/deleted ───

#[test]
fn t2_int_005_capture_artifacts_missing_worktree() {
    // Create a task.md that points to a nonexistent repo
    let dir = TempDir::new().unwrap();
    let bd_dir = dir.path().join(TASK_DIR);
    std::fs::create_dir_all(&bd_dir).unwrap();
    let task_path = bd_dir.join(TASK_FILE);
    std::fs::write(&task_path, base_task_md("running", 1)).unwrap();

    // No git repo exists at dir.path() (only mkdir, no git init)
    // capture_run_artifacts should log error but not panic
    task::capture_run_artifacts(task_path.to_str().unwrap(), "failed", 1000);

    // The function should not modify task.md (it can't read git state)
    let content = std::fs::read_to_string(&task_path).unwrap();
    assert!(
        !content.contains("## Artifacts"),
        "Should not add artifacts when repo is missing"
    );
}

// ─── T2-INT-006: Capture when task.md write fails (read-only dir) ───

#[test]
fn t2_int_006_capture_artifacts_readonly_task() {
    let (_dir, task_path, started_at) = setup_repo_with_task(&base_task_md("running", 1));

    // Make the task file read-only
    let mut perms = std::fs::metadata(&task_path).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    {
        perms.set_readonly(true);
    }
    std::fs::set_permissions(&task_path, perms.clone()).unwrap();

    // Should not panic — logs error and returns
    task::capture_run_artifacts(&task_path, "succeeded", started_at);

    // Restore permissions for cleanup
    #[allow(clippy::permissions_set_readonly_false)]
    {
        perms.set_readonly(false);
    }
    std::fs::set_permissions(&task_path, perms).unwrap();

    // File should be unchanged (write failed)
    let content = std::fs::read_to_string(&task_path).unwrap();
    assert!(
        !content.contains("## Artifacts"),
        "Should not modify file when write fails"
    );
}
