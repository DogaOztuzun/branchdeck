//! Integration tests for the git service (`services::git`).
//!
//! Tests T6-INT-* from test-design-phase1.md plus additional coverage
//! for validate_repo, list_worktrees, create_worktree, remove_worktree,
//! list_branches, get_status, and sanitize_worktree_name edge cases.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::doc_markdown,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

mod common;

use branchdeck_core::services::git;
use git2::{Repository, Signature};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ═══════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════

/// Create a temp dir with a git repo and an initial commit so HEAD exists.
///
/// The repo is created inside `<tempdir>/repo/` so that `repo_path.parent()`
/// resolves to the temp dir itself, avoiding worktree path collisions between
/// tests (`create_worktree` places worktrees at `parent/worktrees/<name>`).
///
/// Returns `(TempDir, path_to_repo)`. Keep `TempDir` alive for the test duration.
fn setup_git_repo() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("create temp dir");
    let rp = dir.path().join("repo");
    std::fs::create_dir_all(&rp).unwrap();
    let repo = Repository::init(&rp).expect("init git repo");

    // Create a dummy file
    std::fs::write(rp.join("README.md"), "# Test repo\n").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    index.write().unwrap();

    let tree_oid = index.write_tree().unwrap();
    {
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
    }

    (dir, rp)
}

/// Make a commit in an existing repo with a new file.
fn make_commit(repo_path: &Path, filename: &str, message: &str) {
    let repo = Repository::open(repo_path).unwrap();

    std::fs::write(repo_path.join(filename), message).unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(Path::new(filename)).unwrap();
    index.write().unwrap();

    let tree_oid = index.write_tree().unwrap();
    {
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
            .unwrap();
    }
}

// ═══════════════════════════════════════════════════════════════════════
// sanitize_worktree_name — pure function, many edge cases
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sanitize_basic_alphanumeric() {
    assert_eq!(git::sanitize_worktree_name("feature-123"), "feature-123");
}

#[test]
fn sanitize_uppercase_to_lowercase() {
    assert_eq!(git::sanitize_worktree_name("Feature-ABC"), "feature-abc");
}

#[test]
fn sanitize_spaces_to_dashes() {
    assert_eq!(
        git::sanitize_worktree_name("my cool feature"),
        "my-cool-feature"
    );
}

#[test]
fn sanitize_special_chars_to_dashes() {
    assert_eq!(
        git::sanitize_worktree_name("feat/add@thing!"),
        "feat-add-thing"
    );
}

#[test]
fn sanitize_consecutive_dashes_collapsed() {
    assert_eq!(git::sanitize_worktree_name("feat---name"), "feat-name");
}

#[test]
fn sanitize_leading_trailing_dashes_trimmed() {
    assert_eq!(git::sanitize_worktree_name("--name--"), "name");
}

#[test]
fn sanitize_leading_trailing_underscores_trimmed() {
    assert_eq!(git::sanitize_worktree_name("__name__"), "name");
}

#[test]
fn sanitize_mixed_leading_trailing() {
    assert_eq!(git::sanitize_worktree_name("-_name_-"), "name");
}

#[test]
fn sanitize_preserves_underscores_in_middle() {
    assert_eq!(
        git::sanitize_worktree_name("my_feature_name"),
        "my_feature_name"
    );
}

#[test]
fn sanitize_truncates_to_80_chars() {
    let long_name = "a".repeat(100);
    let result = git::sanitize_worktree_name(&long_name);
    assert!(result.len() <= 80, "Length is {}", result.len());
    assert_eq!(result.len(), 80);
}

#[test]
fn sanitize_truncate_trims_trailing_dash() {
    // 80th char is a dash — should be trimmed
    let mut name = "a".repeat(79);
    name.push('/'); // becomes dash at position 79 (0-indexed)
    name.push_str("bbb"); // past truncation point
    let result = git::sanitize_worktree_name(&name);
    assert!(result.len() <= 80);
    assert!(!result.ends_with('-'), "Should not end with dash: {result}");
}

