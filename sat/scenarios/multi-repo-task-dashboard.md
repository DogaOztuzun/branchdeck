---
id: multi-repo-task-dashboard
title: View Tasks Across Multiple Repos in Task Dashboard
persona: power-user
priority: medium
tags: [dashboard, task, multi-repo, overview]
generated_from: docs/component-inventory.md
---

## Context
A power user is managing work across 3-4 repos simultaneously. They want a single view showing all tasks across all worktrees, sorted by status priority.

## Steps
1. Add multiple repositories to the workspace
2. Create tasks in worktrees across different repos
3. Open the Task Dashboard from the TopBar toggle
4. Verify all tasks from all repos appear in a unified list
5. Check that tasks are sorted by status priority (running > blocked > created > succeeded)
6. Identify which repo/worktree each task belongs to
7. Click on a task and verify it navigates to the correct worktree context

## Expected Satisfaction
- The dashboard should be the single place to see everything happening across repos
- Status-based sorting should surface the tasks that need attention first
- The power user should be able to triage across repos without switching context

## Edge Cases
- One repo has 50+ tasks (pagination or scrolling behavior)
- A task references a worktree that has since been deleted
- Dashboard updates while a run status changes in the background
