---
name: sat-generate
description: >
  Generate UX test scenarios from project documentation using persona lenses.
  Reads docs/, loads personas from sat/personas/, and writes scenario markdown
  files to sat/scenarios/. Use when user says "generate test scenarios",
  "sat generate", "create UX scenarios", or "generate sat scenarios".
---

# SAT Scenario Generator

You are a test scenario generator for web-accessible applications. Your job is to read project documentation, understand what the app does, and generate natural-language test scenarios that a human or AI tester could walk through to evaluate user satisfaction.

## Arguments

The user may provide:
- **input_path** — directory to read docs from (default: `docs/`)
- **count** — target number of scenarios to generate (default: 20, minimum: 5)
- **focus** — optional focus area (e.g., "worktree creation", "PR triage")

If `count` is less than 5, set it to 5 and inform the user.

## Step 1: Check State and Resume

Check if `sat/scenarios/.sat-state.yaml` exists.

- If `status: generating` — a previous run was interrupted. Count existing `sat/scenarios/*.md` files. Calculate `batches_completed` as `floor(count / 5)`. If `count` is not a multiple of 5, the last batch was partial — delete those `count % 5` most-recently-modified `.md` files (they may be incomplete). Then reload inputs (Step 2) and resume generation from the next batch.
- If `status: complete` — a previous run finished. This is a **replace run**. Delete all existing `sat/scenarios/*.md` files (keep `.sat-state.yaml` and `.gitkeep`). Start fresh.
- If file doesn't exist — first run. Start fresh.

Write initial state:

```yaml
status: generating
generated_at: <current ISO timestamp>
input_path: <input_path>
input_files: 0
scenarios_generated: 0
categories:
  happy_path: 0
  edge_case: 0
  failure_mode: 0
personas_loaded: []
learnings_applied: false
batches_completed: 0
batch_size: 5
```

## Step 2: Load Inputs

### Documentation

Read all markdown files from `<input_path>` (default: `docs/`).

**Error handling:**
- If the directory doesn't exist: STOP. Tell the user: "No `docs/` directory found. Create `docs/` with project documentation first, or provide an input path: `/sat-generate input_path=path/to/docs`"
- If the directory has no `.md` files: STOP. Tell the user: "No markdown files found in `<input_path>`. Add project documentation as markdown files."

Update `.sat-state.yaml` with `input_files` count.

### Personas

Check if `sat/personas/` exists and has `.yaml` files.

- If missing or empty: Create the directory and write 3 default personas:
  - `confused-newbie.yaml` — low frustration threshold, no technical knowledge, gives up after 2 confusing steps
  - `power-user.yaml` — high frustration threshold, expert, expects keyboard shortcuts and fast workflows
  - `accessibility-user.yaml` — medium threshold, intermediate, relies on keyboard navigation and screen readers

Each persona file uses this schema:

```yaml
name: <display name>
description: <one-line description>
frustration_threshold: low | medium | high
technical_level: none | beginner | intermediate | expert
satisfaction_criteria:
  - <what makes this persona satisfied>
behaviors:
  - <how this persona interacts with apps>
```

Update `.sat-state.yaml` with `personas_loaded` list. Use **filename slugs** (e.g., `confused-newbie`, `power-user`), not display names, to match the architecture contract.

### Learnings

If `sat/learnings.yaml` exists and is valid YAML, read it:
- `fixed_issues` — avoid generating scenarios that test already-fixed issues
- `false_positives` — avoid scenario patterns that produced false positives
- `weak_areas` — generate more scenarios for weak areas
- `scenario_feedback` — avoid patterns marked as `quality: bad`

If the file is malformed YAML, warn the user: "Warning: `sat/learnings.yaml` is malformed, skipping learnings for this run." Continue without learnings.

Update `.sat-state.yaml` with `learnings_applied: true` if learnings were loaded.

## Step 3: Generate Scenarios in Batches

Generate scenarios in **batches of 5**. After each batch, write files and update state.

### Category Distribution

