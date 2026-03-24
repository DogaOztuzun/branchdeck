---
id: view-git-file-changes
title: View Git File Changes in the Changes Sidebar
persona: confused-newbie
priority: medium
tags: [git, changes, status, sidebar]
generated_from: docs/component-inventory.md
---

## Context
A newbie developer has been editing files and wants to see what has changed before committing. They open the Changes Sidebar to review modified files.

## Steps
1. Make some edits in a terminal (or via an agent run)
2. Open the Changes Sidebar using the toggle in the TopBar
3. Observe the list of modified files with status badges (M for modified, A for added, D for deleted)
4. Verify each file shows a color-coded status indicator
5. Check that file paths are readable and not truncated confusingly

## Expected Satisfaction
- The status badges (M/A/D/R/C) should be self-explanatory or have tooltips explaining them
- Color coding should clearly distinguish between different states
- The newbie should understand at a glance which files changed and how

## Edge Cases
- A worktree with hundreds of changed files
- Files in deeply nested directories (long paths)
- Renamed files showing both old and new paths
- Conflicted files after a merge
