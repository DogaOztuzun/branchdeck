# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Branchdeck is a Tauri v2 desktop app — terminal-first workflow manager for git repositories and worktrees. Linux-first, open source (MIT).

**Stack:** Tauri v2 (Rust backend) + SolidJS (TypeScript frontend) + xterm.js (terminal)

## Architecture

- **Frontend:** SolidJS + Tailwind CSS v4 + Kobalte (components) + xterm.js (terminal)
- **Backend:** Rust single crate with `commands/` (thin Tauri IPC handlers), `services/` (business logic), `models/` (types)
- **Key rule:** No business logic in Tauri command handlers — services are daemon-extractable
- **Git:** git2 crate (in-process, not CLI shelling)
- **Terminal:** portable-pty (Rust) → xterm.js (frontend) via Tauri events
- **Package manager:** Bun
- **Linting:** Biome (frontend), Clippy pedantic (Rust)

## Commands

```bash
bun install                    # Install frontend deps
cargo tauri dev                # Dev mode (hot reload + Rust rebuild)
cargo tauri build              # Production build
bun run check                  # Biome lint + format check
bun run check:fix              # Biome auto-fix
cargo clippy --all-targets     # Rust linting
cargo fmt --check              # Rust format check
cargo test                     # Rust tests
```

## Code Standards

### TypeScript
- Strict mode, no `any` except IPC boundaries
- Named exports only (no default exports)
- Tauri IPC calls wrapped in `src/lib/commands/`, never call `invoke()` from components
- No barrel files

### Rust
- `unsafe` denied, `unwrap()`/`expect()` warned (use `?` with thiserror)
- Clippy pedantic enabled
- Services take dependencies as parameters, no global state
- All errors via thiserror enums in `error.rs`

### Logging
All new service code must include structured logging via `tauri-plugin-log` (Rust) and `@tauri-apps/plugin-log` (frontend).

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

**Log level config** is in `src-tauri/src/lib.rs`: global `Info`, crate-level `Debug`. Third-party noise stays at `Info`.

### Conventions
- Conventional commits (`feat:`, `fix:`, `refactor:`, `docs:`, `chore:`)
- No commented-out code

## Docs

- `docs/mvp-brief.md` — product design brief and layout decisions
- `docs/tech-stack.md` — full tech stack rationale, file structure, CI config, performance targets
