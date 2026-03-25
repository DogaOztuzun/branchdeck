---
id: github-api-unavailable
title: App Behavior When GitHub API Is Unavailable
persona: any
priority: medium
tags: [github, offline, error-handling, failure-mode]
generated_from: docs/architecture.md
---

## Context
The user is working in Branchdeck but GitHub's API is unreachable — either due to network issues, rate limiting, or no GitHub token configured. PR-related features should degrade gracefully.

## Steps
1. Disconnect from the network (or use a repo with no GitHub remote)
2. Navigate to a worktree that had a PR open
3. Observe how PR badges behave — they should show stale data or a clear "unavailable" state
4. Hover over a PR badge and check the tooltip still shows last-known data
5. Try to refresh PR status manually
6. Observe error feedback — it should explain GitHub is unavailable, not show a cryptic error
7. Reconnect to the network
8. Verify PR data refreshes automatically on the next poll cycle (15-60s)

## Expected Satisfaction
- GitHub being down should not break the rest of the app
- The user should know PR data is stale rather than seeing empty/broken badges
- Reconnection should be automatic without user intervention

## Edge Cases
- GitHub rate limit exceeded (403 response)
- GitHub token expired or revoked
- Repository has no GitHub remote (not a GitHub repo at all)
