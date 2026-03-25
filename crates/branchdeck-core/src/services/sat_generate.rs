//! SAT scenario generation service.
//!
//! Reads persona YAML files and project documentation, then produces
//! scenario markdown files and a machine-readable manifest.
//! Pure functions for parsing; filesystem I/O isolated to load/write helpers.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use log::{debug, error, info, warn};
use yaml_front_matter::YamlFrontMatter;

use crate::error::AppError;
use crate::models::sat::{
    SatGenerationConfig, SatManifest, SatManifestEntry, SatManifestPersona, SatPersona,
    SatScenario, SatScenarioMeta,
};

// ---------------------------------------------------------------------------
// Persona parsing
// ---------------------------------------------------------------------------

/// Parse a single persona YAML file into a `SatPersona`.
///
/// # Errors
/// Returns `AppError::Sat` if the file cannot be read or parsed.
pub fn parse_persona_file(path: &Path) -> Result<SatPersona, AppError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        error!("Failed to read persona file {}: {e}", path.display());
        AppError::Sat(format!("failed to read persona {}: {e}", path.display()))
    })?;
    parse_persona_yaml(&content)
}

/// Parse a YAML string into a `SatPersona`.
///
/// # Errors
/// Returns `AppError::Sat` if the YAML is malformed.
pub fn parse_persona_yaml(content: &str) -> Result<SatPersona, AppError> {
    serde_yaml::from_str(content).map_err(|e| {
        error!("Failed to parse persona YAML: {e}");
        AppError::Sat(format!("persona YAML parse error: {e}"))
    })
}

/// Load all persona files from a directory.
///
/// # Errors
/// Returns `AppError::Sat` if the directory cannot be read.
/// Individual file parse errors are logged as warnings and skipped.
pub fn load_personas(dir: &Path) -> Result<Vec<(String, SatPersona)>, AppError> {
    if !dir.exists() {
        return Err(AppError::Sat(format!(
            "personas directory not found: {}",
            dir.display()
        )));
    }

    let mut personas = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| {
        error!("Failed to read personas directory {}: {e}", dir.display());
        AppError::Sat(format!("failed to read personas dir: {e}"))
    })?;

    for dir_entry in entries {
        let dir_entry =
            dir_entry.map_err(|e| AppError::Sat(format!("failed to read directory entry: {e}")))?;
        let path = dir_entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("yaml") {
            let filename = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            match parse_persona_file(&path) {
                Ok(persona) => {
                    debug!("Loaded persona {:?} from {}", persona.name, path.display());
                    personas.push((filename, persona));
                }
                Err(e) => {
                    warn!("Skipping invalid persona file {}: {e}", path.display());
                }
            }
        }
    }

    personas.sort_by(|a, b| a.0.cmp(&b.0));
    info!("Loaded {} personas from {}", personas.len(), dir.display());
    Ok(personas)
}

// ---------------------------------------------------------------------------
// Scenario parsing
// ---------------------------------------------------------------------------

/// Parse a scenario markdown file (YAML frontmatter + markdown body) into a `SatScenario`.
///
/// # Errors
/// Returns `AppError::Sat` if the file cannot be read or parsed.
pub fn parse_scenario_file(path: &Path) -> Result<SatScenario, AppError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        error!("Failed to read scenario file {}: {e}", path.display());
        AppError::Sat(format!("failed to read scenario {}: {e}", path.display()))
    })?;
    parse_scenario_md(&content)
}

