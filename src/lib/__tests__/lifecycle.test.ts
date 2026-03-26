import { describe, expect, it } from 'vitest';
import type { LifecycleEvent } from '../../types/lifecycle';
import {
  buildTimelineEntry,
  inferTriggerSource,
  inferWorkflowType,
  resolveStatusLabel,
  toCycle,
} from '../utils/lifecycle';

function makeEvent(overrides: Partial<LifecycleEvent> = {}): LifecycleEvent {
  return {
    prKey: 'owner/repo#1',
    worktreePath: '/tmp/worktree/fix-1',
    status: 'running',
    attempt: 1,
    startedAt: 1000,
    ...overrides,
  };
}

describe('inferWorkflowType', () => {
  it('detects verification from rescore key', () => {
    const event = makeEvent({ prKey: 'owner/repo#rescore-1' });
    expect(inferWorkflowType(event)).toBe('verification');
  });

  it('detects issue-resolution from issue key', () => {
    const event = makeEvent({ prKey: 'owner/repo#i42' });
    expect(inferWorkflowType(event)).toBe('issue-resolution');
  });

  it('detects sat-scoring from worktree path', () => {
    const event = makeEvent({ worktreePath: '/tmp/worktrees/sat-run-1/fix' });
    expect(inferWorkflowType(event)).toBe('sat-scoring');
  });

  it('does not false-positive on #i substring like #improvements', () => {
    const event = makeEvent({ prKey: 'owner/repo#improvements' });
    // #improvements should NOT trigger issue-detected in trigger source
    // (inferWorkflowType defaults to issue-resolution regardless, so test via trigger)
    expect(inferTriggerSource(event)).not.toBe('issue-detected');
  });

  it('defaults to issue-resolution', () => {
    const event = makeEvent();
    expect(inferWorkflowType(event)).toBe('issue-resolution');
  });
});

describe('inferTriggerSource', () => {
  it('detects post-merge from rescore key', () => {
    const event = makeEvent({ prKey: 'owner/repo#rescore-1' });
    expect(inferTriggerSource(event)).toBe('post-merge');
  });

  it('detects issue-detected from issue key', () => {
    const event = makeEvent({ prKey: 'owner/repo#i42' });
    expect(inferTriggerSource(event)).toBe('issue-detected');
  });

  it('detects retry from attempt > 1', () => {
    const event = makeEvent({ attempt: 2 });
    expect(inferTriggerSource(event)).toBe('retry');
  });

  it('defaults to pr-poll', () => {
    const event = makeEvent();
    expect(inferTriggerSource(event)).toBe('pr-poll');
  });
});

describe('resolveStatusLabel', () => {
  it('uses custom displayStatus when provided', () => {
    expect(resolveStatusLabel('running', 'Analyzing Code')).toBe('Analyzing Code');
  });

  it('falls back to hardcoded labels without displayStatus', () => {
    expect(resolveStatusLabel('running')).toBe('Analyzing');
    expect(resolveStatusLabel('completed')).toBe('Completed');
    expect(resolveStatusLabel('failed')).toBe('Failed \u2014 retries exhausted');
  });

  it('uses raw status for unknown statuses', () => {
    // Custom workflow statuses not in the hardcoded map
    expect(resolveStatusLabel('patching' as 'running')).toBe('patching');
  });
});

describe('buildTimelineEntry', () => {
  it('creates entry with custom displayStatus', () => {
    const event = makeEvent({ displayStatus: 'Deep Analysis', attempt: 2 });
    const entry = buildTimelineEntry(event);
    expect(entry.status).toBe('running');
    expect(entry.displayStatus).toBe('Deep Analysis');
    expect(entry.detail).toBe('Deep Analysis (attempt 2)');
    expect(entry.timestamp).toBeGreaterThan(0);
  });

  it('falls back to raw status without displayStatus', () => {
    const event = makeEvent();
    const entry = buildTimelineEntry(event);
    expect(entry.displayStatus).toBe('running');
    expect(entry.detail).toBe('running (attempt 1)');
  });
});

describe('toCycle', () => {
  it('converts event to cycle with timeline', () => {
    const event = makeEvent({
      workflowName: 'pr-shepherd',
      displayStatus: 'Analyzing',
      completedAt: 5000,
    });
    const timeline = [
      { timestamp: 1000, status: 'running', displayStatus: 'Analyzing', detail: 'started' },
      { timestamp: 5000, status: 'completed', displayStatus: 'Done', detail: 'finished' },
    ];
    const cycle = toCycle(event, timeline);

    expect(cycle.id).toBe('owner/repo#1-1');
    expect(cycle.workflowName).toBe('pr-shepherd');
    expect(cycle.displayStatus).toBe('Analyzing');
    expect(cycle.completedAt).toBe(5000);
    expect(cycle.timeline).toHaveLength(2);
    expect(cycle.timeline[0].displayStatus).toBe('Analyzing');
  });

  it('handles missing optional fields', () => {
    const event = makeEvent();
    const cycle = toCycle(event, []);

    expect(cycle.workflowName).toBeUndefined();
    expect(cycle.displayStatus).toBeUndefined();
    expect(cycle.completedAt).toBeNull();
    expect(cycle.timeline).toHaveLength(0);
  });
});
