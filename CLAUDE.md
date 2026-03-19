# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Branchdeck is a Tauri v2 desktop app — terminal-first workflow manager for git repositories and worktrees. Linux-first, open source (MIT).

**Stack:** Tauri v2 (Rust backend) + SolidJS (TypeScript frontend) + xterm.js (terminal)

## Critical Rules

- **Package manager is Bun** — NEVER use `npm`, `npx`, `yarn`, or `pnpm`. Use `bun` and `bunx`.
- **Linter is Biome** — NEVER use `eslint`, `prettier`, or `tsc` for linting/formatting. Use `bun run check` / `bun run check:fix`.
- **Dev server:** `bunx tauri dev` (NOT `cargo tauri dev` — it doesn't work in this setup)
- **Build:** `bunx tauri build`

## Commands

```bash
bun install                    # Install frontend deps
bunx tauri dev                 # Dev mode (hot reload + Rust rebuild)
bunx tauri build               # Production build
bun run check                  # Biome lint + format check
bun run check:fix              # Biome auto-fix
cargo clippy --all-targets     # Rust linting (run from src-tauri/)
cargo fmt --check              # Rust format check (run from src-tauri/)
cargo test                     # Rust tests (run from src-tauri/)
```

## Architecture

### Frontend (`src/`)
- **Framework:** SolidJS + Tailwind CSS v4 + Kobalte (components) + xterm.js (terminal)
- **Components:** `src/components/{category}/ComponentName.tsx` — layout/, terminal/, task/, worktree/, pr/, ui/
- **IPC wrappers:** `src/lib/commands/` — git.ts, terminal.ts, task.ts, run.ts, agent.ts, github.ts, knowledge.ts, workspace.ts
- **Stores:** `src/lib/stores/` — repo.ts, task.ts, layout.ts, agent.ts, terminal.ts, knowledge.ts
- **Types:** `src/types/` — git.ts, task.ts, run.ts, agent.ts, github.ts, knowledge.ts, terminal.ts

### Backend (`src-tauri/src/`)
- **Entry:** `lib.rs` (app init, plugin setup, managed state) + `main.rs` (Tauri entry)
- **Commands:** `commands/` — thin IPC handlers, NO business logic
- **Services:** `services/` — all business logic lives here, daemon-extractable
- **Models:** `models/` — domain types with Serialize/Deserialize for IPC
- **Errors:** `error.rs` — single `AppError` enum via thiserror
- **Git:** git2 crate (in-process, not CLI shelling)
- **Terminal:** portable-pty (Rust) → xterm.js (frontend) via Tauri events

### Key rule
No business logic in Tauri command handlers — commands validate args, call a service, return the result. Services take dependencies as parameters, no global state.

## Development Workflow

- Features developed in **git worktrees** (multiple features in parallel)
- Each feature branch gets a **PR to main**
- CI runs on PRs: Biome check, Clippy, fmt, build, tests
- Claude auto-reviews PRs and can auto-fix CI failures

## Code Standards

### TypeScript
- Strict mode, no `any` except IPC boundaries
- Named exports only (no default exports)
- Single quotes, semicolons always, 2-space indent, 100 char line width (Biome enforces)
- Tauri IPC calls wrapped in `src/lib/commands/`, never call `invoke()` from components
- No barrel files

### Rust
- `unsafe` denied, `unwrap()`/`expect()` warned (use `?` with thiserror)
- Clippy pedantic enabled (warns, not errors)
- `#[allow(clippy::needless_pass_by_value)]` on command handlers (Tauri IPC requires owned types)
- All errors via thiserror `AppError` enum in `error.rs`
- Models derive `Serialize`, `Deserialize`, use `#[serde(rename_all = "kebab-case")]`

### Stores (SolidJS)
- Factory function pattern: `getRepoStore()`, `getTaskStore()`, etc.
- Singleton via global variable + lazy init
- Uses `createStore` + `produce` for state mutations
- `batch()` when handling multiple updates from Tauri events
- Listens to Tauri events via `listen<EventType>(event_name, callback)`

### Events (Rust → Frontend)
- Services emit via `app_handle.emit(event_name, payload)`
- Namespaced: `task:updated`, `run:status_changed`, `agent:event`
- Frontend stores listen with `listen<T>()` from `@tauri-apps/api/event`

### Adding a New Feature
1. Models: `src-tauri/src/models/` — domain types
2. Service: `src-tauri/src/services/` — business logic + logging
3. Command: `src-tauri/src/commands/` — thin IPC handler, register in `lib.rs` invoke_handler
4. Frontend types: `src/types/`
5. IPC wrapper: `src/lib/commands/` — with try/catch + `logError`
6. Store: `src/lib/stores/` — if feature needs reactive state
7. Component: `src/components/{category}/`

### Logging
All service code must include structured logging via `tauri-plugin-log` (Rust) and `@tauri-apps/plugin-log` (frontend).

**Rust services** — use `log` crate macros:
- `info!()` — state-changing operations that succeed (create, delete, save)
- `debug!()` — read operations, expected branches (list, load, branch reuse)
- `error!()` — every failure path, including `.map_err()` on `?` propagation
- `trace!()` — hot-path diagnostics only (per-keystroke, per-frame). Never `debug!` on hot paths

```rust
use log::{debug, error, info, trace};

pub fn create_thing(name: &str) -> Result<Thing, AppError> {
    let result = do_work(name).map_err(|e| {
        error!("Failed to create thing {name:?}: {e}");
        e
    })?;
    info!("Created thing {name:?} at {}", result.path.display());
    Ok(result)
}
```

**Frontend IPC wrappers** — wrap every `invoke()` call with try/catch + `logError`:
```typescript
import { error as logError } from '@tauri-apps/plugin-log';

export async function doThing(id: string): Promise<Thing> {
  try {
    return await invoke<Thing>('do_thing', { id });
  } catch (e) {
    logError(`doThing failed: ${e}`);
    throw e;
  }
}
```

### Conventions
- Conventional commits (`feat:`, `fix:`, `refactor:`, `docs:`, `chore:`)
- No commented-out code

## CI/CD

### Workflows (`.github/workflows/`)
- **ci.yml** — PR checks: path-filtered Biome + Clippy + fmt → full build + tests on push/approval
- **claude-code-review.yml** — Auto-reviews every PR with inline comments
- **claude.yml** — Responds to `@claude` mentions on PRs/issues
- **claude-ci-fix.yml** — Auto-fixes CI failures on feature branches (lint, fmt, clippy)
- **release.yml** — Manual dispatch: version bump → tag → build → GitHub release

### CI runs Rust checks from `src-tauri/`
Clippy, fmt, and tests run with `working-directory: src-tauri`. When fixing CI failures locally, run cargo commands from `src-tauri/`:
```bash
cd src-tauri && cargo clippy --all-targets -- -D warnings
cd src-tauri && cargo fmt --check
cd src-tauri && cargo test
```

## Docs

- `docs/mvp-brief.md` — product design brief and layout decisions
- `docs/tech-stack.md` — full tech stack rationale, file structure, CI config, performance targets
