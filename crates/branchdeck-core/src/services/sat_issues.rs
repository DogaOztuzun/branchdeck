//! SAT issue creation service.
//!
//! Reads scored SAT results and creates GitHub issues for high-confidence
//! application-level findings. Uses idempotent fingerprinting to prevent
//! duplicate issues across runs.
//!
//! Architecture:
//! - Pure functions for: fingerprinting, threshold filtering, body building,
//!   secret scrubbing
//! - I/O functions for: reading scores, writing issue results
//! - GitHub API interaction is abstracted behind the `IssueCreator` trait —
//!   the actual HTTP call is an integration concern

use std::fmt::Write as _;
use std::path::Path;

use log::{debug, error, info, warn};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::models::sat::{
    IssueCreationOutcome, SatFinding, SatIssueConfig, SatIssueEntry, SatIssueResult,
    SatScenarioScore, SatScoreResult,
};

// ---------------------------------------------------------------------------
// Issue Creator trait (integration boundary)
// ---------------------------------------------------------------------------

/// Trait for creating GitHub issues.
///
/// Implementations handle the actual API call to GitHub.
/// The issue service builds the title/body/labels; this trait
/// handles the transport.
pub trait IssueCreator {
    /// Create an issue and return (`issue_number`, `issue_url`).
    ///
    /// # Errors
    /// Returns `AppError::GitHub` if the API call fails.
    fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        labels: &[String],
    ) -> Result<(u64, String), AppError>;

    /// Search for an existing open issue whose body contains the fingerprint string.
    ///
    /// Returns `true` if a duplicate exists.
    ///
    /// # Errors
    /// Returns `AppError::GitHub` on API failure.
    fn issue_exists_with_fingerprint(
        &self,
        owner: &str,
        repo: &str,
        fingerprint: &str,
    ) -> Result<bool, AppError>;
}

// ---------------------------------------------------------------------------
// Fingerprinting (pure)
// ---------------------------------------------------------------------------

/// Generate an idempotent fingerprint for a finding.
///
/// SHA-256 of `scenario_id + persona_name + run_id` ensures the same
/// finding from the same run never creates duplicate issues.
#[must_use]
pub fn generate_fingerprint(scenario_id: &str, persona: &str, run_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(scenario_id.as_bytes());
    hasher.update(b"\x00");
    hasher.update(persona.as_bytes());
    hasher.update(b"\x00");
    hasher.update(run_id.as_bytes());
    let hash = hasher.finalize();
    // Use first 16 bytes (32 hex chars) for a readable fingerprint
    hash.iter()
        .take(16)
        .fold(String::with_capacity(32), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

// ---------------------------------------------------------------------------
// Threshold filtering (pure)
// ---------------------------------------------------------------------------

/// Filter findings that meet the threshold for issue creation.
///
/// Returns findings that are:
/// - In one of the allowed categories (default: `App` only)
/// - At or below the maximum severity (1=critical, 2=high by default),
///   or at or below a per-scenario override if one exists
/// - At one of the allowed confidence levels (default: `High` only)
///
/// Per-scenario severity overrides (from `config.severity_overrides`) take
/// precedence over the default `config.max_severity` when present.
#[must_use]
pub fn filter_findings_for_issues<'a>(
    findings: &'a [SatFinding],
    config: &SatIssueConfig,
) -> Vec<&'a SatFinding> {
    findings
        .iter()
        .filter(|f| {
            let effective_max_severity = config
                .severity_overrides
                .get(&f.scenario_id)
                .copied()
                .unwrap_or(config.max_severity);
            config.allowed_categories.contains(&f.category)
                && f.severity <= effective_max_severity
                && config.allowed_confidences.contains(&f.confidence)
        })
        .collect()
}

/// Look up the scenario score (for persona name, satisfaction score) for a finding.
#[must_use]
pub fn find_scenario_score<'a>(
    scores: &'a [SatScenarioScore],
    scenario_id: &str,
) -> Option<&'a SatScenarioScore> {
    scores.iter().find(|s| s.scenario_id == scenario_id)
}

// ---------------------------------------------------------------------------
// Secret scrubbing (pure)
// ---------------------------------------------------------------------------

/// Known secret patterns to scrub from issue bodies.
/// Each tuple is (`pattern_name`, `prefix_marker`).
const SECRET_MARKERS: &[(&str, &str)] = &[
    ("GitHub token", "ghp_"),
    ("GitHub token (old)", "gho_"),
    ("GitHub token (user)", "ghu_"),
    ("GitHub token (server)", "ghs_"),
    ("GitHub token (refresh)", "ghr_"),
    ("AWS access key", "AKIA"),
    ("Slack token", "xoxb-"),
    ("Slack token (user)", "xoxp-"),
    ("Bearer token", "Bearer "),
    ("Basic auth", "Basic "),
    ("npm token", "npm_"),
    ("Anthropic key", "sk-ant-"),
    ("OpenAI key", "sk-"),
];

