//! Tests for WorkflowDef schema parsing and validation.
//!
//! Story 1.2: `WorkflowDef` Schema Spec & Model (revised — markdown + YAML frontmatter format).
//! Covers: valid definition, minimal def, missing required fields, unknown enum values,
//! outcome validation, retry validation, round-trip, all tracker kinds.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::uninlined_format_args,
    clippy::doc_markdown,
    clippy::disallowed_methods,
    clippy::needless_raw_string_hashes
)]

use branchdeck_lib::models::workflow::{
    BackoffStrategy, OutcomeAction, OutcomeDetector, TrackerKind,
};
use branchdeck_lib::services::workflow::{parse_workflow_md, validate_workflow_def};

const VALID_WORKFLOW: &str = r#"---
name: pr-shepherd
description: Fix failing CI on pull requests

tracker:
  kind: github-pr
  filter:
    ci_status: failure

polling:
  interval_ms: 5000

hooks:
  after_create: |
    git clone --depth 1 https://github.com/example/repo .
  before_run: echo "starting"

agent:
  max_concurrent_agents: 1
  max_turns: 25
  max_budget_usd: 5.0
  timeout_minutes: 30
  allowed_directories:
    - "."

outcomes:
  - name: fix-pushed
    detect: ci-passing
    next: complete
  - name: analysis-written
    detect: file-exists
    path: .branchdeck/analysis.json
    next: review
  - name: failed
    detect: run-failed
    next: retry

lifecycle:
  dispatched: Analyzing
  complete: Fixed
  failed: Broken
  retrying: Retrying fix

retry:
  max_attempts: 3
  backoff: exponential
  base_delay_ms: 30000
---

You are working on PR #{{ pr.number }} in {{ pr.repo }}.

## Instructions
1. Analyze CI failures
2. Create a fix plan
3. Implement the fix
"#;

const MINIMAL_WORKFLOW: &str = r#"---
name: minimal-test
tracker:
  kind: manual
---

Do the thing.
"#;

#[test]
fn parse_valid_full_definition() {
    let def = parse_workflow_md(VALID_WORKFLOW).expect("should parse valid workflow");
    let c = &def.config;

    assert_eq!(c.name, "pr-shepherd");
    assert_eq!(
        c.description.as_deref(),
        Some("Fix failing CI on pull requests")
    );
    assert_eq!(c.tracker.kind, TrackerKind::GithubPr);
    assert!(c.tracker.filter.is_some());

    let polling = c.polling.as_ref().expect("should have polling");
    assert_eq!(polling.interval_ms, 5000);

    let hooks = c.hooks.as_ref().expect("should have hooks");
    assert!(hooks.after_create.is_some());
    assert!(hooks.before_run.is_some());
    assert!(hooks.after_run.is_none());

    let agent = c.agent.as_ref().expect("should have agent");
    assert_eq!(agent.max_turns, Some(25));
    assert_eq!(agent.max_budget_usd, Some(5.0));
    assert_eq!(agent.timeout_minutes, Some(30));
    assert_eq!(agent.max_concurrent_agents, Some(1));

    assert_eq!(c.outcomes.len(), 3);
    assert_eq!(c.outcomes[0].detect, OutcomeDetector::CiPassing);
    assert_eq!(c.outcomes[0].next, OutcomeAction::Complete);
    assert_eq!(c.outcomes[1].detect, OutcomeDetector::FileExists);
    assert_eq!(
        c.outcomes[1].path.as_deref(),
        Some(".branchdeck/analysis.json")
    );
    assert_eq!(c.outcomes[2].detect, OutcomeDetector::RunFailed);
    assert_eq!(c.outcomes[2].next, OutcomeAction::Retry);

    let lifecycle = c.lifecycle.as_ref().expect("should have lifecycle");
    assert_eq!(lifecycle.dispatched.as_deref(), Some("Analyzing"));
    assert_eq!(lifecycle.complete.as_deref(), Some("Fixed"));
    assert_eq!(lifecycle.failed.as_deref(), Some("Broken"));
    assert_eq!(lifecycle.retrying.as_deref(), Some("Retrying fix"));

    let retry = c.retry.as_ref().expect("should have retry");
    assert_eq!(retry.max_attempts, 3);
    assert_eq!(retry.backoff, BackoffStrategy::Exponential);
    assert_eq!(retry.base_delay_ms, 30_000);

    assert!(def.prompt.contains("You are working on PR"));
    assert!(def.prompt.contains("## Instructions"));

    let errors = validate_workflow_def(&def);
    assert!(
        errors.is_empty(),
        "valid def should have no errors: {errors:?}"
    );
}

