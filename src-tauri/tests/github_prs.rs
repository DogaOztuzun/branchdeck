//! Integration tests for the github service — pure/local functions only.
//!
//! Tests `resolve_owner_repo` and `parse_github_remote` with real git repos.
//! Does NOT test API-dependent functions (`list_open_prs`, `get_pr_for_branch`).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use branchdeck_core::services::github;
use git2::Repository;
use std::path::Path;
use tempfile::TempDir;

/// Helper: create a bare-minimum git repo with an initial commit.
fn init_repo_with_commit(dir: &Path) -> Repository {
    let repo = Repository::init(dir).expect("init repo");
    // Create an initial commit so HEAD exists
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

// ─── resolve_owner_repo with SSH remote ───

#[test]
fn resolve_owner_repo_ssh_remote() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo_with_commit(dir.path());
    repo.remote("origin", "git@github.com:myowner/myrepo.git")
        .unwrap();

    let (owner, name) = github::resolve_owner_repo(dir.path()).unwrap();
    assert_eq!(owner, "myowner");
    assert_eq!(name, "myrepo");
}

// ─── resolve_owner_repo with HTTPS remote ───

#[test]
fn resolve_owner_repo_https_remote() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo_with_commit(dir.path());
    repo.remote("origin", "https://github.com/acme/widgets.git")
        .unwrap();

    let (owner, name) = github::resolve_owner_repo(dir.path()).unwrap();
    assert_eq!(owner, "acme");
    assert_eq!(name, "widgets");
}

// ─── resolve_owner_repo with HTTPS remote (no .git suffix) ───

#[test]
fn resolve_owner_repo_https_no_suffix() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo_with_commit(dir.path());
    repo.remote("origin", "https://github.com/acme/widgets")
        .unwrap();

    let (owner, name) = github::resolve_owner_repo(dir.path()).unwrap();
    assert_eq!(owner, "acme");
    assert_eq!(name, "widgets");
}

// ─── resolve_owner_repo with no remote → error ───

#[test]
fn resolve_owner_repo_no_remote_returns_error() {
    let dir = TempDir::new().unwrap();
    let _repo = init_repo_with_commit(dir.path());
    // No remote added

    let result = github::resolve_owner_repo(dir.path());
    assert!(result.is_err(), "Should fail when no origin remote exists");
}

// ─── resolve_owner_repo with non-github remote → error ───

#[test]
fn resolve_owner_repo_non_github_remote_returns_error() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo_with_commit(dir.path());
    repo.remote("origin", "https://gitlab.com/owner/repo.git")
        .unwrap();

    let result = github::resolve_owner_repo(dir.path());
    assert!(result.is_err(), "Should fail for non-GitHub remotes");
}

// ─── resolve_owner_repo with nonexistent path → error ───

#[test]
fn resolve_owner_repo_nonexistent_path_returns_error() {
    let result = github::resolve_owner_repo(Path::new("/tmp/nonexistent-repo-path-xyz"));
    assert!(result.is_err(), "Should fail for nonexistent repo path");
}

// ─── parse_github_remote covers edge cases ───

#[test]
fn parse_github_remote_ssh_no_suffix() {
    let result = github::parse_github_remote("git@github.com:owner/repo");
    assert_eq!(result, Some(("owner".to_string(), "repo".to_string())));
}

#[test]
fn parse_github_remote_empty_string() {
    let result = github::parse_github_remote("");
    assert_eq!(result, None);
}
