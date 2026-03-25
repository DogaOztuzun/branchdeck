import { onMount, Show } from 'solid-js';
import { getInboxStore } from '../../lib/stores/inbox';
import { EmptyState } from '../ui/EmptyState';
import { InboxRowList } from './InboxRowList';
import { InboxSummaryBar } from './InboxSummaryBar';

export function InboxView() {
  const inbox = getInboxStore();

  onMount(() => {
    inbox.registerInboxShortcuts();
    // Load mock data for dev — remove when connected to real backend
    if (inbox.items().length === 0) {
      inbox.loadMockData();
    }
  });

  const hasItems = () => inbox.items().length > 0;
  const allCompleted = () => inbox.items().every((i) => i.status === 'completed');

  return (
    <div class="flex-1 overflow-y-auto">
      <div class="mx-auto max-w-[900px] pt-4">
        <Show when={hasItems()}>
          <InboxSummaryBar />
        </Show>

        <Show when={hasItems() && !allCompleted()}>
          <div class="mt-2">
            <InboxRowList />
          </div>
        </Show>

        <Show when={!hasItems() || allCompleted()}>
          <EmptyState
            message="All clear"
            detail={
              hasItems()
                ? `${inbox.items().length} items processed`
                : 'No items yet — workflows will populate this inbox'
            }
          />
        </Show>
      </div>
    </div>
  );
}
