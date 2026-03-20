# Branchdeck - Project Overview

**Version:** 0.2.0 (Alpha)
**License:** MIT
**Generated:** 2026-03-20

## Executive Summary

Branchdeck is a terminal-first desktop application for managing git repositories, worktrees, and autonomous coding sessions. Built with Tauri v2 (Rust backend + SolidJS frontend), it provides a three-pane workspace for parallel development across multiple repos, branches, and worktrees with integrated Claude Code agent orchestration, a local knowledge layer (vector embeddings), and PR monitoring.

## Product Position

> Run parallel coding work across repos, branches, and agent sessions - with traceability, memory, and control.

- **Target user:** AI-native developers, technical founders, small engineering teams
- **Wedge:** Multi-repo/multi-worktree execution cockpit
- **Platform:** Linux-first (Ubuntu 22.04+), local-first, no cloud dependency

## Architecture Type

**Multi-part monolith** - Three tightly-coupled parts in one repository:

| Part | Path | Technology | Role |
|------|------|-----------|------|
| Frontend | `src/` | SolidJS + TypeScript | Desktop UI, terminal, reactive state |
| Backend | `src-tauri/src/` | Rust + Tauri v2 | Business logic, git ops, event bus, knowledge |
| Sidecar | `sidecar/` | Node.js | Claude Agent SDK bridge, MCP server |

## Technology Stack Summary

| Category | Technology | Version |
|----------|-----------|---------|
| Desktop framework | Tauri v2 | 2.x |
| Frontend framework | SolidJS | 1.9.3 |
| Styling | Tailwind CSS v4 | 4.x |
| UI components | Kobalte | 0.13 |
| Terminal emulator | xterm.js + WebGL | 6.0 |
| Backend language | Rust | 2021 edition |
| Git operations | git2 | 0.20 |
| PTY management | portable-pty | 0.9 |
| Async runtime | tokio | 1.x (full features) |
| Error handling | thiserror | 2.x |
| GitHub API | octocrab | 0.44 |
| Agent SDK | Claude Agent SDK | 0.2.77+ |
| Vector embeddings | fastembed (ONNX) | 5.x |
| Vector storage | rvf-runtime | 0.2 |
| Learning engine | ruvector-sona | 0.1 (optional) |
| Bundler | Vite | 6.0 |
| Package manager | Bun | 1.0+ |
| Linter/formatter | Biome | 2.x |
| Testing (frontend) | Vitest | 4.1 |
| Testing (backend) | cargo test | built-in |

## Core Capabilities (Implemented)

1. **Multi-repo workspace** - Add repos, browse worktrees, persistent state
2. **Worktree lifecycle** - Create/delete with branch preview, real-time file status
3. **Embedded terminals** - Shell and Claude Code tabs per worktree, WebGL-rendered PTY
4. **Task/run system** - Durable task.md files, sidecar-based run execution, retry/resume
5. **Agent monitoring** - Hook-based event capture, real-time status, file activity heatmap
6. **PR monitoring** - GitHub PR status, CI checks, reviews, branch tracking
7. **Knowledge layer** - Local vector embeddings (ONNX), hierarchical scoping, MCP server
8. **Session persistence** - Window state, repos, worktrees, tabs - all restored on relaunch

## Communication Architecture

```
External (Claude API)
        |
   Sidecar (Node.js)
   agent-bridge.js -----stdin/stdout----> Rust RunManager
   knowledge-mcp.js ---HTTP/JSON-RPC---> Rust KnowledgeMCP
        |
   Rust Backend (Tauri)
   EventBus (tokio::broadcast) --> ActivityStore, EventBridge, KnowledgeIngestion
   HookReceiver (TCP:13370) <---- Claude Code command hooks (curl POST)
        |
   Tauri IPC boundary (invoke + events)
        |
   SolidJS Frontend
   Stores (reactive) <-- listen("agent:event", "task:updated", "run:status_changed")
   Components --> invoke("command_name", params)
```

## Related Documentation

- [Architecture](./architecture.md) - Detailed system architecture
- [Source Tree Analysis](./source-tree-analysis.md) - Annotated directory structure
- [Component Inventory](./component-inventory.md) - UI component catalog
- [Data Models](./data-models.md) - Domain types and data flow
- [Development Guide](./development-guide.md) - Setup, build, test instructions
