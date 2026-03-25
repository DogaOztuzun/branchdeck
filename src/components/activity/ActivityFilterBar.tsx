import { For } from 'solid-js';
import { getActivityStore } from '../../lib/stores/activity';
import type { EventType, TimeRange } from '../../types/activity';
import { ActionButton } from '../ui/ActionButton';

const typeLabels: Record<EventType, string> = {
  sat: 'SAT',
  orchestrator: 'Orchestrator',
  agent: 'Agent',
  pr: 'PR',
  ci: 'CI',
};

const timeRanges: { label: string; value: TimeRange }[] = [
  { label: 'Last 8h', value: '8h' },
  { label: 'Last 24h', value: '24h' },
  { label: 'All', value: 'all' },
];

export function ActivityFilterBar() {
  const activity = getActivityStore();

  return (
    <div class="flex items-center gap-1.5 px-3 py-2 sticky top-0 bg-bg-main z-10 border-b border-border-subtle">
      <For each={activity.allTypes}>
        {(type) => (
          <ActionButton
            label={typeLabels[type]}
            variant={activity.filter().activeTypes.has(type) ? 'primary' : 'secondary'}
            size="compact"
            onClick={() => activity.toggleType(type)}
          />
        )}
      </For>
      <span class="mx-2 text-text-dim">|</span>
      <For each={timeRanges}>
        {(range) => (
          <ActionButton
            label={range.label}
            variant={activity.filter().timeRange === range.value ? 'primary' : 'secondary'}
            size="compact"
            onClick={() => activity.setTimeRange(range.value)}
          />
        )}
      </For>
    </div>
  );
}
