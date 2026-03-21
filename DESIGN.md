# Design System -- Branchdeck

## Product Context
- **What this is:** Terminal-first desktop workflow manager for git repos and worktrees, with Claude Code integration and a local knowledge layer. Evolving into a multi-repo orchestration platform with an external daemon API.
- **Who it's for:** AI/agent builders and developers who use Claude Code for multi-repo workflows.
- **Space/industry:** Developer tools / terminal multiplexers / agent orchestration (peers: tmux, Zellij, Warp, Cursor).
- **Project type:** Desktop app (Tauri v2), dark-only, terminal-first.

## Aesthetic Direction
- **Direction:** Industrial/Utilitarian -- function-first, data-dense, monospace-native. The terminal IS the product; the UI frames it.
- **Decoration level:** Minimal -- borders and background tints only. No gradients, no shadows, no glow effects. Terminal output is the visual content.
- **Mood:** Precise, fast, professional. Like `htop` or `tmux` with better information hierarchy. The product should feel instant and dense -- zero visual noise.
- **Reference sites:** Linear (restrained dark theme, minimal decoration), Warp (terminal-first but uses sans-serif for chrome -- we don't), Zellij (raw/utilitarian, monospace nav).
- **Anti-patterns:** No purple gradients, no 3-column feature grids with icons in circles, no bubbly border-radius, no gradient buttons, no generic card-grid dashboard patterns.

## Typography
- **Display/Hero:** JetBrains Mono 600 -- same font, heavier weight. No typeface switching.
- **Body:** JetBrains Mono 400 -- all body text is monospace. This is the strongest visual differentiator. It removes the seam between app UI and terminal content.
- **UI/Labels:** JetBrains Mono 400/500 -- same as body.
- **Data/Tables:** JetBrains Mono 400 -- inherently tabular (monospace aligns naturally).
- **Code:** JetBrains Mono 400 -- same font, terminal and code are visually unified.
- **Loading:** Google Fonts CDN (`family=JetBrains+Mono:ital,wght@0,300;0,400;0,500;0,600;0,700;1,400`). Fallback stack: `'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace`.
- **Scale:**
  - 20px / 600 -- page titles, orchestration headers
  - 16px / 600 -- section headings, prominent status text
  - 13px / 400 -- body text, descriptions, primary content
  - 12px / 400 -- secondary content, file paths, tab labels
  - 11px / 400 -- tertiary content, queue status, metadata
  - 10px / 500 -- badges, labels, status indicators (uppercase)

## Color
- **Approach:** Restrained -- one accent + semantic colors. Color is rare and meaningful.
- **Background:** `#1a1b26` (--color-bg) -- deep navy-black, Tokyo Night "Night" variant
- **Surface:** `#24283b` (--color-surface) -- raised surfaces, sidebars, toolbars
- **Surface Raised:** `#292e42` -- hover states, elevated surfaces (use sparingly)
- **Border:** `#414868` (--color-border) -- panel dividers, card borders
- **Primary:** `#7aa2f7` (--color-primary) -- interactive elements, active states, brand color. Cool blue.
- **Text:** `#c0caf5` (--color-text) -- primary text on dark backgrounds
- **Text Muted:** `#565f89` (--color-text-muted) -- secondary text, labels, metadata
- **Semantic:**
  - Success: `#9ece6a` (--color-success) -- completed, passing, green
  - Warning: `#e0af68` (--color-warning) -- running, pending, amber
  - Error: `#f7768e` (--color-error) -- failed, cancelled, red-pink
  - Info: `#7dcfff` (--color-info) -- branches, links, cyan
- **Dark mode:** Dark-only. No light mode. Terminal tools are dark.

## Spacing
- **Base unit:** 4px
- **Density:** Compact -- maximize information density. Terminal users expect dense layouts.
- **Scale:** 2xs(2px) xs(4px) sm(8px) md(16px) lg(24px) xl(32px) 2xl(48px) 3xl(64px)
- **Component heights:**
  - Top bar: 44px (h-11)
  - Tab bar: 36px (h-9)
  - Sidebar items: 28px
  - Buttons: 32px (default), 24px (compact)
  - Badges: auto height, 2px vertical / 8px horizontal padding
- **Padding conventions:**
  - Panel content: 12-16px horizontal
  - List items: 4px vertical, 12px horizontal
  - Sections: 32px vertical between sections

## Layout
- **Approach:** Grid-disciplined -- strict alignment, predictable structure.
- **Workspace view:** 3-panel resizable layout (RepoSidebar | TerminalArea | RightSidebar) using `solid-resizable-panels`. Panel sizes: 18% / 64% / 18% default, collapsible sidebars.
- **Orchestration view:** Dedicated full-app view replacing the Shell layout. CSS grid for run cards (`auto-fill`, `minmax(300px, 1fr)`).
- **Navigation:** Tab/mode switcher in TopBar: `[Workspace] [Orchestrations (N)]`.
- **Max content width:** None (full window width). Desktop app fills available space.
- **Border radius:**
  - 0px -- default for all containers, cards, buttons, inputs. Sharp rectangles.
  - 2px -- subtle rounding for inline elements only (e.g., sidebar item hover).
  - 4px -- scrollbar thumbs only.
  - No border-radius on: panels, run cards, alerts, badges, tab bars, toolbars.

## Motion
- **Approach:** Minimal-functional -- only transitions that aid comprehension. The product should feel instant.
- **Easing:** enter(ease-out) exit(ease-in) move(ease-in-out)
- **Durations:**
  - Hover states (background, color): 150ms
  - Panel resize handle highlight: 150ms
  - Tab/view switching: 0ms (instant)
  - Status badge changes: 0ms (instant)
  - Sidebar collapse/expand: 200ms ease-in-out
- **Forbidden:** No entrance animations, no bounce, no spring physics, no scroll-driven animation, no loading spinners with animation (use static indicators or pulsing opacity only). The interface must never feel like it's "performing."

## Decisions Log
| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-03-21 | Initial design system created | Documented existing Tokyo Night + JetBrains Mono patterns, formalized spacing/motion/border-radius. Created by /design-consultation. |
| 2026-03-21 | All-monospace typography confirmed | Strongest visual differentiator. Removes seam between app UI and terminal output. Unusual for the category -- intentional risk. |
| 2026-03-21 | Zero border-radius confirmed | Reinforces industrial/terminal aesthetic. Sharp corners = precision. Contrasts with rounded-corner competitors. |
| 2026-03-21 | Dark-only confirmed | Terminal tools are dark. No light mode planned. |
| 2026-03-21 | Tokyo Night palette kept unchanged | Beloved palette with proven readability. Recognition with VS Code users builds trust. Not custom-branded. |
