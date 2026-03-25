import { createSignal } from 'solid-js';
import type { ChartDataPoint, PersonaLine } from '../../types/chart';
import type { CategoryFilter, SATCycle, SATFinding } from '../../types/sat';
import { getKeyboardStore } from './keyboard';

const [findings, setFindings] = createSignal<SATFinding[]>([]);
const [cycles, setCycles] = createSignal<SATCycle[]>([]);
const [categoryFilter, setCategoryFilter] = createSignal<CategoryFilter>('all');
const [selectedFindingIndex, setSelectedFindingIndex] = createSignal<number | null>(null);
const [expandedFindingId, setExpandedFindingId] = createSignal<string | null>(null);

const categoryOrder: CategoryFilter[] = ['all', 'app', 'runner', 'scenario'];

function filteredFindings(): SATFinding[] {
  const cat = categoryFilter();
  const all = findings();
  if (cat === 'all') return all;
  return all.filter((f) => f.category === cat);
}

function signalQuality(): number {
  const all = findings();
  if (all.length === 0) return 100;
  const real = all.filter((f) => f.status !== 'false-positive').length;
  return Math.round((real / all.length) * 100);
}

function signalColor(): 'success' | 'warning' | 'error' {
  const sq = signalQuality();
  if (sq >= 80) return 'success';
  if (sq >= 60) return 'warning';
  return 'error';
}

function currentScore(): number {
  const c = cycles();
  if (c.length === 0) return 0;
  return c[c.length - 1].score;
}

function scoreDelta(): number {
  const c = cycles();
  if (c.length < 2) return 0;
  return c[c.length - 1].score - c[c.length - 2].score;
}

function chartData(): ChartDataPoint[] {
  return cycles().map((c) => ({ cycle: c.cycle, score: c.score, date: c.date }));
}

function personaLines(): PersonaLine[] {
  // Group findings by persona and cycle to create trend lines
  const personas = new Map<string, Map<number, number[]>>();
  for (const f of findings()) {
    if (!personas.has(f.persona)) personas.set(f.persona, new Map());
    const cycleMap = personas.get(f.persona);
    if (!cycleMap) continue;
    if (!cycleMap.has(f.cycle)) cycleMap.set(f.cycle, []);
    cycleMap.get(f.cycle)?.push(f.confidence);
  }

  const colors: Record<string, string> = {
    'confused-newbie': '#e0af68',
    'power-user': '#9ece6a',
    'accessibility-user': '#7dcfff',
  };

  return Array.from(personas.entries()).map(([name, cycleMap]) => ({
    name,
    color: colors[name] ?? '#787c99',
    data: Array.from(cycleMap.entries())
      .map(([cycle, scores]) => ({
        cycle,
        score: Math.round(scores.reduce((a, b) => a + b, 0) / scores.length),
      }))
      .sort((a, b) => a.cycle - b.cycle),
  }));
}

function cycleCategoryFilter() {
  const idx = categoryOrder.indexOf(categoryFilter());
  const next = (idx + 1) % categoryOrder.length;
  setCategoryFilter(categoryOrder[next]);
}

function toggleFalsePositive() {
  const filtered = filteredFindings();
  const idx = selectedFindingIndex();
  if (idx === null) return;
  const finding = filtered[idx];
  if (!finding) return;

  setFindings((prev) =>
    prev.map((f) =>
      f.id === finding.id
        ? {
            ...f,
            status: f.status === 'false-positive' ? ('open' as const) : ('false-positive' as const),
          }
        : f,
    ),
  );
}

function selectNextFinding() {
  const filtered = filteredFindings();
  if (filtered.length === 0) return;
  const cur = selectedFindingIndex();
  if (cur === null) {
    setSelectedFindingIndex(0);
  } else if (cur < filtered.length - 1) {
    setSelectedFindingIndex(cur + 1);
  }
}

function selectPrevFinding() {
  const cur = selectedFindingIndex();
  if (cur === null || cur <= 0) return;
  setSelectedFindingIndex(cur - 1);
}

function toggleExpandFinding() {
  const filtered = filteredFindings();
  const idx = selectedFindingIndex();
  if (idx === null) return;
  const finding = filtered[idx];
  if (!finding) return;
  setExpandedFindingId(expandedFindingId() === finding.id ? null : finding.id);
}

function registerSATShortcuts() {
  const keyboard = getKeyboardStore();
  keyboard.registerShortcut({
    key: 'j',
    handler: selectNextFinding,
    label: 'Next finding',
    context: 'sat',
    category: 'SAT Actions',
  });
  keyboard.registerShortcut({
    key: 'k',
    handler: selectPrevFinding,
    label: 'Previous finding',
    context: 'sat',
    category: 'SAT Actions',
  });
  keyboard.registerShortcut({
    key: 'Enter',
    handler: toggleExpandFinding,
    label: 'Expand / collapse',
    context: 'sat',
    category: 'SAT Actions',
  });
  keyboard.registerShortcut({
    key: 'c',
    handler: cycleCategoryFilter,
    label: 'Cycle category filter',
    context: 'sat',
    category: 'SAT Actions',
  });
  keyboard.registerShortcut({
    key: 'f',
    handler: toggleFalsePositive,
    label: 'Toggle false positive',
    context: 'sat',
    category: 'SAT Actions',
  });
}

function loadMockData() {
  setCycles([
    { cycle: 1, score: 62, date: 'Mar 20', findingsCount: 5 },
    { cycle: 2, score: 68, date: 'Mar 21', findingsCount: 4 },
    { cycle: 3, score: 72, date: 'Mar 23', findingsCount: 3 },
    { cycle: 4, score: 78, date: 'Mar 25', findingsCount: 4 },
  ]);
  setFindings([
    {
      id: 'f1',
      title: 'Branch selector overwhelms new users',
      category: 'app',
      severity: 'high',
      status: 'fixed',
      persona: 'confused-newbie',
      cycle: 3,
      confidence: 92,
    },
    {
      id: 'f2',
      title: 'Terminal tab fails to render output',
      category: 'runner',
      severity: 'medium',
      status: 'false-positive',
      persona: 'power-user',
      cycle: 4,
      confidence: 45,
    },
    {
      id: 'f3',
      title: 'Tooltip overlaps action button',
      category: 'app',
      severity: 'high',
      status: 'issue-created',
      persona: 'power-user',
      cycle: 4,
      confidence: 88,
    },
    {
      id: 'f4',
      title: 'Navigation loses context on back',
      category: 'app',
      severity: 'medium',
      status: 'open',
      persona: 'confused-newbie',
      cycle: 4,
      confidence: 76,
    },
    {
      id: 'f5',
      title: 'WebDriver timeout on slow render',
      category: 'runner',
      severity: 'low',
      status: 'false-positive',
      persona: 'accessibility-user',
      cycle: 4,
      confidence: 32,
    },
    {
      id: 'f6',
      title: 'Export button hidden below fold',
      category: 'app',
      severity: 'high',
      status: 'fixed',
      persona: 'power-user',
      cycle: 2,
      confidence: 95,
    },
  ]);
}

export function getSATStore() {
  return {
    findings,
    cycles,
    categoryFilter,
    setCategoryFilter,
    selectedFindingIndex,
    setSelectedFindingIndex,
    expandedFindingId,
    setExpandedFindingId,
    filteredFindings,
    signalQuality,
    signalColor,
    currentScore,
    scoreDelta,
    chartData,
    personaLines,
    cycleCategoryFilter,
    toggleFalsePositive,
    registerSATShortcuts,
    loadMockData,
  };
}
