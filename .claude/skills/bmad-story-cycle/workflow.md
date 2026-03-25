# Epic Development Cycle Workflow

**Goal:** Take an entire epic from epics.md through implementation -- all stories in sequence, each scored for complexity, implemented with the right level of ceremony, and merged. One command, one epic, done.

## Configuration

- **epics_file:** `{project-root}/_bmad-output/planning-artifacts/epics.md`
- **architecture_file:** `{project-root}/_bmad-output/planning-artifacts/architecture.md`
- **project_context:** `{project-root}/_bmad-output/project-context.md`
- **design_file:** `{project-root}/DESIGN.md`
- **worktree_base:** `{project-root}/../branchdeck-worktrees`

## Step 1: Load Epic & Score Stories

**Parse** the epic number (N) from user input. Accept: `1`, `epic 1`, or `1.3` to resume from a specific story.

**Read** the epics file and extract Epic N with all its stories.

**Read** the architecture document's FR-to-Structure Mapping.

**Score each story** based on complexity:

### Complexity Scoring

For each story, evaluate:
- **Files touched:** How many existing files need modification? (grep the codebase for relevant services/models)
- **Refactor vs new:** Is this creating new files or changing existing patterns?
- **Core coupling:** Does it touch EventBus, RunManager, orchestrator, effects, or other core patterns?
- **Cross-boundary:** Does it span Rust + TypeScript, or multiple services?

| Score | Criteria | Approach |
|:--|:--|:--|
| **Simple** | New isolated file/module, no existing code touched, < 3 files | Quick-dev oneshot, auto-merge |
| **Medium** | Extends existing service, 3-6 files, follows established patterns | Quick-dev full flow (plan + implement + review), auto-merge |
| **Complex** | Refactors existing code, 6+ files, touches core patterns, cross-boundary | Create-story first, then quick-dev, then party-mode review before merge |

### Architecture Level Tag

Tag each story for context loading:

| Level | When | Extra context for subagent |
|:--|:--|:--|
| **Core** | Models, services, effects, crate structure | Architecture crate boundaries, FR-to-structure mapping, effect pattern rules |
| **Integration** | GitHub API, sidecar, orchestrator, event system | Sidecar protocol spec, event schema, GitHub polling patterns |
| **Frontend** | Components, stores, views, IPC migration | DESIGN.md, UX-DRs, component patterns from design-artifacts |
| **Infrastructure** | Docker, CI, packaging, deployment | Dockerfile patterns, Axum server setup, release workflow |
| **Standalone** | Isolated concerns, config, documentation | Minimal -- story ACs are sufficient |

**Present** the scored epic:
```
Epic N: [title] — [story count] stories

  N.1: [title] — COMPLEX (core, 8 files, refactors EventBus coupling)
  N.2: [title] — MEDIUM (core, new file + extends registry)
  N.3: [title] — SIMPLE (core, new service, isolated)
  N.4: [title] — MEDIUM (integration, extends orchestrator)
  N.5: [title] — MEDIUM (core, new detectors + lifecycle events)
```

If resuming from a specific story, note: "Resuming from Story N.M"

HALT and ask: **"Ready to run? Adjust any scores or skip any stories?"**

## Step 2: Epic Branch Setup

**Check** if the epic branch exists:
```bash
git branch --list "epic/N-*"
```

**If epic branch does NOT exist:**
```bash
git checkout main
git pull origin main
git checkout -b epic/N-[epic-title-kebab]
```

**If it exists** (resuming):
```bash
git checkout epic/N-[epic-title-kebab]
```

## Step 3: Story Loop

For each story N.M in the epic (in order):

---

### 3a: Worktree Setup

```bash
git worktree add {worktree_base}/story-N.M epic/N-[epic-title-kebab]
cd {worktree_base}/story-N.M
git checkout -b story/N.M-[story-title-kebab]
bun install
```

### 3b: Create Story Context (COMPLEX only)

If story is scored **Complex**, spawn a subagent first:

```
/bmad-create-story Story N.M from {epics_file}. Add implementation context: which existing files to modify, which patterns to follow, known couplings to handle. Write to {worktree_base}/story-N.M/.branchdeck/story-N.M.md
```

Wait for completion. The detailed story file gives quick-dev better context for complex refactors.

### 3c: Implement via quick-dev subagent

Build the subagent prompt based on complexity and architecture level:

**Simple:**
```
/bmad-quick-dev Story N.M from {epics_file} — implement and commit. Work in {worktree_base}/story-N.M only.
```

**Medium:**
```
/bmad-quick-dev Story N.M from {epics_file} — analyze, plan, implement, verify, test, review, fix. Work in {worktree_base}/story-N.M only. Architecture context: {architecture_level_context}
```

**Complex:**
```
/bmad-quick-dev Story N.M using detailed story at {worktree_base}/story-N.M/.branchdeck/story-N.M.md — analyze, plan, implement, verify, test, review, fix. Work in {worktree_base}/story-N.M only. Architecture context: {architecture_level_context}
```

