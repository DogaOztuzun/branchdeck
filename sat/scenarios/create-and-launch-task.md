---
id: create-and-launch-task
title: Create a Task and Launch an Agent Run
persona: any
priority: high
tags: [task, run, agent, core-flow]
generated_from: docs/data-models.md
---

## Context
A developer wants to create a task (e.g., issue-fix or pr-shepherd) for a worktree and launch a Claude Code agent run to work on it.

## Steps
1. Open the Team Sidebar (task management panel)
2. Click the button to create a new task
3. In the Create Task modal, select the task type (issue-fix or pr-shepherd)
4. Optionally enter a PR number and description
5. Submit the task
6. Verify the task appears in the task list with "created" status
7. Launch a run for the task
8. Observe the RunTimeline showing status, duration, and cost updating in real-time
9. Watch the AgentBadge on the Claude tab showing the current tool and file being worked on

## Expected Satisfaction
- Task creation should be quick — just type and task type, no unnecessary fields
- The run timeline should provide real-time visibility into what the agent is doing
- Cost tracking should be visible so the developer knows spend in real-time

## Edge Cases
- Creating a task when one already exists for the worktree
- Network disconnection during an active run
- The agent session timing out (heartbeat failure after 120s)
