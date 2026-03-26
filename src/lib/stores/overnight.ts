import { createMemo, createSignal } from 'solid-js';
import type { ActivityEvent } from '../../types/activity';
import type { TriagePr } from '../../types/lifecycle';
import type { SummaryStatItem } from '../../types/ui';

const LAST_SESSION_KEY = 'branchdeck:lastSessionTimestamp';
const DEFAULT_WINDOW_HOURS = 12;

/** Configurable time window in hours for the overnight summary. */
const [windowHours, setWindowHours] = createSignal(DEFAULT_WINDOW_HOURS);

/**
 * Read the last session timestamp from localStorage.
 * Falls back to `windowHours` ago if not set.
 */
function getLastSessionTimestamp(): number {
  try {
    const stored = localStorage.getItem(LAST_SESSION_KEY);
    if (stored) {
      const ts = Number.parseInt(stored, 10);
      if (!Number.isNaN(ts) && ts > 0) return ts;
    }
  } catch {
    // localStorage unavailable (SSR, permissions)
  }
  return Date.now() - windowHours() * 60 * 60 * 1000;
}

/** Persist the current timestamp as "last session". */
function recordSessionStart(): void {
  try {
    localStorage.setItem(LAST_SESSION_KEY, String(Date.now()));
  } catch {
    // localStorage unavailable
  }
}

/** Cutoff timestamp: events after this are "overnight". */
function getCutoff(): number {
  const lastSession = getLastSessionTimestamp();
  const windowCutoff = Date.now() - windowHours() * 60 * 60 * 1000;
  // Use whichever is more recent — bounds the window to at most windowHours ago
  return Math.max(lastSession, windowCutoff);
}

export type OvernightStats = {
  issuesFound: number;
  prsCreated: number;
  prsMerged: number;
  netScoreChange: number;
  inProgress: number;
};

/**
 * Pure computation: derive overnight stats from activity events and triage PRs.
 * Exported for testing.
 */
export function computeOvernightStats(
  events: ActivityEvent[],
  triagePrs: TriagePr[],
  cutoff: number,
  currentScore: number,
  previousScore: number,
): OvernightStats {
  const recentEvents = events.filter((e) => e.timestamp >= cutoff);

  const issuesFound = recentEvents.filter((e) => e.type === 'sat').length;
  const prsCreated = recentEvents.filter((e) => e.type === 'pr').length;

  // Count merged/completed PRs from triage data
  const prsMerged = triagePrs.filter(
    (t) => t.lifecycle?.status === 'completed' && t.lifecycle.startedAt >= cutoff,
  ).length;

  // In-progress count from triage data
  const inProgress = triagePrs.filter(
    (t) =>
      t.lifecycle?.status === 'running' ||
      t.lifecycle?.status === 'fixing' ||
      t.lifecycle?.status === 'retrying',
  ).length;

  const netScoreChange = currentScore - previousScore;

  return { issuesFound, prsCreated, prsMerged, netScoreChange, inProgress };
}

/**
 * Format overnight stats into SummaryStatItem[] for the SummaryStatsBar.
 * Exported for testing.
 */
export function formatSummaryStats(stats: OvernightStats): SummaryStatItem[] {
  const items: SummaryStatItem[] = [];

  items.push({
    label: 'Issues found',
    value: String(stats.issuesFound),
    color: stats.issuesFound > 0 ? 'warning' : 'success',
  });

  items.push({
    label: 'PRs created',
    value: String(stats.prsCreated),
    color: stats.prsCreated > 0 ? 'info' : 'primary',
  });

  items.push({
    label: 'PRs merged',
    value: String(stats.prsMerged),
    color: stats.prsMerged > 0 ? 'success' : 'primary',
  });

  if (stats.inProgress > 0) {
    items.push({
      label: 'In progress',
      value: String(stats.inProgress),
      color: 'warning',
    });
  }

  if (stats.netScoreChange !== 0) {
    const sign = stats.netScoreChange > 0 ? '+' : '';
    items.push({
      label: 'SAT score',
      value: `${sign}${stats.netScoreChange}`,
      color: stats.netScoreChange > 0 ? 'success' : 'error',
    });
  }

  return items;
}

/**
 * Create a reactive overnight summary store.
 * Accepts data accessors from lifecycle and activity stores.
 */
export function createOvernightSummary(deps: {
  getEvents: () => ActivityEvent[];
  getTriagePrs: () => TriagePr[];
  getCurrentScore: () => number;
  getPreviousScore: () => number;
}) {
  const cutoff = createMemo(() => getCutoff());

  const stats = createMemo(() =>
    computeOvernightStats(
      deps.getEvents(),
      deps.getTriagePrs(),
      cutoff(),
      deps.getCurrentScore(),
      deps.getPreviousScore(),
    ),
  );

  const summaryItems = createMemo(() => formatSummaryStats(stats()));

  const hasActivity = createMemo(() => {
    const s = stats();
    return (
      s.issuesFound > 0 ||
      s.prsCreated > 0 ||
      s.prsMerged > 0 ||
      s.inProgress > 0 ||
      s.netScoreChange !== 0
    );
  });

  return {
    stats,
    summaryItems,
    hasActivity,
    windowHours,
    setWindowHours,
    recordSessionStart,
    getCutoff: cutoff,
  };
}
