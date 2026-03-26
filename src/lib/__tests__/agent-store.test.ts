import { describe, expect, it } from 'vitest';
import type { AgentLogEntry } from '../stores/agent';

// Test pure helper functions extracted from agent store logic
// The store itself requires SolidJS reactive context, so we test the data logic

function shouldAcceptEvent(
  event: { sessionId: string; tabId: string; kind: string },
  extraSessions: Set<string>,
  extraTabs: Set<string>,
  trackedSessions: Set<string>,
): boolean {
  if (extraSessions.has(event.sessionId)) return true;
  if (extraTabs.has(event.tabId)) return true;
  if (event.kind === 'sessionStart' || event.kind === 'sessionStop') return true;
  if (trackedSessions.has(event.sessionId)) return true;
  return false;
}

function filterLogForSession(
  log: AgentLogEntry[],
  sessionId: string,
  max: number,
): AgentLogEntry[] {
  return log.filter((e) => e.sessionId === sessionId).slice(-max);
}

function getActiveRuns(
  sessions: Record<string, { sessionId: string; status: string }>,
): { sessionId: string; status: string }[] {
  return Object.values(sessions).filter((s) => s.status === 'active' || s.status === 'idle');
}

describe('agent store helpers', () => {
  describe('shouldAcceptEvent', () => {
    it('accepts events for explicitly tracked sessions', () => {
      const extraSessions = new Set(['sess-1']);
      const result = shouldAcceptEvent(
        { sessionId: 'sess-1', tabId: 'tab-x', kind: 'toolStart' },
        extraSessions,
        new Set(),
        new Set(),
      );
      expect(result).toBe(true);
    });

    it('accepts events for explicitly tracked tabs', () => {
      const result = shouldAcceptEvent(
        { sessionId: 'sess-x', tabId: 'tab-1', kind: 'toolStart' },
        new Set(),
        new Set(['tab-1']),
        new Set(),
      );
      expect(result).toBe(true);
    });

    it('always accepts session start/stop events', () => {
      const result = shouldAcceptEvent(
        { sessionId: 'sess-unknown', tabId: 'tab-unknown', kind: 'sessionStart' },
        new Set(),
        new Set(),
        new Set(),
      );
      expect(result).toBe(true);
    });

    it('rejects unknown events for unknown sessions/tabs', () => {
      const result = shouldAcceptEvent(
        { sessionId: 'sess-unknown', tabId: 'tab-unknown', kind: 'toolStart' },
        new Set(),
        new Set(),
        new Set(),
      );
      expect(result).toBe(false);
    });

    it('accepts events for already-tracked sessions', () => {
      const result = shouldAcceptEvent(
        { sessionId: 'sess-tracked', tabId: 'tab-x', kind: 'toolEnd' },
        new Set(),
        new Set(),
        new Set(['sess-tracked']),
      );
      expect(result).toBe(true);
    });
  });

  describe('filterLogForSession', () => {
    const entries: AgentLogEntry[] = [
      {
        id: '1',
        kind: 'toolStart',
        tabId: 't1',
        sessionId: 'sess-a',
        toolName: 'Read',
        filePath: null,
        agentId: null,
        message: null,
        ts: 100,
      },
      {
        id: '2',
        kind: 'toolEnd',
        tabId: 't1',
        sessionId: 'sess-a',
        toolName: 'Read',
        filePath: null,
        agentId: null,
        message: null,
        ts: 200,
      },
      {
        id: '3',
        kind: 'toolStart',
        tabId: 't2',
        sessionId: 'sess-b',
        toolName: 'Write',
        filePath: null,
        agentId: null,
        message: null,
        ts: 300,
      },
      {
        id: '4',
        kind: 'toolStart',
        tabId: 't1',
        sessionId: 'sess-a',
        toolName: 'Edit',
        filePath: '/x.ts',
        agentId: null,
        message: null,
        ts: 400,
      },
    ];

    it('filters by session id', () => {
      const result = filterLogForSession(entries, 'sess-a', 50);
      expect(result).toHaveLength(3);
      expect(result.every((e) => e.sessionId === 'sess-a')).toBe(true);
    });

    it('returns empty for unknown session', () => {
      const result = filterLogForSession(entries, 'sess-z', 50);
      expect(result).toHaveLength(0);
    });

    it('caps results to max per session', () => {
      const result = filterLogForSession(entries, 'sess-a', 2);
      expect(result).toHaveLength(2);
      // Should keep the last 2
      expect(result[0].id).toBe('2');
      expect(result[1].id).toBe('4');
    });
  });

  describe('getActiveRuns', () => {
    it('returns only active/idle sessions', () => {
      const sessions = {
        s1: { sessionId: 's1', status: 'active' },
        s2: { sessionId: 's2', status: 'stopped' },
        s3: { sessionId: 's3', status: 'idle' },
      };
      const result = getActiveRuns(sessions);
      expect(result).toHaveLength(2);
      expect(result.map((r) => r.sessionId).sort()).toEqual(['s1', 's3']);
    });

    it('returns empty when all stopped', () => {
      const sessions = {
        s1: { sessionId: 's1', status: 'stopped' },
      };
      const result = getActiveRuns(sessions);
      expect(result).toHaveLength(0);
    });
  });
});