#[test]
fn parse_minimal_definition() {
    let def = parse_workflow_md(MINIMAL_WORKFLOW).expect("should parse minimal workflow");

    assert_eq!(def.config.name, "minimal-test");
    assert_eq!(def.config.tracker.kind, TrackerKind::Manual);
    assert!(def.config.description.is_none());
    assert!(def.config.polling.is_none());
    assert!(def.config.hooks.is_none());
    assert!(def.config.agent.is_none());
    assert!(def.config.outcomes.is_empty());
    assert!(def.config.lifecycle.is_none());
    assert!(def.config.retry.is_none());
    assert!(def.prompt.contains("Do the thing."));

    let errors = validate_workflow_def(&def);
    assert!(
        errors.is_empty(),
        "minimal def should have no errors: {errors:?}"
    );
}

#[test]
fn reject_empty_name() {
    let md = r#"---
name: "   "
tracker:
  kind: manual
---
prompt
"#;
    let def = parse_workflow_md(md).expect("should parse");
    let errors = validate_workflow_def(&def);
    assert!(
        errors.iter().any(|e| e.field == "name"),
        "should error on whitespace name: {errors:?}"
    );
}

#[test]
fn reject_unknown_tracker_kind() {
    let md = r#"---
name: test
tracker:
  kind: foobar
---
prompt
"#;
    let err = parse_workflow_md(md).expect_err("should fail on unknown tracker kind");
    let msg = err.to_string();
    assert!(
        msg.contains("foobar"),
        "error should mention the invalid value: {msg}"
    );
}

#[test]
fn reject_unknown_outcome_detector() {
    let md = r#"---
name: test
tracker:
  kind: manual
outcomes:
  - name: done
    detect: pr-ceated
    next: complete
---
prompt
"#;
    let err = parse_workflow_md(md).expect_err("should fail on unknown detector");
    let msg = err.to_string();
    assert!(
        msg.contains("pr-ceated"),
        "error should mention the invalid value: {msg}"
    );
}

#[test]
fn reject_invalid_backoff_strategy() {
    let md = r#"---
name: test
tracker:
  kind: manual
retry:
  max_attempts: 3
  backoff: linear
  base_delay_ms: 1000
---
prompt
"#;
    let err = parse_workflow_md(md).expect_err("should fail on unknown backoff");
    let msg = err.to_string();
    assert!(
        msg.contains("linear"),
        "error should mention the invalid value: {msg}"
    );
}

#[test]
fn validate_file_exists_without_path() {
    let md = r#"---
name: test
tracker:
  kind: manual
outcomes:
  - name: check-output
    detect: file-exists
    next: complete
---
prompt
"#;
    let def = parse_workflow_md(md).expect("should parse");
    let errors = validate_workflow_def(&def);
    assert!(
        errors.iter().any(|e| e.field == "outcomes[0].path"),
        "should error on file-exists without path: {errors:?}"
    );
}

#[test]
fn validate_negative_budget() {
    let md = r#"---
name: test
tracker:
  kind: manual
agent:
  max_budget_usd: -5.0
---
prompt
"#;
    let def = parse_workflow_md(md).expect("should parse");
    let errors = validate_workflow_def(&def);
    assert!(
        errors.iter().any(|e| e.field == "agent.max_budget_usd"),
        "should error on negative budget: {errors:?}"
    );
}

#[test]
fn validate_retry_zero_attempts() {
    let md = r#"---
name: test
tracker:
  kind: manual
retry:
  max_attempts: 0
  backoff: fixed
  base_delay_ms: 1000
---
prompt
"#;
    let def = parse_workflow_md(md).expect("should parse");
    let errors = validate_workflow_def(&def);
    assert!(
        errors.iter().any(|e| e.field == "retry.max_attempts"),
        "should error on zero max_attempts: {errors:?}"
    );
}

#[test]
fn round_trip_frontmatter() {
    let def = parse_workflow_md(VALID_WORKFLOW).expect("should parse");

    // Serialize config back to YAML, wrap in frontmatter, re-parse
    let yaml = serde_yaml::to_string(&def.config).expect("should serialize");
    let reconstructed = format!("---\n{yaml}---\n{}", def.prompt);
    let def2 = parse_workflow_md(&reconstructed).expect("should re-parse");

    assert_eq!(def2.config.name, def.config.name);
    assert_eq!(def2.config.tracker.kind, def.config.tracker.kind);
    assert_eq!(
        def2.config.agent.as_ref().map(|a| a.max_turns),
        def.config.agent.as_ref().map(|a| a.max_turns)
    );
    assert_eq!(def2.config.outcomes.len(), def.config.outcomes.len());
    for (a, b) in def2.config.outcomes.iter().zip(def.config.outcomes.iter()) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.detect, b.detect);
        assert_eq!(a.next, b.next);
        assert_eq!(a.path, b.path);
    }
    // Verify all four lifecycle fields survive round-trip
    let lc1 = def.config.lifecycle.as_ref().unwrap();
    let lc2 = def2.config.lifecycle.as_ref().unwrap();
    assert_eq!(lc2.dispatched, lc1.dispatched);
    assert_eq!(lc2.complete, lc1.complete);
    assert_eq!(lc2.failed, lc1.failed);
    assert_eq!(lc2.retrying, lc1.retrying);

    // Verify filter survives round-trip
    assert!(def2.config.tracker.filter.is_some());
    assert_eq!(
        def2.config
            .tracker
            .filter
            .as_ref()
            .and_then(|f| f.get("ci_status")),
        def.config
            .tracker
            .filter
            .as_ref()
            .and_then(|f| f.get("ci_status"))
    );
    assert_eq!(
        def2.config.retry.as_ref().map(|r| r.max_attempts),
        def.config.retry.as_ref().map(|r| r.max_attempts)
    );
    assert_eq!(
        def2.config.retry.as_ref().map(|r| r.backoff),
        def.config.retry.as_ref().map(|r| r.backoff)
    );
    assert!(def2.prompt.contains("You are working on PR"));
}