/// Parse a scenario markdown string into a `SatScenario`.
///
/// # Errors
/// Returns `AppError::Sat` if the frontmatter is malformed or sections are missing.
pub fn parse_scenario_md(content: &str) -> Result<SatScenario, AppError> {
    let trimmed_content = content.trim_start();
    if !trimmed_content.starts_with("---") {
        return Err(AppError::Sat(
            "scenario file must start with YAML frontmatter (---)".into(),
        ));
    }

    let document: yaml_front_matter::Document<SatScenarioMeta> = YamlFrontMatter::parse(content)
        .map_err(|e| {
            error!("Failed to parse scenario frontmatter: {e}");
            AppError::Sat(format!("scenario frontmatter parse error: {e}"))
        })?;

    let body = &document.content;
    let scenario_context = extract_section(body, "Context").unwrap_or_default();
    let steps = extract_list_section(body, "Steps");
    let expected_satisfaction = extract_list_section(body, "Expected Satisfaction");
    let edge_cases = extract_list_section(body, "Edge Cases");

    Ok(SatScenario {
        meta: document.metadata,
        context: scenario_context,
        steps,
        expected_satisfaction,
        edge_cases,
    })
}

// ---------------------------------------------------------------------------
// Doc discovery
// ---------------------------------------------------------------------------

/// Find existing project documentation files from a list of candidates.
#[must_use]
pub fn discover_docs(candidates: &[PathBuf]) -> Vec<PathBuf> {
    let found: Vec<PathBuf> = candidates.iter().filter(|p| p.exists()).cloned().collect();
    debug!("Discovered {} project doc files", found.len());
    found
}

/// Read project documentation content from discovered files.
///
/// # Errors
/// Returns `AppError::Sat` if any file cannot be read.
pub fn read_docs(paths: &[PathBuf]) -> Result<Vec<(String, String)>, AppError> {
    let mut docs = Vec::new();
    for path in paths {
        let content = std::fs::read_to_string(path).map_err(|e| {
            error!("Failed to read project doc {}: {e}", path.display());
            AppError::Sat(format!("failed to read doc {}: {e}", path.display()))
        })?;
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        debug!("Read project doc {name} ({} bytes)", content.len());
        docs.push((name, content));
    }
    Ok(docs)
}

// ---------------------------------------------------------------------------
// Scenario rendering (pure)
// ---------------------------------------------------------------------------

