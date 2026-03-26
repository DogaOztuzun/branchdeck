import { describe, expect, it } from 'vitest';
import type { ActivityEvent } from '../../types/activity';
import type { TriagePr } from '../../types/lifecycle';
import { computeOvernightStats, formatSummaryStats } from '../stores/overnight';

function makeEvent(overrides: Partial<ActivityEvent> = {}): ActivityEvent {
  return {
    id: 'e1',
    type: 'sat',
    timestamp: Date.now(),
    description: 'Test event',
    ...overrides,
  };
}

function makeTriagePr(overrides: Partial<TriagePr> = {}): TriagePr {
  return {
    prKey: 'owner/repo#1',
    pr: undefined,
    lifecycle: undefined,
    analysis: undefined,
    currentSessionId: undefined,
    repoPath: undefined,
    ...overrides,
  };
}

describe('computeOvernightStats', () => {
  const now = Date.now();
  const cutoff = now - 8 * 60 * 60 * 1000; // 8 hours ago

  it('counts SAT events as issues found', () => {
    const events = [
      makeEvent({ id: 'e1', type: 'sat', timestamp: now - 1000 }),
      makeEvent({ id: 'e2', type: 'sat', timestamp: now - 2000 }),
      makeEvent({ id: 'e3', type: 'pr', timestamp: now - 3000 }),
    ];
    const stats = computeOvernightStats(events, [], cutoff, 78, 72);
    expect(stats.issuesFound).toBe(2);
  });

  it('counts PR events as PRs created', () => {
    const events = [
      makeEvent({ id: 'e1', type: 'pr', timestamp: now - 1000 }),
      makeEvent({ id: 'e2', type: 'pr', timestamp: now - 2000 }),
    ];
    const stats = computeOvernightStats(events, [], cutoff, 78, 72);
    expect(stats.prsCreated).toBe(2);
  });

  it('counts completed lifecycle entries as PRs merged', () => {
    const triagePrs = [
      makeTriagePr({
        prKey: 'r#1',
        lifecycle: {
          prKey: 'r#1',
          worktreePath: '/tmp/wt',
          status: 'completed',
          attempt: 1,
          startedAt: now - 1000,
        },
      }),
      makeTriagePr({
        prKey: 'r#2',
        lifecycle: {
          prKey: 'r#2',
          worktreePath: '/tmp/wt2',
          status: 'completed',
          attempt: 1,
          startedAt: cutoff - 1000, // before cutoff
        },
      }),
    ];
    const stats = computeOvernightStats([], triagePrs, cutoff, 0, 0);
    expect(stats.prsMerged).toBe(1);
  });

  it('excludes events before the cutoff', () => {
    const events = [
      makeEvent({ id: 'e1', type: 'sat', timestamp: cutoff - 1000 }), // before cutoff
      makeEvent({ id: 'e2', type: 'sat', timestamp: now - 1000 }), // after cutoff
    ];
    const stats = computeOvernightStats(events, [], cutoff, 0, 0);
    expect(stats.issuesFound).toBe(1);
  });

  it('computes net score change from current and previous scores', () => {
    const stats = computeOvernightStats([], [], cutoff, 85, 72);
    expect(stats.netScoreChange).toBe(13);
  });

  it('counts in-progress PRs', () => {
    const triagePrs = [
      makeTriagePr({
        lifecycle: {
          prKey: 'r#1',
          worktreePath: '/tmp',
          status: 'running',
          attempt: 1,
          startedAt: now,
        },
      }),
      makeTriagePr({
        lifecycle: {
          prKey: 'r#2',
          worktreePath: '/tmp',
          status: 'fixing',
          attempt: 1,
          startedAt: now,
        },
      }),
      makeTriagePr({
        lifecycle: {
          prKey: 'r#3',
          worktreePath: '/tmp',
          status: 'reviewReady',
          attempt: 1,
          startedAt: now,
        },
      }),
    ];
    const stats = computeOvernightStats([], triagePrs, cutoff, 0, 0);
    expect(stats.inProgress).toBe(2);
  });
});

describe('formatSummaryStats', () => {
  it('formats stats with semantic colors', () => {
    const items = formatSummaryStats({
      issuesFound: 3,
      prsCreated: 2,
      prsMerged: 1,
      netScoreChange: 6,
      inProgress: 1,
    });

    expect(items).toHaveLength(5);

    const issues = items.find((i) => i.label === 'Issues found');
    expect(issues?.value).toBe('3');
    expect(issues?.color).toBe('warning');

    const created = items.find((i) => i.label === 'PRs created');
    expect(created?.value).toBe('2');
    expect(created?.color).toBe('info');

    const merged = items.find((i) => i.label === 'PRs merged');
    expect(merged?.value).toBe('1');
    expect(merged?.color).toBe('success');

    const inProgress = items.find((i) => i.label === 'In progress');
    expect(inProgress?.value).toBe('1');
    expect(inProgress?.color).toBe('warning');

    const score = items.find((i) => i.label === 'SAT score');
    expect(score?.value).toBe('+6');
    expect(score?.color).toBe('success');
  });

  it('uses error color for negative score change', () => {
    const items = formatSummaryStats({
      issuesFound: 0,
      prsCreated: 0,
      prsMerged: 0,
      netScoreChange: -3,
      inProgress: 0,
    });

    const score = items.find((i) => i.label === 'SAT score');
    expect(score?.value).toBe('-3');
    expect(score?.color).toBe('error');
  });

  it('omits SAT score when no change', () => {
    const items = formatSummaryStats({
      issuesFound: 1,
      prsCreated: 0,
      prsMerged: 0,
      netScoreChange: 0,
      inProgress: 0,
    });

    expect(items.find((i) => i.label === 'SAT score')).toBeUndefined();
  });

  it('omits in-progress when zero', () => {
    const items = formatSummaryStats({
      issuesFound: 1,
      prsCreated: 0,
      prsMerged: 0,
      netScoreChange: 0,
      inProgress: 0,
    });

    expect(items.find((i) => i.label === 'In progress')).toBeUndefined();
  });

  it('uses success color for zero issues found', () => {
    const items = formatSummaryStats({
      issuesFound: 0,
      prsCreated: 0,
      prsMerged: 0,
      netScoreChange: 0,
      inProgress: 0,
    });

    const issues = items.find((i) => i.label === 'Issues found');
    expect(issues?.color).toBe('success');
  });
});
