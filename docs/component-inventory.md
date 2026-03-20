# Component Inventory

**Generated:** 2026-03-20

## Overview

17 SolidJS components organized in 6 categories. All use TypeScript strict mode, named exports, and access Tauri IPC only via `src/lib/commands/` wrappers.

## Layout Components

### Shell.tsx
**Role:** Root layout - 3-panel resizable container
**Uses:** solid-resizable-panels (PanelGroup, Panel, PanelResizeHandle)
**Children:** RepoSidebar (18%), TerminalArea (64%), right sidebar (18%)
**Store deps:** layout (panelApi, sidebar visibility)

### TopBar.tsx
**Role:** Header bar with repo/branch display and toggle buttons
**Shows:** Active repo name, current branch
**Actions:** Toggle repo sidebar, team sidebar, dashboard, changes sidebar
**Store deps:** repo (activeRepoPath, worktrees), layout (toggle functions)

### RepoSidebar.tsx
**Role:** Repository tree with worktree list and PR badges
**Features:**
- Expandable repo tree with worktree children
- PR badges per worktree: state color, review icon, checks icon
- PrTooltip on hover with full PR details
- Context menu: Checkout Branch, Close Project, Delete Worktree
- Auto-refresh: tracking (60s), active PR (15s), inactive repos (60s)
**Modals:** AddWorktreeModal, BranchWorktreeModal, DeleteWorktreeDialog
**Store deps:** repo (repos, worktrees, tracking, PR), task (tasksByWorktree)

### ChangesSidebar.tsx
**Role:** Git file status display
**Shows:** Modified files with status badges (M/A/D/R/C), color-coded
**Store deps:** repo (statuses)

### TeamSidebar.tsx
**Role:** Task management + agent monitoring panel
**Sections:**
1. FileGrid (agent file activity heatmap)
2. Tasks (per-worktree task cards with action buttons)
3. RunTimeline (active run event history)
4. ApprovalDialog (pending permission requests)
5. Active Agents (current tool + file per agent)
6. Agent Definitions (from .claude/agents/*.md)
**Store deps:** repo, task, agent, terminal

### TaskDashboard.tsx
**Role:** Cross-repo task overview
**Features:** Loads all tasks from all worktrees, sorted by status priority
**Store deps:** repo (all worktrees), task

### FileGrid.tsx
**Role:** Dot-based heatmap of agent file activity
**Visual:** Colored dots sized by: access count, agent count, active/modified status
**Hover:** Tooltip with file path, state, tool, access count
**Store deps:** repo (statuses), agent (file activity)

## Terminal Components

### TerminalArea.tsx
**Role:** Multi-tab terminal container
**Children:** TabBar, TerminalView (per tab), PresetManager, AgentActivity
**Store deps:** terminal (tabs, activeTab), repo (activeWorktree), agent

### TerminalView.tsx
**Role:** xterm.js wrapper
**Features:**
- WebGL addon (fallback to canvas)
- FitAddon for auto-resize
- ResizeObserver for container changes
- Input via onData -> writeTerminal IPC
- Output via registered callback from terminal store

### TabBar.tsx
**Role:** Tab management
**Features:**
- Tab buttons with close (X), active state
- Dropdown: New Terminal (Ctrl+Shift+T), New Claude (Ctrl+Shift+A), Presets
- AgentBadge for claude tabs
**Store deps:** terminal, agent

### AgentActivity.tsx
**Role:** Scrolling agent event log
**Shows:** Timestamped events with type, tool, file
**Store deps:** agent (log)

### AgentBadge.tsx
**Role:** Inline status indicator for claude tabs
**Shows:** Status dot (active/idle/stopped), current tool + file (truncated), subagent count
**Store deps:** agent (getTabAgent)

### PresetManager.tsx
**Role:** Create/edit terminal presets (shell or claude commands)
**Store deps:** repo (activeRepoPath), workspace commands (getPresets, savePresets)

## Task Components

### CreateTaskModal.tsx
**Role:** New task creation form
**Fields:** Task type (issue-fix/pr-shepherd), PR number, description
**Store deps:** repo, task

### TaskBadge.tsx
**Role:** Status indicator dot
**Visual:** Color-coded by status, pulsing animation for "running"

### RunTimeline.tsx
**Role:** Run event history
**Shows:** Status + duration (auto-updating) + cost, event list (step/text/tool/status)
**Store deps:** task (activeRun, runLog)

### ApprovalDialog.tsx
**Role:** Permission approval UI
**Shows:** Tool name, command, Approve/Deny buttons
**Store deps:** task (pendingPermissions, respondToPermission)

## Worktree Components

### AddWorktreeModal.tsx
**Role:** Create new worktree with live preview
**Features:** Name input with debounced preview (200ms), conflict detection, base branch selector

### BranchWorktreeModal.tsx
**Role:** Checkout existing branch as worktree
**Features:** Branch search/filter, remote/in-use badges, derived worktree name

### DeleteWorktreeDialog.tsx
**Role:** Confirmation dialog with optional branch deletion

## Other Components

### PrTooltip.tsx (pr/)
**Role:** Rich PR information tooltip
**Shows:** PR #, state badge, title, reviews, checks (collapsible), GitHub link

### ContextMenu.tsx (ui/)
**Role:** Generic right-click context menu with danger variant

## Store Architecture

| Store | Pattern | Event Listeners | Key Exports |
|-------|---------|-----------------|-------------|
| repo | Factory singleton | None | repos, worktrees, selectRepo, createWorktree, loadPrStatus |
| task | Factory singleton | task:updated, run:status_changed, run:step, run:permission_request | tasks, activeRun, runLog, loadTasks |
| terminal | Factory singleton | None (callback) | tabs, openShellTab, openClaudeTab, closeTab |
| agent | Factory singleton | agent:event | agentsByTab, log, startListening |
| layout | Signal-based | None | panelApi, toggleRepoSidebar, toggleDashboard |
| knowledge | Signal-based | None | stats, loadStats |
