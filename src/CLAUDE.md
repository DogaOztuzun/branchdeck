# Frontend (SolidJS)

Shared frontend for desktop and web modes. Connects to daemon via HTTP/SSE.

## Rules

- All HTTP calls through `src/lib/api/client.ts` — never call `ky` or `fetch` directly from components
- All SSE subscriptions through `src/lib/api/events.ts` — use `onEvent<T>()` helper
- All UI must conform to DESIGN.md — Tokyo Night palette, JetBrains Mono, dark-only
- No default exports — named exports only
- No barrel files
- Strict TypeScript — no `any` except at IPC/API boundaries

## Structure

- `components/{category}/ComponentName.tsx` — layout/, terminal/, task/, worktree/, pr/, ui/
- `lib/api/client.ts` — ky instance with base URL from window.location (never hardcoded)
- `lib/api/events.ts` — EventSource connection, typed `onEvent<T>()`
- `lib/stores/` — factory function pattern, `createStore` + `produce`, `batch()` for multiple updates
- `types/` — domain types mirroring Rust models

## Design System

Always read @DESIGN.md before visual/UI work. Highlights:
- 0px border-radius on all containers (2px for inline hover only)
- No spinners — pulsing opacity (0.4→1.0, 2s) for loading
- No modals for primary actions — one click, not two
- Inline error text in error color — never toast/modal
- Monospace characters as icons (`>_` for agent, 8x8 squares for status)