/// Render a `SatScenario` back to markdown format (frontmatter + body).
#[must_use]
pub fn render_scenario_md(scenario: &SatScenario) -> String {
    let mut out = String::new();

    // Frontmatter
    out.push_str("---\n");
    let _ = writeln!(out, "id: {}", scenario.meta.id);
    let _ = writeln!(out, "title: {}", scenario.meta.title);
    let _ = writeln!(out, "persona: {}", scenario.meta.persona);
    let _ = writeln!(out, "priority: {}", scenario.meta.priority);
    if !scenario.meta.tags.is_empty() {
        let _ = writeln!(out, "tags: [{}]", scenario.meta.tags.join(", "));
    }
    if let Some(ref source) = scenario.meta.generated_from {
        let _ = writeln!(out, "generated_from: {source}");
    }
    out.push_str("---\n\n");

    // Context
    out.push_str("## Context\n");
    out.push_str(&scenario.context);
    out.push_str("\n\n");

    // Steps
    out.push_str("## Steps\n");
    for (i, step) in scenario.steps.iter().enumerate() {
        let _ = writeln!(out, "{}. {step}", i + 1);
    }
    out.push('\n');

    // Expected Satisfaction
    out.push_str("## Expected Satisfaction\n");
    for signal in &scenario.expected_satisfaction {
        let _ = writeln!(out, "- {signal}");
    }

    // Edge Cases (optional)
    if !scenario.edge_cases.is_empty() {
        out.push_str("\n## Edge Cases\n");
        for case in &scenario.edge_cases {
            let _ = writeln!(out, "- {case}");
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Manifest generation (pure)
// ---------------------------------------------------------------------------

/// Build a manifest from loaded personas and scenarios.
#[must_use]
pub fn build_manifest(
    personas: &[(String, SatPersona)],
    scenarios: &[SatScenario],
    generated_at: &str,
) -> SatManifest {
    let manifest_personas: Vec<SatManifestPersona> = personas
        .iter()
        .map(|(filename, persona)| SatManifestPersona {
            name: persona.name.clone(),
            file: format!("{filename}.yaml"),
        })
        .collect();

    let manifest_entries: Vec<SatManifestEntry> = scenarios
        .iter()
        .map(|s| SatManifestEntry {
            id: s.meta.id.clone(),
            title: s.meta.title.clone(),
            persona: s.meta.persona.clone(),
            priority: s.meta.priority,
            file: format!("{}.md", s.meta.id),
            tags: s.meta.tags.clone(),
        })
        .collect();

    SatManifest {
        generated_at: generated_at.to_string(),
        persona_count: personas.len(),
        scenario_count: scenarios.len(),
        personas: manifest_personas,
        scenarios: manifest_entries,
    }
}

// ---------------------------------------------------------------------------
// Manifest I/O
// ---------------------------------------------------------------------------

/// Write the manifest JSON to the scenarios directory.
///
/// # Errors
/// Returns `AppError::Sat` if the file cannot be written.
pub fn write_manifest(manifest: &SatManifest, scenarios_dir: &Path) -> Result<PathBuf, AppError> {
    let manifest_path = scenarios_dir.join("manifest.json");
    let json = serde_json::to_string_pretty(manifest).map_err(|e| {
        error!("Failed to serialize manifest: {e}");
        AppError::Sat(format!("manifest serialization error: {e}"))
    })?;
    crate::util::write_atomic(&manifest_path, json.as_bytes()).map_err(|e| {
        error!(
            "Failed to write manifest to {}: {e}",
            manifest_path.display()
        );
        AppError::Sat(format!("failed to write manifest: {e}"))
    })?;
    info!("Wrote manifest to {}", manifest_path.display());
    Ok(manifest_path)
}

/// Write a scenario markdown file to the scenarios directory.
///
/// # Errors
/// Returns `AppError::Sat` if the file cannot be written.
pub fn write_scenario(scenario: &SatScenario, scenarios_dir: &Path) -> Result<PathBuf, AppError> {
    let filename = format!("{}.md", scenario.meta.id);
    let path = scenarios_dir.join(&filename);
    let content = render_scenario_md(scenario);
    crate::util::write_atomic(&path, content.as_bytes()).map_err(|e| {
        error!("Failed to write scenario to {}: {e}", path.display());
        AppError::Sat(format!("failed to write scenario {filename}: {e}"))
    })?;
    debug!("Wrote scenario {} to {}", scenario.meta.id, path.display());
    Ok(path)
}

// ---------------------------------------------------------------------------
// Full generation pipeline
// ---------------------------------------------------------------------------

/// Run the full scenario generation pipeline:
/// 1. Load personas from `sat/personas/`
/// 2. Discover and read project docs
/// 3. Load existing scenarios from `sat/scenarios/`
/// 4. Build and write a manifest
///
/// This function does NOT generate new scenarios (that requires an LLM).
/// It inventories the existing personas and scenarios and produces a manifest.
///
/// # Errors
/// Returns `AppError::Sat` on I/O or parse failures.
pub fn generate_manifest(config: &SatGenerationConfig) -> Result<SatManifest, AppError> {
    info!(
        "Starting SAT manifest generation for {}",
        config.project_root.display()
    );

    // 1. Load personas
    let personas = load_personas(&config.personas_dir)?;
    if personas.is_empty() {
        return Err(AppError::Sat(
            "no personas found — add YAML files to sat/personas/".into(),
        ));
    }

    // 2. Discover project docs
    let docs = discover_docs(&config.doc_paths);
    if docs.is_empty() {
        warn!("No project documentation found — manifest will lack doc references");
    } else {
        info!("Found {} project docs for scenario generation", docs.len());
    }

    // 3. Load existing scenarios
    let scenarios = load_scenarios(&config.scenarios_dir)?;
    info!("Found {} existing scenarios", scenarios.len());

    // 4. Build manifest
    let now = chrono::Utc::now().to_rfc3339();
    let manifest = build_manifest(&personas, &scenarios, &now);

    // 5. Write manifest
    std::fs::create_dir_all(&config.scenarios_dir)
        .map_err(|e| AppError::Sat(format!("failed to create scenarios dir: {e}")))?;
    write_manifest(&manifest, &config.scenarios_dir)?;

    info!(
        "SAT manifest generated: {} personas, {} scenarios",
        manifest.persona_count, manifest.scenario_count
    );

    Ok(manifest)
}

/// Load all scenario markdown files from a directory.
///
/// # Errors
/// Returns `AppError::Sat` if the directory cannot be read.
/// Individual file parse errors are logged as warnings and skipped.
pub fn load_scenarios(dir: &Path) -> Result<Vec<SatScenario>, AppError> {
    if !dir.exists() {
        debug!("Scenarios directory does not exist yet: {}", dir.display());
        return Ok(Vec::new());
    }

    let mut scenarios = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| {
        error!("Failed to read scenarios directory {}: {e}", dir.display());
        AppError::Sat(format!("failed to read scenarios dir: {e}"))
    })?;

    for dir_entry in entries {
        let dir_entry =
            dir_entry.map_err(|e| AppError::Sat(format!("failed to read directory entry: {e}")))?;
        let path = dir_entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            match parse_scenario_file(&path) {
                Ok(scenario) => {
                    debug!("Loaded scenario {:?}", scenario.meta.id);
                    scenarios.push(scenario);
                }
                Err(e) => {
                    warn!("Skipping invalid scenario file {}: {e}", path.display());
                }
            }
        }
    }

    scenarios.sort_by(|a, b| a.meta.id.cmp(&b.meta.id));
    Ok(scenarios)
}

// ---------------------------------------------------------------------------
// Helpers (pure)
// ---------------------------------------------------------------------------

/// Extract the text content of a `## Section` heading from markdown.
fn extract_section(body: &str, heading: &str) -> Option<String> {
    let marker = format!("## {heading}");
    let start = body.find(&marker)?;
    let after_heading = &body[start + marker.len()..];
    let content_start = after_heading.find('\n').map_or(0, |i| i + 1);
    let rest = &after_heading[content_start..];

    // Find the next ## heading or end of string
    let end = rest.find("\n## ").unwrap_or(rest.len());
    let section = rest[..end].trim().to_string();

    if section.is_empty() {
        None
    } else {
        Some(section)
    }
}

/// Extract a numbered or bulleted list from a `## Section` heading.
fn extract_list_section(body: &str, heading: &str) -> Vec<String> {
    let Some(text) = extract_section(body, heading) else {
        return Vec::new();
    };

    text.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            // Strip leading "1. ", "- ", "* " etc.
            let stripped = trimmed
                .trim_start_matches(|c: char| {
                    c.is_ascii_digit() || c == '.' || c == '-' || c == '*'
                })
                .trim_start();
            if stripped.is_empty() {
                None
            } else {
                Some(stripped.to_string())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::sat::{FrustrationThreshold, TechnicalLevel};

    const POWER_USER_YAML: &str = r"
name: Power User
description: Experienced developer who expects keyboard shortcuts
frustration_threshold: high
technical_level: expert
satisfaction_criteria:
  - Workflows should complete in minimal clicks
  - Keyboard shortcuts should exist for frequent actions
behaviors:
  - Skips onboarding and tooltips entirely
  - Tries keyboard shortcuts before clicking
";

    const NEWBIE_YAML: &str = r"
name: Confused Newbie
description: First-time user with no domain knowledge
frustration_threshold: low
technical_level: none
satisfaction_criteria:
  - Every action should have clear feedback
behaviors:
  - Reads every label and tooltip before acting
";

    const SCENARIO_MD: &str = "---
id: test-scenario
title: Test Scenario Title
persona: power-user
priority: high
tags: [testing, example]
generated_from: README.md
---

## Context
A user wants to test a feature in the app.

## Steps
1. Open the app
2. Click the test button
3. Verify the result

## Expected Satisfaction
- The feature should work as expected
- Feedback should be immediate

## Edge Cases
- The button is disabled when offline
- The result takes longer than 5 seconds
";

    #[test]
    fn parse_power_user_persona() {
        let persona = parse_persona_yaml(POWER_USER_YAML).unwrap();
        assert_eq!(persona.name, "Power User");
        assert_eq!(persona.frustration_threshold, FrustrationThreshold::High);
        assert_eq!(persona.technical_level, TechnicalLevel::Expert);
        assert_eq!(persona.satisfaction_criteria.len(), 2);
        assert_eq!(persona.behaviors.len(), 2);
    }

    #[test]
    fn parse_newbie_persona() {
        let persona = parse_persona_yaml(NEWBIE_YAML).unwrap();
        assert_eq!(persona.name, "Confused Newbie");
        assert_eq!(persona.frustration_threshold, FrustrationThreshold::Low);
        assert_eq!(persona.technical_level, TechnicalLevel::None);
    }

    #[test]
    fn parse_scenario_markdown() {
        let scenario = parse_scenario_md(SCENARIO_MD).unwrap();
        assert_eq!(scenario.meta.id, "test-scenario");
        assert_eq!(scenario.meta.title, "Test Scenario Title");
        assert_eq!(scenario.meta.persona, "power-user");
        assert_eq!(scenario.meta.tags, vec!["testing", "example"]);
        assert_eq!(scenario.steps.len(), 3);
        assert_eq!(scenario.expected_satisfaction.len(), 2);
        assert_eq!(scenario.edge_cases.len(), 2);
        assert!(scenario.context.contains("test a feature"));
    }

    #[test]
    fn scenario_missing_frontmatter() {
        let result = parse_scenario_md("No frontmatter here");
        assert!(result.is_err());
    }

    #[test]
    fn render_scenario_roundtrip() {
        let scenario = parse_scenario_md(SCENARIO_MD).unwrap();
        let rendered = render_scenario_md(&scenario);

        // Re-parse the rendered output
        let reparsed = parse_scenario_md(&rendered).unwrap();
        assert_eq!(reparsed.meta.id, scenario.meta.id);
        assert_eq!(reparsed.meta.title, scenario.meta.title);
        assert_eq!(reparsed.steps.len(), scenario.steps.len());
        assert_eq!(
            reparsed.expected_satisfaction.len(),
            scenario.expected_satisfaction.len()
        );
    }

    #[test]
    fn build_manifest_basic() {
        let personas = vec![
            (
                "power-user".to_string(),
                parse_persona_yaml(POWER_USER_YAML).unwrap(),
            ),
            (
                "confused-newbie".to_string(),
                parse_persona_yaml(NEWBIE_YAML).unwrap(),
            ),
        ];
        let scenarios = vec![parse_scenario_md(SCENARIO_MD).unwrap()];
        let manifest = build_manifest(&personas, &scenarios, "2026-03-26T00:00:00Z");

        assert_eq!(manifest.persona_count, 2);
        assert_eq!(manifest.scenario_count, 1);
        assert_eq!(manifest.personas.len(), 2);
        assert_eq!(manifest.scenarios.len(), 1);
        assert_eq!(manifest.scenarios[0].id, "test-scenario");
        assert_eq!(manifest.scenarios[0].file, "test-scenario.md");
    }

    #[test]
    fn extract_section_works() {
        let body = "## Context\nSome context here.\n\n## Steps\n1. Step one\n";
        let ctx = extract_section(body, "Context");
        assert_eq!(ctx, Some("Some context here.".to_string()));
    }

    #[test]
    fn extract_list_section_works() {
        let body = "## Steps\n1. First step\n2. Second step\n3. Third step\n";
        let steps = extract_list_section(body, "Steps");
        assert_eq!(steps.len(), 3);
        assert_eq!(steps[0], "First step");
        assert_eq!(steps[2], "Third step");
    }

    #[test]
    fn persona_yaml_invalid() {
        let result = parse_persona_yaml("not: valid: yaml: [[[");
        assert!(result.is_err());
    }
}
