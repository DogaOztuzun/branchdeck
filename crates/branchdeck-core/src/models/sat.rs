use serde::{Deserialize, Serialize};

/// A parsed SAT persona definition loaded from YAML files in `sat/personas/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatPersona {
    pub name: String,
    pub description: String,
    pub frustration_threshold: FrustrationThreshold,
    pub technical_level: TechnicalLevel,
    #[serde(default)]
    pub satisfaction_criteria: Vec<String>,
    #[serde(default)]
    pub behaviors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrustrationThreshold {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for FrustrationThreshold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => f.write_str("low"),
            Self::Medium => f.write_str("medium"),
            Self::High => f.write_str("high"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TechnicalLevel {
    None,
    Beginner,
    Intermediate,
    Expert,
}

impl std::fmt::Display for TechnicalLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str("none"),
            Self::Beginner => f.write_str("beginner"),
            Self::Intermediate => f.write_str("intermediate"),
            Self::Expert => f.write_str("expert"),
        }
    }
}

/// YAML frontmatter of a SAT scenario markdown file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatScenarioMeta {
    pub id: String,
    pub title: String,
    pub persona: String,
    #[serde(default = "default_priority")]
    pub priority: ScenarioPriority,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub generated_from: Option<String>,
}

fn default_priority() -> ScenarioPriority {
    ScenarioPriority::Medium
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ScenarioPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => f.write_str("low"),
            Self::Medium => f.write_str("medium"),
            Self::High => f.write_str("high"),
            Self::Critical => f.write_str("critical"),
        }
    }
}

/// A fully parsed SAT scenario: frontmatter metadata + markdown body sections.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatScenario {
    pub meta: SatScenarioMeta,
    pub context: String,
    pub steps: Vec<String>,
    pub expected_satisfaction: Vec<String>,
    #[serde(default)]
    pub edge_cases: Vec<String>,
}

/// Machine-readable manifest produced alongside generated scenario files.
/// Written to `sat/scenarios/manifest.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatManifest {
    pub generated_at: String,
    pub persona_count: usize,
    pub scenario_count: usize,
    pub personas: Vec<SatManifestPersona>,
    pub scenarios: Vec<SatManifestEntry>,
}

/// Summary of a persona in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatManifestPersona {
    pub name: String,
    pub file: String,
}

/// Summary of a generated scenario in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SatManifestEntry {
    pub id: String,
    pub title: String,
    pub persona: String,
    pub priority: ScenarioPriority,
    pub file: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Configuration for the scenario generation service.
#[derive(Debug, Clone)]
pub struct SatGenerationConfig {
    /// Root directory of the project (where sat/ lives).
    pub project_root: std::path::PathBuf,
    /// Path to personas directory (default: `sat/personas/`).
    pub personas_dir: std::path::PathBuf,
    /// Path to scenarios output directory (default: `sat/scenarios/`).
    pub scenarios_dir: std::path::PathBuf,
    /// Paths to project doc files to read for scenario generation.
    pub doc_paths: Vec<std::path::PathBuf>,
}

impl SatGenerationConfig {
    /// Create a config with standard defaults for a project root.
    #[must_use]
    pub fn new(project_root: std::path::PathBuf) -> Self {
        let personas_dir = project_root.join("sat/personas");
        let scenarios_dir = project_root.join("sat/scenarios");

        // Default doc paths — checked at runtime for existence
        let doc_paths = vec![
            project_root.join("docs/prd.md"),
            project_root.join("docs/PRD.md"),
            project_root.join("README.md"),
            project_root.join("docs/mvp-brief.md"),
        ];

        Self {
            project_root,
            personas_dir,
            scenarios_dir,
            doc_paths,
        }
    }
}
