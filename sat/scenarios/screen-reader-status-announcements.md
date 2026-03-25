---
id: screen-reader-status-announcements
title: Screen Reader Announces Status Changes
persona: accessibility-user
priority: medium
tags: [accessibility, screen-reader, aria, status]
generated_from: docs/component-inventory.md
---

## Context
An accessibility user relies on a screen reader. Status changes — task updates, run completion, permission requests — should be announced via ARIA live regions without requiring the user to navigate to them.

## Steps
1. Start an agent run while using a screen reader
2. Listen for the screen reader to announce when the run status changes (starting, running, completed)
3. Verify that permission requests are announced prominently (assertive live region)
4. Check that PR badge changes trigger announcements
5. Verify that TaskBadge status dots have meaningful ARIA labels (not just color)
6. Navigate to the RunTimeline and verify step events are readable

## Expected Satisfaction
- Critical status changes (permission needed, run failed) should be announced immediately
- Non-critical updates (step progress) should be polite, not interrupting
- Color-coded states (badges, status dots) should have text alternatives

## Edge Cases
- Rapid succession of status changes overwhelming the screen reader
- Terminal output competing with ARIA announcements
- Modals not announcing their opening/closing to the screen reader
