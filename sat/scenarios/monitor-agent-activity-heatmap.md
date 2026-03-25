---
id: monitor-agent-activity-heatmap
title: Monitor Agent File Activity via FileGrid Heatmap
persona: power-user
priority: medium
tags: [agent, monitoring, heatmap, file-activity]
generated_from: docs/component-inventory.md
---

## Context
A developer has a Claude Code agent running and wants to see which files the agent is reading and writing in real-time, using the FileGrid heatmap visualization.

## Steps
1. Start an agent run on a worktree
2. Open the Team Sidebar to view the FileGrid heatmap
3. Observe colored dots appearing as the agent accesses files
4. Notice dot sizes reflecting access count and modification status
5. Hover over a dot to see the tooltip with file path, state, tool name, and access count
6. Watch dots update in real-time as the agent works through files
7. After the run completes, observe the final state showing which files were most touched

## Expected Satisfaction
- The heatmap should update in real-time without manual refresh
- Dot sizing and coloring should make hot files immediately obvious
- The power user should be able to quickly identify which files the agent focused on

## Edge Cases
- Agent accesses 100+ files (does the heatmap become unreadable?)
- Multiple agents accessing the same file simultaneously
- Files accessed briefly vs. files modified extensively should be visually distinct
