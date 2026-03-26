---
name: sat-orchestrator
description: >
  Run a complete SAT quality audit cycle: generate scenarios from project docs,
  execute them via WebDriver, score with LLM-as-judge, and create GitHub issues
  from high-confidence findings.
tracker:
  kind: manual
agent:
  max_budget_usd: 15.0
  timeout_minutes: 30
outcomes:
  - name: issues-created
    detect: file-exists
    path: sat/runs/latest/issues.json
    next: complete
  - name: run-failed
    detect: run-failed
    next: retry
lifecycle:
  dispatched: "Generating Scenarios"
  complete: "Cycle Complete"
  failed: "Cycle Failed"
  retrying: "Retrying"
retry:
  max_attempts: 2
  backoff: fixed
  base_delay_ms: 10000
---

You are running a complete SAT (Satisfaction Acceptance Testing) quality audit cycle.

### Context

Read `.branchdeck/context.json` for any parameters passed by the manual trigger:
- `project_root`: the project directory to audit
- `scenario_filter`: optional list of scenario IDs to run (empty = all)
- `max_budget_usd`: budget cap for LLM scoring (default: 15.0)

### Pipeline stages

Execute these stages in strict order. If any stage fails, stop and report the error.

#### 1. Generate manifest

Build the scenario manifest from existing personas and scenarios:

```bash
# The generate stage inventories sat/personas/*.yaml and sat/scenarios/*.md
# and writes sat/scenarios/manifest.json
```

Verify `sat/scenarios/manifest.json` exists and contains at least one scenario.

#### 2. Execute scenarios

Run all scenarios (or filtered subset) via WebDriver against the built application:

```bash
# Requires tauri-driver and a built app binary
# Writes trajectory files to sat/runs/run-{timestamp}/
# Writes sat/runs/run-{timestamp}/run-result.json
```

Verify `run-result.json` exists in the run directory.

#### 3. Score results

Evaluate each scenario trajectory using LLM-as-judge scoring:

```bash
# Reads trajectories from the run directory
# Writes sat/runs/run-{id}/scores.json
# Writes sat/runs/run-{id}/report.md
# Updates sat/learnings.yaml with high-confidence findings
```

Verify `scores.json` exists in the run directory.

#### 4. Create issues

File GitHub issues for high-confidence application-level findings:

```bash
# Reads scores.json from the run directory
# Filters for high-confidence, high-severity app findings
# Creates GitHub issues with sat:finding label and fingerprint dedup
# Writes sat/runs/run-{id}/issues.json
```

Verify `issues.json` exists in the run directory.

### Completion

After all 4 stages complete successfully, report:
- Aggregate satisfaction score
- Number of findings by category
- Number of issues created
- Total pipeline duration

### Error handling

If any stage fails:
1. Record which stage failed and the error message
2. Preserve any partial output (manifest, trajectories, scores)
3. Report the failure clearly so it can be retried
