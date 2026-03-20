//! P0 unit tests for task.md parsing and frontmatter manipulation.
//!
//! Tests T1-UNIT-001 through T1-UNIT-006 from test-design-phase1.md.
//! TDD red phase: these tests define the expected contract for task parsing.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use branchdeck_lib::models::task::{TaskScope, TaskStatus, TaskType};
use branchdeck_lib::services::task;

// ─── T1-UNIT-001: Parse valid task.md with all frontmatter fields ───

#[test]
fn t1_unit_001_parse_valid_task_md_all_fields() {
    let content = common::task_md_with_body("created", 0, Some(42), "\n## Instructions\n\nAs you work, update this file.\n\n## Goal\n\nFix the login bug.\n\n## Progress\n\n- [ ] Identify approach\n- [ ] Implement\n- [ ] Verify\n\n## Log\n");
    let result = task::parse_task_md(&content, "/fake/path/task.md");

    assert!(result.is_ok(), "Should parse valid task.md without error");
    let info = result.unwrap();

    assert_eq!(info.frontmatter.task_type, TaskType::IssueFix);
    assert_eq!(info.frontmatter.scope, TaskScope::Worktree);
    assert_eq!(info.frontmatter.status, TaskStatus::Created);
    assert_eq!(info.frontmatter.repo, "owner/repo");
    assert_eq!(info.frontmatter.branch, "fix/bug-123");
    assert_eq!(info.frontmatter.pr, Some(42));
    assert_eq!(info.frontmatter.run_count, 0);
    assert_eq!(info.path, "/fake/path/task.md");
    assert!(
        info.body.contains("## Goal"),
        "Body should contain markdown content"
    );
}

// ─── T1-UNIT-002: Parse task.md with missing optional fields ───

#[test]
fn t1_unit_002_parse_task_md_missing_optional_fields() {
    let content = "\
---
type: pr-shepherd
scope: workspace
status: running
repo: owner/repo
branch: feat/new-thing
created: 2026-03-20T10:00:00+00:00
run-count: 3
---

Minimal body.
";

    let result = task::parse_task_md(content, "/fake/path/task.md");

    assert!(
        result.is_ok(),
        "Should parse task.md without optional pr field"
    );
    let info = result.unwrap();

    assert_eq!(info.frontmatter.task_type, TaskType::PrShepherd);
    assert_eq!(info.frontmatter.scope, TaskScope::Workspace);
    assert_eq!(info.frontmatter.status, TaskStatus::Running);
    assert_eq!(info.frontmatter.pr, None, "pr should default to None");
    assert_eq!(info.frontmatter.run_count, 3);
}

// ─── T1-UNIT-003: Parse task.md with malformed YAML ───

#[test]
fn t1_unit_003a_parse_task_md_missing_frontmatter_delimiters() {
    let content = "type: issue-fix\nstatus: created\nNo frontmatter delimiters here.\n";

    let result = task::parse_task_md(content, "/fake/path/task.md");

    assert!(
        result.is_err(),
        "Should return error for missing --- delimiters"
    );
}

#[test]
fn t1_unit_003b_parse_task_md_broken_yaml_indent() {
    let content = "\
---
type: issue-fix
  scope: worktree
    status: created
repo: owner/repo
branch: fix/bug
created: 2026-03-20T10:00:00+00:00
run-count: 0
---

Body text.
";

    let result = task::parse_task_md(content, "/fake/path/task.md");

    assert!(
        result.is_err(),
        "Should return error for broken YAML indentation"
    );
}

#[test]
fn t1_unit_003c_parse_task_md_missing_required_field() {
    // Missing 'repo' which is required
    let content = "\
---
type: issue-fix
scope: worktree
status: created
branch: fix/bug
created: 2026-03-20T10:00:00+00:00
run-count: 0
---

Body text.
";

    let result = task::parse_task_md(content, "/fake/path/task.md");

    assert!(
        result.is_err(),
        "Should return error when required field 'repo' is missing"
    );
}

#[test]
fn t1_unit_003d_parse_task_md_invalid_enum_value() {
    let content = "\
---
type: invalid-type
scope: worktree
status: created
repo: owner/repo
branch: fix/bug
created: 2026-03-20T10:00:00+00:00
run-count: 0
---

Body text.
";

    let result = task::parse_task_md(content, "/fake/path/task.md");

    assert!(
        result.is_err(),
        "Should return error for invalid task type enum value"
    );
}

