---
id: open-terminal-tabs
title: Open Shell and Claude Terminal Tabs
persona: power-user
priority: high
tags: [terminal, tabs, core-flow]
generated_from: docs/component-inventory.md
---

## Context
A power user wants to work in parallel — a shell tab for running commands and a Claude Code tab for agent-assisted work, both in the same worktree.

## Steps
1. Select a worktree in the sidebar
2. Open the tab dropdown in the terminal area
3. Click "New Terminal" (or use Ctrl+Shift+T)
4. Verify a shell terminal opens in the worktree directory
5. Open the tab dropdown again
6. Click "New Claude" (or use Ctrl+Shift+A)
7. Verify a Claude Code session starts in the worktree directory
8. Switch between tabs and confirm each retains its state
9. Close one tab and verify the other remains active

## Expected Satisfaction
- Keyboard shortcuts should work immediately without configuration
- Tab switching should be instant with no visible re-render
- The power user appreciates that both shell and Claude tabs share the same worktree context

## Edge Cases
- Opening many tabs (10+) and checking if the tab bar handles overflow
- Closing the last tab in a worktree
- Opening a Claude tab when no API key is configured
