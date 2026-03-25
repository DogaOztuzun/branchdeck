import { For } from 'solid-js';
import { getSATStore } from '../../lib/stores/sat';
import type { CategoryFilter } from '../../types/sat';
import { ActionButton } from '../ui/ActionButton';
import { SectionHeader } from '../ui/SectionHeader';
import { FindingRow } from './FindingRow';

const categories: { label: string; value: CategoryFilter }[] = [
  { label: 'All', value: 'all' },
  { label: 'App', value: 'app' },
  { label: 'Runner', value: 'runner' },
  { label: 'Scenario', value: 'scenario' },
];

export function FindingsList() {
  const sat = getSATStore();

  const filtered = () => sat.filteredFindings();

  return (
    <div>
      {/* Filter bar */}
      <div class="flex items-center gap-1.5 px-3 py-2 sticky top-0 bg-bg-main z-10">
        <For each={categories}>
          {(cat) => (
            <ActionButton
              label={cat.label}
              variant={sat.categoryFilter() === cat.value ? 'primary' : 'secondary'}
              size="compact"
              onClick={() => sat.setCategoryFilter(cat.value)}
            />
          )}
        </For>
      </div>

      <SectionHeader label="Findings" count={filtered().length} />

      <For each={filtered()}>
        {(finding, i) => {
          const handleClick = () => {
            if (sat.selectedFindingIndex() === i() && sat.expandedFindingId() === finding.id) {
              sat.setExpandedFindingId(null);
            } else {
              sat.setSelectedFindingIndex(i());
              sat.setExpandedFindingId(finding.id);
            }
          };

          return (
            <FindingRow
              finding={finding}
              selected={sat.selectedFindingIndex() === i()}
              expanded={sat.expandedFindingId() === finding.id}
              onClick={handleClick}
            />
          );
        }}
      </For>
    </div>
  );
}
