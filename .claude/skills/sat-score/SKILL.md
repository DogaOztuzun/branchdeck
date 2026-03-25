---
name: sat-score
description: >
  Score user satisfaction from SAT run trajectories and screenshots.
  Reads trajectory JSON and screenshots from sat/runs/run-{timestamp}/,
  personas from sat/personas/, scenarios from sat/scenarios/. Produces
  scores.json, report.md, and updates learnings.yaml.
  Use when user says "score sat run", "sat score", "evaluate satisfaction",
  or "grade the sat results".
---

# SAT Satisfaction Scorer

You evaluate user satisfaction by analyzing trajectory data and screenshots from a SAT run. For each scenario, you score satisfaction through each persona's behavioral lens, identify UX issues, and produce an actionable report.

## Arguments

The user may provide:
- **run** — path to a specific run directory (default: most recent `sat/runs/run-*`)
- **scenario** — score only a specific scenario (default: all trajectories in the run)

## Step 1: Load Inputs

### Find the run directory

If no `run` argument provided, find the most recent run:
```bash
ls -dt sat/runs/run-*/ | head -1
```

If no run directories exist, STOP: "No SAT runs found. Run `/sat-run` first."

Set `RUN_DIR` to the selected run directory.

### Load trajectories

Read all `trajectory-*.json` files from `RUN_DIR`. Each trajectory has:
- `scenario_id`, `status`, `steps[]` (with `step_text`, `status`, `action_taken`, `failure_reason`, `before_screenshot`, `after_screenshot`, `page_summary`)

If no trajectory files exist, STOP: "No trajectory files in `RUN_DIR`."

### Load personas

Read all `sat/personas/*.yaml` files. Each persona has:
- `name`, `description`, `frustration_threshold`, `technical_level`, `satisfaction_criteria[]`, `behaviors[]`

### Load scenarios

For each trajectory, read the matching scenario from `sat/scenarios/{scenario_id}.md` to get:
- `## Expected Satisfaction` — what the persona should feel
- `## Edge Cases` — what could go wrong
- `## Context` — the user's situation

### Load previous scores (if any)

Check if a previous `scores.json` exists in any earlier run directory. If found, read `overall_score` to calculate delta.

### Load learnings

Read `sat/learnings.yaml`. Use `fixed_issues` and `false_positives` to avoid re-reporting known items.

## Step 2: Score Each Trajectory

For each trajectory, analyze through **every persona's lens**:

### Screenshot Analysis

For each step in the trajectory:
1. Read the **before screenshot** — what the user sees before acting
2. Read the **after screenshot** — what changed after the action
3. Consider the `page_summary` for additional context
4. Note whether the step succeeded or failed, and why

### Persona Scoring

For each persona, evaluate the trajectory against their `satisfaction_criteria` and `behaviors`:

- **Confused Newbie**: Is every action obvious? Are errors explained? Would they give up?
- **Power User**: Is the flow efficient? Are there unnecessary clicks? Is it fast?
- **Accessibility User**: Is everything keyboard-reachable? Are status changes announced?

Score each scenario 0-100 per persona based on:
- Step success rate (mechanical: did steps work?)
- UX quality visible in screenshots (visual: does it look right?)
- Persona-specific satisfaction criteria alignment
- Edge case handling visible in the flow

### Issue Identification

For each problem found, create an issue entry:
```json
{
  "severity": "high | medium | low",
  "type": "ux-friction | missing-feedback | accessibility | performance | error-handling",
  "scenario_id": "<id>",
  "step": <step_number>,
  "description": "<what's wrong>",
  "persona": "<which persona is most affected>",
  "persona_impact": "<how this persona experiences the problem>",
  "classification": "app_issue | runner_issue | scenario_issue"
}
```

**Classification rules:**
- `app_issue` — real UX problem in the application
- `runner_issue` — the WebDriver bridge couldn't execute the step (not an app problem)
- `scenario_issue` — the scenario itself is flawed (hallucinated feature, ambiguous step)

**Severity rules:**
- `high` — blocks the user's goal or causes data loss
- `medium` — slows the user down or causes confusion
- `low` — cosmetic or minor annoyance

**Check against learnings:** If an issue matches a `fixed_issues` entry, skip it. If it matches a `false_positives` pattern, classify as `runner_issue` instead.

## Step 3: Write scores.json

Write to `{RUN_DIR}/scores.json` following the architecture contract:

```json
{
  "run_id": "<run directory name>",
  "timestamp": "<current ISO timestamp>",
  "scenarios_scored": <count>,
  "overall_score": <weighted average 0-100>,
  "previous_score": <from previous run or null>,
  "delta": <current - previous or null>,
  "issues": [ ... ],
  "persona_scores": {
    "confused-newbie": <0-100>,
    "power-user": <0-100>,
    "accessibility-user": <0-100>
  },
  "scenario_scores": {
    "<scenario-id>": {
      "score": <0-100>,
      "step_pass_rate": <0.0-1.0>,
      "issues_found": <count>,
      "persona_scores": {
        "confused-newbie": <0-100>,
        "power-user": <0-100>,
        "accessibility-user": <0-100>
      }
    }
  }
}
```

**Overall score calculation:**
- Weight `app_issue` findings heavily (each high=-10, medium=-5, low=-2)
- Start from 100, subtract issue penalties
- Floor at 0
- `runner_issue` and `scenario_issue` do NOT reduce the score

## Step 4: Write report.md

Write a human-readable report to `{RUN_DIR}/report.md`:

```markdown
# SAT Satisfaction Report

**Run:** {run_id}
**Date:** {timestamp}
**Scenarios scored:** {count}
**Overall score:** {score}/100 (delta: {delta})

## Persona Scores

| Persona | Score | Key Finding |
|---------|-------|-------------|
| Confused Newbie | {score} | {one-line summary} |
| Power User | {score} | {one-line summary} |
| Accessibility User | {score} | {one-line summary} |

## Issues ({count})

### High Severity

- **{scenario_id} step {n}**: {description}
  - Persona impact ({persona}): {persona_impact}

### Medium Severity
...

### Low Severity
...

## Scenario Details

### {scenario_title} — {score}/100
- Steps: {passed}/{total} passed
- Issues: {list}
- Screenshots: {link to before/after of worst step}

## Runner Issues (not app problems)

{list of runner_issue classified items — these need sat-run improvements, not app fixes}

## Scenario Issues (need regeneration)

{list of scenario_issue classified items — these need sat-generate improvements}
```

## Step 5: Update learnings.yaml

Read the current `sat/learnings.yaml` and append new findings:

- **`weak_areas`**: If a scenario area has 2+ issues, add or update the weak area entry
- **`scenario_feedback`**: For each scenario, add a quality assessment:
  - `good` if the scenario found real app issues
  - `bad` if the scenario was about a nonexistent feature or produced only runner issues

Do NOT modify `fixed_issues` or `false_positives` — those are human-curated.

Write the updated `sat/learnings.yaml`.

## Step 6: Present Summary

Report to the user:
- Overall score with delta
- Persona score table
- Top 3 highest-severity issues with one-line descriptions
- Count of app issues vs runner issues vs scenario issues
- Recommendation: what to fix first

If score > 80: "The app is in good shape. Focus on the medium-severity items."
If score 60-80: "Several UX friction points need attention. Start with the high-severity issues."
If score < 60: "Significant satisfaction gaps. The high-severity issues are blocking core flows."
