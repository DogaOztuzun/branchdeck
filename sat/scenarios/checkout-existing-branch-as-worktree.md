---
id: checkout-existing-branch-as-worktree
title: Checkout an Existing Branch as a New Worktree
persona: power-user
priority: medium
tags: [worktree, branch, checkout]
generated_from: docs/component-inventory.md
---

## Context
A developer wants to check out an existing remote branch as a local worktree for review or continued work, rather than creating a brand new branch.

## Steps
1. Right-click on a repository in the sidebar
2. Select the option to checkout a branch as worktree
3. In the BranchWorktreeModal, observe the list of available branches
4. Use the search/filter to find a specific branch
5. Notice remote branches are labeled and branches already in use show an "in-use" badge
6. Select a remote branch
7. Confirm creation and verify the worktree appears in the sidebar
8. Open a terminal in the new worktree and verify the correct branch is checked out

## Expected Satisfaction
- Branch search should be fast and responsive, even with hundreds of branches
- The in-use badges should prevent accidentally creating duplicate worktrees
- The derived worktree name should be sensible without manual editing

## Edge Cases
- Searching for a branch that doesn't exist
- Selecting a branch that another worktree already has checked out
- The remote branch has been force-pushed since last fetch
