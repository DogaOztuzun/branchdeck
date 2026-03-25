import { createSignal } from 'solid-js';

export type SATSettingsData = {
  minSeverity: 'critical' | 'high' | 'medium' | 'all';
  categoryFilters: { app: boolean; runner: boolean; scenario: boolean };
  confidenceThreshold: number;
  docsPath: string;
  runSchedule: 'manual' | 'nightly' | 'post-merge';
};

const defaultSettings: SATSettingsData = {
  minSeverity: 'high',
  categoryFilters: { app: true, runner: false, scenario: false },
  confidenceThreshold: 70,
  docsPath: 'docs/',
  runSchedule: 'manual',
};

const [current, setCurrent] = createSignal<SATSettingsData>({ ...defaultSettings });
const [draft, setDraft] = createSignal<SATSettingsData>({ ...defaultSettings });
const [saveStatus, setSaveStatus] = createSignal<'idle' | 'saved' | 'error'>('idle');

function isDirty(): boolean {
  return JSON.stringify(current()) !== JSON.stringify(draft());
}

function updateDraft<K extends keyof SATSettingsData>(key: K, value: SATSettingsData[K]) {
  setDraft((prev) => ({ ...prev, [key]: value }));
}

function updateCategoryFilter(category: keyof SATSettingsData['categoryFilters'], value: boolean) {
  setDraft((prev) => ({
    ...prev,
    categoryFilters: { ...prev.categoryFilters, [category]: value },
  }));
}

function save() {
  setCurrent({ ...draft() });
  setSaveStatus('saved');
  setTimeout(() => setSaveStatus('idle'), 3000);
}

function discard() {
  setDraft({ ...current() });
  setSaveStatus('idle');
}

function previewImpact(totalFindings: number, falsePositives: number): string {
  const d = draft();
  // Simplified preview: estimate how many findings would pass the draft filters
  const severityFilter =
    d.minSeverity === 'all'
      ? 1
      : d.minSeverity === 'medium'
        ? 0.8
        : d.minSeverity === 'high'
          ? 0.6
          : 0.3;
  const estimated = Math.round((totalFindings - falsePositives) * severityFilter);
  return `With current settings, last cycle would have created ${estimated} issues (vs. ${totalFindings - falsePositives} actual)`;
}

export function getSettingsStore() {
  return {
    current,
    draft,
    saveStatus,
    isDirty,
    updateDraft,
    updateCategoryFilter,
    save,
    discard,
    previewImpact,
  };
}