#[test]
fn sanitize_rejects_head() {
    assert_eq!(git::sanitize_worktree_name("HEAD"), "");
}

#[test]
fn sanitize_rejects_head_case_insensitive() {
    assert_eq!(git::sanitize_worktree_name("Head"), "");
    assert_eq!(git::sanitize_worktree_name("head"), "");
    assert_eq!(git::sanitize_worktree_name("hEaD"), "");
}

#[test]
fn sanitize_empty_input() {
    assert_eq!(git::sanitize_worktree_name(""), "");
}

#[test]
fn sanitize_only_special_chars() {
    // All chars become dashes, then trimmed
    assert_eq!(git::sanitize_worktree_name("///"), "");
}

#[test]
fn sanitize_single_valid_char() {
    assert_eq!(git::sanitize_worktree_name("a"), "a");
}

#[test]
fn sanitize_numbers_only() {
    assert_eq!(git::sanitize_worktree_name("123"), "123");
}

#[test]
fn sanitize_unicode_replaced() {
    assert_eq!(
        git::sanitize_worktree_name("feat-emoji-\u{1F600}"),
        "feat-emoji"
    );
}

#[test]
fn sanitize_mixed_special_and_collapse() {
    assert_eq!(
        git::sanitize_worktree_name("feat//add..thing"),
        "feat-add-thing"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// validate_repo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn validate_repo_valid_path() {
    let (_dir, rp) = setup_git_repo();

    let info = git::validate_repo(&rp).unwrap();

    assert!(!info.name.is_empty(), "Repo name should not be empty");
    assert_eq!(info.path, rp);
    assert!(!info.current_branch.is_empty());
}

#[test]
fn validate_repo_bare_repo_rejected() {
    let dir = TempDir::new().unwrap();
    Repository::init_bare(dir.path()).unwrap();

    let result = git::validate_repo(dir.path());
    assert!(result.is_err(), "Bare repo should be rejected");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Bare repository"),
        "Error should mention bare: {err_msg}"
    );
}

#[test]
fn validate_repo_non_repo_path() {
    let dir = TempDir::new().unwrap();

    let result = git::validate_repo(dir.path());
    assert!(result.is_err(), "Non-repo path should fail");
}

#[test]
fn validate_repo_discovers_from_subdirectory() {
    let (_dir, rp) = setup_git_repo();

    let subdir = rp.join("sub");
    std::fs::create_dir_all(&subdir).unwrap();

    let info = git::validate_repo(&subdir).unwrap();
    assert_eq!(info.path, rp);
}

// ═══════════════════════════════════════════════════════════════════════
// list_worktrees
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn list_worktrees_main_only() {
    let (_dir, rp) = setup_git_repo();

    let worktrees = git::list_worktrees(&rp).unwrap();

    assert_eq!(worktrees.len(), 1, "Should have exactly 1 (main) worktree");
    assert!(worktrees[0].is_main, "First worktree should be main");
    assert_eq!(worktrees[0].path, rp);
}

#[test]
fn list_worktrees_with_linked_worktree() {
    let (_dir, rp) = setup_git_repo();

    git::create_worktree(&rp, "feature-one", None, None).unwrap();

    let worktrees = git::list_worktrees(&rp).unwrap();

    assert_eq!(worktrees.len(), 2, "Should have main + 1 linked worktree");

    let main_wt = worktrees.iter().find(|w| w.is_main).unwrap();
    let linked_wt = worktrees.iter().find(|w| !w.is_main).unwrap();

    assert_eq!(main_wt.path, rp);
    assert_eq!(linked_wt.name, "feature-one");
    assert_eq!(linked_wt.branch, "feature-one");
    assert!(!linked_wt.is_main);
}

