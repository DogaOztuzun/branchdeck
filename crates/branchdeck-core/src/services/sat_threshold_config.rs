//! SAT severity threshold configuration service.
//!
//! Loads and saves per-project threshold configuration from
//! `.branchdeck/sat-thresholds.json`. Resolves per-scenario severity
//! overrides by matching scenario tags against configured tag overrides.
//!
//! Configuration changes take effect on the next SAT cycle without restart
//! because the config is read fresh from disk each time.

use std::collections::HashMap;
use std::path::Path;

use log::{debug, error, info};

use crate::error::AppError;
use crate::models::sat::{SatIssueConfig, SatManifestEntry, SatThresholdConfig};

/// Relative path to the threshold config file within a project.
const THRESHOLD_CONFIG_PATH: &str = ".branchdeck/sat-thresholds.json";

/// Load the threshold configuration for a project.
///
/// Returns the stored config, or the default if the file does not exist.
/// The config is read fresh from disk on each call so that changes
/// take effect on the next SAT cycle without restart.
///
/// # Errors
/// Returns `AppError::Config` if the file exists but cannot be parsed.
pub fn load_threshold_config(project_root: &Path) -> Result<SatThresholdConfig, AppError> {
    let path = project_root.join(THRESHOLD_CONFIG_PATH);
    if let Some(config) = crate::util::read_optional::<SatThresholdConfig>(&path)? {
        debug!("Loaded SAT threshold config from {}", path.display());
        Ok(config)
    } else {
        debug!(
            "No SAT threshold config at {} — using defaults",
            path.display()
        );
        Ok(SatThresholdConfig::default())
    }
}

/// Save threshold configuration to the project directory.
///
/// Creates the `.branchdeck/` directory if it does not exist.
///
/// # Errors
/// Returns `AppError` if serialization or writing fails.
pub fn save_threshold_config(
    project_root: &Path,
    config: &SatThresholdConfig,
) -> Result<(), AppError> {
    let path = project_root.join(THRESHOLD_CONFIG_PATH);
    let json = serde_json::to_string_pretty(config).map_err(|e| {
        error!("Failed to serialize SAT threshold config: {e}");
        AppError::Config(format!("threshold config serialization error: {e}"))
    })?;
    crate::util::write_atomic(&path, json.as_bytes())?;
    info!("Saved SAT threshold config to {}", path.display());
    Ok(())
}

/// Resolve per-scenario severity overrides from threshold config + scenario manifest.
///
/// For each scenario in the manifest, checks if any of its tags match a
/// `tag_overrides` entry. If multiple tags match, the strictest (lowest
/// numeric value = most critical) threshold wins.
///
/// Returns a map from `scenario_id` to the effective max severity for that scenario.
#[must_use]
pub fn resolve_severity_overrides(
    threshold_config: &SatThresholdConfig,
    scenarios: &[SatManifestEntry],
) -> HashMap<String, u8> {
    let mut overrides = HashMap::new();

    if threshold_config.tag_overrides.is_empty() {
        return overrides;
    }

    for scenario in scenarios {
        let mut strictest: Option<u8> = None;

        for tag in &scenario.tags {
            if let Some(&threshold) = threshold_config.tag_overrides.get(tag) {
                strictest = Some(match strictest {
                    Some(current) => current.min(threshold),
                    None => threshold,
                });
            }
        }

        if let Some(threshold) = strictest {
            overrides.insert(scenario.id.clone(), threshold);
        }
    }

    overrides
}