/// Scrub potential secrets from a string.
///
/// Replaces known token prefixes with `[REDACTED]`.
/// Also strips anything that looks like an API key pattern
/// (long alphanumeric strings following common key-value patterns).
#[must_use]
pub fn scrub_secrets(input: &str) -> String {
    let mut output = input.to_string();

    // Replace known token prefixes
    for &(name, prefix) in SECRET_MARKERS {
        let mut search_from = 0;
        while let Some(rel_pos) = output[search_from..].find(prefix) {
            let pos = search_from + rel_pos;
            // Find the end of the token (first whitespace, quote, or end of string)
            let token_start = pos;
            let rest = &output[pos..];
            let token_end = rest
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '`')
                .map_or(output.len(), |e| pos + e);
            let token = &output[token_start..token_end];
            // Only redact if it's long enough to plausibly be a real token
            if token.len() > prefix.len() + 4 {
                output.replace_range(token_start..token_end, &format!("[REDACTED {name}]"));
                // After replacement, continue searching from end of replacement
                search_from = token_start + format!("[REDACTED {name}]").len();
            } else {
                // Skip past this short match and keep scanning
                search_from = token_end;
            }
        }
    }

    // Scrub generic patterns: KEY=<value>, key: <value> where value is long alphanumeric
    // Simple heuristic: any unbroken alphanumeric string > 30 chars is suspicious
    let mut result = String::with_capacity(output.len());
    let chars = output.chars();
    let mut current_run = String::new();

    for c in chars {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            current_run.push(c);
        } else {
            if current_run.len() > 30 && looks_like_secret(&current_run) {
                result.push_str("[REDACTED]");
            } else {
                result.push_str(&current_run);
            }
            current_run.clear();
            result.push(c);
        }
    }
    // Flush remaining
    if current_run.len() > 30 && looks_like_secret(&current_run) {
        result.push_str("[REDACTED]");
    } else {
        result.push_str(&current_run);
    }

    result
}

/// Heuristic: a string looks like a secret if it has mixed case/digits
/// and isn't a normal word or path segment.
fn looks_like_secret(s: &str) -> bool {
    let has_upper = s.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = s.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = s.chars().any(|c| c.is_ascii_digit());

    // Must have at least two of three character classes
    let classes = u8::from(has_upper) + u8::from(has_lower) + u8::from(has_digit);
    classes >= 2
}

// ---------------------------------------------------------------------------
// Pre-POST secret safety check (NFR9)
// ---------------------------------------------------------------------------

/// Safety regex prefixes checked before any GitHub issue POST.
///
/// If ANY of these prefixes are found in the issue body (followed by
/// enough characters to be a real token), issue creation is blocked
/// and an error is returned. This is the last line of defense after
/// `scrub_secrets()` — if scrubbing missed something, this catches it.
const SAFETY_CHECK_PREFIXES: &[(&str, &str)] = &[
    ("OpenAI/Anthropic key", "sk-"),
    ("GitHub token", "ghp_"),
    ("GitHub token (old)", "gho_"),
    ("GitHub token (user)", "ghu_"),
    ("GitHub token (server)", "ghs_"),
    ("GitHub token (refresh)", "ghr_"),
    ("AWS access key", "AKIA"),
    ("Slack bot token", "xoxb-"),
    ("Slack user token", "xoxp-"),
    ("npm token", "npm_"),
    ("Private key header", "-----BEGIN"),
];

