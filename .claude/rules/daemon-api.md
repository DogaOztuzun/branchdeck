---
description: Daemon HTTP API, SSE, and frontend transport patterns
globs: ["crates/branchdeck-daemon/**/*.rs", "src/lib/api/**/*.ts", "src/lib/stores/**/*.ts"]
---

# Daemon API Patterns

## REST API

- OpenAPI 3.1 via utoipa, served at `/api/docs` and `/api/openapi.json`
- Schema version in `X-Branchdeck-Schema` response header (no URL versioning)
- Errors: RFC 7807 Problem Details format
- Auth: no-auth on localhost (default), bearer token when `--require-auth` or `--bind 0.0.0.0`

## SSE Events

- Endpoint: `GET /api/events`
- Format: JSON typed envelope `{ "id": "evt_<ulid>", "type": "<namespace>:<action>", "timestamp": ..., "data": {} }`
- Namespaces: `run:`, `agent:`, `workflow:`, `sat:`, `system:`
- Naming: `namespace:snake_case_action` — enforced. No camelCase, no dots, no UPPER_CASE.

## Frontend Transport

- **Base URL — never hardcode:**
  ```typescript
  const BASE_URL = import.meta.env.VITE_API_URL
    ?? `${window.location.protocol}//${window.location.host}/api`;
  ```
- **All HTTP calls through `src/lib/api/client.ts`** — components never call `ky` or `fetch` directly
- **All SSE subscriptions through `src/lib/api/events.ts`** — stores use `onEvent<T>()` helper
- **Connection states:** connected (no indicator), reconnecting (TopBar text), disconnected (banner after 30s)

## Route Handler Thinness Test

Can you describe the handler as "Call core.X and return the result"? If yes: thin. If no: logic leaked.

```rust
// THIN — correct
async fn cancel_run(Path(id): Path<RunId>, State(s): State<AppState>) -> Result<(), AppError> {
    s.core.cancel_run(&id)
}

// NOT THIN — move branching logic to core
async fn cancel_run(Path(id): Path<RunId>, State(s): State<AppState>) -> Result<(), AppError> {
    let run = s.core.get_run(&id)?;
    if run.status.is_active() { /* logic leaked */ }
}
```
