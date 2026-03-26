//! SAT satisfaction scoring service.
//!
//! Reads run results (trajectories, screenshots) and produces per-persona
//! satisfaction scores using LLM-as-judge evaluation. Classifies findings
//! by category (app bug / runner artifact / bad scenario) with confidence levels.
//!
//! Architecture:
//! - Pure functions for: prompt building, response parsing, classification,
//!   budget tracking, report rendering, score aggregation
//! - I/O functions for: reading run data, writing scores/reports/learnings
//! - LLM API interaction is abstracted behind the `LlmJudge` trait —
//!   the actual HTTP call is an integration concern

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use log::{debug, error, info, warn};

use crate::error::AppError;
use crate::models::sat::{
    ConfidenceLevel, FindingCategory, FindingCounts, SatFinding, SatLearning, SatLearningsFile,
    SatRunResult, SatScenario, SatScenarioScore, SatScoreConfig, SatScoreDimensions,
    SatScoreResult, SatTrajectory, ScoringBudget, TokenUsage, TrajectoryStatus,
};

// ---------------------------------------------------------------------------
// LLM Judge trait (integration boundary)
// ---------------------------------------------------------------------------

/// Trait for the LLM judge that scores scenarios.
///
/// Implementations handle the actual API call to Claude/other LLMs.
/// The scoring service builds prompts and parses responses; this trait
/// handles the transport.
pub trait LlmJudge {
    /// Send a scoring prompt and return the raw response text + token usage.
    ///
    /// # Errors
    /// Returns `AppError::Sat` if the LLM call fails.
    fn score(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<(String, TokenUsage), AppError>;
}

// ---------------------------------------------------------------------------
// Budget tracking (pure)
// ---------------------------------------------------------------------------

/// Calculate the estimated cost from token usage.
#[must_use]
#[allow(clippy::cast_precision_loss)] // Token counts won't exceed f64 mantissa range in practice
pub fn estimate_cost(budget: &ScoringBudget) -> f64 {
    let input_cost = (budget.input_tokens_used as f64 / 1000.0) * budget.input_cost_per_1k;
    let output_cost = (budget.output_tokens_used as f64 / 1000.0) * budget.output_cost_per_1k;
    input_cost + output_cost
}

/// Check whether the budget has been exceeded.
#[must_use]
pub fn is_budget_exceeded(budget: &ScoringBudget) -> bool {
    estimate_cost(budget) >= budget.max_cost_dollars
}

/// Record token usage into the budget, returning the updated budget.
#[must_use]
pub fn record_usage(budget: &ScoringBudget, usage: &TokenUsage) -> ScoringBudget {
    ScoringBudget {
        input_tokens_used: budget.input_tokens_used + usage.input_tokens,
        output_tokens_used: budget.output_tokens_used + usage.output_tokens,
        ..budget.clone()
    }
}

// ---------------------------------------------------------------------------
// Prompt building (pure)
// ---------------------------------------------------------------------------

/// Build the system prompt for the LLM scoring judge.
#[must_use]
pub fn build_system_prompt() -> String {
    r#"You are a SAT (Satisfaction Acceptance Testing) judge. Your job is to evaluate user experience scenarios executed against a desktop application and produce satisfaction scores.

You will receive trajectory data from automated scenario execution. For each scenario, analyze:
1. Whether each step succeeded or failed
2. The nature of any failures (app bug vs. runner/infrastructure issue vs. bad test)
3. How a real user with the given persona would feel about the experience

Score each scenario on these dimensions (0-100 each):
- functionality: Did the feature work as expected?
- usability: Was the experience smooth and intuitive?
- error_handling: Were errors clear and recoverable?
- performance: Was the app responsive?

Produce an overall satisfaction score (0-100) as a weighted average.

For each issue found, classify it:
- "app": A real bug in the application
- "runner": An infrastructure/WebDriver/test-runner artifact (not a real bug)
- "scenario": A problem with the test scenario itself (unreliable, poorly defined)

For each finding, assign a confidence level: "high", "medium", or "low".
Assign severity 1-5 (1=critical, 5=cosmetic).

IMPORTANT: Respond ONLY with valid JSON matching this exact schema:
{
  "score": <0-100>,
  "dimensions": {
    "functionality": <0-100>,
    "usability": <0-100>,
    "error_handling": <0-100>,
    "performance": <0-100>
  },
  "reasoning": "<explanation of the score>",
  "findings": [
    {
      "step_number": <1-based or 0 for overall>,
      "summary": "<short description>",
      "detail": "<detailed explanation>",
      "category": "app" | "runner" | "scenario",
      "confidence": "high" | "medium" | "low",
      "evidence": ["<screenshot path or step text>"],
      "severity": <1-5>
    }
  ]
}"#
    .to_string()
}

/// Build the user prompt for scoring a single scenario trajectory.
#[must_use]
pub fn build_scoring_prompt(
    scenario: &SatScenario,
    trajectory: &SatTrajectory,
    persona_description: &str,
) -> String {
    let mut prompt = String::new();

    let _ = writeln!(prompt, "# Scenario: {}", scenario.meta.title);
    let _ = writeln!(prompt, "**ID:** {}", scenario.meta.id);
    let _ = writeln!(
        prompt,
        "**Persona:** {} ({})",
        scenario.meta.persona, persona_description
    );
    let _ = writeln!(prompt, "**Priority:** {}", scenario.meta.priority);
    let _ = writeln!(prompt);

    // Context
    let _ = writeln!(prompt, "## Context");
    let _ = writeln!(prompt, "{}", scenario.context);
    let _ = writeln!(prompt);

    // Expected satisfaction criteria
    let _ = writeln!(prompt, "## Expected Satisfaction Criteria");
    for criterion in &scenario.expected_satisfaction {
        let _ = writeln!(prompt, "- {criterion}");
    }
    let _ = writeln!(prompt);

    // Trajectory status
    let _ = writeln!(prompt, "## Execution Result");
    let _ = writeln!(prompt, "**Status:** {}", trajectory.status);
    let _ = writeln!(
        prompt,
        "**Duration:** {}ms",
        trajectory.performance.total_duration_ms
    );
    let _ = writeln!(prompt);

    // Step-by-step results
    let _ = writeln!(prompt, "## Step Results");
    for step in &trajectory.steps {
        let _ = writeln!(
            prompt,
            "### Step {} — {} [{}]",
            step.step_number, step.step_text, step.status
        );
        let _ = writeln!(prompt, "- **Action taken:** {}", step.action_taken);
        let _ = writeln!(prompt, "- **Duration:** {}ms", step.duration_ms);
        if let Some(ref summary) = step.page_summary {
            let _ = writeln!(prompt, "- **Page state:** {summary}");
        }
        if let Some(ref reason) = step.failure_reason {
            let _ = writeln!(prompt, "- **Failure reason:** {reason}");
        }
        if let Some(ref cat) = step.failure_category {
            let _ = writeln!(prompt, "- **Initial classification:** {cat}");
        }
        if let Some(ref before) = step.before_screenshot {
            let _ = writeln!(prompt, "- **Before screenshot:** {before}");
        }
        if let Some(ref after) = step.after_screenshot {
            let _ = writeln!(prompt, "- **After screenshot:** {after}");
        }
        let _ = writeln!(prompt);
    }

    // Edge cases
    if !scenario.edge_cases.is_empty() {
        let _ = writeln!(prompt, "## Known Edge Cases");
        for case in &scenario.edge_cases {
            let _ = writeln!(prompt, "- {case}");
        }
        let _ = writeln!(prompt);
    }

    let _ = writeln!(
        prompt,
        "Please evaluate this scenario execution and produce a JSON score."
    );

    prompt
}

// ---------------------------------------------------------------------------
// Response parsing (pure)
// ---------------------------------------------------------------------------

/// Parsed LLM scoring response (intermediate representation).
#[derive(Debug, Clone, serde::Deserialize)]
struct RawScoreResponse {
    score: u32,
    dimensions: RawDimensions,
    reasoning: String,
    #[serde(default)]
    findings: Vec<RawFinding>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct RawDimensions {
    functionality: u32,
    usability: u32,
    error_handling: u32,
    performance: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct RawFinding {
    step_number: u32,
    summary: String,
    detail: String,
    category: String,
    confidence: String,
    #[serde(default)]
    evidence: Vec<String>,
    severity: u8,
}

/// Parse a finding category string into the enum.
/// Defaults to `App` for unrecognized values — safer to over-report real bugs.
#[must_use]
pub fn parse_finding_category(s: &str) -> FindingCategory {
    match s.to_lowercase().trim() {
        "runner" => FindingCategory::Runner,
        "scenario" => FindingCategory::Scenario,
        // Default to App for unrecognized values — safer to over-report
        _ => FindingCategory::App,
    }
}

/// Parse a confidence level string into the enum.
/// Defaults to `Medium` for unrecognized values.
#[must_use]
pub fn parse_confidence_level(s: &str) -> ConfidenceLevel {
    match s.to_lowercase().trim() {
        "high" => ConfidenceLevel::High,
        "low" => ConfidenceLevel::Low,
        // Default to Medium for "medium", "med", or unrecognized values
        _ => ConfidenceLevel::Medium,
    }
}

/// Clamp a score to the 0-100 range.
#[must_use]
pub fn clamp_score(score: u32) -> u32 {
    score.min(100)
}

/// Clamp a severity value to the valid 1-5 range.
#[must_use]
pub fn clamp_severity(raw: u8) -> u8 {
    raw.clamp(1, 5)
}

/// Parse the raw LLM response text into a `SatScenarioScore`.
///
/// Extracts JSON from the response (handles markdown code fences),
/// validates score ranges, and normalizes finding categories.
///
/// # Errors
/// Returns `AppError::Sat` if the response cannot be parsed.
pub fn parse_score_response(
    response: &str,
    scenario_id: &str,
    persona: &str,
) -> Result<SatScenarioScore, AppError> {
    // Extract JSON from potential markdown code fences
    let json_str = extract_json(response);

    let raw: RawScoreResponse = serde_json::from_str(json_str).map_err(|e| {
        error!(
            "Failed to parse LLM score response for {scenario_id}: {e}. Response: {}",
            &response[..response.len().min(500)]
        );
        AppError::Sat(format!(
            "failed to parse score response for {scenario_id}: {e}"
        ))
    })?;

    let findings: Vec<SatFinding> = raw
        .findings
        .into_iter()
        .map(|f| SatFinding {
            scenario_id: scenario_id.to_string(),
            step_number: f.step_number,
            summary: f.summary,
            detail: f.detail,
            category: parse_finding_category(&f.category),
            confidence: parse_confidence_level(&f.confidence),
            evidence: f.evidence,
            severity: clamp_severity(f.severity),
        })
        .collect();

    Ok(SatScenarioScore {
        scenario_id: scenario_id.to_string(),
        persona: persona.to_string(),
        score: clamp_score(raw.score),
        dimensions: SatScoreDimensions {
            functionality: clamp_score(raw.dimensions.functionality),
            usability: clamp_score(raw.dimensions.usability),
            error_handling: clamp_score(raw.dimensions.error_handling),
            performance: clamp_score(raw.dimensions.performance),
        },
        reasoning: raw.reasoning,
        findings,
    })
}

/// Extract JSON from a response that may be wrapped in markdown code fences.
#[must_use]
pub fn extract_json(response: &str) -> &str {
    let trimmed = response.trim();

    // Try ```json ... ``` first
    if let Some(start) = trimmed.find("```json") {
        let after_fence = &trimmed[start + 7..];
        if let Some(end) = after_fence.find("```") {
            return after_fence[..end].trim();
        }
    }

    // Try ``` ... ```
    if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        if let Some(end) = after_fence.find("```") {
            return after_fence[..end].trim();
        }
    }

    // Try finding a JSON object directly
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return &trimmed[start..=end];
        }
    }

