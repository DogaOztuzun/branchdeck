---
id: terminal-preset-creation
title: Create and Use Terminal Presets
persona: power-user
priority: low
tags: [terminal, presets, customization]
generated_from: docs/component-inventory.md
---

## Context
A power user wants to save frequently used terminal commands as presets so they can launch "bun run dev" or a custom Claude command with one click.

## Steps
1. Open the PresetManager from the terminal tab dropdown
2. Create a new preset with a name (e.g., "Dev Server") and command ("bun run dev")
3. Select the tab type (shell or claude)
4. Save the preset
5. Open the tab dropdown and verify the preset appears in the list
6. Click the preset to launch a new tab with the command pre-loaded
7. Edit an existing preset and verify changes persist
8. Delete a preset and verify it's removed

## Expected Satisfaction
- Presets should be per-repo (different repos need different commands)
- Creating a preset should be quick — name + command, done
- Presets should persist across app restarts

## Edge Cases
- Preset with a command that fails immediately on launch
- Duplicate preset names
- Preset referencing a command that no longer exists (e.g., removed from package.json)
