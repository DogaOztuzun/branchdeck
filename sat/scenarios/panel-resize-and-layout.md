---
id: panel-resize-and-layout
title: Resize Panels and Toggle Sidebars
persona: power-user
priority: medium
tags: [layout, panels, resize, customization]
generated_from: docs/component-inventory.md
---

## Context
A power user wants to customize the three-panel layout — maximize the terminal area while still having quick access to sidebars when needed.

## Steps
1. Drag the resize handle between the left sidebar and the terminal area to make the terminal wider
2. Drag the resize handle between the terminal area and the right sidebar
3. Use the TopBar toggle buttons to hide the repo sidebar entirely
4. Verify the terminal area expands to fill the freed space
5. Toggle the team sidebar on and off
6. Toggle the dashboard view
7. Toggle the changes sidebar
8. Verify that toggling sidebars doesn't lose terminal state or scroll position

## Expected Satisfaction
- Resize handles should feel smooth and responsive — no jank or layout jumps
- Toggle buttons should be easy to find in the top bar
- Panel sizes should persist across app restarts (session persistence)

## Edge Cases
- Dragging a panel to its minimum size (near zero)
- Rapidly toggling sidebars on/off
- Resizing while a terminal has active output scrolling
