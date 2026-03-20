# Development Guide

**Generated:** 2026-03-20

## Prerequisites

| Requirement | Version | Install |
|-------------|---------|---------|
| **OS** | Linux (Ubuntu 22.04+) | - |
| **Rust** | Stable (2021 edition) | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **Bun** | 1.0+ | `curl -fsSL https://bun.sh/install \| bash` |
| **System libs** | See below | apt-get |

```bash
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  libjavascriptcoregtk-4.1-dev \
  libsoup-3.0-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

## Quick Start

```bash
git clone https://github.com/DogaOztuzun/branchdeck.git
cd branchdeck
bun install              # Install frontend deps
bunx tauri dev           # Dev mode (hot reload + Rust rebuild)
```

## Commands

| Command | Purpose |
|---------|---------|
| `bun install` | Install frontend dependencies |
| `bunx tauri dev` | Dev mode (hot reload frontend + Rust rebuild) |
| `bunx tauri build` | Production build (AppImage, deb, rpm) |
| `bun run check` | Biome lint + format check |
| `bun run check:fix` | Biome auto-fix |
| `bun test` | Frontend tests (vitest) |
| `cd src-tauri && cargo clippy --all-targets` | Rust linting |
| `cd src-tauri && cargo fmt --check` | Rust format check |
| `cd src-tauri && cargo test` | Rust tests |

**CRITICAL:** Package manager is **Bun**. Never use npm, npx, yarn, or pnpm.

## Before Every Commit/PR

```bash
bun run check && bun test && cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

CI will reject PRs that fail any step.

## Project Structure

```
branchdeck/
  src/                    # SolidJS frontend (TypeScript)
    components/           # UI components (layout, terminal, task, worktree, pr, ui)
    lib/commands/         # Tauri IPC wrappers (ONLY place invoke() is called)
    lib/stores/           # SolidJS reactive state (singleton factory pattern)
    types/                # TypeScript type definitions
  src-tauri/src/          # Rust backend
    commands/             # Thin IPC handlers (NO business logic)
    services/             # All business logic (daemon-extractable)
    models/               # Domain types (Serialize/Deserialize)
    error.rs              # Single AppError enum
  sidecar/                # Node.js sidecar processes
    agent-bridge.js       # Claude SDK bridge (stdin/stdout protocol)
    knowledge-mcp.js      # MCP server for knowledge tools
```

## Adding a New Feature

1. **Models:** `src-tauri/src/models/` - domain types
2. **Service:** `src-tauri/src/services/` - business logic (pure functions, return effects)
3. **Tests:** `src-tauri/tests/` - test pure functions with `common/mod.rs` helpers
4. **Command:** `src-tauri/src/commands/` - thin IPC handler, register in `lib.rs` invoke_handler
5. **Frontend types:** `src/types/`
6. **IPC wrapper:** `src/lib/commands/` - with try/catch + logError
7. **Store:** `src/lib/stores/` - if feature needs reactive state
8. **Component:** `src/components/{category}/`
9. **Verify:** Run the full check suite before committing

## Code Standards

### TypeScript
- Strict mode, no `any` except IPC boundaries
- Named exports only (no default exports)
- Single quotes, semicolons, 2-space indent, 100 char width (Biome enforces)
- Tauri IPC calls wrapped in `src/lib/commands/`, never from components
- No barrel files

### Rust
- `unsafe` denied, `unwrap()`/`expect()` warned
- Clippy pedantic enabled
- All errors via thiserror `AppError` enum
- No business logic in command handlers
- Services take dependencies as parameters, no global state
- Structured logging: info (state changes), debug (reads), error (failures), trace (hot paths)

### Commits
- Conventional commits: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`
- No commented-out code

## Testing

**Pattern: Functional Core / Imperative Shell**

Pure `apply_*` functions return `(state, Vec<RunEffect>)` - fully testable without mocking Tauri.

```
src-tauri/tests/
  common/mod.rs          # Shared helpers
  task_parsing.rs        # T1: parse_task_md, frontmatter
  artifact_capture.rs    # T2: git artifact capture with temp repos
  run_lifecycle.rs       # T4: pure state machine + persistence
  git_operations.rs      # T6: worktree CRUD, branches
  agent_monitoring.rs    # T5: event bus, activity store

src/lib/__tests__/
  utils.test.ts          # T7: parseArtifactSummary, statusColor, shortPath
```

**Test conventions:**
- Shared helpers in `tests/common/mod.rs`
- `tempfile::TempDir` for filesystem tests, `git2::Repository` for git tests
- Assert effects with `.iter().any(|e| matches!(e, ...))` (order-independent)
- Test happy path + one error + one edge case per function

## CI/CD Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci.yml` | PR, push | Biome + Clippy + fmt + build + tests |
| `claude-code-review.yml` | PR | Auto-review with inline comments |
| `claude.yml` | @claude mention | Respond to mentions on PRs/issues |
| `claude-ci-fix.yml` | CI failure | Auto-fix lint, fmt, clippy on feature branches |
| `release.yml` | Manual dispatch | Version bump -> tag -> build -> GitHub release |

## Development Methodology

This project uses the [BMAD-METHOD](https://github.com/bmadcode/BMAD-METHOD) for AI-assisted development. The `_bmad/` directory contains the framework. BMAD artifacts (tech specs, test plans, research) are in `_bmad-output/`.

## Feature Flags

| Feature | Default | Deps | Purpose |
|---------|---------|------|---------|
| `knowledge` | on | rvf-runtime, fastembed, bincode, tokio-util | Vector storage + embeddings |
| `sona` | off | ruvector-sona + knowledge | Learning engine (experimental) |