#[test]
fn list_worktrees_multiple_linked() {
    let (_dir, rp) = setup_git_repo();

    git::create_worktree(&rp, "feature-a", None, None).unwrap();
    git::create_worktree(&rp, "feature-b", None, None).unwrap();

    let worktrees = git::list_worktrees(&rp).unwrap();

    assert_eq!(worktrees.len(), 3, "Should have main + 2 linked worktrees");

    let linked: Vec<_> = worktrees.iter().filter(|w| !w.is_main).collect();
    assert_eq!(linked.len(), 2);

    let names: Vec<&str> = linked.iter().map(|w| w.name.as_str()).collect();
    assert!(names.contains(&"feature-a"));
    assert!(names.contains(&"feature-b"));
}

// ═══════════════════════════════════════════════════════════════════════
// create_worktree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn create_worktree_basic() {
    let (_dir, rp) = setup_git_repo();

    let wt = git::create_worktree(&rp, "basic-wt-test", None, None).unwrap();

    assert_eq!(wt.name, "basic-wt-test");
    assert_eq!(wt.branch, "basic-wt-test");
    assert!(!wt.is_main);
    assert!(wt.path.exists(), "Worktree directory should exist");
}

#[test]
fn create_worktree_custom_branch_name() {
    let (_dir, rp) = setup_git_repo();

    let wt = git::create_worktree(&rp, "my-wt", Some("custom-branch"), None).unwrap();

    assert_eq!(wt.name, "my-wt");
    assert_eq!(wt.branch, "custom-branch");
}

#[test]
fn create_worktree_sanitizes_name() {
    let (_dir, rp) = setup_git_repo();

    let wt = git::create_worktree(&rp, "My Feature!!!", None, None).unwrap();

    assert_eq!(wt.name, "my-feature");
    assert_eq!(wt.branch, "my-feature");
}

#[test]
fn create_worktree_reuses_existing_branch() {
    let (_dir, rp) = setup_git_repo();

    // Create a branch manually first
    {
        let repo = Repository::open(&rp).unwrap();
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("reuse-me", &head_commit, false).unwrap();
    }

    let wt = git::create_worktree(&rp, "reuse-me", None, None).unwrap();

    assert_eq!(wt.branch, "reuse-me");
    assert!(wt.path.exists());
}

#[test]
fn create_worktree_with_base_branch() {
    let (_dir, rp) = setup_git_repo();

    make_commit(&rp, "base.txt", "base commit");
    {
        let repo = Repository::open(&rp).unwrap();
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("develop", &head_commit, false).unwrap();
    }

    let wt = git::create_worktree(&rp, "from-develop", None, Some("develop")).unwrap();

    assert_eq!(wt.name, "from-develop");
    assert!(wt.path.exists());
}

#[test]
fn create_worktree_empty_name_after_sanitize_fails() {
    let (_dir, rp) = setup_git_repo();

    let result = git::create_worktree(&rp, "///", None, None);
    assert!(result.is_err(), "Empty sanitized name should fail");
}

#[test]
fn create_worktree_head_name_fails() {
    let (_dir, rp) = setup_git_repo();

    let result = git::create_worktree(&rp, "HEAD", None, None);
    assert!(result.is_err(), "HEAD name should be rejected");
}

// ═══════════════════════════════════════════════════════════════════════
// remove_worktree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn remove_worktree_basic() {
    let (_dir, rp) = setup_git_repo();

    let wt = git::create_worktree(&rp, "to-remove", None, None).unwrap();
    assert!(wt.path.exists());

    git::remove_worktree(&rp, "to-remove", false).unwrap();

    assert!(!wt.path.exists(), "Worktree dir should be removed");

    // Branch should still exist (delete_branch = false)
    let repo = Repository::open(&rp).unwrap();
    assert!(
        repo.find_branch("to-remove", git2::BranchType::Local)
            .is_ok(),
        "Branch should still exist when delete_branch=false"
    );
}

