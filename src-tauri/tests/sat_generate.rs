//! Integration tests for SAT scenario generation service.
//!
//! Story 3.1: Scenario Generation from Project Docs.
//! Covers: persona loading from filesystem, scenario loading, manifest generation.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::uninlined_format_args,
    clippy::doc_markdown,
    clippy::needless_raw_string_hashes
)]

use std::fs;

use branchdeck_core::models::sat::SatGenerationConfig;
use branchdeck_core::services::sat_generate::{generate_manifest, load_personas, load_scenarios};

/// Set up a temp directory with persona files and scenario files.
fn setup_sat_dir() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    // Create personas
    let personas_dir = root.join("sat/personas");
    fs::create_dir_all(&personas_dir).unwrap();

    fs::write(
        personas_dir.join("power-user.yaml"),
        r#"name: Power User
description: Fast and experienced
frustration_threshold: high
technical_level: expert
satisfaction_criteria:
  - Minimal clicks
behaviors:
  - Uses keyboard shortcuts
"#,
    )
    .unwrap();

    fs::write(
        personas_dir.join("newbie.yaml"),
        r#"name: Confused Newbie
description: Needs guidance
frustration_threshold: low
technical_level: none
satisfaction_criteria:
  - Clear feedback
behaviors:
  - Reads tooltips
"#,
    )
    .unwrap();

    // Create scenarios dir with one scenario
    let scenarios_dir = root.join("sat/scenarios");
    fs::create_dir_all(&scenarios_dir).unwrap();

    fs::write(
        scenarios_dir.join("test-feature.md"),
        r#"---
id: test-feature
title: Test a Feature
persona: power-user
priority: high
tags: [testing]
generated_from: README.md
---

## Context
User wants to test a feature.

## Steps
1. Open the app
2. Click button

## Expected Satisfaction
- Feature works

## Edge Cases
- Button is disabled
"#,
    )
    .unwrap();

    // Create a project doc
    fs::write(root.join("README.md"), "# Test Project\nA test project.\n").unwrap();

    tmp
}

#[test]
fn load_personas_from_directory() {
    let tmp = setup_sat_dir();
    let personas = load_personas(&tmp.path().join("sat/personas")).unwrap();

    assert_eq!(personas.len(), 2);
    // Sorted alphabetically by filename
    assert_eq!(personas[0].0, "newbie");
    assert_eq!(personas[1].0, "power-user");
    assert_eq!(personas[1].1.name, "Power User");
}

#[test]
fn load_personas_missing_dir() {
    let result = load_personas(std::path::Path::new("/nonexistent/path"));
    assert!(result.is_err());
}

#[test]
fn load_scenarios_from_directory() {
    let tmp = setup_sat_dir();
    let scenarios = load_scenarios(&tmp.path().join("sat/scenarios")).unwrap();

    assert_eq!(scenarios.len(), 1);
    assert_eq!(scenarios[0].meta.id, "test-feature");
    assert_eq!(scenarios[0].steps.len(), 2);
}

#[test]
fn load_scenarios_missing_dir() {
    let scenarios = load_scenarios(std::path::Path::new("/nonexistent/path")).unwrap();
    assert!(scenarios.is_empty());
}

#[test]
fn full_manifest_generation() {
    let tmp = setup_sat_dir();
    let root = tmp.path().to_path_buf();

    let config = SatGenerationConfig {
        project_root: root.clone(),
        personas_dir: root.join("sat/personas"),
        scenarios_dir: root.join("sat/scenarios"),
        doc_paths: vec![root.join("README.md")],
    };

    let manifest = generate_manifest(&config).unwrap();

    assert_eq!(manifest.persona_count, 2);
    assert_eq!(manifest.scenario_count, 1);

    // Verify manifest.json was written
    let manifest_path = root.join("sat/scenarios/manifest.json");
    assert!(manifest_path.exists());

    let manifest_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest_json["persona_count"], 2);
    assert_eq!(manifest_json["scenario_count"], 1);
}

#[test]
fn manifest_generation_no_personas() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();

    // Create empty personas dir
    fs::create_dir_all(root.join("sat/personas")).unwrap();

    let config = SatGenerationConfig {
        project_root: root.clone(),
        personas_dir: root.join("sat/personas"),
        scenarios_dir: root.join("sat/scenarios"),
        doc_paths: vec![],
    };

    let result = generate_manifest(&config);
    assert!(result.is_err());
}
