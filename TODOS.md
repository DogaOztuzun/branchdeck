# TODOS

Deferred work tracked from design reviews and implementation sessions.

## SAT: Scenario-driven AI Testing (from eng review 2026-03-24)

### Phase 1 (implement during skill creation)

- [ ] **Resume-on-crash logic** — Skill checks sat/scenarios/ and .sat-state.yaml before starting. If status is 'generating' with N scenarios written, resume from next batch instead of replacing. Without this, batch-of-5 crash safety is meaningless. Blocked by: nothing.
- [ ] **Bash-based YAML validation** — Self-check step uses a bash command (e.g., extracting frontmatter and parsing) instead of Claude re-reading its own output. Same-model validation catches nothing the model already missed. Blocked by: nothing.

### Phase 2 (defer to sat-run.md)

- [ ] **Pre-Phase-2 spike: /chrome + Tauri webview** — Verify Claude Code /chrome can navigate localhost:1420, click elements, and read page state in Tauri's webview. If it can't, Phase 2 needs alternative browser approach. Blocked by: Phase 1 complete.
- [ ] **Persona-specific app state setup** — Newcomer persona implies fresh state (no repos). Power user implies populated state. Runner needs to set up different app states before executing scenarios for different personas. Phase 1 scenarios should include preconditions in Context section. Blocked by: Phase 1 complete.

### Phase 3 (defer to sat-score.md)

- [ ] **Scenario ID stability across regenerations** — learnings.yaml references scenario IDs. If IDs change when titles change, learnings become orphaned. Need deterministic IDs or ID matching logic for learning continuity. Blocked by: Phase 2 complete.

## Phase 2: Daemon Extraction (from eng review 2026-03-20)

### Deferred to post-Steps 1-3

- [ ] **Step 4: Knowledge flow across runs** — Post-run artifact capture feeds into next run's context. Cross-repo knowledge injection during orchestration. Blocked by: Steps 1-3 complete.
- [ ] **Step 5: Multi-pane dashboard UI** — Grid layout showing N terminals simultaneously. Orchestration status bar, per-run badges. The "demo screen." Blocked by: Steps 1-3 complete.
- [ ] **OpenAPI spec generation** — utoipa or similar for auto-generated API docs. Nice-to-have for Step 3, not blocking. Can add after API surface stabilizes.
- [ ] **CLI wrapper (branchdeck-cli)** — `branchdeck-cli orchestrate --prompt "..." --repos repo1,repo2,repo3`. curl scripts suffice for initial demo. Add when API surface is stable.
- [ ] **Docker containerization** — Daemon should be containerizable for VPS hosted path. No Tauri deps in branchdeck-daemon makes this straightforward. Blocked by: Step 1 complete.
- [ ] **Token auth for non-localhost** — Localhost binding is sufficient for Phase 2. Token auth needed when daemon serves remote clients (hosted version). Blocked by: hosted path decision.
- [ ] **DELETE endpoints** — `DELETE /api/repos/:id`, `DELETE /api/repos/:id/worktrees/:name`. Not needed for the orchestration demo. Add when full CRUD is required.
- [ ] **MultiplexedSidecarHandle** — Upgrade from one-Node.js-per-run to multiplexed sidecar. Fix 3 JS globals (activeAbort, activeSessionId, pendingPermissions) to be Maps keyed by session ID. Optimization for when parallel run count exceeds 4-5. Blocked by: ProcessSidecarHandle working.

### From CEO review 2026-03-20

- [ ] **WebSocket event playground** (P3) — Vanilla JS page at `GET /playground` showing live WS events. Interactive demo surface for developers evaluating the API. Zero deps. Ship with Step 2 or 3 when there are run events to show. Blocked by: Step 1 (daemon + WS endpoint).
- [ ] **Go-to-market plan for API launch** (P2) — Document announcement strategy: r/ClaudeAI, HN, X posts, 60-sec screencast, curl quick-start as viral hook. Write when Steps 2-3 are in progress. Blocked by: Steps 1-3 nearing completion.
- [ ] **WS connection limit** (P3) — Max concurrent WebSocket connections (e.g., 32). Not needed for localhost-only Phase 2. Add when daemon serves remote clients. Blocked by: hosted path.

## Step 5: Dashboard UI Prerequisites (from design review 2026-03-21)

- [ ] **Create DESIGN.md via /design-consultation** — Full design system (color palette, typography, spacing, component patterns, motion) before implementing the orchestration dashboard. The existing codebase has implicit patterns (dark theme, Tailwind v4, status colors) but nothing documented. Prerequisite for Step 5 implementation. Blocked by: nothing.
- [ ] **Full ARIA accessibility for orchestration dashboard** — Screen reader announcements for run status changes, ARIA landmarks for dashboard regions, live regions for real-time updates. Deferred because it's a desktop dev tool for terminal power users, but worth adding as the product matures. Blocked by: Step 5 complete.

## Phase 1: PR Shepherd (from sprint plan 2026-03-20)

- [ ] **Phase C: Knowledge injection** — Pre-run knowledge query, inject context into task.md. The "it learns" moment. Blocked by: Phase B complete.
- [ ] **Demo video** — Phase 1 graduation moment. Record after Phase B proves the workflow.
