# WorkflowDef Schema Specification

**Format:** Markdown file with YAML frontmatter (`WORKFLOW.md`)

WorkflowDef files define how Branchdeck discovers, triggers, executes, and evaluates autonomous agent workflows. The YAML frontmatter contains runtime configuration (parsed by the daemon). The markdown body is the agent prompt — passed to the agent as-is, supporting Liquid-style template variables.

Based on [OpenAI Symphony's WORKFLOW.md format](https://github.com/openai/symphony/blob/main/SPEC.md) with Branchdeck extensions for outcome detection, lifecycle display, and retry policies.

## File Location

- Project-local: `.branchdeck/workflows/<name>/WORKFLOW.md`
- Global defaults: `~/.config/branchdeck/workflows/<name>/WORKFLOW.md`
- Registry merges both; project-local wins on name conflict.

## Full Schema

```markdown
---
# === Identity (Branchdeck) ===
name: string                              # Required. Unique workflow identifier.
description: string                       # Optional. Human-readable purpose.

# === Tracker (Symphony-compatible) ===
tracker:                                  # Required. What triggers this workflow.
  kind: enum                              # Required. See Tracker Kinds below.
  filter: map                             # Optional. Kind-specific key/value filters.
  project_slug: string                    # Optional. For Linear tracker compatibility.
  active_states: list<string>             # Optional. For Linear tracker compatibility.
  terminal_states: list<string>           # Optional. For Linear tracker compatibility.

# === Polling (Symphony-compatible) ===
polling:                                  # Optional.
  interval_ms: integer                    # Default: 30000. How often to check for triggers.

# === Workspace (Symphony-compatible) ===
workspace:                                # Optional.
  root: string                            # Override for worktree location. Supports ~ expansion.

# === Hooks (Symphony-compatible) ===
hooks:                                    # Optional. Shell scripts at lifecycle points.
  after_create: string                    # After worktree/workspace creation.
  before_run: string                      # Before each agent attempt.
  after_run: string                       # After each attempt.
  before_remove: string                   # Before worktree deletion.
  timeout_ms: integer                     # Hook timeout. Default: 60000.

# === Agent (Symphony-compatible + Branchdeck extensions) ===
agent:                                    # Optional.
  max_concurrent_agents: integer          # Max parallel runs for this workflow type.
  max_turns: integer                      # Agent turn limit.
  max_budget_usd: float                   # Cost cap in USD (FR38). Must be >= 0.
  timeout_minutes: integer                # Hard timeout.
  allowed_directories: list<string>       # Worktree isolation paths (FR40).

# === Outcomes (Branchdeck extension) ===
outcomes:                                 # Optional. Ordered list, first match wins.
  - name: string                          # Outcome identifier.
    detect: enum                          # See Outcome Detectors below.
    path: string                          # Required when detect is 'file-exists'.
    next: enum                            # See Outcome Actions below.

# === Lifecycle (Branchdeck extension) ===
lifecycle:                                # Optional. UI status display names (FR33).
  dispatched: string
  complete: string
  failed: string
  retrying: string

# === Retry (Branchdeck extension) ===
retry:                                    # Optional.
  max_attempts: integer                   # Must be >= 1.
  backoff: enum                           # See Backoff Strategies below.
  base_delay_ms: integer                  # Must be >= 1.
---

Agent prompt goes here. Supports {{ template_variables }}.

The daemon does not interpret the prompt body — it passes it to the agent runner as-is.
```

## Tracker Kinds

| Value | Description |
|-------|-------------|
| `github-issue` | New or updated GitHub issue |
| `github-pr` | New or updated GitHub pull request |
| `linear` | Linear issue tracker (Symphony compatibility) |
| `manual` | User-initiated via UI or CLI |
| `post-merge` | After a PR is merged |
| `schedule` | Cron-based schedule |
| `webhook` | External HTTP webhook |

## Outcome Detectors

| Value | Description |
|-------|-------------|
| `file-exists` | Check if a file exists at `path` in the worktree |
| `pr-created` | A pull request was created |
| `ci-passing` | CI checks are all passing |
| `run-failed` | The agent run failed or errored |
| `custom` | Custom detection logic (future extension) |

## Outcome Actions

| Value | Description |
|-------|-------------|
| `complete` | Mark workflow as successfully completed |
| `retry` | Retry the workflow (respects retry config) |
| `review` | Mark as needing human review |
| `custom-state` | Transition to a custom lifecycle state |

## Backoff Strategies

| Value | Description |
|-------|-------------|
| `exponential` | Delay doubles each attempt: `base_delay_ms * 2^(attempt-1)` |
| `fixed` | Constant delay: `base_delay_ms` every attempt |

## Example: PR Shepherd

```markdown
---
name: pr-shepherd
description: Fix failing CI on pull requests

tracker:
  kind: github-pr
  filter:
    ci_status: failure

polling:
  interval_ms: 30000

hooks:
  before_run: echo "Starting PR shepherd"

agent:
  max_concurrent_agents: 1
  max_turns: 25
  max_budget_usd: 5.0
  timeout_minutes: 30

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

CI is failing. Analyze the failures and fix them.

## Instructions
1. Read the CI failure logs
2. Identify root cause
3. Create a fix plan
4. Implement and push
```

## Example: Symphony-Compatible (Linear)

```markdown
---
name: linear-worker
tracker:
  kind: linear
  project_slug: "my-project-abc123"
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

You are working on Linear ticket {{ issue.identifier }}.

Title: {{ issue.title }}
Description: {{ issue.description }}
```

## Validation Rules

- `name` must be a non-empty, non-whitespace string
- `tracker.kind` must be a valid tracker kind
- `agent.max_budget_usd` must be a finite non-negative number if provided
- `outcomes[].path` is required when `detect` is `file-exists`
- `outcomes[].name` must be non-empty
- `retry.max_attempts` must be >= 1 if retry is specified
- `retry.base_delay_ms` must be >= 1 if retry is specified

Validation errors include the field path (e.g. `outcomes[0].path`, `retry.max_attempts`).