#[test]
fn remove_worktree_with_branch_deletion() {
    let (_dir, rp) = setup_git_repo();

    let wt = git::create_worktree(&rp, "to-delete", None, None).unwrap();
    assert!(wt.path.exists());

    git::remove_worktree(&rp, "to-delete", true).unwrap();

    assert!(!wt.path.exists(), "Worktree dir should be removed");

    let repo = Repository::open(&rp).unwrap();
    assert!(
        repo.find_branch("to-delete", git2::BranchType::Local)
            .is_err(),
        "Branch should be deleted when delete_branch=true"
    );
}

#[test]
fn remove_worktree_updates_list() {
    let (_dir, rp) = setup_git_repo();

    git::create_worktree(&rp, "ephemeral", None, None).unwrap();
    assert_eq!(git::list_worktrees(&rp).unwrap().len(), 2);

    git::remove_worktree(&rp, "ephemeral", true).unwrap();
    assert_eq!(git::list_worktrees(&rp).unwrap().len(), 1);
}

// ═══════════════════════════════════════════════════════════════════════
// list_branches
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn list_branches_initial_repo() {
    let (_dir, rp) = setup_git_repo();

    let branches = git::list_branches(&rp).unwrap();

    assert!(!branches.is_empty(), "Should have at least one branch");

    let local_branches: Vec<_> = branches.iter().filter(|b| !b.is_remote).collect();
    assert!(!local_branches.is_empty());
    assert!(
        local_branches.iter().any(|b| b.is_head),
        "One branch should be HEAD"
    );
}

#[test]
fn list_branches_multiple_local() {
    let (_dir, rp) = setup_git_repo();

    {
        let repo = Repository::open(&rp).unwrap();
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature-x", &head_commit, false).unwrap();
        repo.branch("feature-y", &head_commit, false).unwrap();
    }

    let branches = git::list_branches(&rp).unwrap();
    let local_branches: Vec<_> = branches.iter().filter(|b| !b.is_remote).collect();

    assert!(
        local_branches.len() >= 3,
        "Should have default + 2 feature branches"
    );

    let names: Vec<&str> = local_branches.iter().map(|b| b.name.as_str()).collect();
    assert!(names.contains(&"feature-x"));
    assert!(names.contains(&"feature-y"));
}

#[test]
fn list_branches_sorted_locals_first() {
    let (_dir, rp) = setup_git_repo();

    {
        let repo = Repository::open(&rp).unwrap();
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("zzz-branch", &head_commit, false).unwrap();
        repo.branch("aaa-branch", &head_commit, false).unwrap();
    }

    let branches = git::list_branches(&rp).unwrap();
    let local_branches: Vec<_> = branches.iter().filter(|b| !b.is_remote).collect();

    for window in local_branches.windows(2) {
        assert!(
            window[0].name <= window[1].name,
            "Branches should be sorted alphabetically: {} vs {}",
            window[0].name,
            window[1].name
        );
    }
}

#[test]
fn list_branches_worktree_flag() {
    let (_dir, rp) = setup_git_repo();

    git::create_worktree(&rp, "wt-branch", None, None).unwrap();

    let branches = git::list_branches(&rp).unwrap();

    let wt_branch = branches.iter().find(|b| b.name == "wt-branch");
    assert!(wt_branch.is_some(), "Should find the worktree branch");
    assert!(
        wt_branch.unwrap().has_worktree,
        "Branch should be marked as having a worktree"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// get_status
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn get_status_clean_repo() {
    let (_dir, rp) = setup_git_repo();

    let statuses = git::get_status(&rp).unwrap();
    assert!(
        statuses.is_empty(),
        "Clean repo should have no status entries"
    );
}

#[test]
fn get_status_modified_file() {
    let (_dir, rp) = setup_git_repo();

    std::fs::write(rp.join("README.md"), "# Modified\n").unwrap();

    let statuses = git::get_status(&rp).unwrap();

    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].path, "README.md");
    assert_eq!(statuses[0].status, "modified");
}