/// Apply threshold configuration to a `SatIssueConfig`.
///
/// Updates the config's default severity, allowed confidences, and
/// per-scenario severity overrides based on the threshold config and
/// scenario manifest.
pub fn apply_threshold_config(
    issue_config: &mut SatIssueConfig,
    threshold_config: &SatThresholdConfig,
    scenarios: &[SatManifestEntry],
) {
    issue_config.max_severity = threshold_config.default_max_severity;
    issue_config
        .allowed_confidences
        .clone_from(&threshold_config.allowed_confidences);
    issue_config.severity_overrides = resolve_severity_overrides(threshold_config, scenarios);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::sat::{ConfidenceLevel, ScenarioPriority};

    fn make_manifest_entry(id: &str, tags: &[&str]) -> SatManifestEntry {
        SatManifestEntry {
            id: id.to_string(),
            title: format!("Test scenario {id}"),
            persona: "test-persona".to_string(),
            priority: ScenarioPriority::Medium,
            file: format!("{id}.md"),
            tags: tags.iter().map(|t| (*t).to_string()).collect(),
        }
    }

    #[test]
    fn default_config_has_expected_values() {
        let config = SatThresholdConfig::default();
        assert_eq!(config.default_max_severity, 2);
        assert_eq!(config.allowed_confidences, vec![ConfidenceLevel::High]);
        assert!(config.tag_overrides.is_empty());
    }

    #[test]
    fn resolve_overrides_empty_when_no_tag_overrides() {
        let config = SatThresholdConfig::default();
        let scenarios = vec![make_manifest_entry("s1", &["terminal", "basic"])];
        let overrides = resolve_severity_overrides(&config, &scenarios);
        assert!(overrides.is_empty());
    }

    #[test]
    fn resolve_overrides_matches_scenario_tag() {
        let mut config = SatThresholdConfig::default();
        config.tag_overrides.insert("terminal".to_string(), 1);

        let scenarios = vec![
            make_manifest_entry("s1", &["terminal", "basic"]),
            make_manifest_entry("s2", &["onboarding"]),
        ];

        let overrides = resolve_severity_overrides(&config, &scenarios);
        assert_eq!(overrides.get("s1"), Some(&1));
        assert!(!overrides.contains_key("s2"));
    }

    #[test]
    fn resolve_overrides_picks_strictest_for_multiple_tags() {
        let mut config = SatThresholdConfig::default();
        config.tag_overrides.insert("terminal".to_string(), 1); // critical only
        config.tag_overrides.insert("basic".to_string(), 3); // medium+

        let scenarios = vec![make_manifest_entry("s1", &["terminal", "basic"])];

        let overrides = resolve_severity_overrides(&config, &scenarios);
        // strictest = 1 (terminal wins over basic)
        assert_eq!(overrides.get("s1"), Some(&1));
    }

    #[test]
    fn apply_threshold_config_updates_issue_config() {
        let mut issue_config = SatIssueConfig::new(std::path::PathBuf::from("/tmp/test-project"));
        let mut threshold_config = SatThresholdConfig::default();
        threshold_config.default_max_severity = 3;
        threshold_config
            .allowed_confidences
            .push(ConfidenceLevel::Medium);
        threshold_config
            .tag_overrides
            .insert("terminal".to_string(), 1);

        let scenarios = vec![make_manifest_entry("s1", &["terminal"])];

        apply_threshold_config(&mut issue_config, &threshold_config, &scenarios);

        assert_eq!(issue_config.max_severity, 3);
        assert!(issue_config
            .allowed_confidences
            .contains(&ConfidenceLevel::Medium));
        assert_eq!(issue_config.severity_overrides.get("s1"), Some(&1));
    }

    #[test]
    fn load_save_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project_root = tmp.path();

        let mut config = SatThresholdConfig::default();
        config.default_max_severity = 3;
        config.tag_overrides.insert("terminal".to_string(), 1);

        save_threshold_config(project_root, &config).unwrap();
        let loaded = load_threshold_config(project_root).unwrap();

        assert_eq!(loaded.default_max_severity, 3);
        assert_eq!(loaded.tag_overrides.get("terminal"), Some(&1));
    }

    #[test]
    fn load_returns_default_when_file_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = load_threshold_config(tmp.path()).unwrap();
        assert_eq!(config.default_max_severity, 2);
        assert!(config.tag_overrides.is_empty());
    }
}