// ─── T1-UNIT-004: Parse task.md with empty body ───

#[test]
fn t1_unit_004_parse_task_md_empty_body() {
    let content = "\
---
type: issue-fix
scope: worktree
status: created
repo: owner/repo
branch: fix/bug
created: 2026-03-20T10:00:00+00:00
run-count: 0
---
";

    let result = task::parse_task_md(content, "/fake/path/task.md");

    assert!(result.is_ok(), "Should succeed with empty body");
    let info = result.unwrap();
    assert!(
        info.body.trim().is_empty(),
        "Body should be empty or whitespace, got: {:?}",
        info.body
    );
}

// ─── T1-UNIT-005: update_task_status writes correct status to file ───

#[test]
fn t1_unit_005_update_task_status_writes_correctly() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let file_path = dir.path().join("task.md");

    // Write a task.md with status: created
    std::fs::write(&file_path, common::valid_task_md()).unwrap();

    let path_str = file_path.to_str().unwrap();

    // Update status to running
    task::update_task_status(path_str, TaskStatus::Running);

    // Read back and verify
    let updated = std::fs::read_to_string(&file_path).unwrap();
    let result = task::parse_task_md(&updated, path_str);
    assert!(result.is_ok(), "Should still parse after status update");
    assert_eq!(
        result.unwrap().frontmatter.status,
        TaskStatus::Running,
        "Status should be updated to running"
    );

    // Update again to succeeded
    task::update_task_status(path_str, TaskStatus::Succeeded);

    let updated = std::fs::read_to_string(&file_path).unwrap();
    let result = task::parse_task_md(&updated, path_str);
    assert_eq!(
        result.unwrap().frontmatter.status,
        TaskStatus::Succeeded,
        "Status should be updated to succeeded"
    );
}

// ─── T1-UNIT-006: increment_run_count increments by exactly 1 ───

#[test]
fn t1_unit_006_increment_run_count() {
    let dir = tempfile::tempdir().expect("create temp dir");
    let file_path = dir.path().join("task.md");

    // Write a task.md with run-count: 0
    std::fs::write(&file_path, common::valid_task_md()).unwrap();

    let path_str = file_path.to_str().unwrap();

    // Increment once
    task::increment_run_count(path_str);

    let updated = std::fs::read_to_string(&file_path).unwrap();
    let result = task::parse_task_md(&updated, path_str);
    assert!(result.is_ok(), "Should still parse after increment");
    assert_eq!(
        result.unwrap().frontmatter.run_count,
        1,
        "run-count should be 1 after first increment"
    );

    // Increment again
    task::increment_run_count(path_str);

    let updated = std::fs::read_to_string(&file_path).unwrap();
    let result = task::parse_task_md(&updated, path_str);
    assert_eq!(
        result.unwrap().frontmatter.run_count,
        2,
        "run-count should be 2 after second increment"
    );
}

// ─── T1-UNIT-005b: replace_frontmatter_status edge cases ───

#[test]
fn t1_unit_005b_replace_frontmatter_status_preserves_body() {
    let content = common::task_md_with_body("created", 0, Some(42), "\n## Goal\n\nFix the login bug.\n");
    let result = task::replace_frontmatter_status(&content, "failed");

    assert!(result.is_some(), "Should find and replace status field");
    let updated = result.unwrap();

    assert!(
        updated.contains("status: failed"),
        "Should contain new status"
    );
    assert!(
        updated.contains("## Goal"),
        "Should preserve body content"
    );
    assert!(
        updated.contains("repo: owner/repo"),
        "Should preserve other frontmatter fields"
    );
}

#[test]
fn t1_unit_005c_replace_frontmatter_status_no_frontmatter() {
    let content = "Just plain text, no frontmatter.";
    let result = task::replace_frontmatter_status(content, "running");

    assert!(
        result.is_none(),
        "Should return None when no frontmatter present"
    );
}

#[test]
fn t1_unit_005d_replace_frontmatter_status_no_status_field() {
    let content = "\
---
type: issue-fix
repo: owner/repo
---

Body.
";
    let result = task::replace_frontmatter_status(content, "running");

    assert!(
        result.is_none(),
        "Should return None when status field is missing from frontmatter"
    );
}
