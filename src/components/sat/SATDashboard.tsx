import { onMount, Show } from 'solid-js';
import { getSATStore } from '../../lib/stores/sat';
import { EmptyState } from '../ui/EmptyState';
import { SatisfactionChart } from '../ui/SatisfactionChart';
import { FindingsList } from './FindingsList';
import { SATSummaryBar } from './SATSummaryBar';

export function SATDashboard() {
  const sat = getSATStore();

  onMount(() => {
    sat.registerSATShortcuts();
    if (sat.cycles().length === 0) {
      sat.loadMockData();
    }
  });

  const hasData = () => sat.cycles().length > 0;

  return (
    <div class="flex-1 overflow-y-auto">
      <div class="mx-auto max-w-[960px] pt-4">
        <Show
          when={hasData()}
          fallback={
            <EmptyState
              message="No findings"
              detail="Run your first SAT cycle to see results here"
            />
          }
        >
          <SATSummaryBar />
          <div class="mt-4 relative">
            <SatisfactionChart data={sat.chartData()} personas={sat.personaLines()} />
          </div>
          <div class="mt-4">
            <FindingsList />
          </div>
        </Show>
      </div>
    </div>
  );
}
