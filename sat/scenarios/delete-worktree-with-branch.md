---
id: delete-worktree-with-branch
title: Delete a Worktree with Optional Branch Deletion
persona: confused-newbie
priority: medium
tags: [worktree, deletion, destructive, confirmation]
generated_from: docs/component-inventory.md
---

## Context
A newbie has finished working on a feature branch and wants to clean up by deleting the worktree. They need to understand the consequences — especially whether the branch will also be deleted.

## Steps
1. Right-click on a worktree in the sidebar
2. Select "Delete Worktree" from the context menu
3. Observe the DeleteWorktreeDialog confirmation
4. Read the confirmation message about what will be deleted
5. Notice the optional checkbox to also delete the associated branch
6. Leave the branch deletion unchecked and confirm deletion
7. Verify the worktree disappears from the sidebar
8. Verify the branch still exists (visible in branch list)

## Expected Satisfaction
- The confirmation dialog should clearly explain what "delete worktree" means (files removed, not just hidden)
- The branch deletion option should be clearly separate and defaulted to off (safe default)
- The newbie should not accidentally delete a branch they still need

## Edge Cases
- Attempting to delete the main worktree (should be prevented)
- Deleting a worktree that has uncommitted changes
- Deleting a worktree while an agent run is active on it
- Deleting a worktree that has an open PR