Where `{architecture_level_context}` is:
- **Core:** "Read architecture.md sections: Crate Structure, Implementation Patterns, Effect Pattern. Follow crate boundary rules."
- **Integration:** "Read architecture.md sections: Sidecar Strategy, GitHub Integration, Event Schema, Communication Protocol."
- **Frontend:** "Read DESIGN.md for visual specs. Read architecture.md sections: Frontend Responsibility Boundary, Frontend Transport Migration."
- **Infrastructure:** "Read architecture.md sections: Docker deployment, REST API, MCP Server, Desktop <-> Daemon Lifecycle."
- **Standalone:** (no extra context)

Wait for the subagent to complete.

### 3d: Ensure Commit

Quick-dev should have committed, but verify. If the worktree has uncommitted changes:

```bash
cd {worktree_base}/story-N.M
git status --porcelain
```

If dirty, stage and commit:
```bash
git add -A
git commit -m "$(cat <<'EOF'
feat(epic-N): Story N.M — [story title]

FRs: [FR numbers]

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>
EOF
)"
```

### 3e: Verify

```bash
cd {worktree_base}/story-N.M
bun run verify
```

**If verify fails:** HALT. Options:
- **[F] Fix** — provide guidance, re-run quick-dev subagent with fix instructions
- **[S] Skip** — skip this story, mark as skipped, continue
- **[X] Exit** — stop epic cycle, leave worktree for manual work

### 3f: Party-Mode Review (COMPLEX only)

If story is scored **Complex**, spawn a party-mode review subagent:

```
/bmad-party-mode Review the implementation of Story N.M for Epic N.
Diff: run `git diff epic/N-[branch]..HEAD` in {worktree_base}/story-N.M
Winston: check architecture compliance — crate boundaries, thin handlers, effect pattern.
Amelia: check code quality — error handling, logging, test coverage, no unwrap.
Quinn: check test coverage — happy path, error case, edge case per function.
Report: list findings as BLOCK (must fix), WARN (should fix), or OK.
```

Wait for review. If any BLOCK findings:
- HALT and present to user
- **[F] Fix** — re-run quick-dev with the findings as fix instructions
- **[O] Override** — merge anyway (user accepts the risk)
- **[X] Exit** — leave for manual work

If only WARN or OK, auto-continue.

### 3g: Merge Story into Epic Branch

```bash
cd {project-root}
git checkout epic/N-[epic-title-kebab]
git merge --no-ff story/N.M-[story-title-kebab] -m "feat(epic-N): Story N.M — [story title]

FRs: [FR numbers]
Complexity: [simple/medium/complex]

Co-Authored-By: Claude Opus 4.6 (1M context) <noreply@anthropic.com>"
git worktree remove {worktree_base}/story-N.M
git branch -d story/N.M-[story-title-kebab]
```

### 3h: Update Sprint Status

Update the sprint status file to reflect story completion:

Find the sprint status file at `{project-root}/_bmad-output/implementation-artifacts/sprint-status*.yaml` (or `.yml`). Update Story N.M status to `done` with the commit hash and date.

If the sprint status file doesn't exist, skip this step.

Report: `Story N.M [SIMPLE|MEDIUM|COMPLEX] done. [files changed] files. Sprint status updated. Next: Story N.{M+1}`

### 3i: Continue to next story

Loop back to 3a. Continue until all stories complete or user exits.

---

## Step 4: Epic Complete

After all stories are merged into the epic branch:

```bash
cd {project-root}
git checkout epic/N-[epic-title-kebab]
bun run verify
```

**Update sprint status** — mark Epic N as complete in the sprint status file.

**Present epic summary:**
```
Epic N: [title] — ALL STORIES MERGED

Stories: [completed] done, [skipped] skipped
Complexity: [X simple, Y medium, Z complex]
Total files changed: [count]
FRs covered: [list]
Verify: [pass/fail]
```

**Suggest next steps (user handles manually):**
- `bmad-code-review` — full review of epic diff vs main
- `bmad-retrospective` — post-epic retro for lessons learned
- Push epic branch + create PR when ready

Workflow complete. HALT.

---

## Parallel Execution

Run multiple epic cycles in separate terminals:

```
Terminal 1: /bmad-story-cycle epic 1    (critical path)
Terminal 2: /bmad-story-cycle epic 6    (trust controls — parallel)
Terminal 3: /bmad-story-cycle epic 7    (platform — parallel)
```

Each epic gets its own branch. No conflicts between independent epics.

## Error Recovery

- **Worktree exists:** Previous failed run. Offer to clean up or resume from that story.
- **Story subagent fails:** HALT with options (Fix/Skip/Exit). Never auto-continue past a failure.
- **Merge conflict:** Present to user. Do not auto-resolve.
- **Resume:** Pass `N.M` to start from a specific story (e.g., `/bmad-story-cycle 1.3`).
- **Context limit:** If the epic has many stories and context grows large, the story loop handles this naturally — each quick-dev subagent gets a fresh context. The parent workflow only tracks progress state.
