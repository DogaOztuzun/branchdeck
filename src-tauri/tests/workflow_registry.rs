//! Tests for `WorkflowRegistry` discovery and precedence.
//!
//! Story 1.3: Workflow Registry.
//! Covers: empty dirs, valid discovery, override precedence, mixed valid/invalid, missing directory.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;

use branchdeck_core::services::workflow::WorkflowRegistry;
use tempfile::TempDir;

/// Helper: create a workflow directory with a WORKFLOW.md inside.
fn write_workflow(base: &std::path::Path, dir_name: &str, content: &str) {
    let dir = base.join(dir_name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("WORKFLOW.md"), content).unwrap();
}

const WORKFLOW_A: &str = r"---
name: alpha
description: Alpha workflow
tracker:
  kind: manual
---

Do alpha things.
";

const WORKFLOW_B: &str = r"---
name: beta
description: Beta workflow
tracker:
  kind: github-pr
  filter:
    ci_status: failure
outcomes:
  - name: done
    detect: ci-passing
    next: complete
---

Do beta things.
";

/// Alpha workflow with different description (for override testing).
const WORKFLOW_A_OVERRIDE: &str = r"---
name: alpha
description: Alpha workflow (project-local override)
tracker:
  kind: github-issue
---

Do alpha things locally.
";

const INVALID_WORKFLOW: &str = "---\nname: \"   \"\ntracker:\n  kind: manual\n---\n";

const MALFORMED_WORKFLOW: &str = "This is not a workflow file at all.";

#[test]
fn empty_directories() {
    let tmp = TempDir::new().unwrap();
    let empty_dir = tmp.path().join("empty");
    fs::create_dir_all(&empty_dir).unwrap();

    let registry = WorkflowRegistry::scan(&[empty_dir]);
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(registry.list_workflows().is_empty());
}

#[test]
fn missing_directory_skipped() {
    let nonexistent = PathBuf::from("/tmp/branchdeck-test-nonexistent-dir-abc123");
    let registry = WorkflowRegistry::scan(&[nonexistent]);
    assert!(registry.is_empty());
}

#[test]
fn valid_discovery() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("workflows");
    fs::create_dir_all(&dir).unwrap();

    write_workflow(&dir, "alpha", WORKFLOW_A);
    write_workflow(&dir, "beta", WORKFLOW_B);

    let registry = WorkflowRegistry::scan(&[dir]);
    assert_eq!(registry.len(), 2);

    let alpha = registry.get_workflow("alpha").expect("should find alpha");
    assert_eq!(alpha.config.name, "alpha");
    assert_eq!(alpha.config.description.as_deref(), Some("Alpha workflow"));

    let beta = registry.get_workflow("beta").expect("should find beta");
    assert_eq!(beta.config.name, "beta");

    assert!(registry.get_workflow("nonexistent").is_none());
}

#[test]
fn override_precedence() {
    let tmp = TempDir::new().unwrap();
    let global_dir = tmp.path().join("global");
    let local_dir = tmp.path().join("local");
    fs::create_dir_all(&global_dir).unwrap();
    fs::create_dir_all(&local_dir).unwrap();

    write_workflow(&global_dir, "alpha", WORKFLOW_A);
    write_workflow(&local_dir, "alpha", WORKFLOW_A_OVERRIDE);

    // local is later in the slice, so it should win
    let registry = WorkflowRegistry::scan(&[global_dir, local_dir]);
    assert_eq!(registry.len(), 1);

    let alpha = registry.get_workflow("alpha").expect("should find alpha");
    assert_eq!(
        alpha.config.description.as_deref(),
        Some("Alpha workflow (project-local override)")
    );
}

#[test]
fn mixed_valid_and_invalid() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("workflows");
    fs::create_dir_all(&dir).unwrap();

    write_workflow(&dir, "good-alpha", WORKFLOW_A);
    write_workflow(&dir, "good-beta", WORKFLOW_B);
    write_workflow(&dir, "bad-empty-name", INVALID_WORKFLOW);
    write_workflow(&dir, "bad-malformed", MALFORMED_WORKFLOW);

    let registry = WorkflowRegistry::scan(&[dir]);
    // Only the 2 valid workflows should be loaded
    assert_eq!(registry.len(), 2);
    assert!(registry.get_workflow("alpha").is_some());
    assert!(registry.get_workflow("beta").is_some());
}

#[test]
fn list_workflows_returns_all() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("workflows");
    fs::create_dir_all(&dir).unwrap();

    write_workflow(&dir, "alpha", WORKFLOW_A);
    write_workflow(&dir, "beta", WORKFLOW_B);

    let registry = WorkflowRegistry::scan(&[dir]);
    let all = registry.list_workflows();
    assert_eq!(all.len(), 2);

    let names: Vec<&str> = all.iter().map(|w| w.config.name.as_str()).collect();
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));
}

#[test]
fn subdirectory_without_workflow_md_ignored() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("workflows");

    // Create a subdir with no WORKFLOW.md
    let empty_subdir = dir.join("empty-workflow");
    fs::create_dir_all(&empty_subdir).unwrap();
    fs::write(empty_subdir.join("README.md"), "not a workflow").unwrap();

    // Create a valid one
    write_workflow(&dir, "valid", WORKFLOW_A);

    let registry = WorkflowRegistry::scan(&[dir]);
    assert_eq!(registry.len(), 1);
    assert!(registry.get_workflow("alpha").is_some());
}

#[test]
fn multiple_tiers_merge() {
    let tmp = TempDir::new().unwrap();
    let embedded = tmp.path().join("embedded");
    let global = tmp.path().join("global");
    let local = tmp.path().join("local");
    fs::create_dir_all(&embedded).unwrap();
    fs::create_dir_all(&global).unwrap();
    fs::create_dir_all(&local).unwrap();

    write_workflow(&embedded, "alpha", WORKFLOW_A);
    write_workflow(&global, "beta", WORKFLOW_B);
    write_workflow(&local, "alpha", WORKFLOW_A_OVERRIDE);

    let registry = WorkflowRegistry::scan(&[embedded, global, local]);
    assert_eq!(registry.len(), 2);

    // alpha should be the local override
    let alpha = registry.get_workflow("alpha").unwrap();
    assert_eq!(
        alpha.config.description.as_deref(),
        Some("Alpha workflow (project-local override)")
    );

    // beta should come from global
    let beta = registry.get_workflow("beta").unwrap();
    assert_eq!(beta.config.name, "beta");
}

#[test]
fn duplicate_name_same_tier() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("workflows");
    fs::create_dir_all(&dir).unwrap();

    // Two subdirectories with the same workflow name in YAML
    write_workflow(&dir, "alpha-v1", WORKFLOW_A);
    write_workflow(&dir, "alpha-v2", WORKFLOW_A);

    let registry = WorkflowRegistry::scan(&[dir]);
    // Both have name "alpha" — one wins (filesystem order), registry has exactly 1 entry
    assert_eq!(registry.len(), 1);
    assert!(registry.get_workflow("alpha").is_some());
}
