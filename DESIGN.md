# Design System -- Branchdeck

## Product Context
- **What this is:** A dark software factory — self-improving autonomous development system that detects quality issues, implements fixes, validates improvements, and learns from results. Desktop-first orchestration platform with terminal integration.
- **Who it's for:** AI-native solo builders shipping at high velocity across multiple projects (web apps, web3, AI agents). The primary user is the builder who can't manually QA at 10+ PRs/day.
- **Space/industry:** Dark software factories / autonomous development / agent orchestration (peers: Fabro, Linear Agent, StrongDM Agate). Positioned as Level 5 + satisfaction feedback loop.
- **Project type:** Desktop app (Tauri v2), dark-only, terminal-first. Linux-first.
- **Design philosophy:** Screenshots are the primary distribution channel. Every UI surface is designed assuming someone will share it. The app must look as good as it works.

## Aesthetic Direction
- **Direction:** Industrial/Utilitarian -- function-first, data-dense, monospace-native. The terminal IS the product; the UI frames it.
- **Decoration level:** Minimal -- borders and background tints only. No gradients, no shadows, no glow effects. Terminal output is the visual content.
- **Mood:** Precise, fast, professional. Like `htop` or `tmux` with better information hierarchy. The product should feel instant and dense -- zero visual noise.
- **Reference sites:** Linear (restrained dark theme, inbox model, keyboard-first), Warp (terminal-first but uses sans-serif for chrome -- we don't), Zellij (raw/utilitarian, monospace nav).
- **Anti-patterns:** No purple gradients, no 3-column feature grids with icons in circles, no bubbly border-radius, no gradient buttons, no generic card-grid dashboard patterns. No modals for primary actions (power-user tool -- one click, not two).

## Typography
- **Display/Hero:** JetBrains Mono 600 -- same font, heavier weight. No typeface switching.
- **Body:** JetBrains Mono 400 -- all body text is monospace. This is the strongest visual differentiator. It removes the seam between app UI and terminal content.
- **UI/Labels:** JetBrains Mono 400/500 -- same as body.
- **Data/Tables:** JetBrains Mono 400 -- inherently tabular (monospace aligns naturally).
- **Code:** JetBrains Mono 400 -- same font, terminal and code are visually unified.
- **Loading:** Google Fonts CDN (`family=JetBrains+Mono:ital,wght@0,300;0,400;0,500;0,600;0,700;1,400`). Fallback stack: `'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace`.
- **Scale:**
  - 48px / 600 -- hero metric (SAT score on dashboard)
  - 20px / 600 -- page titles, orchestration headers, prominent scores
  - 16px / 600 -- section headings, prominent status text, score deltas
  - 13px / 400 -- body text, descriptions, primary content, card titles (500)
  - 12px / 400 -- secondary content, file paths, tab labels, action buttons (500)
  - 11px / 400 -- tertiary content, queue status, metadata, timestamps
  - 10px / 500 -- badges, labels, status indicators, section headers (uppercase, letter-spacing 0.06-0.08em)

## Color
- **Approach:** Restrained -- one accent + semantic colors. Color is rare and meaningful.
- **Background:** `#1a1b26` (--color-bg) -- editor background, Tokyo Night "Night" variant
- **Surface:** `#16161e` (--color-surface) -- sidebars, panels, tabs, kanban columns. Matches VS Code sideBar.background.
- **Surface Raised:** `#1e202e` -- hover states, elevated surfaces, expanded detail areas (use sparingly)
- **Border:** `#101014` (--color-border) -- panel dividers, card borders, row separators. Subtle, not prominent.
- **Input:** `#14141b` -- input field backgrounds
- **Primary:** `#7aa2f7` (--color-primary) -- interactive elements, active states, brand color, chart lines. Cool blue.
- **Text:** `#a9b1d6` (--color-text) -- primary text. Matches VS Code editor.foreground.
- **Text Muted:** `#787c99` (--color-text-muted) -- secondary text, labels, metadata. Matches VS Code sideBar.foreground.
- **Semantic:**
  - Success: `#9ece6a` (--color-success) -- completed, passing, merged, fixed, positive delta
  - Warning: `#e0af68` (--color-warning) -- running, pending, analyzing, confused-newbie persona
  - Error: `#f7768e` (--color-error) -- failed, cancelled, critical severity, negative delta
  - Info: `#7dcfff` (--color-info) -- branches, links, PR numbers, accessibility persona
- **Dark mode:** Dark-only. No light mode. Terminal tools are dark.

## Spacing
- **Base unit:** 4px
- **Density:** Compact -- maximize information density. Terminal users expect dense layouts.
- **Scale:** 2xs(2px) xs(4px) sm(8px) md(16px) lg(24px) xl(32px) 2xl(48px) 3xl(64px)
- **Component heights:**
  - Top bar: 44px (h-11)
  - Tab bar: 36px (h-9)
  - Inbox rows: 36px (collapsed)
  - Sidebar items: 28px
  - Finding rows: 36px
  - Buttons: 32px (default), 24px (compact)
  - Badges: auto height, 2px vertical / 8px horizontal padding
- **Padding conventions:**
  - Panel content: 12-16px horizontal
  - List items / inbox rows: 4px vertical, 8px horizontal
  - Sections: 32px vertical between sections
  - Cards: 8-10px all sides
  - Kanban columns: 8px body padding, 10-12px header padding

## Layout
- **Approach:** Grid-disciplined -- strict alignment, predictable structure.
- **Workspace view:** 3-panel resizable layout (RepoSidebar | TerminalArea | RightSidebar) using `solid-resizable-panels`. Panel sizes: 18% / 64% / 18% default, collapsible sidebars.
- **Triage view:** Single-column inbox, max-width 900px centered. Summary bar at top, grouped rows below.
- **SAT Dashboard:** Single-column, max-width 960px centered. Hero score + trend chart + tabbed findings list.
- **Task Board:** Horizontal kanban columns, 280px fixed width each, horizontal scroll. Board/List toggle.
- **Navigation:** Tab/mode switcher in TopBar: `[Workspace] [PR Triage] [SAT] [Tasks]`. Active tab has 2px bottom border in primary color.
- **Max content width:** 900-960px for single-column views. Full width for workspace and board views.
- **Border radius:**
  - 0px -- default for all containers, cards, buttons, inputs, badges, columns. Sharp rectangles.
  - 2px -- subtle rounding for inline elements only (e.g., sidebar item hover).
  - 4px -- scrollbar thumbs only.

## Motion
- **Approach:** Minimal-functional -- only transitions that aid comprehension. The product should feel instant.
- **Easing:** enter(ease-out) exit(ease-in) move(ease-in-out)
- **Durations:**
  - Hover states (background, color): 150ms
  - Panel resize handle highlight: 150ms
  - Tab/view switching: 0ms (instant)
  - Status badge changes: 0ms (instant)
  - Score updates after merge: 0ms (instant)
  - Sidebar collapse/expand: 200ms ease-in-out
  - Detail row expand/collapse: 200ms ease-in-out
- **Allowed animation:** Pulsing opacity only -- for "agent running" indicators. 0.4 to 1.0 opacity, 2s cycle, ease-in-out.
- **Forbidden:** No entrance animations, no bounce, no spring physics, no scroll-driven animation, no loading spinners with animation. The interface must never feel like it's "performing."

## Component Patterns

### Inbox Row (Triage, Findings)
- **Collapsed:** 36px height. Status dot (8x8px) + PR number (info color) + branch/title (truncated) + badge + repo + time. Single line.
- **Expanded:** Surface raised background. Detail sections (reasoning, files, evidence) + action buttons. 200ms expand.
- **Selected:** Surface raised bg + 2px left border in primary color. Padding-left reduced by 2px to compensate.
- **Keyboard:** j/k navigate, Enter expand/collapse, action shortcuts shown in buttons.

### Status Dot
- 8x8px square (no border-radius). Color maps to state:
  - Error/failing: `--color-error`
  - Success/passing: `--color-success`
  - Warning/running: `--color-warning`
  - Info/approved: `--color-info`
  - Inactive: `--color-text-muted` at 40% opacity

### Badge
- 10px/500 uppercase. Padding: 2px 6-8px. Two styles:
  - **Filled:** Background color + dark text (for severity: CRIT, HIGH)
  - **Outlined:** Transparent bg + border + text in semantic color (for status: PASSING, FAILING, ANALYZING)
  - **Pulsing:** Outlined warning with pulsing opacity (for ANALYZING -- agent running)

### Card (Task Board)
- Background: `--color-bg` (darker than column surface -- inverted card pattern). Border: `--color-border`.
- **Priority indicator:** Left border 2px in severity color (critical: error, high: warning, normal: none).
- **Source badge:** SAT (primary outlined), MANUAL (muted outlined), ISSUE (info outlined).
- **Agent indicator:** `>_` monospace character + status text. Running = pulsing warning. Completed = success. Failed = error.
- **SAT delta:** `+N` in success color on completed cards.
- Draggable between kanban columns.

### Summary Stats Bar
- Single row, inline stats separated by `|` dividers. Values in primary or semantic color, labels in muted 11px.
- Used at top of Triage view and SAT Dashboard.

### Satisfaction Trend Chart
- Minimal SVG chart. No gridlines (or very subtle). Axis labels in 10px muted.
- **Overall line:** Primary color, 2px stroke, subtle area fill (primary at 8-10% opacity).
- **Per-persona lines:** Semantic colors, 1.5px stroke, dashed (4,4).
- **Data points:** Circles, 3px radius. Current point: 4px radius with bg stroke.
- **Sparkline variant:** 80x24px, used in summary bars. Line only, no labels.
- This is the #1 shareable asset. Must look beautiful standalone on dark background.

### Section Header
- 10px/500 uppercase, letter-spacing 0.06-0.08em. Semantic color matching section type.
- Count in parentheses, lower opacity. E.g., `NEEDS ATTENTION (2)` in error color.

### Action Button
- 32px height default, 24px compact. JetBrains Mono 12px/500.
- **Primary action:** Semantic border + text color, bg on hover (e.g., Approve = success border/text, success bg + dark text on hover).
- **Secondary action:** Muted border + text. Text brightens on hover.
- **Keyboard shortcut hint:** 10px muted text after label, 60% opacity.
- No border-radius. No shadows.

### Empty State / All Clear
- Centered vertically. Check mark in success color (32px, font-weight 300).
- Primary message: 16px/600.
- Secondary: 12px muted with stats summary (e.g., "4 PRs handled today . SAT score 86 (+12) . Next run: tonight 11pm").
- This is an emotional payoff, not an error. Design it as a reward.

### Loop Complete Label
- Inline with done items. 10px success color. Format: `found > fixed > verified`.
- Visible on merged PRs that completed a full SAT cycle. The autonomous loop proof.

## Iconography
- **No icon library.** Use monospace characters and color as the icon system:
  - Agent: `>_` (terminal prompt)
  - Status: 8x8px colored squares (status dots)
  - Navigation: Text labels only, no icons in tabs
  - Priority: Left border color, not icons
  - Source: Text badges (SAT, MANUAL, ISSUE), not icons
- **Exception:** Lucide icons for utility actions only (refresh, back arrow, settings gear). Keep to minimum.

## Keyboard Navigation
- **Global:** Number keys (1-4) switch views. `Cmd+K` / `Ctrl+K` command palette.
- **Triage inbox:** `j`/`k` navigate rows. `Enter` expand/collapse. `a` approve. `s` skip. `m` merge.
- **Task board:** Arrow keys navigate cards. `Enter` open detail. Drag via mouse only.
- **Shortcut hints:** Shown inline in action buttons as muted text (e.g., `Approve [a]`). Visible on hover or when row is selected.

## States

### Loading
- No spinners. Use pulsing opacity (0.4 → 1.0, 2s) on the element that's loading.
- For initial data load: show section headers with muted "Loading..." text at 11px.

### Empty
- Context-specific message, not generic "No data."
  - Triage empty: "All clear" reward state with score summary
  - Task board empty: "No tasks. SAT or GitHub issues create tasks automatically."
  - SAT no runs: "No SAT runs yet. Run /sat to start."
- Empty states should suggest the next action.

### Error
- Inline error text in error color. Never a modal or toast.
- Format: "Failed to [action]: [reason]" at 12px.

### Disabled
- 40% opacity on the element. No color change. Cursor: not-allowed.

## Data Visualization
- **Palette:** Primary for overall metric. Semantic colors for per-category breakdown.
- **Chart chrome:** Minimal. Subtle grid lines at `--color-border` if needed. Labels in 10px muted.
- **Fill:** Subtle area fill under lines, 8-10% opacity of the line color. Only exception to no-gradient rule.
- **Interaction:** Data points clickable where drill-down exists. Hover shows value tooltip (surface raised bg, 11px).
- **Responsive:** Charts fill available width. Fixed height (180px for full chart, 24px for sparkline).

## Decisions Log
| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-03-21 | Initial design system created | Documented existing Tokyo Night + JetBrains Mono patterns, formalized spacing/motion/border-radius. Created by /design-consultation. |
| 2026-03-21 | All-monospace typography confirmed | Strongest visual differentiator. Removes seam between app UI and terminal output. Unusual for the category -- intentional risk. |
| 2026-03-21 | Zero border-radius confirmed | Reinforces industrial/terminal aesthetic. Sharp corners = precision. Contrasts with rounded-corner competitors. |
| 2026-03-21 | Dark-only confirmed | Terminal tools are dark. No light mode planned. |
| 2026-03-21 | Tokyo Night palette kept unchanged | Beloved palette with proven readability. Recognition with VS Code users builds trust. Not custom-branded. |
| 2026-03-25 | Product context updated to dark software factory | Reflects evolved vision: self-improving autonomous development system, not just terminal manager. |
| 2026-03-25 | Four-view navigation added | Workspace / PR Triage / SAT / Tasks. Triage is the primary inbox. |
| 2026-03-25 | Component patterns codified | Inbox rows, badges, cards, status dots, action buttons, charts extracted from HTML previews. |
| 2026-03-25 | Keyboard navigation system defined | j/k/Enter for inbox, action shortcuts (a/s/m), global view switching (1-4). Linear-inspired. |
| 2026-03-25 | No icon library — monospace characters as icons | `>_` for agents, colored squares for status, text badges for source. Consistent with monospace-native aesthetic. |
| 2026-03-25 | Inverted card pattern for task board | Cards darker than column background. Creates depth without shadows in dark theme. |
| 2026-03-25 | Satisfaction trend chart is primary shareable asset | Must look beautiful standalone. Subtle area fill is only exception to no-gradient rule. |
| 2026-03-25 | "All clear" as reward state, not empty state | Inbox zero is the emotional payoff. Shows score summary + next run time. Designed as celebration. |
| 2026-03-25 | No modals for primary actions | Power-user tool. Approve/merge/skip are one-click. Modals break flow. |
