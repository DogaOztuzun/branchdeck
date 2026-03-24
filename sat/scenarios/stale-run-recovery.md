---
id: stale-run-recovery
title: Recovery from a Crashed or Stale Agent Run
persona: any
priority: high
tags: [recovery, crash, stale, run, failure-mode]
generated_from: docs/architecture.md
---

## Context
The application crashed or was force-killed while an agent run was active. On restart, the stale run should be detected and recovered gracefully rather than leaving the user in a broken state.

## Steps
1. Start an agent run on a worktree
2. Force-quit the application (simulate crash)
3. Reopen Branchdeck
4. Observe the startup recovery scanning for orphaned run.json files
5. Verify the stale run is detected and marked as failed
6. Check the task status has been updated to reflect the failure
7. Verify the user can retry the task without manual cleanup

## Expected Satisfaction
- Recovery should be automatic — no manual file deletion needed
- The user should see a clear indication of what happened (run failed due to crash)
- Retrying should work immediately without residual state issues

## Edge Cases
- Multiple stale runs across different worktrees
- A run.json file exists but the corresponding task.md was deleted
- The sidecar process is still alive (zombie) after the main app crashed
