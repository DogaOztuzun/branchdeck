import { createSignal } from 'solid-js';
import type { InboxGroup, InboxItem, InboxItemStatus } from '../../types/inbox';
import { getKeyboardStore } from './keyboard';

const [items, setItems] = createSignal<InboxItem[]>([]);
const [selectedIndex, setSelectedIndex] = createSignal<number | null>(null);
const [expandedId, setExpandedId] = createSignal<string | null>(null);

function flatItems(): InboxItem[] {
  // Return items in group order: needs-attention, ready-to-merge, completed
  const order: InboxItemStatus[] = ['needs-attention', 'ready-to-merge', 'completed'];
  const all = items();
  const sorted: InboxItem[] = [];
  for (const status of order) {
    for (const item of all) {
      if (item.status === status) sorted.push(item);
    }
  }
  return sorted;
}

function isMultiRepo(): boolean {
  const repos = new Set(items().map((i) => i.repo));
  return repos.size > 1;
}

function groups(): InboxGroup[] {
  const config: { label: string; status: InboxItemStatus; color: 'error' | 'success' | 'muted' }[] =
    [
      { label: 'Needs attention', status: 'needs-attention', color: 'error' },
      { label: 'Ready to merge', status: 'ready-to-merge', color: 'success' },
      { label: 'Completed', status: 'completed', color: 'muted' },
    ];
  return config
    .map((g) => ({
      ...g,
      items: items().filter((i) => i.status === g.status),
    }))
    .filter((g) => g.items.length > 0);
}

function selectNext() {
  const flat = flatItems();
  if (flat.length === 0) return;
  const cur = selectedIndex();
  if (cur === null) {
    setSelectedIndex(0);
  } else if (cur < flat.length - 1) {
    setSelectedIndex(cur + 1);
  }
}

function selectPrev() {
  const cur = selectedIndex();
  if (cur === null || cur <= 0) return;
  setSelectedIndex(cur - 1);
}

function toggleExpand() {
  const idx = selectedIndex();
  if (idx === null) return;
  const flat = flatItems();
  const item = flat[idx];
  if (!item) return;
  setExpandedId(expandedId() === item.id ? null : item.id);
}

function advanceToNextUnprocessed() {
  const flat = flatItems();
  const cur = selectedIndex() ?? 0;
  const next = flat.findIndex((item, i) => i >= cur && item.status !== 'completed');
  if (next >= 0) {
    setSelectedIndex(next);
  } else {
    setExpandedId(null);
    setSelectedIndex(null);
  }
}

function mergeSelected() {
  if (!expandedId()) return;
  const flat = flatItems();
  const idx = selectedIndex();
  if (idx === null) return;
  const item = flat[idx];
  if (!item || item.type !== 'pr' || item.ciStatus !== 'passing') return;

  setItems((prev) =>
    prev.map((i) => (i.id === item.id ? { ...i, status: 'completed' as const } : i)),
  );
  setExpandedId(null);
  advanceToNextUnprocessed();
}

function dismissSelected() {
  if (!expandedId()) return;
  const flat = flatItems();
  const idx = selectedIndex();
  if (idx === null) return;
  const item = flat[idx];
  if (!item) return;

  setItems((prev) =>
    prev.map((i) => (i.id === item.id ? { ...i, status: 'completed' as const } : i)),
  );
  setExpandedId(null);
  advanceToNextUnprocessed();
}

function registerInboxShortcuts() {
  const keyboard = getKeyboardStore();
  keyboard.registerShortcut({
    key: 'j',
    handler: selectNext,
    label: 'Next item',
    context: 'inbox',
    category: 'Inbox Actions',
  });
  keyboard.registerShortcut({
    key: 'k',
    handler: selectPrev,
    label: 'Previous item',
    context: 'inbox',
    category: 'Inbox Actions',
  });
  keyboard.registerShortcut({
    key: 'Enter',
    handler: toggleExpand,
    label: 'Expand / collapse',
    context: 'inbox',
    category: 'Inbox Actions',
  });
  keyboard.registerShortcut({
    key: 'm',
    handler: mergeSelected,
    label: 'Merge PR',
    context: 'inbox',
    category: 'Inbox Actions',
  });
  keyboard.registerShortcut({
    key: 'x',
    handler: dismissSelected,
    label: 'Dismiss',
    context: 'inbox',
    category: 'Inbox Actions',
  });
}

function loadMockData() {
  setItems([
    {
      id: 'pr-42',
      type: 'pr',
      title: 'fix/sat-spacing-regression',
      identifier: '#42',
      branch: 'fix/sat-spacing-regression',
      status: 'ready-to-merge',
      source: 'sat',
      repo: 'branchdeck',
      timestamp: Date.now() - 3 * 60 * 60 * 1000,
      satDelta: 4,
      persona: 'confused-newbie',
      ciStatus: 'passing',
      filesChanged: 3,
      agentDuration: '12m 34s',
    },
    {
      id: 'pr-43',
      type: 'pr',
      title: 'fix/tooltip-overlap-buttons',
      identifier: '#43',
      branch: 'fix/tooltip-overlap',
      status: 'needs-attention',
      source: 'sat',
      repo: 'branchdeck',
      timestamp: Date.now() - 5 * 60 * 60 * 1000,
      satDelta: -2,
      persona: 'power-user',
      ciStatus: 'failing',
      filesChanged: 1,
      agentDuration: '8m 12s',
    },
    {
      id: 'pr-41',
      type: 'pr',
      title: 'fix/branch-selector-search',
      identifier: '#41',
      branch: 'fix/branch-selector',
      status: 'completed',
      source: 'sat',
      repo: 'branchdeck',
      timestamp: Date.now() - 8 * 60 * 60 * 1000,
      satDelta: 43,
      persona: 'confused-newbie',
      ciStatus: 'passing',
      filesChanged: 5,
      agentDuration: '15m 02s',
      loopComplete: true,
    },
  ]);
}

export function getInboxStore() {
  return {
    items,
    selectedIndex,
    expandedId,
    isMultiRepo,
    groups,
    flatItems,
    selectNext,
    selectPrev,
    toggleExpand,
    mergeSelected,
    dismissSelected,
    setSelectedIndex,
    setExpandedId,
    registerInboxShortcuts,
    loadMockData,
    setItems,
  };
}
