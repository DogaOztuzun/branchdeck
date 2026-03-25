---
name: sat-run
description: >
  Execute SAT test scenarios against the real Branchdeck app via WebDriver.
  Reads scenarios from sat/scenarios/, runs each against a debug binary using
  tauri-driver + WebdriverIO, captures screenshots and trajectory data.
  Use when user says "run sat scenarios", "sat run", "execute scenarios",
  or "test the app with sat".
---

# SAT Scenario Runner

You execute SAT test scenarios against the real Branchdeck application using WebDriver automation. Each scenario's natural-language steps are translated into browser actions, with before/after screenshots and performance metrics captured per step.

## Prerequisites

Before running, verify these are in place:

1. **Debug binary exists** — Check if `src-tauri/target/debug/branchdeck` exists. If not, tell the user:
   "Debug binary not found. Building now..." and run:
   ```bash
   bunx tauri build --debug --no-bundle
   ```

2. **tauri-driver installed** — Check if `~/.cargo/bin/tauri-driver` exists. If not:
   "tauri-driver not found. Install with: `cargo install tauri-driver --locked`"

3. **WebKitWebDriver available** — Check if `WebKitWebDriver` is in PATH. If not:
   "WebKitWebDriver not found. Install with: `sudo apt install webkit2gtk-driver`"

4. **Scenarios exist** — Check if `sat/scenarios/*.md` files exist (exclude `.gitkeep` and `.sat-state.yaml`). If none:
   "No scenarios found. Run `/sat-generate` first."

## Arguments

The user may provide:
- **scenario** — path to a single scenario file to run (default: run all scenarios)
- **limit** — max number of scenarios to run (default: all)

## Step 1: Setup Run Directory

Create the output directory:
```bash
RUN_DIR="sat/runs/run-$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN_DIR/screenshots"
```

## Step 2: Execute Scenarios

For each scenario file (or the single specified one):

Run the scenario via the SAT WebdriverIO bridge:

```bash
SAT_SCENARIO_FILE="<scenario-path>" SAT_RUN_DIR="<run-dir>" \
  xvfb-run bunx wdio run sat/scripts/wdio.sat.conf.ts 2>&1
```

This will:
1. Start `tauri-driver`
2. Launch the debug binary in the WebKitGTK webview
3. Wait for the app to load ("Branchdeck" title)
4. Execute each scenario step:
   - Take before screenshot
   - Interpret the natural-language step and execute via WebDriver
   - Take after screenshot
   - Record result in trajectory
5. Write `trajectory-{scenario-id}.json` to the run directory
6. Kill the session

**Error handling per scenario:**
- If the WebdriverIO run exits non-zero, check the trajectory file — partial results are still written
- If no trajectory file was created, log the error and continue to the next scenario
- Each scenario gets its own WebDriver session (app restarts fresh)

## Step 3: Review Results

After all scenarios complete, read each `trajectory-*.json` file from the run directory and compile a summary.

**For each trajectory, report:**
- Scenario ID and title
- Status (completed / execution_failed)
- Steps passed vs failed
- Total duration
- Any failure reasons

**Aggregate metrics:**
- Total scenarios run
- Pass rate (completed scenarios / total)
- Average step success rate
- Total screenshots captured
- Total run duration
- Memory usage

Present as a table:

```
| Scenario | Status | Steps | Passed | Failed | Duration |
|----------|--------|-------|--------|--------|----------|
| ...      | ...    | ...   | ...    | ...    | ...      |
```

## Step 4: Performance Report

Read performance data from each trajectory and report:
- **Per-step timing** — average, min, max across all steps
- **Per-scenario timing** — total duration per scenario
- **Memory usage** — heap MB at end of each scenario
- **Screenshot count** — total before + after screenshots

## Common Issues

### "Cannot find module 'run-scenario.ts'"
The SAT scripts need the project's WebdriverIO types. Run `bun install` first.

### "Debug binary not found"
The `onPrepare` hook checks for the binary. Build with:
```bash
bunx tauri build --debug --no-bundle
```

### "Connection refused on port 4444"
`tauri-driver` failed to start. Check if it's installed:
```bash
which tauri-driver
```

### Session dies mid-scenario
The app may have crashed. Check `src-tauri/target/debug/branchdeck` is a recent build. The trajectory file will still be written with partial results — remaining steps are marked as failed.
