---
id: worktree-name-collision
title: Handle Worktree Name and Path Collisions
persona: confused-newbie
priority: medium
tags: [worktree, creation, collision, edge-case, error-handling]
generated_from: docs/data-models.md
---

## Context
A newbie tries to create a worktree but the name or path they chose conflicts with an existing worktree or branch. The live preview should catch this before they submit.

## Steps
1. Open the Add Worktree modal
2. Enter a name that matches an existing worktree
3. Observe the live preview showing a conflict warning (branch_exists or path_exists flag)
4. Note whether the create button is disabled or shows a warning
5. Change the name to something unique
6. Verify the preview now shows green/clear status
7. Create the worktree successfully

## Expected Satisfaction
- Conflicts should be caught in the preview, not after clicking create
- Error messages should explain what conflicts and suggest how to fix it
- The newbie should never end up in a broken state from a name collision

## Edge Cases
- Name that sanitizes to the same slug as an existing worktree (e.g., "my feature" vs "my-feature")
- Branch exists remotely but not locally
- Path exists on disk but isn't a git worktree