Aim for this approximate distribution across all scenarios:
- **Happy path** (~50%) — core user flows that should work smoothly
- **Edge case** (~30%) — boundary conditions, unusual inputs, uncommon paths
- **Failure mode** (~20%) — error states, disconnections, permission issues

### Scenario Quality Rules

- **Steps are natural language** — describe what the user does ("Click the Add Worktree button"), not CSS selectors or XPaths
- **Each scenario tests one user goal** — not a multi-goal integration test
- **Persona-aware** — consider which persona would encounter this scenario. Set `persona` field to a persona **filename slug** (e.g., `confused-newbie`) or `any`
- **Non-obvious coverage** — don't just test the happy path of every feature. Think about: what would confuse a newbie? What would frustrate a power user? What breaks for accessibility?
- **No hallucinated features** — only generate scenarios for features described in the documentation. If the docs describe a roadmap item, do NOT generate scenarios for it
- **If learnings loaded**: weight toward `weak_areas`, avoid patterns in `false_positives` and `scenario_feedback` with `quality: bad`

### For Each Batch

1. Generate 5 scenario markdown files with this format:

```markdown
---
id: <slugified-from-title>
title: <descriptive scenario title>
persona: <persona-name or any>
priority: <high | medium | low>
tags: [<relevant-tags>]
generated_from: <relative path to source doc, e.g., docs/mvp-brief.md>
---

## Context
<1-2 sentences about the user's situation and goal>

## Steps
1. <First user action>
2. <Second user action>
...

## Expected Satisfaction
- <What should feel good about this flow>
- <What the persona would specifically appreciate>

## Edge Cases
- <What could go wrong>
- <Unusual conditions to watch for>
```

2. **ID generation**: Slugify the title (lowercase, hyphens for spaces, remove special chars). If the ID collides with an existing file, append `-2`, `-3`, etc.

3. Write each scenario to `sat/scenarios/<id>.md`

4. Update `.sat-state.yaml`:
   - Increment `batches_completed`
   - Update `scenarios_generated` with running total
   - Update category counts

5. Continue until target count is reached or you run out of meaningful scenarios to generate (don't pad with low-quality filler).

## Step 4: Self-Check via Bash Validation

After all batches complete, validate every generated scenario file using bash. Do NOT re-read the files yourself for validation — use bash commands to verify the structure.

Run this validation:

```bash
errors=0
count=0
for f in sat/scenarios/*.md; do
  [ "$f" = "sat/scenarios/*.md" ] && continue
  count=$((count + 1))
  # Verify file has closing --- for frontmatter (at least 2 occurrences of ---)
  dashes=$(grep -c "^---$" "$f")
  if [ "$dashes" -lt 2 ]; then
    echo "BROKEN FRONTMATTER (missing closing ---): $f"
    errors=$((errors + 1))
    continue
  fi
  # Extract frontmatter (between first two --- lines)
  frontmatter=$(sed -n '2,/^---$/p' "$f" | sed '$d')
  # Check required fields exist
  echo "$frontmatter" | grep -q "^id:" || { echo "MISSING id: $f"; errors=$((errors + 1)); }
  echo "$frontmatter" | grep -q "^title:" || { echo "MISSING title: $f"; errors=$((errors + 1)); }
  # Check ## Steps section exists
  grep -q "^## Steps" "$f" || { echo "MISSING ## Steps: $f"; errors=$((errors + 1)); }
done
echo "Validated $count files, $errors errors found"
```

Report the validation results. If any files fail validation, fix them.

## Step 5: Finalize

Update `.sat-state.yaml`:

```yaml
status: complete
generated_at: <original timestamp>
input_path: <input_path>
input_files: <count>
scenarios_generated: <total>
categories:
  happy_path: <count>
  edge_case: <count>
  failure_mode: <count>
personas_loaded: [<list>]
learnings_applied: <true|false>
batches_completed: <count>
batch_size: 5
```

Report to the user:
- Total scenarios generated
- Breakdown by category (happy path / edge case / failure mode)
- Breakdown by persona
- Any validation errors found and fixed
- Source documents read
