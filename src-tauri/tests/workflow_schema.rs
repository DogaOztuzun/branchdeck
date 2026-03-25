//! Tests for WorkflowDef schema parsing and validation.
//!
//! Story 1.2: WorkflowDef Schema Spec & Model.
//! Covers: valid definition, missing required fields, unknown enum values,
//! invalid retry config, round-trip serialization.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use branchdeck_lib::models::workflow::{
    BackoffStrategy, OutcomeAction, OutcomeDetector, TriggerType, WorkflowDef,
};
use branchdeck_lib::services::workflow::{parse_workflow_yaml, validate_workflow_def};

const VALID_WORKFLOW: &str = r#"
schema_version: 1
name: pr-shepherd
description: Fix failing CI on pull requests

trigger:
  type: github-pr
  filter:
    ci_status: failure

context:
  template: templates/pr-context.md.hbs
  output: pr-context.json

execution:
  skill: skills/pr-shepherd
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
"#;

const MINIMAL_WORKFLOW: &str = r#"
schema_version: 1
name: minimal-test
description: A minimal workflow

trigger:
  type: manual

context:
  template: tpl.md
  output: ctx.json

execution:
  skill: skills/test

outcomes:
  - name: done
    detect: file-exists
    path: output.txt
    next: complete
"#;

#[test]
fn parse_valid_full_definition() {
    let def = parse_workflow_yaml(VALID_WORKFLOW).expect("should parse valid YAML");

    assert_eq!(def.schema_version, 1);
    assert_eq!(def.name, "pr-shepherd");
    assert_eq!(def.trigger.trigger_type, TriggerType::GithubPr);
    assert!(def.trigger.filter.is_some());
    assert_eq!(def.context.template, "templates/pr-context.md.hbs");
    assert_eq!(def.context.output, "pr-context.json");
    assert_eq!(def.execution.skill, "skills/pr-shepherd");
    assert_eq!(def.execution.max_turns, Some(25));
    assert_eq!(def.execution.max_budget_usd, Some(5.0));
    assert_eq!(def.execution.timeout_minutes, Some(30));
    assert_eq!(def.outcomes.len(), 3);
    assert_eq!(def.outcomes[0].detect, OutcomeDetector::CiPassing);
    assert_eq!(def.outcomes[0].next, OutcomeAction::Complete);
    assert_eq!(def.outcomes[1].detect, OutcomeDetector::FileExists);
    assert_eq!(def.outcomes[1].next, OutcomeAction::Review);
    assert_eq!(def.outcomes[2].detect, OutcomeDetector::RunFailed);
    assert_eq!(def.outcomes[2].next, OutcomeAction::Retry);

    let lifecycle = def.lifecycle.as_ref().expect("should have lifecycle");
    assert_eq!(lifecycle.dispatched.as_deref(), Some("Analyzing"));
    assert_eq!(lifecycle.complete.as_deref(), Some("Fixed"));

    let retry = def.retry.as_ref().expect("should have retry");
    assert_eq!(retry.max_attempts, 3);
    assert_eq!(retry.backoff, BackoffStrategy::Exponential);
    assert_eq!(retry.base_delay_ms, 30_000);

    let errors = validate_workflow_def(&def);
    assert!(
        errors.is_empty(),
        "valid def should have no errors: {errors:?}"
    );
}

#[test]
fn parse_minimal_definition_with_defaults() {
    let def = parse_workflow_yaml(MINIMAL_WORKFLOW).expect("should parse minimal YAML");

    assert_eq!(def.name, "minimal-test");
    assert_eq!(def.trigger.trigger_type, TriggerType::Manual);
    assert!(def.trigger.filter.is_none());
    assert!(def.execution.max_turns.is_none());
    assert!(def.execution.max_budget_usd.is_none());
    assert!(def.execution.timeout_minutes.is_none());
    assert!(def.execution.allowed_directories.is_none());
    assert!(def.lifecycle.is_none());
    assert!(def.retry.is_none());

    let errors = validate_workflow_def(&def);
    assert!(
        errors.is_empty(),
        "minimal def should have no errors: {errors:?}"
    );
}

#[test]
fn reject_missing_required_field_name() {
    let yaml = r#"
schema_version: 1
name: ""
description: test

trigger:
  type: manual
context:
  template: tpl.md
  output: ctx.json
execution:
  skill: skills/test
outcomes:
  - name: done
    detect: file-exists
    path: out.txt
    next: complete
"#;
    let def = parse_workflow_yaml(yaml).expect("should parse");
    let errors = validate_workflow_def(&def);
    assert!(
        errors.iter().any(|e| e.field == "name"),
        "should error on empty name: {errors:?}"
    );
}

