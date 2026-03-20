# Branchdeck - Documentation Index

**Version:** 0.2.0 | **Generated:** 2026-03-20 | **Scan:** Deep

## Project Overview

- **Type:** Multi-part monolith (Frontend + Rust Backend + Node.js Sidecar)
- **Primary Language:** Rust (backend), TypeScript (frontend)
- **Architecture:** Event-driven desktop app with functional core / imperative shell
- **Platform:** Linux-first (Tauri v2)

## Quick Reference

- **Tech Stack:** Tauri v2 + SolidJS + Rust + xterm.js + git2 + Claude Agent SDK
- **Entry Points:** `src-tauri/src/lib.rs` (backend), `src/App.tsx` (frontend), `sidecar/agent-bridge.js` (sidecar)
- **Config dir:** `~/.config/branchdeck/`
- **Hook port:** TCP 13370
- **Package manager:** Bun (never npm/yarn/pnpm)
- **Dev command:** `bunx tauri dev`

## Generated Documentation

- [Project Overview](./project-overview.md) - Executive summary, tech stack, capabilities
- [Architecture](./architecture.md) - System design, service layer, event flow, startup sequence
- [Source Tree Analysis](./source-tree-analysis.md) - Annotated directory structure, code stats
- [Data Models](./data-models.md) - All domain types, persistence model, serde conventions
- [Component Inventory](./component-inventory.md) - 17 SolidJS components, 6 stores
- [Development Guide](./development-guide.md) - Setup, commands, standards, testing, CI/CD

## Project Documentation

- [README.md](../README.md) - Public-facing project readme
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guide, branch strategy, code standards
- [CLAUDE.md](../CLAUDE.md) - AI development instructions (authoritative for code conventions)

## BMAD Artifacts

Located in `_bmad-output/`:

**Tech Specs (17):** Sidecar bridge, run manager, artifact capture, task model, frontend stores, agent monitoring UI, knowledge service, MCP server, PR monitoring, SONA learning, restart-safe recovery, and more.

**Research (4):** Worktree creation, high-priority feasibility, RVF integration, Claude Agent SDK integration.

**Test Artifacts:** ATDD checklist, test design for Phase 1.

## Getting Started

1. Read [Development Guide](./development-guide.md) for setup instructions
2. Read [Architecture](./architecture.md) for system understanding
3. Read [CLAUDE.md](../CLAUDE.md) for code conventions (authoritative)
4. When planning new features, use this index as context input

## AI-Assisted Development

When creating a brownfield PRD or planning features, provide this index as context input. Key architectural constraints:

- No business logic in Tauri command handlers
- Services must be daemon-extractable (no Tauri types)
- Pure functions should return effects, not execute them
- Event-driven: use EventBus for internal events, Tauri events for frontend
- File-based persistence (task.md, run.json) - no database yet
- Knowledge service is feature-gated behind `knowledge` flag
