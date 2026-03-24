---
id: keyboard-only-navigation
title: Navigate the Entire App Using Only Keyboard
persona: accessibility-user
priority: high
tags: [accessibility, keyboard, navigation, focus]
generated_from: docs/component-inventory.md
---

## Context
An accessibility user navigates Branchdeck entirely via keyboard. They need to reach all interactive elements, manage focus, and complete core workflows without a mouse.

## Steps
1. Launch Branchdeck and press Tab to move through the top bar controls
2. Tab into the repo sidebar and navigate the repo/worktree tree
3. Press Enter to select a worktree
4. Tab to the terminal area and verify the terminal receives keyboard focus
5. Use Ctrl+Shift+T to open a new terminal tab
6. Use Ctrl+Shift+A to open a new Claude tab
7. Tab through the Team Sidebar controls (task cards, approval dialog)
8. Use Tab/Shift+Tab to navigate within modals (Add Worktree, Create Task)
9. Press Escape to close modals and return focus to the previous element

## Expected Satisfaction
- Every interactive element should be reachable via Tab/Shift+Tab
- Focus indicators should be clearly visible against the dark theme
- Modals should trap focus properly and return it on close
- Keyboard shortcuts should work from any panel context

## Edge Cases
- Focus gets trapped in the xterm.js terminal (needs a clear escape mechanism)
- Context menus (right-click) should have a keyboard trigger alternative
- Focus order changes when panels are toggled on/off
- Rapid Tab key presses shouldn't skip over elements