#[test]
fn reject_unknown_trigger_type() {
    let yaml = r#"
schema_version: 1
name: test
description: test
trigger:
  type: foobar
context:
  template: tpl.md
  output: ctx.json
execution:
  skill: skills/test
outcomes:
  - name: done
    detect: file-exists
    next: complete
"#;
    let err = parse_workflow_yaml(yaml).expect_err("should fail on unknown trigger type");
    let msg = err.to_string();
    assert!(
        msg.contains("foobar"),
        "error should mention the invalid value: {msg}"
    );
}

#[test]
fn reject_unknown_outcome_detector() {
    let yaml = r#"
schema_version: 1
name: test
description: test
trigger:
  type: manual
context:
  template: tpl.md
  output: ctx.json
execution:
  skill: skills/test
outcomes:
  - name: done
    detect: pr-ceated
    next: complete
"#;
    let err = parse_workflow_yaml(yaml).expect_err("should fail on unknown detector");
    let msg = err.to_string();
    assert!(
        msg.contains("pr-ceated"),
        "error should mention the invalid value: {msg}"
    );
}

#[test]
fn reject_invalid_backoff_strategy() {
    let yaml = r#"
schema_version: 1
name: test
description: test
trigger:
  type: manual
context:
  template: tpl.md
  output: ctx.json
execution:
  skill: skills/test
outcomes:
  - name: done
    detect: file-exists
    next: complete
retry:
  max_attempts: 3
  backoff: linear
  base_delay_ms: 1000
"#;
    let err = parse_workflow_yaml(yaml).expect_err("should fail on unknown backoff");
    let msg = err.to_string();
    assert!(
        msg.contains("linear"),
        "error should mention the invalid value: {msg}"
    );
}

#[test]
fn validate_empty_outcomes_list() {
    let yaml = r#"
schema_version: 1
name: test
description: test
trigger:
  type: manual
context:
  template: tpl.md
  output: ctx.json
execution:
  skill: skills/test
outcomes: []
"#;
    let def = parse_workflow_yaml(yaml).expect("should parse");
    let errors = validate_workflow_def(&def);
    assert!(
        errors.iter().any(|e| e.field == "outcomes"),
        "should error on empty outcomes: {errors:?}"
    );
}

#[test]
fn validate_retry_zero_attempts() {
    let yaml = r#"
schema_version: 1
name: test
description: test
trigger:
  type: manual
context:
  template: tpl.md
  output: ctx.json
execution:
  skill: skills/test
outcomes:
  - name: done
    detect: file-exists
    next: complete
retry:
  max_attempts: 0
  backoff: fixed
  base_delay_ms: 1000
"#;
    let def = parse_workflow_yaml(yaml).expect("should parse");
    let errors = validate_workflow_def(&def);
    assert!(
        errors.iter().any(|e| e.field == "retry.max_attempts"),
        "should error on zero max_attempts: {errors:?}"
    );
}

#[test]
fn round_trip_serde() {
    let def = parse_workflow_yaml(VALID_WORKFLOW).expect("should parse");

    let serialized = serde_yaml::to_string(&def).expect("should serialize");
    let roundtripped: WorkflowDef =
        serde_yaml::from_str(&serialized).expect("should deserialize roundtrip");

    assert_eq!(roundtripped.name, def.name);
    assert_eq!(roundtripped.schema_version, def.schema_version);
    assert_eq!(roundtripped.description, def.description);
    assert_eq!(roundtripped.trigger.trigger_type, def.trigger.trigger_type);
    assert_eq!(roundtripped.context.template, def.context.template);
    assert_eq!(roundtripped.context.output, def.context.output);
    assert_eq!(roundtripped.execution.skill, def.execution.skill);
    assert_eq!(roundtripped.execution.max_turns, def.execution.max_turns);
    assert_eq!(roundtripped.outcomes.len(), def.outcomes.len());
    for (a, b) in roundtripped.outcomes.iter().zip(def.outcomes.iter()) {
        assert_eq!(a.name, b.name);
        assert_eq!(a.detect, b.detect);
        assert_eq!(a.next, b.next);
        assert_eq!(a.path, b.path);
    }
    assert_eq!(
        roundtripped.retry.as_ref().map(|r| r.max_attempts),
        def.retry.as_ref().map(|r| r.max_attempts)
    );
    assert_eq!(
        roundtripped.retry.as_ref().map(|r| r.backoff),
        def.retry.as_ref().map(|r| r.backoff)
    );
}

#[test]
fn all_trigger_types_parse() {
    for tt in TriggerType::ALL {
        let yaml = format!(
            r#"
schema_version: 1
name: test-{t}
description: test
trigger:
  type: {t}
context:
  template: tpl.md
  output: ctx.json
execution:
  skill: skills/test
outcomes:
  - name: done
    detect: file-exists
    next: complete
"#,
            t = tt
        );
        let def = parse_workflow_yaml(&yaml)
            .unwrap_or_else(|e| panic!("should parse trigger type {tt}: {e}"));
        assert_eq!(def.trigger.trigger_type, *tt);
    }
}
