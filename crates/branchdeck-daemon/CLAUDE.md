# branchdeck-daemon

Axum server binary wrapping branchdeck-core. Two modes: `serve` (HTTP + SSE + WS) and CLI subcommands.

## Rules

- Route handlers MUST be thin — "Call core.X and return the result"
- No business logic in handlers — if it has `if/match` branching, move it to core
- Serde validates request shape, core validates semantics
- Error format: RFC 7807 Problem Details
- SSE events: `namespace:snake_case_action` naming enforced
- OpenAPI 3.1 auto-generated via utoipa

## Key Routes

```
GET    /api/events          → SSE stream
GET    /api/workflows       → Vec<WorkflowDef>
POST   /api/runs            → RunInfo
GET    /api/runs/:id        → RunInfo
POST   /api/runs/:id/cancel → ()
GET    /api/health          → HealthInfo
WS     /api/terminal/:tab_id → PTY stream
GET    /mcp                 → MCP-over-HTTP
```

## Auth

- Localhost: no auth (default)
- Remote (`--bind 0.0.0.0` or `--require-auth`): bearer token in `Authorization` header
- SSE/WS: token as `?token=...` query parameter (EventSource can't set headers)

See @.claude/rules/daemon-api.md for patterns and examples.
