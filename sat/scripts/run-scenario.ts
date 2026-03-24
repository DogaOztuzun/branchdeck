/**
 * SAT Scenario Runner — WebdriverIO test file
 *
 * Reads a scenario markdown file (path via SAT_SCENARIO_FILE env),
 * executes each step via WebdriverIO against the real Branchdeck app,
 * captures before/after screenshots, and writes trajectory.json.
 *
 * This is the "bridge" between natural-language scenario steps and
 * WebDriver browser actions. Step interpretation is intentionally
 * simple — Claude refines it via the sat-run skill.
 */

import fs from 'node:fs';
import path from 'node:path';

interface ScenarioStep {
  step_number: number;
  step_text: string;
  status: 'success' | 'failed' | 'skipped';
  action_taken: string;
  before_screenshot: string;
  after_screenshot: string;
  page_summary: string;
  failure_reason: string | null;
  duration_ms: number;
}

interface Trajectory {
  scenario_id: string;
  scenario_file: string;
  started_at: string;
  completed_at: string;
  status: 'completed' | 'execution_failed' | 'aborted';
  steps: ScenarioStep[];
  performance: {
    total_duration_ms: number;
    memory_usage_mb: number;
    step_durations_ms: number[];
  };
}

function parseScenarioFile(filePath: string): {
  id: string;
  title: string;
  steps: string[];
} {
  const content = fs.readFileSync(filePath, 'utf-8');

  // Parse YAML frontmatter
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  const frontmatter = fmMatch ? fmMatch[1] : '';
  const id = frontmatter.match(/^id:\s*(.+)$/m)?.[1]?.trim() || 'unknown';
  const title = frontmatter.match(/^title:\s*(.+)$/m)?.[1]?.trim() || 'Unknown';

  // Parse steps from ## Steps section
  const stepsSection = content.match(/## Steps\n([\s\S]*?)(?=\n##|\n*$)/);
  const stepsText = stepsSection ? stepsSection[1] : '';
  const steps = stepsText
    .split('\n')
    .filter((line) => /^\d+\.\s/.test(line.trim()))
    .map((line) => line.replace(/^\d+\.\s*/, '').trim());

  return { id, title, steps };
}

async function captureScreenshot(
  stepNum: number,
  phase: 'before' | 'after',
): Promise<string> {
  const runDir = process.env.SAT_RUN_DIR || 'sat/runs/run-default';
  const screenshotDir = path.resolve(runDir, 'screenshots');
  fs.mkdirSync(screenshotDir, { recursive: true });

  const filename = `step-${stepNum}-${phase}.png`;
  const filepath = path.resolve(screenshotDir, filename);

  const base64 = await browser.takeScreenshot();
  fs.writeFileSync(filepath, Buffer.from(base64, 'base64'));

  return `screenshots/${filename}`;
}

async function getPageSummary(): Promise<string> {
  try {
    const summary = await browser.execute(() => {
      const title = document.title;
      const buttons = Array.from(document.querySelectorAll('button'))
        .map((b) => b.textContent?.trim())
        .filter(Boolean)
        .slice(0, 10);
      const inputs = document.querySelectorAll('input').length;
      const modals = document.querySelectorAll('[role="dialog"]').length;
      const panels = document.querySelectorAll('[data-resizable-panel-id]').length;
      return `Title: ${title} | ${buttons.length} buttons (${buttons.join(', ')}) | ${inputs} inputs | ${modals} modals | ${panels} panels`;
    });
    return summary;
  } catch {
    return 'Unable to get page summary';
  }
}

/**
 * Interpret a natural-language step and execute it via WebDriver.
 *
 * This is a best-effort interpreter. It handles common patterns:
 * - "Click [element]" → find and click
 * - "Enter/Type [text] in [field]" → find input and type
 * - "Verify/Check/Observe [something]" → find element and check existence
 * - "Navigate to [area]" → look for navigation elements
 * - "Press [key]" → send keys
 * - "Wait/Pause" → browser.pause
 *
 * For steps it can't interpret, it captures the page state and marks as failed.
 */
async function interpretAndExecuteStep(stepText: string): Promise<{
  action_taken: string;
  success: boolean;
  failure_reason: string | null;
}> {
  const lower = stepText.toLowerCase();

  try {
    // Press/keyboard actions
    if (lower.match(/^press\s/)) {
      const key = stepText.replace(/^press\s+/i, '').replace(/"/g, '');
      await browser.keys(key);
      return { action_taken: `Pressed key: ${key}`, success: true, failure_reason: null };
    }

    // Wait/pause actions
    if (lower.match(/^(wait|pause)/)) {
      await browser.pause(1000);
      return { action_taken: 'Paused for 1s', success: true, failure_reason: null };
    }

    // Click actions
    if (lower.match(/^(click|select|tap|open|toggle|expand|collapse)/)) {
      const target = stepText.replace(/^(click|select|tap|open|toggle|expand|collapse)\s+(on\s+|the\s+)?/i, '');
      const element = await findElementByDescription(target);
      if (element) {
        await element.click();
        await browser.pause(500); // Let UI settle
        return { action_taken: `Clicked element matching: ${target}`, success: true, failure_reason: null };
      }
      return { action_taken: `Could not find: ${target}`, success: false, failure_reason: `Element not found: ${target}` };
    }

    // Type/enter text
    if (lower.match(/^(type|enter|input|fill)/)) {
      const match = stepText.match(/(?:type|enter|input|fill)\s+"?([^"]+)"?\s+(?:in|into|in the)\s+(.+)/i);
      if (match) {
        const [, text, fieldDesc] = match;
        const input = await findElementByDescription(fieldDesc);
        if (input) {
          await input.setValue(text);
          return { action_taken: `Typed "${text}" into ${fieldDesc}`, success: true, failure_reason: null };
        }
        // Fallback: try any visible input
        const anyInput = await $('input');
        if (await anyInput.isExisting()) {
          await anyInput.setValue(text);
          return { action_taken: `Typed "${text}" into first input (fallback)`, success: true, failure_reason: null };
        }
      }
      return { action_taken: `Could not parse type command: ${stepText}`, success: false, failure_reason: 'Could not parse type command' };
    }

    // Verify/observe/check actions — these are assertions, not interactions
    if (lower.match(/^(verify|check|observe|confirm|note|notice|look|see|ensure)/)) {
      const target = stepText.replace(/^(verify|check|observe|confirm|note|notice|look at|look for|see|ensure)\s+(that\s+|the\s+|if\s+)?/i, '');
      const element = await findElementByDescription(target);
      if (element) {
        const text = await element.getText();
        return { action_taken: `Verified element exists: ${target} (text: "${text.slice(0, 100)}")`, success: true, failure_reason: null };
      }
      // For observe/note steps, success even if specific element not found — capture page state
      if (lower.match(/^(observe|note|notice)/)) {
        return { action_taken: `Observed page state for: ${target}`, success: true, failure_reason: null };
      }
      return { action_taken: `Could not verify: ${target}`, success: false, failure_reason: `Verification target not found: ${target}` };
    }

    // Navigate actions
    if (lower.match(/^(navigate|go to|switch to|move to)/)) {
      const target = stepText.replace(/^(navigate|go|switch|move)\s+(to\s+)?/i, '');
      const element = await findElementByDescription(target);
      if (element) {
        await element.click();
        await browser.pause(500);
        return { action_taken: `Navigated to: ${target}`, success: true, failure_reason: null };
      }
      return { action_taken: `Could not navigate to: ${target}`, success: false, failure_reason: `Navigation target not found: ${target}` };
    }

    // Right-click
    if (lower.match(/^right-?click/)) {
      const target = stepText.replace(/^right-?click\s+(on\s+)?/i, '');
      const element = await findElementByDescription(target);
      if (element) {
        await element.click({ button: 'right' });
        await browser.pause(500);
        return { action_taken: `Right-clicked on: ${target}`, success: true, failure_reason: null };
      }
      return { action_taken: `Could not find for right-click: ${target}`, success: false, failure_reason: `Element not found: ${target}` };
    }

    // Hover
    if (lower.match(/^hover/)) {
      const target = stepText.replace(/^hover\s+(over\s+|on\s+)?/i, '');
      const element = await findElementByDescription(target);
      if (element) {
        await element.moveTo();
        await browser.pause(300);
        return { action_taken: `Hovered over: ${target}`, success: true, failure_reason: null };
      }
      return { action_taken: `Could not find for hover: ${target}`, success: false, failure_reason: `Element not found: ${target}` };
    }

    // Fallback: try to find something related and interact
    return {
      action_taken: `Unrecognized step pattern: ${stepText}`,
      success: false,
      failure_reason: `Could not interpret step: ${stepText}`,
    };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { action_taken: `Error executing: ${stepText}`, success: false, failure_reason: msg };
  }
}

/**
 * Find a DOM element from a natural-language description.
 * Tries multiple strategies in order of specificity.
 */
async function findElementByDescription(description: string): Promise<WebdriverIO.Element | null> {
  const desc = description.toLowerCase().replace(/['"]/g, '');

  // Strategy 1: aria-label match
  try {
    const ariaEl = await $(`[aria-label*="${desc}"]`);
    if (await ariaEl.isExisting()) return ariaEl;
  } catch { /* continue */ }

  // Strategy 2: button/link text match
  try {
    const btnEl = await $(`button*=${description}`);
    if (await btnEl.isExisting()) return btnEl;
  } catch { /* continue */ }

  // Strategy 3: any element containing the text
  try {
    const textEl = await $(`*=${description}`);
    if (await textEl.isExisting()) return textEl;
  } catch { /* continue */ }

  // Strategy 4: data-testid or data-resizable-panel-id
  const slugified = desc.replace(/\s+/g, '-');
  try {
    const dataEl = await $(`[data-testid="${slugified}"], [data-resizable-panel-id="${slugified}"]`);
    if (await dataEl.isExisting()) return dataEl;
  } catch { /* continue */ }

  // Strategy 5: input by placeholder
  try {
    const placeholderEl = await $(`input[placeholder*="${desc}"]`);
    if (await placeholderEl.isExisting()) return placeholderEl;
  } catch { /* continue */ }

  // Strategy 6: heading text
  try {
    const headingEl = await $(`h1*=${description}, h2*=${description}, h3*=${description}`);
    if (await headingEl.isExisting()) return headingEl;
  } catch { /* continue */ }

  return null;
}

// Main test execution
describe('SAT Scenario Run', () => {
  const scenarioFile = process.env.SAT_SCENARIO_FILE;

  if (!scenarioFile) {
    it('should have SAT_SCENARIO_FILE env set', () => {
      throw new Error('SAT_SCENARIO_FILE environment variable not set');
    });
    return;
  }

  const scenario = parseScenarioFile(scenarioFile);
  const runDir = process.env.SAT_RUN_DIR || 'sat/runs/run-default';
  const trajectoryPath = path.resolve(runDir, `trajectory-${scenario.id}.json`);

  const trajectory: Trajectory = {
    scenario_id: scenario.id,
    scenario_file: scenarioFile,
    started_at: new Date().toISOString(),
    completed_at: '',
    status: 'completed',
    steps: [],
    performance: {
      total_duration_ms: 0,
      memory_usage_mb: 0,
      step_durations_ms: [],
    },
  };

  let consecutiveFailures = 0;
  let aborted = false;
  const startTime = Date.now();

  before(async () => {
    // Wait for app to load
    await browser.waitUntil(
      async () => (await browser.getTitle()) === 'Branchdeck',
      { timeout: 15000, timeoutMsg: 'App did not load within 15s' },
    );
    await browser.pause(1000); // Let SolidJS hydrate
  });

  for (let i = 0; i < scenario.steps.length; i++) {
    const stepNum = i + 1;
    const stepText = scenario.steps[i];

    it(`Step ${stepNum}: ${stepText}`, async () => {
      if (aborted) {
        trajectory.steps.push({
          step_number: stepNum,
          step_text: stepText,
          status: 'skipped',
          action_taken: 'Skipped — scenario aborted after 3 consecutive failures',
          before_screenshot: '',
          after_screenshot: '',
          page_summary: '',
          failure_reason: 'Scenario aborted',
          duration_ms: 0,
        });
        return;
      }

      const stepStart = Date.now();

      // Before screenshot
      const beforeScreenshot = await captureScreenshot(stepNum, 'before');
      const beforeSummary = await getPageSummary();

      // Execute step
      const result = await interpretAndExecuteStep(stepText);

      // After screenshot
      const afterScreenshot = await captureScreenshot(stepNum, 'after');
      const afterSummary = await getPageSummary();

      const stepDuration = Date.now() - stepStart;

      const step: ScenarioStep = {
        step_number: stepNum,
        step_text: stepText,
        status: result.success ? 'success' : 'failed',
        action_taken: result.action_taken,
        before_screenshot: beforeScreenshot,
        after_screenshot: afterScreenshot,
        page_summary: afterSummary,
        failure_reason: result.failure_reason,
        duration_ms: stepDuration,
      };

      trajectory.steps.push(step);
      trajectory.performance.step_durations_ms.push(stepDuration);

      if (result.success) {
        consecutiveFailures = 0;
      } else {
        consecutiveFailures++;
        if (consecutiveFailures >= 3) {
          aborted = true;
          trajectory.status = 'execution_failed';
        }
      }
    });
  }

  after(() => {
    trajectory.completed_at = new Date().toISOString();
    trajectory.performance.total_duration_ms = Date.now() - startTime;
    trajectory.performance.memory_usage_mb = Math.round(
      process.memoryUsage().heapUsed / 1024 / 1024,
    );

    // Write trajectory
    fs.mkdirSync(path.dirname(trajectoryPath), { recursive: true });
    fs.writeFileSync(trajectoryPath, JSON.stringify(trajectory, null, 2));
  });
});
