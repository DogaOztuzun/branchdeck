import { createSignal } from 'solid-js';
import type { ActivityEvent, EventFilter, EventType, TimeRange } from '../../types/activity';

const allTypes: EventType[] = ['sat', 'orchestrator', 'agent', 'pr', 'ci'];

const [events, setEvents] = createSignal<ActivityEvent[]>([]);
const [filter, setFilter] = createSignal<EventFilter>({
  activeTypes: new Set(allTypes),
  timeRange: 'all',
});
const [selectedIndex, setSelectedIndex] = createSignal<number | null>(null);
const [expandedId, setExpandedId] = createSignal<string | null>(null);

function filteredEvents(): ActivityEvent[] {
  const f = filter();
  let result = events().filter((e) => f.activeTypes.has(e.type));

  if (f.timeRange !== 'all') {
    const hours = f.timeRange === '8h' ? 8 : 24;
    const cutoff = Date.now() - hours * 60 * 60 * 1000;
    result = result.filter((e) => e.timestamp >= cutoff);
  }

  return result.sort((a, b) => b.timestamp - a.timestamp);
}

function toggleType(type: EventType) {
  setFilter((prev) => {
    const next = new Set(prev.activeTypes);
    if (next.has(type)) next.delete(type);
    else next.add(type);
    return { ...prev, activeTypes: next };
  });
}

function setTimeRange(range: TimeRange) {
  setFilter((prev) => ({ ...prev, timeRange: range }));
}

function selectNext() {
  const filtered = filteredEvents();
  if (filtered.length === 0) return;
  const cur = selectedIndex();
  if (cur === null) setSelectedIndex(0);
  else if (cur < filtered.length - 1) setSelectedIndex(cur + 1);
}

function selectPrev() {
  const cur = selectedIndex();
  if (cur === null || cur <= 0) return;
  setSelectedIndex(cur - 1);
}

function toggleExpand() {
  const filtered = filteredEvents();
  const idx = selectedIndex();
  if (idx === null) return;
  const event = filtered[idx];
  if (!event) return;
  setExpandedId(expandedId() === event.id ? null : event.id);
}

function loadMockData() {
  const now = Date.now();
  const h = (hours: number) => now - hours * 60 * 60 * 1000;

  setEvents([
    {
      id: 'e1',
      type: 'sat',
      timestamp: h(7.5),
      description: "SAT found 'spacing regression' — confused-newbie persona, severity: high",
      detail: { scenario: 'Branch selector', persona: 'confused-newbie', severity: 'high' },
    },
    {
      id: 'e2',
      type: 'orchestrator',
      timestamp: h(7.4),
      description: 'Dispatched implement-issue workflow for Issue #15',
      detail: { workflow: 'implement-issue', worktree: 'fix/sat-spacing-regression' },
    },
    {
      id: 'e3',
      type: 'agent',
      timestamp: h(7.3),
      description: 'Agent started: fix/sat-spacing-regression',
      entityLink: '#42',
      detail: { duration: '12m 34s', files: '3', outcome: 'PR created' },
    },
    {
      id: 'e4',
      type: 'pr',
      timestamp: h(7.0),
      description: 'PR #42 created: fix spacing regression in branch selector',
      entityLink: '#42',
      detail: {
        title: 'Fix spacing regression',
        branch: 'fix/sat-spacing-regression',
        ci: 'passing',
      },
    },
    {
      id: 'e5',
      type: 'ci',
      timestamp: h(6.8),
      description: 'CI passed for PR #42',
      entityLink: '#42',
    },
    {
      id: 'e6',
      type: 'sat',
      timestamp: h(5.0),
      description: "SAT found 'tooltip overlap' — power-user persona, severity: high",
      detail: { scenario: 'Action buttons', persona: 'power-user', severity: 'high' },
    },
    {
      id: 'e7',
      type: 'orchestrator',
      timestamp: h(4.9),
      description: 'Dispatched implement-issue workflow for Issue #16',
      detail: { workflow: 'implement-issue', worktree: 'fix/tooltip-overlap' },
    },
    {
      id: 'e8',
      type: 'agent',
      timestamp: h(4.8),
      description: 'Agent started: fix/tooltip-overlap',
      entityLink: '#43',
      detail: { duration: '8m 12s', files: '1', outcome: 'PR created' },
    },
    {
      id: 'e9',
      type: 'pr',
      timestamp: h(4.5),
      description: 'PR #43 created: fix tooltip overlap on action buttons',
      entityLink: '#43',
      detail: { title: 'Fix tooltip overlap', branch: 'fix/tooltip-overlap', ci: 'failing' },
    },
    {
      id: 'e10',
      type: 'ci',
      timestamp: h(4.3),
      description: 'CI failed for PR #43 — lint error in tooltip component',
      entityLink: '#43',
    },
    {
      id: 'e11',
      type: 'sat',
      timestamp: h(3.0),
      description: 'SAT re-score completed: confused-newbie score 31 → 74 (+43)',
      detail: { persona: 'confused-newbie', before: '31', after: '74', delta: '+43' },
    },
  ]);
}

export function getActivityStore() {
  return {
    events,
    filter,
    selectedIndex,
    expandedId,
    filteredEvents,
    toggleType,
    setTimeRange,
    selectNext,
    selectPrev,
    toggleExpand,
    setSelectedIndex,
    setExpandedId,
    loadMockData,
    allTypes,
  };
}
