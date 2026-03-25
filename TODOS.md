# TODOS

Deferred work tracked from design reviews and implementation sessions.

## SAT: Scenario-driven AI Testing

### POC Complete (2026-03-25)

Full pipeline validated: generate → run → score → fix → re-score (72 → 88, +16). 3 real app issues found.

- [x] **Phase 1: sat-generate** — 20 scenarios, 3 personas, bash validation, resume-on-crash. Commit `edb0462`.
- [x] **Phase 2: sat-run** — WebDriver bridge via tauri-driver + WebdriverIO + xvfb-run. 73% step pass rate. Commits `3205610`, `1777b54`.
- [x] **Phase 2 spike** — tauri-driver + WebKitWebDriver works on Linux. /chrome approach rejected (no Tauri IPC outside webview).
- [x] **Phase 3: sat-score** — Persona-lens scoring, issue classification (app/runner/scenario), learnings update. Commit `fbebb30`.
- [x] **Resume-on-crash logic** — Implemented in sat-generate skill.
- [x] **Bash-based YAML validation** — Implemented in sat-generate skill.

### Deferred (iteration, not proof)

- [ ] **`/sat` orchestrator** — Wire generate → run → score into one command. ~15 min. Blocked by: nothing.
- [ ] **Step interpreter improvements** — Push 73% → 90%+ pass rate. Add Kobalte component handling, compound step splitting, content-aware verification. Blocked by: nothing.
- [ ] **Persona-specific app state setup** — Newcomer persona implies fresh state (no repos). Runner needs precondition setup per persona. Blocked by: nothing.
- [ ] **Scenario ID stability across regenerations** — learnings.yaml references scenario IDs. Need deterministic IDs or matching logic for learning continuity. Blocked by: nothing.
- [ ] **CI integration** — Run SAT in GitHub Actions via `claude-code-action`. Blocked by: orchestrator.
- [ ] **Scenario preconditions** — add-first-repository scenario assumes empty workspace but test env has repos. Scenarios need `## Preconditions` section. Blocked by: nothing.

### App issues found by SAT

- [ ] **Empty state onboarding** (medium) — "Add Repository" button at sidebar bottom has low visual weight. No guidance for first-time users.
- [ ] **No auto-terminal on worktree click** (medium) — Clicking a worktree shows "No terminal open" instead of auto-opening a shell.
- [ ] **Disabled button without explanation** (low) — Create Worktree button disabled when input empty, no tooltip explains why.

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
