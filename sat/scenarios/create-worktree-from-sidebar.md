---
id: create-worktree-from-sidebar
title: Create a New Worktree from the Sidebar
persona: any
priority: high
tags: [worktree, creation, core-flow]
generated_from: docs/component-inventory.md
---

## Context
A developer has a repo added and wants to start a new feature branch in a separate worktree. They use the Add Worktree modal to create one.

## Steps
1. Right-click on a repository in the left sidebar
2. Select the option to create a new worktree
3. Enter a name for the worktree in the modal input
4. Observe the live preview showing the sanitized name, branch name, and worktree path
5. Select a base branch from the dropdown
6. Click the create button
7. Verify the new worktree appears under the repo in the sidebar
8. Click on the new worktree and confirm a terminal opens in that directory

## Expected Satisfaction
- The live preview (200ms debounce) should show exactly what will be created before committing
- Conflict detection should warn if the branch or path already exists
- The worktree should be immediately usable after creation

## Edge Cases
- Name input contains characters that need sanitization (spaces, special chars)
- The base branch has diverged significantly from remote
- Creating a worktree when disk space is low
