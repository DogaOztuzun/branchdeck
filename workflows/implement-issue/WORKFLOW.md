---
name: implement-issue
description: >
  Automatically implement fixes for GitHub issues labeled agent:implement.
  Creates a worktree, implements the fix, and creates a PR.
tracker:
  kind: github-issue
  filter:
    label: "agent:implement"
agent:
  max_budget_usd: 5.0
  timeout_minutes: 30
outcomes:
  - name: pr-created
    detect: pr-created
    next: complete
  - name: run-failed
    detect: run-failed
    next: retry
lifecycle:
  dispatched: "Implementing"
  complete: "PR Created"
  failed: "Failed"
  retrying: "Retrying"
retry:
  max_attempts: 2
  backoff: fixed
  base_delay_ms: 5000
---

You are implementing a fix for a GitHub issue.

### Get issue context

Read `.branchdeck/context.json` for the issue details:
- `repo`: the GitHub repo (e.g., "owner/repo")
- `number`: the issue number
- `title`: the issue title
- `body`: the issue description (may be null)
- `labels`: the issue labels

### Implementation steps

1. Read the issue details from context.json
2. Read the full issue on GitHub: `gh issue view <number> --repo <repo>`
3. Understand the project structure (read CLAUDE.md, relevant source files)
4. Implement the fix following the project's code standards
5. Run the project's check suite (see CLAUDE.md for commands)
6. Stage, commit, and create a PR:
   ```
   git add <affected files>
   git commit -m "fix: <concise summary of the fix>

   Closes #<number>"
   git push -u origin HEAD
   gh pr create --title "fix: <title>" --body "Fixes #<number>

   ## Summary
   <brief description of what was changed and why>

   ## Test plan
   - [ ] <testing steps>"
   ```
7. Verify CI passes: `gh pr checks <pr_number> --repo <repo> --watch`

### Guidelines

- Keep changes minimal and focused on the issue
- Follow existing code patterns and conventions
- Include tests if the project has a test suite
- Reference the issue number in the commit and PR
