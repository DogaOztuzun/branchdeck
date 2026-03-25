---
id: view-pr-status-from-sidebar
title: View PR Status Badges and Tooltip from Sidebar
persona: any
priority: high
tags: [pr, github, monitoring, sidebar]
generated_from: docs/component-inventory.md
---

## Context
A developer has worktrees with open PRs on GitHub. They want to quickly see the status of their PRs — CI checks, reviews, and merge readiness — without leaving Branchdeck.

## Steps
1. Look at the worktree list in the left sidebar
2. Identify PR badges next to worktrees that have open PRs (state color, review icon, checks icon)
3. Hover over a PR badge to see the PrTooltip
4. In the tooltip, verify PR number, title, state, review status, and CI checks are shown
5. Click the GitHub link in the tooltip to open the PR in a browser
6. Observe that inactive repos refresh PR data every 60 seconds and active PRs every 15 seconds

## Expected Satisfaction
- PR status should be visible at a glance without any extra clicks
- The tooltip should provide enough detail to decide if action is needed
- The developer shouldn't need to context-switch to GitHub for routine status checks

## Edge Cases
- A worktree has no associated PR (no badge should appear)
- GitHub API is unavailable or rate-limited
- A PR has 10+ CI checks (tooltip should handle overflow gracefully)