#[test]
fn get_status_untracked_file() {
    let (_dir, rp) = setup_git_repo();

    std::fs::write(rp.join("new_file.txt"), "hello").unwrap();

    let statuses = git::get_status(&rp).unwrap();

    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].path, "new_file.txt");
    assert_eq!(statuses[0].status, "new");
}

#[test]
fn get_status_multiple_changes() {
    let (_dir, rp) = setup_git_repo();

    std::fs::write(rp.join("README.md"), "# Modified\n").unwrap();
    std::fs::write(rp.join("added.txt"), "new content").unwrap();

    let statuses = git::get_status(&rp).unwrap();

    assert_eq!(statuses.len(), 2);

    let paths: Vec<&str> = statuses.iter().map(|s| s.path.as_str()).collect();
    assert!(paths.contains(&"README.md"));
    assert!(paths.contains(&"added.txt"));
}

#[test]
fn get_status_staged_file() {
    let (_dir, rp) = setup_git_repo();

    std::fs::write(rp.join("staged.txt"), "staged content").unwrap();
    {
        let repo = Repository::open(&rp).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("staged.txt")).unwrap();
        index.write().unwrap();
    }

    let statuses = git::get_status(&rp).unwrap();

    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].path, "staged.txt");
    assert_eq!(statuses[0].status, "new");
}

#[test]
fn get_status_deleted_file() {
    let (_dir, rp) = setup_git_repo();

    std::fs::remove_file(rp.join("README.md")).unwrap();

    let statuses = git::get_status(&rp).unwrap();

    assert_eq!(statuses.len(), 1);
    assert_eq!(statuses[0].path, "README.md");
    assert_eq!(statuses[0].status, "deleted");
}

// ═══════════════════════════════════════════════════════════════════════
// preview_worktree
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn preview_worktree_new_name() {
    let (_dir, rp) = setup_git_repo();

    let preview = git::preview_worktree(&rp, "new-feature").unwrap();

    assert_eq!(preview.sanitized_name, "new-feature");
    assert_eq!(preview.branch_name, "new-feature");
    assert!(!preview.branch_exists);
    assert!(!preview.path_exists);
    assert!(!preview.worktree_exists);
}

#[test]
fn preview_worktree_existing_branch() {
    let (_dir, rp) = setup_git_repo();

    {
        let repo = Repository::open(&rp).unwrap();
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("existing", &head_commit, false).unwrap();
    }

    let preview = git::preview_worktree(&rp, "existing").unwrap();

    assert!(preview.branch_exists, "Should detect existing branch");
    assert!(!preview.worktree_exists);
}

#[test]
fn preview_worktree_existing_worktree() {
    let (_dir, rp) = setup_git_repo();

    git::create_worktree(&rp, "active-wt", None, None).unwrap();

    let preview = git::preview_worktree(&rp, "active-wt").unwrap();

    assert!(preview.branch_exists);
    assert!(preview.worktree_exists);
    assert!(preview.path_exists);
}

// ═══════════════════════════════════════════════════════════════════════
// get_branch_tracking (limited — no remote available in test)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn get_branch_tracking_no_upstream() {
    let (_dir, rp) = setup_git_repo();

    let head_name = {
        let repo = Repository::open(&rp).unwrap();
        let name = repo.head().unwrap().shorthand().unwrap().to_string();
        name
    };

    let tracking = git::get_branch_tracking(&rp, &head_name).unwrap();
    assert!(
        tracking.is_none(),
        "Local-only branch should have no tracking info"
    );
}

#[test]
fn get_branch_tracking_nonexistent_branch() {
    let (_dir, rp) = setup_git_repo();

    let tracking = git::get_branch_tracking(&rp, "nonexistent").unwrap();
    assert!(tracking.is_none(), "Nonexistent branch should return None");
}
