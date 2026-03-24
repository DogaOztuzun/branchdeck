---
id: agent-event-log-scrolling
title: Agent Activity Log Scrolling and Readability
persona: any
priority: low
tags: [agent, activity-log, scrolling, monitoring]
generated_from: docs/component-inventory.md
---

## Context
During an active agent run, the AgentActivity panel shows a scrolling log of events. The user wants to read past events while new ones arrive without losing their scroll position.

## Steps
1. Start an agent run that generates many events
2. Open the Agent Activity panel in the terminal area
3. Watch events stream in with timestamps, types, tools, and files
4. Scroll up to read an earlier event
5. Verify new events don't force-scroll you back to the bottom while reading
6. Scroll back to the bottom to resume live-following
7. After the run completes, scroll through the full history

## Expected Satisfaction
- The log should auto-scroll when the user is at the bottom (following mode)
- Scrolling up should "detach" and hold position while new events appear
- Timestamps should be readable and help correlate events with the run timeline

## Edge Cases
- A run that generates 500+ events (performance and memory)
- Events arriving faster than one per second (batching/throttling)
- Very long file paths or tool names in event entries (truncation)
