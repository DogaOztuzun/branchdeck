import { For, onMount, Show } from 'solid-js';
import { getActivityStore } from '../../lib/stores/activity';
import { EmptyState } from '../ui/EmptyState';
import { ActivityFilterBar } from './ActivityFilterBar';
import { EventRow } from './EventRow';

export function ActivityTimeline() {
  const activity = getActivityStore();

  onMount(() => {
    if (activity.events().length === 0) {
      activity.loadMockData();
    }
    // Register timeline-specific shortcuts as inbox sub-context
    // (Activity is accessed from Inbox view, shares its context)
  });

  const filtered = () => activity.filteredEvents();

  const handleClick = (eventId: string, idx: number) => {
    if (activity.selectedIndex() === idx && activity.expandedId() === eventId) {
      activity.setExpandedId(null);
    } else {
      activity.setSelectedIndex(idx);
      activity.setExpandedId(eventId);
    }
  };

  return (
    <div class="flex-1 overflow-y-auto">
      <div class="mx-auto max-w-[900px]">
        <ActivityFilterBar />
        <Show
          when={filtered().length > 0}
          fallback={
            <EmptyState
              message="No events"
              detail="Activity will appear as workflows run"
              showIcon={false}
            />
          }
        >
          <For each={filtered()}>
            {(event, i) => (
              <EventRow
                event={event}
                selected={activity.selectedIndex() === i()}
                expanded={activity.expandedId() === event.id}
                onClick={() => handleClick(event.id, i())}
              />
            )}
          </For>
        </Show>
      </div>
    </div>
  );
}