    trimmed
}

// ---------------------------------------------------------------------------
// Score aggregation (pure)
// ---------------------------------------------------------------------------

/// Compute the aggregate score from individual scenario scores.
///
/// Uses weighted average: critical scenarios count 3x, high 2x, medium/low 1x.
/// Returns 0 if there are no scores.
#[must_use]
pub fn aggregate_scores(scores: &[SatScenarioScore], scenarios: &[SatScenario]) -> u32 {
    if scores.is_empty() {
        return 0;
    }

    let mut weighted_sum: f64 = 0.0;
    let mut weight_total: f64 = 0.0;

    for scenario_score in scores {
        // Find matching scenario for priority-based weighting
        let weight = scenarios
            .iter()
            .find(|s| s.meta.id == scenario_score.scenario_id)
            .map_or(1.0, |s| match s.meta.priority {
                crate::models::sat::ScenarioPriority::Critical => 3.0,
                crate::models::sat::ScenarioPriority::High => 2.0,
                crate::models::sat::ScenarioPriority::Medium
                | crate::models::sat::ScenarioPriority::Low => 1.0,
            });

        weighted_sum += f64::from(scenario_score.score) * weight;
        weight_total += weight;
    }

    if weight_total == 0.0 {
        return 0;
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let result = (weighted_sum / weight_total).round() as u32;
    result.min(100)
}

/// Count findings by category.
#[must_use]
pub fn count_findings(findings: &[SatFinding]) -> FindingCounts {
    let app = findings
        .iter()
        .filter(|f| f.category == FindingCategory::App)
        .count();
    let runner = findings
        .iter()
        .filter(|f| f.category == FindingCategory::Runner)
        .count();
    let scenario = findings
        .iter()
        .filter(|f| f.category == FindingCategory::Scenario)
        .count();

    FindingCounts {
        app,
        runner,
        scenario,
        total: findings.len(),
    }
}

/// Build a `SatScoreResult` from individual scenario scores.
#[must_use]
pub fn build_score_result(
    run_id: &str,
    scored_at: &str,
    scenario_scores: Vec<SatScenarioScore>,
    scenarios: &[SatScenario],
    budget: &ScoringBudget,
) -> SatScoreResult {
    let aggregate = aggregate_scores(&scenario_scores, scenarios);

    let all_findings: Vec<SatFinding> = scenario_scores
        .iter()
        .flat_map(|s| s.findings.clone())
        .collect();

    let finding_counts = count_findings(&all_findings);

    let token_usage = TokenUsage {
        input_tokens: budget.input_tokens_used,
        output_tokens: budget.output_tokens_used,
    };

    SatScoreResult {
        run_id: run_id.to_string(),
        scored_at: scored_at.to_string(),
        scenario_scores,
        aggregate_score: aggregate,
        all_findings,
        finding_counts,
        token_usage,
        estimated_cost_dollars: estimate_cost(budget),
    }
}

// ---------------------------------------------------------------------------
// Report rendering (pure)
// ---------------------------------------------------------------------------

/// Render a human-readable markdown report from the score result.
#[must_use]
pub fn render_report(result: &SatScoreResult) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "# SAT Scoring Report");
    let _ = writeln!(out, "**Run:** {}", result.run_id);
    let _ = writeln!(out, "**Scored at:** {}", result.scored_at);
    let _ = writeln!(out, "**Aggregate Score:** {}/100", result.aggregate_score);
    let _ = writeln!(out);

    // Cost summary
    let _ = writeln!(out, "## Cost Summary");
    let _ = writeln!(out, "- Input tokens: {}", result.token_usage.input_tokens);
    let _ = writeln!(out, "- Output tokens: {}", result.token_usage.output_tokens);
    let _ = writeln!(
        out,
        "- Estimated cost: ${:.4}",
        result.estimated_cost_dollars
    );
    let _ = writeln!(out);

    // Findings summary
    let _ = writeln!(out, "## Findings Summary");
    let _ = writeln!(out, "| Category | Count |");
    let _ = writeln!(out, "|:---------|------:|");
    let _ = writeln!(out, "| App bugs | {} |", result.finding_counts.app);
    let _ = writeln!(
        out,
        "| Runner artifacts | {} |",
        result.finding_counts.runner
    );
    let _ = writeln!(
        out,
        "| Scenario issues | {} |",
        result.finding_counts.scenario
    );
    let _ = writeln!(out, "| **Total** | **{}** |", result.finding_counts.total);
    let _ = writeln!(out);

    // Per-scenario scores
    let _ = writeln!(out, "## Scenario Scores");
    let _ = writeln!(out);
    for ss in &result.scenario_scores {
        let _ = writeln!(out, "### {} (Persona: {})", ss.scenario_id, ss.persona);
        let _ = writeln!(out, "**Score:** {}/100", ss.score);
        let _ = writeln!(out);
        let _ = writeln!(out, "| Dimension | Score |");
        let _ = writeln!(out, "|:----------|------:|");
        let _ = writeln!(out, "| Functionality | {} |", ss.dimensions.functionality);
        let _ = writeln!(out, "| Usability | {} |", ss.dimensions.usability);
        let _ = writeln!(out, "| Error Handling | {} |", ss.dimensions.error_handling);
        let _ = writeln!(out, "| Performance | {} |", ss.dimensions.performance);
        let _ = writeln!(out);
        let _ = writeln!(out, "**Reasoning:** {}", ss.reasoning);
        let _ = writeln!(out);

        if !ss.findings.is_empty() {
            let _ = writeln!(out, "**Findings:**");
            for finding in &ss.findings {
                let _ = writeln!(
                    out,
                    "- [{}/{}] (step {}, severity {}) {}",
                    finding.category,
                    finding.confidence,
                    finding.step_number,
                    finding.severity,
                    finding.summary
                );
            }
            let _ = writeln!(out);
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Learnings (pure + I/O)
// ---------------------------------------------------------------------------

/// Build learning entries from a score result.
#[must_use]
pub fn build_learnings(result: &SatScoreResult) -> Vec<SatLearning> {
    result
        .all_findings
        .iter()
        .filter(|f| f.confidence == ConfidenceLevel::High)
        .map(|f| SatLearning {
            recorded_at: result.scored_at.clone(),
            run_id: result.run_id.clone(),
            scenario_id: Some(f.scenario_id.clone()),
            category: f.category,
            confidence: f.confidence,
            summary: f.summary.clone(),
        })
        .collect()
}

/// Load existing learnings from a YAML file, or return empty if not found.
///
/// # Errors
/// Returns `AppError::Sat` if the file exists but cannot be parsed.
pub fn load_learnings(path: &Path) -> Result<SatLearningsFile, AppError> {
    if !path.exists() {
        debug!(
            "Learnings file not found, starting fresh: {}",
            path.display()
        );
        return Ok(SatLearningsFile::default());
    }

    let content = std::fs::read_to_string(path).map_err(|e| {
        error!("Failed to read learnings file {}: {e}", path.display());
        AppError::Sat(format!("failed to read learnings: {e}"))
    })?;

    serde_yaml::from_str(&content).map_err(|e| {
        error!("Failed to parse learnings YAML: {e}");
        AppError::Sat(format!("learnings YAML parse error: {e}"))
    })
}

/// Write learnings to a YAML file (appends new learnings to existing ones).
///
/// # Errors
/// Returns `AppError::Sat` if the file cannot be written.
pub fn write_learnings(
    path: &Path,
    existing: &SatLearningsFile,
    new_learnings: &[SatLearning],
) -> Result<(), AppError> {
    let mut merged = existing.clone();
    merged.learnings.extend(new_learnings.iter().cloned());

    let yaml = serde_yaml::to_string(&merged).map_err(|e| {
        error!("Failed to serialize learnings: {e}");
        AppError::Sat(format!("learnings serialization error: {e}"))
    })?;

    crate::util::write_atomic(path, yaml.as_bytes()).map_err(|e| {
        error!("Failed to write learnings to {}: {e}", path.display());
        AppError::Sat(format!("failed to write learnings: {e}"))
    })?;

    info!(
        "Updated learnings file ({} total entries) at {}",
        merged.learnings.len(),
        path.display()
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// Score I/O
// ---------------------------------------------------------------------------

/// Write the score result atomically to `scores.json` in the run directory.
///
/// # Errors
/// Returns `AppError::Sat` if serialization or file write fails.
pub fn write_scores(result: &SatScoreResult, run_dir: &Path) -> Result<PathBuf, AppError> {
    let path = run_dir.join("scores.json");
    let json = serde_json::to_string_pretty(result).map_err(|e| {
        error!("Failed to serialize score result: {e}");
        AppError::Sat(format!("score result serialization error: {e}"))
    })?;
    crate::util::write_atomic(&path, json.as_bytes()).map_err(|e| {
        error!("Failed to write scores to {}: {e}", path.display());
        AppError::Sat(format!("failed to write scores: {e}"))
    })?;
    info!("Wrote scores to {}", path.display());
    Ok(path)
}

/// Write the human-readable report to `report.md` in the run directory.
///
/// # Errors
/// Returns `AppError::Sat` if the file cannot be written.
pub fn write_report(report: &str, run_dir: &Path) -> Result<PathBuf, AppError> {
    let path = run_dir.join("report.md");
    crate::util::write_atomic(&path, report.as_bytes()).map_err(|e| {
        error!("Failed to write report to {}: {e}", path.display());
        AppError::Sat(format!("failed to write report: {e}"))
    })?;
    info!("Wrote report to {}", path.display());
    Ok(path)
}

/// Read a run result from a run directory.
///
/// # Errors
/// Returns `AppError::Sat` if the file cannot be read or parsed.
pub fn read_run_result(run_dir: &Path) -> Result<SatRunResult, AppError> {
    let path = run_dir.join("run-result.json");
    let content = std::fs::read_to_string(&path).map_err(|e| {
        error!("Failed to read run result from {}: {e}", path.display());
        AppError::Sat(format!("failed to read run result: {e}"))
    })?;
    serde_json::from_str(&content).map_err(|e| {
        error!("Failed to parse run result JSON: {e}");
        AppError::Sat(format!("run result parse error: {e}"))
    })
}

/// Read a trajectory from a run directory.
///
/// # Errors
/// Returns `AppError::Sat` if the file cannot be read or parsed.
pub fn read_trajectory(run_dir: &Path, scenario_id: &str) -> Result<SatTrajectory, AppError> {
    let safe_id = super::sat_generate::sanitize_id_for_filename(scenario_id)
        .unwrap_or_else(|_| scenario_id.to_string());
    let path = run_dir.join(format!("trajectory-{safe_id}.json"));
    let content = std::fs::read_to_string(&path).map_err(|e| {
        error!("Failed to read trajectory from {}: {e}", path.display());
        AppError::Sat(format!("failed to read trajectory {scenario_id}: {e}"))
    })?;
    serde_json::from_str(&content).map_err(|e| {
        error!("Failed to parse trajectory JSON for {scenario_id}: {e}");
        AppError::Sat(format!("trajectory parse error for {scenario_id}: {e}"))
    })
}

// ---------------------------------------------------------------------------
// Full scoring pipeline
// ---------------------------------------------------------------------------

/// Look up the persona description for a scenario from loaded personas.
fn find_persona_description(
    scenario: Option<&SatScenario>,
    personas: &[(String, crate::models::sat::SatPersona)],
) -> String {
    scenario
        .and_then(|s| {
            personas.iter().find(|(name, p)| {
                p.name.to_lowercase().replace(' ', "-") == s.meta.persona || name == &s.meta.persona
            })
        })
        .map_or_else(
            || "Unknown persona".to_string(),
            |(_, p)| p.description.clone(),
        )
}

/// Score a single trajectory using the LLM judge. Returns the score and updated budget.
fn score_trajectory(
    trajectory: &SatTrajectory,
    scenario: Option<&SatScenario>,
    persona_desc: &str,
    system_prompt: &str,
    judge: &dyn LlmJudge,
    budget: &ScoringBudget,
) -> (Option<SatScenarioScore>, ScoringBudget) {
    let scoring_prompt = if let Some(s) = scenario {
        build_scoring_prompt(s, trajectory, persona_desc)
    } else {
        warn!(
            "Scenario file not found for {} — scoring from trajectory only",
            trajectory.scenario_id
        );
        format!(
            "Score this scenario execution:\n\nScenario ID: {}\nStatus: {}\nSteps: {}\n",
            trajectory.scenario_id,
            trajectory.status,
            trajectory.steps.len()
        )
    };

    match judge.score(system_prompt, &scoring_prompt) {
        Ok((response, usage)) => {
            let updated_budget = record_usage(budget, &usage);
            debug!(
                "LLM response for {} ({} input, {} output tokens)",
                trajectory.scenario_id, usage.input_tokens, usage.output_tokens
            );

            let persona_name =
                scenario.map_or_else(|| "unknown".to_string(), |s| s.meta.persona.clone());

            match parse_score_response(&response, &trajectory.scenario_id, &persona_name) {
                Ok(score) => {
                    info!(
                        "Scored {}: {}/100 ({} findings)",
                        trajectory.scenario_id,
                        score.score,
                        score.findings.len()
                    );
                    (Some(score), updated_budget)
                }
                Err(e) => {
                    error!("Failed to parse score for {}: {e}", trajectory.scenario_id);
                    (None, updated_budget)
                }
            }
        }
        Err(e) => {
            error!(
                "LLM scoring call failed for {}: {e}",
                trajectory.scenario_id
            );
            (None, budget.clone())
        }
    }
}

/// Run the full scoring pipeline for a completed SAT run.
///
/// 1. Read run result from `sat/runs/run-{id}/`
/// 2. Load scenarios and personas for context
/// 3. For each trajectory, build scoring prompt and call LLM judge
/// 4. Parse scores, classify findings, track budget
/// 5. Write `scores.json` atomically
/// 6. Write `report.md`
/// 7. Update `sat/learnings.yaml`
///
/// # Errors
/// Returns `AppError::Sat` on fatal errors (can't read run, budget exceeded).
/// Individual scenario scoring failures are logged and skipped.
pub fn score_run(
    config: &SatScoreConfig,
    run_id: &str,
    judge: &dyn LlmJudge,
) -> Result<SatScoreResult, AppError> {
    let run_dir = config.runs_dir.join(run_id);
    info!(
        "Starting SAT scoring for run {run_id} in {}",
        run_dir.display()
    );

    let run_result = read_run_result(&run_dir)?;
    let personas = crate::services::sat_generate::load_personas(&config.personas_dir)
        .unwrap_or_else(|e| {
            warn!("Failed to load personas: {e}");
            Vec::new()
        });
    let scenarios_dir = config.project_root.join("sat/scenarios");
    let scenarios =
        crate::services::sat_generate::load_scenarios(&scenarios_dir).unwrap_or_else(|e| {
            warn!("Failed to load scenarios: {e}");
            Vec::new()
        });

    let system_prompt = build_system_prompt();
    let mut budget = config.budget.clone();
    let mut scenario_scores = Vec::new();

    for trajectory in &run_result.trajectories {
        if trajectory.status == TrajectoryStatus::RunnerFailure && trajectory.steps.is_empty() {
            debug!(
                "Skipping scoring for {} — runner failure with no steps",
                trajectory.scenario_id
            );
            continue;
        }
        if is_budget_exceeded(&budget) {
            warn!(
                "Budget exceeded (${:.4}) — stopping scoring",
                estimate_cost(&budget)
            );
            break;
        }

        let scenario = scenarios
            .iter()
            .find(|s| s.meta.id == trajectory.scenario_id);
        let persona_desc = find_persona_description(scenario, &personas);
        let (score, updated_budget) = score_trajectory(
            trajectory,
            scenario,
            &persona_desc,
            &system_prompt,
            judge,
            &budget,
        );
        budget = updated_budget;
        if let Some(s) = score {
            scenario_scores.push(s);
        }
    }

    let scored_at = chrono::Utc::now().to_rfc3339();
    let score_result = build_score_result(run_id, &scored_at, scenario_scores, &scenarios, &budget);

    write_scores(&score_result, &run_dir)?;
    write_report(&render_report(&score_result), &run_dir)?;

    let new_learnings = build_learnings(&score_result);
    if !new_learnings.is_empty() {
        let existing = load_learnings(&config.learnings_path).unwrap_or_default();
        if let Err(e) = write_learnings(&config.learnings_path, &existing, &new_learnings) {
            warn!("Failed to update learnings: {e}");
        }
    }

    info!(
        "SAT scoring complete for {run_id}: aggregate {}/100, {} findings (${:.4})",
        score_result.aggregate_score,
        score_result.finding_counts.total,
        score_result.estimated_cost_dollars,
    );
    Ok(score_result)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::models::sat::{
        ConfidenceLevel, FindingCategory, SatPerformance, SatScenarioMeta, SatStepResult,
        ScenarioPriority, StepStatus, TrajectoryStatus,
    };

    // -- Budget tracking ------------------------------------------------------

    #[test]
    fn estimate_cost_zero_usage() {
        let budget = ScoringBudget::default();
        assert!((estimate_cost(&budget) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn estimate_cost_with_usage() {
        let budget = ScoringBudget {
            input_tokens_used: 10_000,
            output_tokens_used: 2_000,
            ..ScoringBudget::default()
        };
        // 10K input * 0.003/1K = 0.03
        // 2K output * 0.015/1K = 0.03
        let cost = estimate_cost(&budget);
        assert!((cost - 0.06).abs() < 0.001);
    }

    #[test]
    fn budget_not_exceeded_initially() {
        assert!(!is_budget_exceeded(&ScoringBudget::default()));
    }

    #[test]
    fn budget_exceeded_at_limit() {
        let budget = ScoringBudget {
            max_cost_dollars: 0.05,
            input_tokens_used: 10_000,
            output_tokens_used: 2_000,
            ..ScoringBudget::default()
        };
        assert!(is_budget_exceeded(&budget));
    }

    #[test]
    fn record_usage_accumulates() {
        let budget = ScoringBudget {
            input_tokens_used: 100,
            output_tokens_used: 50,
            ..ScoringBudget::default()
        };
        let usage = TokenUsage {
            input_tokens: 200,
            output_tokens: 100,
        };
        let updated = record_usage(&budget, &usage);
        assert_eq!(updated.input_tokens_used, 300);
        assert_eq!(updated.output_tokens_used, 150);
    }

    // -- JSON extraction ------------------------------------------------------

    #[test]
    fn extract_json_from_code_fence() {
        let response = "```json\n{\"score\": 85}\n```";
        assert_eq!(extract_json(response), "{\"score\": 85}");
    }

    #[test]
    fn extract_json_from_plain_fence() {
        let response = "Here is the result:\n```\n{\"score\": 85}\n```\nDone.";
        assert_eq!(extract_json(response), "{\"score\": 85}");
    }

    #[test]
    fn extract_json_bare_object() {
        let response = "  {\"score\": 85}  ";
        assert_eq!(extract_json(response), "{\"score\": 85}");
    }

    #[test]
    fn extract_json_with_surrounding_text() {
        let response = "The result is {\"score\": 85} and that's it.";
        assert_eq!(extract_json(response), "{\"score\": 85}");
    }

    // -- Finding category parsing ---------------------------------------------

    #[test]
    fn parse_finding_categories() {
        assert_eq!(parse_finding_category("app"), FindingCategory::App);
        assert_eq!(parse_finding_category("runner"), FindingCategory::Runner);
        assert_eq!(
            parse_finding_category("scenario"),
            FindingCategory::Scenario
        );
        assert_eq!(parse_finding_category("APP"), FindingCategory::App);
        assert_eq!(parse_finding_category("unknown"), FindingCategory::App); // default
    }

    #[test]
    fn parse_confidence_levels() {
        assert_eq!(parse_confidence_level("high"), ConfidenceLevel::High);
        assert_eq!(parse_confidence_level("medium"), ConfidenceLevel::Medium);
        assert_eq!(parse_confidence_level("med"), ConfidenceLevel::Medium);
        assert_eq!(parse_confidence_level("low"), ConfidenceLevel::Low);
        assert_eq!(parse_confidence_level("HIGH"), ConfidenceLevel::High);
        assert_eq!(parse_confidence_level("???"), ConfidenceLevel::Medium); // default
    }

    // -- Score parsing --------------------------------------------------------

    const VALID_SCORE_JSON: &str = r#"{
        "score": 72,
        "dimensions": {
            "functionality": 80,
            "usability": 65,
            "error_handling": 70,
            "performance": 75
        },
        "reasoning": "The feature mostly works but has some rough edges.",
        "findings": [
            {
                "step_number": 3,
                "summary": "Button does not respond on first click",
                "detail": "The create button requires a double-click to activate.",
                "category": "app",
                "confidence": "high",
                "evidence": ["screenshots/3-after.png"],
                "severity": 3
            },
            {
                "step_number": 0,
                "summary": "Screenshot capture missed timing",
                "detail": "Before-screenshot was taken after the action completed.",
                "category": "runner",
                "confidence": "medium",
                "evidence": [],
                "severity": 5
            }
        ]
    }"#;

    #[test]
    fn parse_valid_score_response() {
        let score = parse_score_response(VALID_SCORE_JSON, "test-01", "power-user").unwrap();
        assert_eq!(score.scenario_id, "test-01");
        assert_eq!(score.persona, "power-user");
        assert_eq!(score.score, 72);
        assert_eq!(score.dimensions.functionality, 80);
        assert_eq!(score.dimensions.usability, 65);
        assert_eq!(score.findings.len(), 2);
        assert_eq!(score.findings[0].category, FindingCategory::App);
        assert_eq!(score.findings[0].confidence, ConfidenceLevel::High);
        assert_eq!(score.findings[1].category, FindingCategory::Runner);
    }

    #[test]
    fn parse_score_response_in_code_fence() {
        let wrapped = format!("```json\n{VALID_SCORE_JSON}\n```");
        let score = parse_score_response(&wrapped, "test-02", "newbie").unwrap();
        assert_eq!(score.score, 72);
    }

    #[test]
    fn parse_score_response_clamps_values() {
        let json = r#"{
            "score": 150,
            "dimensions": {
                "functionality": 200,
                "usability": 0,
                "error_handling": 100,
                "performance": 50
            },
            "reasoning": "overflows",
            "findings": []
        }"#;
        let score = parse_score_response(json, "test-03", "user").unwrap();
        assert_eq!(score.score, 100);
        assert_eq!(score.dimensions.functionality, 100);
    }

    #[test]
    fn parse_score_response_invalid_json() {
        let result = parse_score_response("not json at all", "test-04", "user");
        assert!(result.is_err());
    }

    // -- Score aggregation ----------------------------------------------------

    fn make_scenario(id: &str, priority: ScenarioPriority) -> SatScenario {
        SatScenario {
            meta: SatScenarioMeta {
                id: id.to_string(),
                title: format!("Scenario {id}"),
                persona: "test".to_string(),
                priority,
                tags: Vec::new(),
                generated_from: None,
            },
            context: String::new(),
            steps: Vec::new(),
            expected_satisfaction: Vec::new(),
            edge_cases: Vec::new(),
        }
    }

    fn make_score(id: &str, score: u32) -> SatScenarioScore {
        SatScenarioScore {
            scenario_id: id.to_string(),
            persona: "test".to_string(),
            score,
            dimensions: SatScoreDimensions {
                functionality: score,
                usability: score,
                error_handling: score,
                performance: score,
            },
            reasoning: String::new(),
            findings: Vec::new(),
        }
    }

    #[test]
    fn aggregate_empty_scores() {
        assert_eq!(aggregate_scores(&[], &[]), 0);
    }

    #[test]
    fn aggregate_single_score() {
        let scores = vec![make_score("s1", 80)];
        let scenarios = vec![make_scenario("s1", ScenarioPriority::Medium)];
        assert_eq!(aggregate_scores(&scores, &scenarios), 80);
    }

    #[test]
    fn aggregate_weighted_by_priority() {
        let scores = vec![make_score("critical", 50), make_score("low", 100)];
        let scenarios = vec![
            make_scenario("critical", ScenarioPriority::Critical),
            make_scenario("low", ScenarioPriority::Low),
        ];
        // critical: 50 * 3 = 150, low: 100 * 1 = 100
        // total weight: 4, sum: 250, avg: 62.5 -> 63
        let result = aggregate_scores(&scores, &scenarios);
        assert_eq!(result, 63);
    }

    // -- Finding counts -------------------------------------------------------

    #[test]
    fn count_findings_by_category() {
        let findings = vec![
            SatFinding {
                scenario_id: "s1".into(),
                step_number: 1,
                summary: "Bug".into(),
                detail: String::new(),
                category: FindingCategory::App,
                confidence: ConfidenceLevel::High,
                evidence: Vec::new(),
                severity: 2,
            },
            SatFinding {
                scenario_id: "s1".into(),
                step_number: 2,
                summary: "Runner issue".into(),
                detail: String::new(),
                category: FindingCategory::Runner,
                confidence: ConfidenceLevel::Medium,
                evidence: Vec::new(),
                severity: 5,
            },
            SatFinding {
                scenario_id: "s2".into(),
                step_number: 0,
                summary: "Bad test".into(),
                detail: String::new(),
                category: FindingCategory::Scenario,
                confidence: ConfidenceLevel::Low,
                evidence: Vec::new(),
                severity: 4,
            },
            SatFinding {
                scenario_id: "s2".into(),
                step_number: 3,
                summary: "Another bug".into(),
                detail: String::new(),
                category: FindingCategory::App,
                confidence: ConfidenceLevel::High,
                evidence: Vec::new(),
                severity: 1,
            },
        ];

        let counts = count_findings(&findings);
        assert_eq!(counts.app, 2);
        assert_eq!(counts.runner, 1);
        assert_eq!(counts.scenario, 1);
        assert_eq!(counts.total, 4);
    }

    // -- Report rendering -----------------------------------------------------

    #[test]
    fn render_report_contains_key_sections() {
        let result = SatScoreResult {
            run_id: "run-test".into(),
            scored_at: "2026-03-26T00:00:00Z".into(),
            scenario_scores: vec![SatScenarioScore {
                scenario_id: "s1".into(),
                persona: "power-user".into(),
                score: 85,
                dimensions: SatScoreDimensions {
                    functionality: 90,
                    usability: 80,
                    error_handling: 85,
                    performance: 85,
                },
                reasoning: "Good overall experience.".into(),
                findings: Vec::new(),
            }],
            aggregate_score: 85,
            all_findings: Vec::new(),
            finding_counts: FindingCounts::default(),
            token_usage: TokenUsage {
                input_tokens: 5000,
                output_tokens: 1000,
            },
            estimated_cost_dollars: 0.03,
        };

        let report = render_report(&result);
        assert!(report.contains("# SAT Scoring Report"));
        assert!(report.contains("run-test"));
        assert!(report.contains("85/100"));
        assert!(report.contains("Cost Summary"));
        assert!(report.contains("Findings Summary"));
        assert!(report.contains("Scenario Scores"));
        assert!(report.contains("power-user"));
    }

    // -- Learnings ------------------------------------------------------------

    #[test]
    fn build_learnings_filters_high_confidence() {
        let result = SatScoreResult {
            run_id: "run-test".into(),
            scored_at: "2026-03-26T00:00:00Z".into(),
            scenario_scores: Vec::new(),
            aggregate_score: 0,
            all_findings: vec![
                SatFinding {
                    scenario_id: "s1".into(),
                    step_number: 1,
                    summary: "High confidence bug".into(),
                    detail: String::new(),
                    category: FindingCategory::App,
                    confidence: ConfidenceLevel::High,
                    evidence: Vec::new(),
                    severity: 2,
                },
                SatFinding {
                    scenario_id: "s1".into(),
                    step_number: 2,
                    summary: "Low confidence issue".into(),
                    detail: String::new(),
                    category: FindingCategory::Runner,
                    confidence: ConfidenceLevel::Low,
                    evidence: Vec::new(),
                    severity: 5,
                },
            ],
            finding_counts: FindingCounts {
                app: 1,
                runner: 1,
                scenario: 0,
                total: 2,
            },
            token_usage: TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
            },
            estimated_cost_dollars: 0.0,
        };

        let learnings = build_learnings(&result);
        assert_eq!(learnings.len(), 1);
        assert_eq!(learnings[0].summary, "High confidence bug");
    }

    // -- Prompt building ------------------------------------------------------

    #[test]
    fn scoring_prompt_includes_key_info() {
        let scenario = make_scenario("test-01", ScenarioPriority::High);
        let trajectory = SatTrajectory {
            scenario_id: "test-01".into(),
            scenario_file: "test-01.md".into(),
            started_at: "t1".into(),
            completed_at: "t2".into(),
            status: TrajectoryStatus::Completed,
            steps: vec![SatStepResult {
                step_number: 1,
                step_text: "Open the app".into(),
                status: StepStatus::Pass,
                action_taken: "Opened".into(),
                before_screenshot: None,
                after_screenshot: None,
                page_summary: Some("Home page loaded".into()),
                failure_reason: None,
                failure_category: None,
                duration_ms: 200,
                started_at: "t1".into(),
            }],
            performance: SatPerformance {
                total_duration_ms: 200,
                step_durations_ms: vec![200],
            },
        };

        let prompt = build_scoring_prompt(&scenario, &trajectory, "Expert developer");
        assert!(prompt.contains("test-01"));
        assert!(prompt.contains("Expert developer"));
        assert!(prompt.contains("Open the app"));
        assert!(prompt.contains("pass"));
        assert!(prompt.contains("Home page loaded"));
    }

    // -- Atomic writes (filesystem) -------------------------------------------

    #[test]
    fn write_and_read_scores() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path().join("run-test");
        std::fs::create_dir_all(&run_dir).unwrap();

        let result = SatScoreResult {
            run_id: "run-test".into(),
            scored_at: "2026-03-26T00:00:00Z".into(),
            scenario_scores: Vec::new(),
            aggregate_score: 75,
            all_findings: Vec::new(),
            finding_counts: FindingCounts::default(),
            token_usage: TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
            },
            estimated_cost_dollars: 0.0,
        };

        let path = write_scores(&result, &run_dir).unwrap();
        assert!(path.exists());
        assert_eq!(path.file_name().unwrap(), "scores.json");

        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: SatScoreResult = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.aggregate_score, 75);
    }

    #[test]
    fn write_and_read_report() {
        let tmp = tempfile::TempDir::new().unwrap();
        let run_dir = tmp.path().join("run-test");
        std::fs::create_dir_all(&run_dir).unwrap();

        let report = "# Test Report\nScore: 85/100\n";
        let path = write_report(report, &run_dir).unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("85/100"));
    }

    // -- Learnings I/O -------------------------------------------------------

    #[test]
    fn learnings_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("learnings.yaml");

        // Start empty
        let existing = load_learnings(&path).unwrap();
        assert!(existing.learnings.is_empty());

        // Add some learnings
        let new = vec![SatLearning {
            recorded_at: "2026-03-26T00:00:00Z".into(),
            run_id: "run-1".into(),
            scenario_id: Some("s1".into()),
            category: FindingCategory::App,
            confidence: ConfidenceLevel::High,
            summary: "Button is broken".into(),
        }];

        write_learnings(&path, &existing, &new).unwrap();

        // Read back
        let loaded = load_learnings(&path).unwrap();
        assert_eq!(loaded.learnings.len(), 1);
        assert_eq!(loaded.learnings[0].summary, "Button is broken");

        // Append more
        let more = vec![SatLearning {
            recorded_at: "2026-03-26T01:00:00Z".into(),
            run_id: "run-2".into(),
            scenario_id: Some("s2".into()),
            category: FindingCategory::Runner,
            confidence: ConfidenceLevel::High,
            summary: "WebDriver flaky".into(),
        }];

        write_learnings(&path, &loaded, &more).unwrap();

        let reloaded = load_learnings(&path).unwrap();
        assert_eq!(reloaded.learnings.len(), 2);
    }

    // -- System prompt --------------------------------------------------------

    #[test]
    fn system_prompt_is_non_empty_and_contains_schema() {
        let prompt = build_system_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("\"score\""));
        assert!(prompt.contains("\"findings\""));
        assert!(prompt.contains("\"category\""));
        assert!(prompt.contains("app"));
        assert!(prompt.contains("runner"));
        assert!(prompt.contains("scenario"));
    }

    // -- clamp_score ----------------------------------------------------------

    #[test]
    fn clamp_score_within_range() {
        assert_eq!(clamp_score(50), 50);
        assert_eq!(clamp_score(0), 0);
        assert_eq!(clamp_score(100), 100);
    }

    #[test]
    fn clamp_score_over_100() {
        assert_eq!(clamp_score(150), 100);
        assert_eq!(clamp_score(u32::MAX), 100);
    }
}
