# WorkflowDef Schema Specification

**Schema version:** 1

WorkflowDef files are YAML documents that define how Branchdeck discovers, triggers, executes, and evaluates autonomous agent workflows. They live in `.branchdeck/workflows/` (project-local) or `~/.config/branchdeck/workflows/` (global defaults). Project-local definitions override global ones on name conflict.

## Full Schema

```yaml
# Required
schema_version: 1                    # Must be 1
name: string                         # Unique identifier (e.g. "pr-shepherd")
description: string                  # Human-readable purpose

# Required: what fires this workflow
trigger:
  type: enum                         # See Trigger Types below
  filter: map                        # Optional, type-specific key/value pairs

# Required: context generation
context:
  template: string                   # Path to context template (relative to workflow dir)
  output: string                     # Filename written to worktree/.branchdeck/

# Required: agent execution config
execution:
  skill: string                      # Path to skill directory (relative to workflow dir)
  max_turns: integer                 # Optional: agent turn limit
  max_budget_usd: float              # Optional: cost cap in USD
  timeout_minutes: integer           # Optional: hard timeout
  allowed_directories: list<string>  # Optional: worktree isolation paths

# Required: ordered list of outcome checks (first match wins)
outcomes:
  - name: string                     # Outcome identifier
    detect: enum                     # See Outcome Detectors below
    path: string                     # Optional: for file-exists detector
    next: enum                       # See Outcome Actions below

# Optional: custom status display names for the UI
lifecycle:
  dispatched: string
  complete: string
  failed: string
  retrying: string

# Optional: retry configuration
retry:
  max_attempts: integer              # Must be >= 1
  backoff: enum                      # See Backoff Strategies below
  base_delay_ms: integer             # Must be > 0
```

## Trigger Types

| Value | Description |
|-------|-------------|
| `github-issue` | New or updated GitHub issue |
| `github-pr` | New or updated GitHub pull request |
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

```yaml
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
```

## Validation Rules

- `schema_version` must be `1`
- `name` and `description` must be non-empty strings
- `trigger.type` must be a valid trigger type
- `context.template` and `context.output` must be non-empty
- `execution.skill` must be non-empty
- `outcomes` must contain at least one entry, each with a non-empty `name`
- `retry.max_attempts` must be >= 1 if retry is specified
- `retry.base_delay_ms` must be > 0 if retry is specified

Validation errors include the field path (e.g. `outcomes[0].name`, `retry.max_attempts`) and valid values where applicable.
