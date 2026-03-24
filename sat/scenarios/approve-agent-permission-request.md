---
id: approve-agent-permission-request
title: Approve or Deny an Agent Permission Request
persona: any
priority: high
tags: [permissions, agent, approval, security]
generated_from: docs/data-models.md
---

## Context
A Claude Code agent run is active and the agent needs permission to execute a potentially dangerous tool (e.g., file write, bash command). The user must approve or deny within 300 seconds before auto-deny.

## Steps
1. While an agent run is active, wait for a permission request to appear
2. Observe the ApprovalDialog showing the tool name and command details
3. Read the command to understand what the agent wants to do
4. Click "Approve" to allow the action
5. Verify the run continues and the agent proceeds with the approved tool
6. Wait for another permission request and click "Deny"
7. Verify the run handles the denial gracefully (continues with alternative or stops)

## Expected Satisfaction
- Permission requests should be prominent and impossible to miss
- The tool name and command should be clear enough to make an informed decision
- Response should feel immediate — no lag between clicking Approve and the agent continuing

## Edge Cases
- Permission request arrives while user is on a different tab or panel
- Multiple permission requests queue up simultaneously
- The 300-second timeout expires before the user responds (auto-deny)