#[test]
fn all_tracker_kinds_parse() {
    for kind in TrackerKind::ALL {
        let md = format!(
            "---\nname: test-{k}\ntracker:\n  kind: {k}\n---\nprompt\n",
            k = kind
        );
        let def = parse_workflow_md(&md)
            .unwrap_or_else(|e| panic!("should parse tracker kind {kind}: {e}"));
        assert_eq!(def.config.tracker.kind, *kind);
    }
}

#[test]
fn symphony_compatible_workflow_parses() {
    let md = r#"---
name: symphony-compat
tracker:
  kind: linear
  project_slug: "symphony-abc123"
  active_states:
    - Todo
    - In Progress
  terminal_states:
    - Done
    - Cancelled
polling:
  interval_ms: 5000
workspace:
  root: ~/code/workspaces
hooks:
  after_create: |
    git clone --depth 1 https://github.com/org/repo .
agent:
  max_concurrent_agents: 10
  max_turns: 20
---

You are working on a Linear ticket {{ issue.identifier }}.
"#;
    let def = parse_workflow_md(md).expect("should parse Symphony-style workflow");
    assert_eq!(def.config.tracker.kind, TrackerKind::Linear);
    assert_eq!(
        def.config.tracker.project_slug.as_deref(),
        Some("symphony-abc123")
    );
    assert_eq!(
        def.config.tracker.active_states.as_ref().map(Vec::len),
        Some(2)
    );
    assert_eq!(
        def.config.polling.as_ref().map(|p| p.interval_ms),
        Some(5000)
    );
    assert!(def.config.hooks.as_ref().unwrap().after_create.is_some());
    assert!(def.prompt.contains("Linear ticket"));
}

#[test]
fn reject_no_frontmatter() {
    let md = "Just some markdown without frontmatter.";
    let err = parse_workflow_md(md).expect_err("should fail without frontmatter");
    let msg = err.to_string();
    assert!(
        msg.contains("must start with YAML frontmatter"),
        "should give clear error: {msg}"
    );
}

#[test]
fn reject_unclosed_frontmatter() {
    let md = "---\nname: test\ntracker:\n  kind: manual\n\nsome prompt without closing";
    let err = parse_workflow_md(md).expect_err("should fail with unclosed frontmatter");
    let msg = err.to_string();
    assert!(
        msg.contains("unclosed frontmatter"),
        "should give clear error: {msg}"
    );
}

#[test]
fn validate_nan_budget() {
    let md =
        "---\nname: test\ntracker:\n  kind: manual\nagent:\n  max_budget_usd: .nan\n---\nprompt\n";
    let def = parse_workflow_md(md).expect("should parse");
    let errors = validate_workflow_def(&def);
    assert!(
        errors.iter().any(|e| e.field == "agent.max_budget_usd"),
        "should error on NaN budget: {errors:?}"
    );
}

#[test]
fn validate_inf_budget() {
    let md =
        "---\nname: test\ntracker:\n  kind: manual\nagent:\n  max_budget_usd: .inf\n---\nprompt\n";
    let def = parse_workflow_md(md).expect("should parse");
    let errors = validate_workflow_def(&def);
    assert!(
        errors.iter().any(|e| e.field == "agent.max_budget_usd"),
        "should error on infinite budget: {errors:?}"
    );
}

#[test]
fn validate_whitespace_description() {
    let md = "---\nname: test\ndescription: \"   \"\ntracker:\n  kind: manual\n---\nprompt\n";
    let def = parse_workflow_md(md).expect("should parse");
    let errors = validate_workflow_def(&def);
    assert!(
        errors.iter().any(|e| e.field == "description"),
        "should error on whitespace description: {errors:?}"
    );
}
