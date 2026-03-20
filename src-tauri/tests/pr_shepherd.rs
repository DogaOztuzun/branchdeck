//! Integration tests for the PR shepherd feature.
//!
//! Tests local/pure shepherd logic. Does NOT test functions that require
//! the GitHub API (`shepherd_pr`, `find_pr_by_number`).
//! Focuses on git-local operations: worktree conflict detection,
//! branch availability checks.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use branchdeck_lib::services::git;
use git2::Repository;
use std::path::Path;
use tempfile::TempDir;

/// Helper: create a git repo with an initial commit inside a `repo/` subdirectory
/// of the temp dir, so that `worktrees/` sibling directory stays within the temp dir.
fn init_repo_nested(parent: &Path) -> Repository {
    let repo_dir = parent.join("repo");
    std::fs::create_dir_all(&repo_dir).unwrap();
    let repo = Repository::init(&repo_dir).expect("init repo");
    let sig = repo
        .signature()
        .unwrap_or_else(|_| git2::Signature::now("Test", "test@test.com").unwrap());
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    {
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
    }
    repo
}

/// Get the repo path from inside the nested structure.
fn repo_path(dir: &TempDir) -> std::path::PathBuf {
    dir.path().join("repo")
}

// ─── Worktree already exists for branch → shepherd should detect conflict ───

#[test]
fn worktree_conflict_detected_for_existing_branch() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo_nested(dir.path());
    let rp = repo_path(&dir);

    // Create a branch
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("fix/ci-failure", &head, false).unwrap();

    // Create a worktree for that branch
    let _wt = git::create_worktree(&rp, "fix/ci-failure", Some("fix/ci-failure"), None)
        .expect("create worktree");

    // List worktrees and check if the branch is taken
    let worktrees = git::list_worktrees(&rp).expect("list worktrees");
    let branch_taken = worktrees.iter().any(|wt| wt.branch == "fix/ci-failure");
    assert!(
        branch_taken,
        "Worktree list should show the branch as already checked out"
    );
}

// ─── No worktree for branch → branch is available ───

#[test]
fn no_worktree_conflict_for_unused_branch() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo_nested(dir.path());
    let rp = repo_path(&dir);

    // Create a branch but do NOT create a worktree
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("feat/new-feature", &head, false).unwrap();

    let worktrees = git::list_worktrees(&rp).expect("list worktrees");
    let branch_taken = worktrees.iter().any(|wt| wt.branch == "feat/new-feature");
    assert!(
        !branch_taken,
        "Branch without a worktree should not show as conflict"
    );
}

// ─── Multiple worktrees → only the matching branch is a conflict ───

#[test]
fn worktree_conflict_is_branch_specific() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo_nested(dir.path());
    let rp = repo_path(&dir);

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("fix/a", &head, false).unwrap();
    repo.branch("fix/b", &head, false).unwrap();

    // Create worktree only for fix/a
    let _wt = git::create_worktree(&rp, "fix/a", Some("fix/a"), None).expect("create worktree");

    let worktrees = git::list_worktrees(&rp).expect("list worktrees");
    assert!(
        worktrees.iter().any(|wt| wt.branch == "fix/a"),
        "fix/a should be taken"
    );
    assert!(
        !worktrees.iter().any(|wt| wt.branch == "fix/b"),
        "fix/b should still be available"
    );
}

// ─── Creating worktree for already-taken branch fails ───

#[test]
fn creating_duplicate_worktree_for_branch_fails() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo_nested(dir.path());
    let rp = repo_path(&dir);

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch("fix/duplicate-test", &head, false).unwrap();

    // First worktree succeeds
    let _wt = git::create_worktree(&rp, "fix/duplicate-test", Some("fix/duplicate-test"), None)
        .expect("first worktree should succeed");

    // Second worktree for the same branch should fail
    let result = git::create_worktree(
        &rp,
        "fix/duplicate-test-2",
        Some("fix/duplicate-test"),
        None,
    );
    assert!(
        result.is_err(),
        "Creating a second worktree for the same branch should fail"
    );
}
