import { describe, expect, it } from 'vitest';
import type { TriagePr } from '../../types/lifecycle';
import { groupTriagePrs } from '../utils/triage';

function makePr(overrides: Partial<TriagePr> = {}): TriagePr {
  return {
    prKey: 'owner/repo#1',
    pr: {
      number: 1,
      title: 'Test PR',
      branch: 'fix/test',
      url: 'https://github.com/owner/repo/pull/1',
      ciStatus: 'FAILURE',
      reviewDecision: null,
      repoName: 'owner/repo',
      author: 'user',
      additions: 10,
      deletions: 5,
      changedFiles: 2,
      createdAt: '2026-03-20T00:00:00Z',
    },
    lifecycle: undefined,
    analysis: undefined,
    currentSessionId: undefined,
    repoPath: '/tmp/repo',
    ...overrides,
  };
}

describe('groupTriagePrs', () => {
  it('groups PR with reviewReady lifecycle into needsAttention', () => {
    const item = makePr({
      lifecycle: {
        prKey: 'owner/repo#1',
        worktreePath: '/tmp/wt',
        status: 'reviewReady',
        attempt: 1,
        startedAt: 1000,
      },
    });
    const groups = groupTriagePrs([item]);
    expect(groups.needsAttention).toHaveLength(1);
    expect(groups.inProgress).toHaveLength(0);
  });

  it('groups PR with failed lifecycle into needsAttention', () => {
    const item = makePr({
      lifecycle: {
        prKey: 'owner/repo#1',
        worktreePath: '/tmp/wt',
        status: 'failed',
        attempt: 5,
        startedAt: 1000,
      },
    });
    const groups = groupTriagePrs([item]);
    expect(groups.needsAttention).toHaveLength(1);
  });

  it('groups PR with running lifecycle into inProgress', () => {
    const item = makePr({
      lifecycle: {
        prKey: 'owner/repo#1',
        worktreePath: '/tmp/wt',
        status: 'running',
        attempt: 1,
        startedAt: 1000,
      },
    });
    const groups = groupTriagePrs([item]);
    expect(groups.inProgress).toHaveLength(1);
  });

  it('groups PR with no lifecycle into newPrs', () => {
    const item = makePr({ lifecycle: undefined });
    const groups = groupTriagePrs([item]);
    expect(groups.newPrs).toHaveLength(1);
  });

  it('groups PR with completed lifecycle into done', () => {
    const item = makePr({
      lifecycle: {
        prKey: 'owner/repo#1',
        worktreePath: '/tmp/wt',
        status: 'completed',
        attempt: 1,
        startedAt: 1000,
      },
    });
    const groups = groupTriagePrs([item]);
    expect(groups.done).toHaveLength(1);
  });

  it('groups PR with stale lifecycle into watching', () => {
    const item = makePr({
      lifecycle: {
        prKey: 'owner/repo#1',
        worktreePath: '/tmp/wt',
        status: 'stale',
        attempt: 1,
        startedAt: 1000,
      },
    });
    const groups = groupTriagePrs([item]);
    expect(groups.watching).toHaveLength(1);
  });

  it('handles PR with lifecycle but no PrSummary — still groups correctly', () => {
    const item = makePr({
      pr: undefined,
      lifecycle: {
        prKey: 'owner/repo#1',
        worktreePath: '/tmp/wt',
        status: 'running',
        attempt: 1,
        startedAt: 1000,
      },
    });
    const groups = groupTriagePrs([item]);
    expect(groups.inProgress).toHaveLength(1);
    // Should not be in newPrs (no pr = not a new PR)
    expect(groups.newPrs).toHaveLength(0);
  });

  it('sorts newPrs with failing CI first', () => {
    const failing = makePr({
      prKey: 'owner/repo#1',
      pr: {
        number: 1,
        title: 'Failing',
        branch: 'fix/fail',
        url: '',
        ciStatus: 'FAILURE',
        reviewDecision: null,
        repoName: 'owner/repo',
        author: 'user',
        additions: null,
        deletions: null,
        changedFiles: null,
        createdAt: null,
      },
    });
    const passing = makePr({
      prKey: 'owner/repo#2',
      pr: {
        number: 2,
        title: 'Passing',
        branch: 'fix/pass',
        url: '',
        ciStatus: 'SUCCESS',
        reviewDecision: null,
        repoName: 'owner/repo',
        author: 'user',
        additions: null,
        deletions: null,
        changedFiles: null,
        createdAt: null,
      },
    });
    // Pass in reverse order
    const groups = groupTriagePrs([passing, failing]);
    expect(groups.newPrs[0].prKey).toBe('owner/repo#1');
    expect(groups.newPrs[1].prKey).toBe('owner/repo#2');
  });
});
