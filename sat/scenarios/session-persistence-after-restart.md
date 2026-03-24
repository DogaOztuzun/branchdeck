---
id: session-persistence-after-restart
title: Verify Session State Persists After App Restart
persona: power-user
priority: high
tags: [persistence, session, restart, state]
generated_from: docs/project-overview.md
---

## Context
A power user has been working across multiple repos and worktrees. They close and reopen Branchdeck and expect everything to be restored exactly as they left it.

## Steps
1. Add 2-3 repositories to the workspace
2. Create worktrees in different repos
3. Open multiple terminal tabs (shell and Claude)
4. Adjust panel sizes and sidebar visibility
5. Close Branchdeck completely
6. Reopen Branchdeck
7. Verify all repos are still listed in the sidebar
8. Verify the last active repo and worktree are selected
9. Verify window dimensions are restored
10. Check that panel layout preferences are restored

## Expected Satisfaction
- Reopening should feel like picking up exactly where they left off
- No "re-add your repos" friction on every launch
- The power user's customized layout should be preserved

## Edge Cases
- A repo directory was moved or deleted while Branchdeck was closed
- Config file becomes corrupted between launches
- First launch after an update that changes the config schema
