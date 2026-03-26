---
description: Workflow definition authoring rules and WorkflowDef schema
globs: ["workflows/**/*", ".branchdeck/workflows/**/*"]
---

# Workflow Authoring

## WorkflowDef Schema (YAML, schema_version: 1)

Every workflow directory must contain:
- `workflow.yaml` — definition with all required fields
- `SKILL.md` — natural language instructions for the agent
- `context-template.md` (optional) — template for injecting runtime context

## Required Fields

```yaml
schema_version: 1
name: string                    # unique identifier
description: string             # human-readable purpose
trigger:
  type: enum                    # github-issue | github-pr | manual | post-merge | schedule | webhook
  filter: map                   # type-specific filter fields
execution:
  skill: path                   # relative to workflow dir
  max_turns: integer            # agent turn limit
  max_budget_usd: float         # cost cap
  timeout_minutes: integer      # hard timeout
outcomes:                       # ordered list, first match wins
  - name: string
    detect: enum                # file-exists | pr-created | ci-passing | run-failed | custom
    next: enum                  # complete | retry | review | custom-state
```

## Registry Merge Order

1. Embedded defaults (compiled into binary via `include_dir!`)
2. `~/.config/branchdeck/workflows/` (user global overrides)
3. `.branchdeck/workflows/` (project-local overrides)

Later layers override by workflow `name` field.

## Validation Rules

- Invalid definitions logged as warnings, not silently skipped
- Error messages include field path and valid values
- Invalid definitions don't prevent other valid ones from loading