/// Check whether a string contains obvious secret patterns.
///
/// Returns `Ok(())` if no secrets are found, or `Err(AppError::Sat)` with
/// a description of the detected pattern if a secret is found.
///
/// This is a safety gate applied AFTER `scrub_secrets()` — it catches
/// anything the scrubber might have missed (e.g., secrets embedded in
/// unusual formatting).
///
/// # Errors
/// Returns `AppError::Sat` if a secret pattern is detected.
pub fn check_for_secrets(text: &str) -> Result<(), AppError> {
    for &(name, prefix) in SAFETY_CHECK_PREFIXES {
        let uses_line_boundary = prefix.starts_with("-----");
        let mut search_from = 0;
        while let Some(rel_pos) = text[search_from..].find(prefix) {
            let pos = search_from + rel_pos;
            // Check that what follows the prefix is long enough to be a real token.
            // For multi-word patterns (e.g. PEM headers), use newline as boundary.
            let rest = &text[pos..];
            let token_end = if uses_line_boundary {
                rest.find('\n').unwrap_or(rest.len())
            } else {
                rest.find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '`')
                    .unwrap_or(rest.len())
            };
            let token_len = token_end;

            // A real token is at least prefix + 8 chars
            if token_len > prefix.len() + 8 {
                error!(
                    "Secret safety check BLOCKED issue creation: detected {name} pattern at position {pos}"
                );
                return Err(AppError::Sat(format!(
                    "issue body contains secret pattern ({name}) — issue creation blocked for safety"
                )));
            }
            // Move past this match and keep scanning
            search_from = pos + token_end.max(1);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Issue body building (pure)
// ---------------------------------------------------------------------------

/// Build the GitHub issue title from a finding.
#[must_use]
pub fn build_issue_title(finding: &SatFinding) -> String {
    let severity_label = match finding.severity {
        1 => "Critical",
        2 => "High",
        3 => "Medium",
        4 => "Low",
        _ => "Cosmetic",
    };
    format!("[SAT/{severity_label}] {}", finding.summary)
}

/// Build the full GitHub issue body from a finding using a safe template.
///
/// The safe template includes ONLY:
/// - Persona name
/// - Scenario ID
/// - Satisfaction score
/// - Severity level
/// - Natural language summary (from `finding.summary`)
///
/// Explicitly excluded (FR41 / NFR9):
/// - Source code or file paths
/// - Logs or raw output
/// - API keys, credentials, or tokens
/// - Detailed evidence paths
/// - Score dimensions breakdown
/// - Reasoning text (may contain raw LLM output)
///
/// The body is still scrubbed via `scrub_secrets()` as defense-in-depth.
#[must_use]
pub fn build_issue_body(
    finding: &SatFinding,
    scenario_score: Option<&SatScenarioScore>,
    run_id: &str,
    fingerprint: &str,
) -> String {
    let mut body = String::new();

    // Header
    let _ = writeln!(body, "## SAT Finding");
    let _ = writeln!(body);

    // Safe metadata table — only persona, scenario, score, severity, summary
    let _ = writeln!(body, "| Field | Value |");
    let _ = writeln!(body, "|:------|:------|");
    let _ = writeln!(body, "| **Scenario** | {} |", finding.scenario_id);
    if let Some(score) = scenario_score {
        let _ = writeln!(body, "| **Persona** | {} |", score.persona);
        let _ = writeln!(body, "| **Satisfaction Score** | {}/100 |", score.score);
    }
    let severity_label = match finding.severity {
        1 => "Critical (1)",
        2 => "High (2)",
        3 => "Medium (3)",
        4 => "Low (4)",
        _ => "Cosmetic (5)",
    };
    let _ = writeln!(body, "| **Severity** | {severity_label} |");
    let _ = writeln!(body, "| **Category** | {} |", finding.category);
    let _ = writeln!(body, "| **Confidence** | {} |", finding.confidence);
    let _ = writeln!(body, "| **Run** | {run_id} |");
    let _ = writeln!(body);

    // Natural language summary only — no raw detail, no logs, no source code
    let _ = writeln!(body, "## Summary");
    let _ = writeln!(body);
    let _ = writeln!(body, "{}", finding.summary);
    let _ = writeln!(body);

    // NOTE: The following are intentionally excluded from the safe template:
    // - finding.detail (may contain raw LLM output or log fragments)
    // - finding.evidence (file paths could leak project structure)
    // - score.reasoning (may contain raw analysis output)
    // - score.dimensions (unnecessary detail for the issue)

    // Scrub secrets as defense-in-depth (summary should be clean, but be safe)
    let mut scrubbed = scrub_secrets(&body);

    // Fingerprint for dedup — appended after scrubbing so it survives intact
    let _ = writeln!(scrubbed, "---");
    let _ = writeln!(scrubbed, "<!-- sat-fingerprint:{fingerprint} -->");

    scrubbed
}

/// Build the labels for a SAT issue.
#[must_use]
pub fn build_issue_labels(finding: &SatFinding) -> Vec<String> {
    let mut labels = vec!["agent:implement".to_string(), "sat:finding".to_string()];

    // Add severity label
    match finding.severity {
        1 => labels.push("severity:critical".to_string()),
        2 => labels.push("severity:high".to_string()),
        _ => {}
    }

    labels
}

// ---------------------------------------------------------------------------
// Score I/O
// ---------------------------------------------------------------------------

/// Read scored results from a run directory.
///
/// # Errors
/// Returns `AppError::Sat` if the scores file cannot be read or parsed.
pub fn read_scores(run_dir: &Path) -> Result<SatScoreResult, AppError> {
    let path = run_dir.join("scores.json");
    let content = std::fs::read_to_string(&path).map_err(|e| {
        error!("Failed to read scores from {}: {e}", path.display());
        AppError::Sat(format!("failed to read scores: {e}"))
    })?;
    serde_json::from_str(&content).map_err(|e| {
        error!("Failed to parse scores JSON: {e}");
        AppError::Sat(format!("scores parse error: {e}"))
    })
}

/// Write issue creation results atomically.
///
/// # Errors
/// Returns `AppError::Sat` if serialization or write fails.
pub fn write_issue_results(result: &SatIssueResult, run_dir: &Path) -> Result<(), AppError> {
    let path = run_dir.join("issues.json");
    let json = serde_json::to_string_pretty(result).map_err(|e| {
        error!("Failed to serialize issue results: {e}");
        AppError::Sat(format!("issue result serialization error: {e}"))
    })?;
    crate::util::write_atomic(&path, json.as_bytes()).map_err(|e| {
        error!("Failed to write issue results to {}: {e}", path.display());
        AppError::Sat(format!("failed to write issue results: {e}"))
    })?;
    info!("Wrote issue results to {}", path.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Full issue creation pipeline
// ---------------------------------------------------------------------------

/// Create GitHub issues from scored SAT findings.
///
/// 1. Read `scores.json` from the run directory
/// 2. Filter findings by threshold (category, severity, confidence)
/// 3. For each eligible finding:
///    a. Generate idempotent fingerprint
///    b. Check for existing issue with same fingerprint (dedup)
///    c. Build issue title, body (scrubbed), labels
///    d. Create GitHub issue via `IssueCreator` trait
/// 4. Write `issues.json` to run directory
///
/// # Errors
/// Returns `AppError::Sat` on fatal errors (can't read scores, no repo info).
/// Individual issue creation failures are logged and recorded.
#[allow(clippy::too_many_lines)]
pub fn create_issues_from_run(
    config: &SatIssueConfig,
    run_id: &str,
    owner: &str,
    repo: &str,
    creator: &dyn IssueCreator,
) -> Result<SatIssueResult, AppError> {
    let run_dir = config.runs_dir.join(run_id);
    info!(
        "Starting SAT issue creation for run {run_id} in {}",
        run_dir.display()
    );

    let score_result = read_scores(&run_dir)?;
    let eligible = filter_findings_for_issues(&score_result.all_findings, config);

    info!(
        "Found {} eligible findings out of {} total for issue creation",
        eligible.len(),
        score_result.all_findings.len()
    );

    let mut entries = Vec::new();
    let mut created_count = 0_usize;
    let mut skipped_count = 0_usize;
    let mut failed_count = 0_usize;

    for finding in &eligible {
        let scenario_score =
            find_scenario_score(&score_result.scenario_scores, &finding.scenario_id);
        let persona = scenario_score.map_or_else(|| "unknown".to_string(), |s| s.persona.clone());
        let fingerprint = generate_fingerprint(&finding.scenario_id, &persona, run_id);

        // Check for duplicate
        match creator.issue_exists_with_fingerprint(owner, repo, &fingerprint) {
            Ok(true) => {
                debug!(
                    "Skipping duplicate issue for {} (fingerprint {fingerprint})",
                    finding.scenario_id
                );
                skipped_count += 1;
                entries.push(SatIssueEntry {
                    scenario_id: finding.scenario_id.clone(),
                    persona: persona.clone(),
                    fingerprint: fingerprint.clone(),
                    summary: finding.summary.clone(),
                    outcome: IssueCreationOutcome::SkippedDuplicate { fingerprint },
                });
                continue;
            }
            Ok(false) => { /* proceed */ }
            Err(e) => {
                warn!(
                    "Fingerprint check failed for {} — proceeding: {e}",
                    finding.scenario_id
                );
            }
        }

        let title = build_issue_title(finding);
        let body = build_issue_body(finding, scenario_score, run_id, &fingerprint);
        let labels = build_issue_labels(finding);

        // Safety gate: block issue creation if secrets are detected (NFR9)
        if let Err(e) = check_for_secrets(&body) {
            error!(
                "Secret detected in issue body for {} — blocking creation: {e}",
                finding.scenario_id
            );
            failed_count += 1;
            entries.push(SatIssueEntry {
                scenario_id: finding.scenario_id.clone(),
                persona: persona.clone(),
                fingerprint,
                summary: finding.summary.clone(),
                outcome: IssueCreationOutcome::Failed {
                    reason: format!("blocked by secret safety check: {e}"),
                },
            });
            continue;
        }

        match creator.create_issue(owner, repo, &title, &body, &labels) {
            Ok((issue_number, issue_url)) => {
                info!(
                    "Created issue #{issue_number} for {} ({fingerprint})",
                    finding.scenario_id
                );
                created_count += 1;
                entries.push(SatIssueEntry {
                    scenario_id: finding.scenario_id.clone(),
                    persona: persona.clone(),
                    fingerprint,
                    summary: finding.summary.clone(),
                    outcome: IssueCreationOutcome::Created {
                        issue_number,
                        issue_url,
                    },
                });
            }
            Err(e) => {
                error!("Failed to create issue for {}: {e}", finding.scenario_id);
                failed_count += 1;
                entries.push(SatIssueEntry {
                    scenario_id: finding.scenario_id.clone(),
                    persona: persona.clone(),
                    fingerprint,
                    summary: finding.summary.clone(),
                    outcome: IssueCreationOutcome::Failed {
                        reason: e.to_string(),
                    },
                });
            }
        }
    }

    let result = SatIssueResult {
        run_id: run_id.to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        entries,
        created_count,
        skipped_count,
        failed_count,
    };

    write_issue_results(&result, &run_dir)?;

    info!(
        "SAT issue creation complete for {run_id}: {created_count} created, {skipped_count} skipped, {failed_count} failed",
    );

    Ok(result)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::sat::{
        ConfidenceLevel, FindingCategory, FindingCounts, SatFinding, SatIssueConfig,
        SatScenarioScore, SatScoreDimensions, SatScoreResult, TokenUsage,
    };
    use std::path::PathBuf;

    fn make_config() -> SatIssueConfig {
        SatIssueConfig::new(PathBuf::from("/tmp/test-project"))
    }

    fn make_finding(
        scenario_id: &str,
        category: FindingCategory,
        severity: u8,
        confidence: ConfidenceLevel,
    ) -> SatFinding {
        SatFinding {
            scenario_id: scenario_id.to_string(),
            step_number: 1,
            summary: format!("Finding in {scenario_id}"),
            detail: "Detailed description of the issue.".to_string(),
            category,
            confidence,
            evidence: vec!["screenshots/1-after.png".to_string()],
            severity,
        }
    }

    fn make_scenario_score(scenario_id: &str, persona: &str, score: u32) -> SatScenarioScore {
        SatScenarioScore {
            scenario_id: scenario_id.to_string(),
            persona: persona.to_string(),
            score,
            dimensions: SatScoreDimensions {
                functionality: score,
                usability: score,
                error_handling: score,
                performance: score,
            },
            reasoning: "Test reasoning.".to_string(),
            findings: Vec::new(),
        }
    }

    // -- Fingerprinting -------------------------------------------------------

    #[test]
    fn fingerprint_is_deterministic() {
        let fp1 = generate_fingerprint("scenario-01", "power-user", "run-20260326");
        let fp2 = generate_fingerprint("scenario-01", "power-user", "run-20260326");
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn fingerprint_differs_for_different_inputs() {
        let fp1 = generate_fingerprint("scenario-01", "power-user", "run-20260326");
        let fp2 = generate_fingerprint("scenario-02", "power-user", "run-20260326");
        let fp3 = generate_fingerprint("scenario-01", "newbie", "run-20260326");
        let fp4 = generate_fingerprint("scenario-01", "power-user", "run-20260327");
        assert_ne!(fp1, fp2);
        assert_ne!(fp1, fp3);
        assert_ne!(fp1, fp4);
    }

    #[test]
    fn fingerprint_is_32_hex_chars() {
        let fp = generate_fingerprint("test", "user", "run-1");
        assert_eq!(fp.len(), 32);
        assert!(fp.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // -- Threshold filtering --------------------------------------------------

    #[test]
    fn filter_passes_high_severity_app_findings() {
        let config = make_config();
        let findings = vec![
            make_finding("s1", FindingCategory::App, 1, ConfidenceLevel::High),
            make_finding("s2", FindingCategory::App, 2, ConfidenceLevel::High),
        ];
        let result = filter_findings_for_issues(&findings, &config);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_rejects_low_severity() {
        let config = make_config();
        let findings = vec![
            make_finding("s1", FindingCategory::App, 3, ConfidenceLevel::High),
            make_finding("s2", FindingCategory::App, 5, ConfidenceLevel::High),
        ];
        let result = filter_findings_for_issues(&findings, &config);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_rejects_runner_category() {
        let config = make_config();
        let findings = vec![make_finding(
            "s1",
            FindingCategory::Runner,
            1,
            ConfidenceLevel::High,
        )];
        let result = filter_findings_for_issues(&findings, &config);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_rejects_low_confidence() {
        let config = make_config();
        let findings = vec![make_finding(
            "s1",
            FindingCategory::App,
            1,
            ConfidenceLevel::Low,
        )];
        let result = filter_findings_for_issues(&findings, &config);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_mixed_findings() {
        let config = make_config();
        let findings = vec![
            make_finding("s1", FindingCategory::App, 1, ConfidenceLevel::High), // pass
            make_finding("s2", FindingCategory::Runner, 1, ConfidenceLevel::High), // reject: runner
            make_finding("s3", FindingCategory::App, 4, ConfidenceLevel::High), // reject: severity
            make_finding("s4", FindingCategory::App, 2, ConfidenceLevel::Medium), // reject: confidence
            make_finding("s5", FindingCategory::App, 2, ConfidenceLevel::High),   // pass
        ];
        let result = filter_findings_for_issues(&findings, &config);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].scenario_id, "s1");
        assert_eq!(result[1].scenario_id, "s5");
    }

    // -- Severity overrides ---------------------------------------------------

    #[test]
    fn filter_uses_per_scenario_severity_override() {
        let mut config = make_config();
        // Default max_severity is 2 (critical + high)
        // Override s1 to critical-only (1)
        config
            .severity_overrides
            .insert("s1".to_string(), 1);

        let findings = vec![
            make_finding("s1", FindingCategory::App, 2, ConfidenceLevel::High), // reject: override says 1
            make_finding("s1", FindingCategory::App, 1, ConfidenceLevel::High), // pass: critical meets override
            make_finding("s2", FindingCategory::App, 2, ConfidenceLevel::High), // pass: no override, uses default
        ];
        let result = filter_findings_for_issues(&findings, &config);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].scenario_id, "s1");
        assert_eq!(result[0].severity, 1);
        assert_eq!(result[1].scenario_id, "s2");
    }

    #[test]
    fn filter_override_more_lenient_than_default() {
        let mut config = make_config();
        // Default max_severity is 2, but s1 is more lenient at 4
        config
            .severity_overrides
            .insert("s1".to_string(), 4);

        let findings = vec![
            make_finding("s1", FindingCategory::App, 3, ConfidenceLevel::High), // pass: override allows up to 4
            make_finding("s2", FindingCategory::App, 3, ConfidenceLevel::High), // reject: default is 2
        ];
        let result = filter_findings_for_issues(&findings, &config);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].scenario_id, "s1");
    }

    // -- Secret scrubbing -----------------------------------------------------

    #[test]
    fn scrub_github_token() {
        let input = "Token: ghp_abcdefghijklmnopqrstuvwxyz1234567890 is secret";
        let scrubbed = scrub_secrets(input);
        assert!(!scrubbed.contains("ghp_"));
        assert!(scrubbed.contains("[REDACTED"));
    }

    #[test]
    fn scrub_bearer_token() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0";
        let scrubbed = scrub_secrets(input);
        assert!(!scrubbed.contains("eyJhbG"));
        assert!(scrubbed.contains("[REDACTED"));
    }

    #[test]
    fn scrub_long_alphanumeric_string() {
        let input = "key=aB3cD4eF5gH6iJ7kL8mN9oP0qR1sT2uV3wX4";
        let scrubbed = scrub_secrets(input);
        assert!(scrubbed.contains("[REDACTED]"));
    }

    #[test]
    fn scrub_preserves_normal_text() {
        let input = "This is a normal issue body with no secrets.";
        let scrubbed = scrub_secrets(input);
        assert_eq!(scrubbed, input);
    }

    #[test]
    fn scrub_preserves_short_tokens() {
        let input = "Use sk- prefix for keys.";
        let scrubbed = scrub_secrets(input);
        assert_eq!(scrubbed, input);
    }

    // -- Issue body building --------------------------------------------------

    #[test]
    fn issue_body_contains_safe_template_fields() {
        let finding = make_finding(
            "scenario-01",
            FindingCategory::App,
            1,
            ConfidenceLevel::High,
        );
        let score = make_scenario_score("scenario-01", "power-user", 45);
        let body = build_issue_body(&finding, Some(&score), "run-20260326", "abc123");

        // Safe template fields present
        assert!(body.contains("scenario-01"));
        assert!(body.contains("power-user"));
        assert!(body.contains("45/100"));
        assert!(body.contains("Critical (1)"));
        assert!(body.contains("sat-fingerprint:abc123"));
        assert!(body.contains("run-20260326"));
        // Summary included
        assert!(body.contains("Finding in scenario-01"));

        // Unsafe fields excluded from safe template
        assert!(!body.contains("screenshots/1-after.png"), "evidence paths should not leak");
        assert!(!body.contains("Test reasoning."), "reasoning should not leak");
        assert!(!body.contains("Score Dimensions"), "score dimensions should not leak");
        assert!(!body.contains("Detailed description"), "detail should not leak");
    }

    #[test]
    fn issue_body_without_score() {
        let finding = make_finding(
            "scenario-01",
            FindingCategory::App,
            2,
            ConfidenceLevel::High,
        );
        let body = build_issue_body(&finding, None, "run-test", "fp123");

        assert!(body.contains("scenario-01"));
        assert!(body.contains("High (2)"));
        assert!(body.contains("sat-fingerprint:fp123"));
        // Should not contain persona/score sections
        assert!(!body.contains("Satisfaction Score"));
    }

    // -- Pre-POST secret safety check -----------------------------------------

    #[test]
    fn check_for_secrets_passes_clean_text() {
        let clean = "This is a normal issue body with persona: power-user, score: 45";
        assert!(check_for_secrets(clean).is_ok());
    }

    #[test]
    fn check_for_secrets_blocks_github_token() {
        let dirty = "Some text ghp_abcdefghijklmnopqrstuvwxyz1234567890 more text";
        let result = check_for_secrets(dirty);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("secret pattern"));
        assert!(err.contains("GitHub token"));
    }

    #[test]
    fn check_for_secrets_blocks_aws_key() {
        let dirty = "AWS key: AKIAIOSFODNN7EXAMPLE9 is here";
        let result = check_for_secrets(dirty);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("AWS access key"));
    }

    #[test]
    fn check_for_secrets_blocks_openai_key() {
        let dirty = "key=sk-proj-abcdefghijklmnop12345678 was leaked";
        let result = check_for_secrets(dirty);
        assert!(result.is_err());
    }

    #[test]
    fn check_for_secrets_ignores_short_prefixes() {
        // "sk-" alone or with few chars should not trigger
        let short = "Use the sk-prefix format";
        assert!(check_for_secrets(short).is_ok());
    }

    // -- Secret safety check tests -------------------------------------------

    #[test]
    fn check_for_secrets_detects_openai_key() {
        let text = "Here is a key: sk-proj-abcdefghijklmnop and more text";
        assert!(check_for_secrets(text).is_err());
    }

    #[test]
    fn check_for_secrets_detects_github_token() {
        let text = "Token: ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcd";
        assert!(check_for_secrets(text).is_err());
    }

    #[test]
    fn check_for_secrets_detects_aws_key() {
        let text = "AKIAIOSFODNN7EXAMPLE is the key";
        assert!(check_for_secrets(text).is_err());
    }

    #[test]
    fn check_for_secrets_detects_private_key() {
        let text = "-----BEGIN RSA PRIVATE KEY-----\nMIIEpA...";
        assert!(check_for_secrets(text).is_err());
    }

    #[test]
    fn check_for_secrets_ignores_short_prefix() {
        // Too short to be a real token (prefix + <8 chars)
        let text = "Here is sk-abc and that's it";
        assert!(check_for_secrets(text).is_ok());
    }

    #[test]
    fn check_for_secrets_passes_clean_finding_text() {
        let text = "This is a normal SAT finding with no secrets at all.";
        assert!(check_for_secrets(text).is_ok());
    }

    #[test]
    fn check_for_secrets_detects_second_occurrence() {
        // First "sk-" match is too short to trigger, but the second is a real token
        let text = "Use sk-abc prefix. Also sk-ant-api01234567890123456789 was leaked.";
        let result = check_for_secrets(text);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("secret pattern"));
    }

    #[test]
    fn issue_title_includes_severity() {
        let finding = make_finding("s1", FindingCategory::App, 1, ConfidenceLevel::High);
        let title = build_issue_title(&finding);
        assert!(title.starts_with("[SAT/Critical]"));
        assert!(title.contains("Finding in s1"));
    }

    #[test]
    fn issue_labels_include_required() {
        let finding = make_finding("s1", FindingCategory::App, 1, ConfidenceLevel::High);
        let labels = build_issue_labels(&finding);
        assert!(labels.contains(&"agent:implement".to_string()));
        assert!(labels.contains(&"sat:finding".to_string()));
        assert!(labels.contains(&"severity:critical".to_string()));
    }

    // -- Dedup / pipeline integration -----------------------------------------

    /// Mock issue creator for testing the pipeline.
    struct MockCreator {
        existing_fingerprints: Vec<String>,
        created: std::cell::RefCell<Vec<(String, String)>>,
    }

    impl MockCreator {
        fn new(existing_fingerprints: Vec<String>) -> Self {
            Self {
                existing_fingerprints,
                created: std::cell::RefCell::new(Vec::new()),
            }
        }
    }

    impl IssueCreator for MockCreator {
        fn create_issue(
            &self,
            _owner: &str,
            _repo: &str,
            title: &str,
            body: &str,
            _labels: &[String],
        ) -> Result<(u64, String), AppError> {
            let num = self.created.borrow().len() as u64 + 1;
            self.created
                .borrow_mut()
                .push((title.to_string(), body.to_string()));
            Ok((num, format!("https://github.com/test/repo/issues/{num}")))
        }

        fn issue_exists_with_fingerprint(
            &self,
            _owner: &str,
            _repo: &str,
            fingerprint: &str,
        ) -> Result<bool, AppError> {
            Ok(self
                .existing_fingerprints
                .contains(&fingerprint.to_string()))
        }
    }

    #[test]
    fn pipeline_creates_issues_for_eligible_findings() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path().join("run-test");
        std::fs::create_dir_all(&run_dir).unwrap();

        let score_result = SatScoreResult {
            run_id: "run-test".into(),
            scored_at: "2026-03-26T00:00:00Z".into(),
            scenario_scores: vec![make_scenario_score("s1", "power-user", 40)],
            aggregate_score: 40,
            all_findings: vec![make_finding(
                "s1",
                FindingCategory::App,
                1,
                ConfidenceLevel::High,
            )],
            finding_counts: FindingCounts {
                app: 1,
                runner: 0,
                scenario: 0,
                total: 1,
            },
            token_usage: TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
            },
            estimated_cost_dollars: 0.0,
        };
        let json = serde_json::to_string_pretty(&score_result).unwrap();
        crate::util::write_atomic(&run_dir.join("scores.json"), json.as_bytes()).unwrap();

        let mut config = make_config();
        config.runs_dir = tmp.path().to_path_buf();

        let creator = MockCreator::new(Vec::new());
        let result =
            create_issues_from_run(&config, "run-test", "owner", "repo", &creator).unwrap();

        assert_eq!(result.created_count, 1);
        assert_eq!(result.skipped_count, 0);
        assert_eq!(result.failed_count, 0);
        assert_eq!(creator.created.borrow().len(), 1);

        // Verify issue body was scrubbed and contains fingerprint
        let (title, body) = &creator.created.borrow()[0];
        assert!(title.contains("[SAT/Critical]"));
        assert!(body.contains("sat-fingerprint:"));
    }

    #[test]
    fn pipeline_skips_duplicates() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path().join("run-test");
        std::fs::create_dir_all(&run_dir).unwrap();

        let score_result = SatScoreResult {
            run_id: "run-test".into(),
            scored_at: "2026-03-26T00:00:00Z".into(),
            scenario_scores: vec![make_scenario_score("s1", "power-user", 40)],
            aggregate_score: 40,
            all_findings: vec![make_finding(
                "s1",
                FindingCategory::App,
                1,
                ConfidenceLevel::High,
            )],
            finding_counts: FindingCounts {
                app: 1,
                runner: 0,
                scenario: 0,
                total: 1,
            },
            token_usage: TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
            },
            estimated_cost_dollars: 0.0,
        };
        let json = serde_json::to_string_pretty(&score_result).unwrap();
        crate::util::write_atomic(&run_dir.join("scores.json"), json.as_bytes()).unwrap();

        // Pre-populate existing fingerprint
        let fingerprint = generate_fingerprint("s1", "power-user", "run-test");
        let creator = MockCreator::new(vec![fingerprint]);

        let mut config = make_config();
        config.runs_dir = tmp.path().to_path_buf();

        let result =
            create_issues_from_run(&config, "run-test", "owner", "repo", &creator).unwrap();

        assert_eq!(result.created_count, 0);
        assert_eq!(result.skipped_count, 1);
        assert!(creator.created.borrow().is_empty());
    }

    #[test]
    fn pipeline_filters_ineligible_findings() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path().join("run-test");
        std::fs::create_dir_all(&run_dir).unwrap();

        let score_result = SatScoreResult {
            run_id: "run-test".into(),
            scored_at: "2026-03-26T00:00:00Z".into(),
            scenario_scores: Vec::new(),
            aggregate_score: 80,
            all_findings: vec![
                make_finding("s1", FindingCategory::Runner, 1, ConfidenceLevel::High), // wrong category
                make_finding("s2", FindingCategory::App, 5, ConfidenceLevel::High), // wrong severity
                make_finding("s3", FindingCategory::App, 1, ConfidenceLevel::Low), // wrong confidence
            ],
            finding_counts: FindingCounts {
                app: 2,
                runner: 1,
                scenario: 0,
                total: 3,
            },
            token_usage: TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
            },
            estimated_cost_dollars: 0.0,
        };
        let json = serde_json::to_string_pretty(&score_result).unwrap();
        crate::util::write_atomic(&run_dir.join("scores.json"), json.as_bytes()).unwrap();

        let mut config = make_config();
        config.runs_dir = tmp.path().to_path_buf();

        let creator = MockCreator::new(Vec::new());
        let result =
            create_issues_from_run(&config, "run-test", "owner", "repo", &creator).unwrap();

        assert_eq!(result.created_count, 0);
        assert_eq!(result.entries.len(), 0);
    }

    // -- Issue result I/O -----------------------------------------------------

    #[test]
    fn write_and_read_issue_results() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path().join("run-test");
        std::fs::create_dir_all(&run_dir).unwrap();

        let result = SatIssueResult {
            run_id: "run-test".into(),
            created_at: "2026-03-26T00:00:00Z".into(),
            entries: vec![SatIssueEntry {
                scenario_id: "s1".into(),
                persona: "power-user".into(),
                fingerprint: "abc123".into(),
                summary: "A bug".into(),
                outcome: IssueCreationOutcome::Created {
                    issue_number: 42,
                    issue_url: "https://github.com/test/repo/issues/42".into(),
                },
            }],
            created_count: 1,
            skipped_count: 0,
            failed_count: 0,
        };

        write_issue_results(&result, &run_dir).unwrap();
        let path = run_dir.join("issues.json");
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: SatIssueResult = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.created_count, 1);
        assert_eq!(parsed.entries[0].scenario_id, "s1");
    }
}
