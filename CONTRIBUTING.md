# Contributing to Branchdeck

Thanks for your interest in contributing! Branchdeck is in early alpha and we welcome all contributions — bug reports, feature ideas, code, and docs.

## Branch Strategy

| Branch | Purpose |
|:-------|:--------|
| `main` | Stable releases only — CI builds release packages |
| `dev` | Active development — all PRs target this branch |

**All pull requests should target the `dev` branch**, not `main`. Merges from `dev` to `main` are done by maintainers when cutting a release.

## Getting Started

1. Fork the repo and clone your fork
2. Create a branch from `dev`:
   ```bash
   git checkout dev
   git checkout -b feat/my-feature
   ```
3. Install dependencies:
   ```bash
   # System libraries (Ubuntu/Debian)
   sudo apt-get install -y \
     libwebkit2gtk-4.1-dev \
     libjavascriptcoregtk-4.1-dev \
     libsoup-3.0-dev \
     libgtk-3-dev \
     libayatana-appindicator3-dev

   # Frontend
   bun install
   ```
4. Run the app:
   ```bash
   bunx tauri dev
   ```

## Before Submitting a PR

Run all checks and make sure they pass:

```bash
# Frontend
bun run check            # Biome lint + format
bun run check:fix        # Auto-fix (if needed)

# Rust (from src-tauri/)
cargo fmt --check        # Format check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Code Standards

### TypeScript
- Strict mode — no `any` except at IPC boundaries
- Named exports only (no default exports)
- Never call `invoke()` from components — use wrappers in `src/lib/commands/`
- No barrel files

### Rust
- `unsafe` is denied
- No `unwrap()` or `expect()` — use `?` with thiserror
- Clippy pedantic is enabled
- Services (`src-tauri/src/services/`) must not import any Tauri types — they should be daemon-extractable
- Commands (`src-tauri/src/commands/`) are thin wrappers that delegate to services

### Commits
- Use [conventional commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`
- No commented-out code

## Architecture Overview

```
src-tauri/src/
  commands/    # Thin Tauri IPC handlers
  services/    # Business logic (no Tauri imports)
  models/      # Shared types
  error.rs     # thiserror enums

src/
  components/  # SolidJS UI components
  lib/
    commands/  # Typed IPC wrappers (frontend ↔ backend)
    stores/    # SolidJS state management
  types/       # TypeScript type definitions
```

Key rule: **no business logic in command handlers**. Services are kept free of Tauri dependencies so they can be extracted into a standalone daemon in the future.

## Development Methodology

This project uses the [BMAD-METHOD](https://github.com/bmadcode/BMAD-METHOD) for AI-assisted development. The `_bmad/` directory contains the framework — you can use it with Claude Code to plan and implement features following the same workflow.

## Reporting Issues

Open an issue on GitHub. Include:
- What you expected vs what happened
- Steps to reproduce
- OS and version
- Any relevant logs (check the terminal where `bunx tauri dev` is running)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
