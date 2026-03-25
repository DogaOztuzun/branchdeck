import { For, Show } from 'solid-js';
import type { ActivityEvent } from '../../types/activity';

type EventDetailProps = {
  event: ActivityEvent;
};

export function EventDetail(props: EventDetailProps) {
  const detail = () => props.event.detail;

  return (
    <div class="bg-surface-raised px-3 py-2 border-b border-border-subtle">
      <Show when={detail()} keyed>
        {(d) => (
          <div class="flex flex-wrap gap-x-4 gap-y-1">
            <For each={Object.entries(d)}>
              {([key, value]) => (
                <span class="text-[11px]">
                  <span class="text-text-dim">{key}:</span>{' '}
                  <span class="text-text-main">{value}</span>
                </span>
              )}
            </For>
          </div>
        )}
      </Show>
      <Show when={!detail()}>
        <span class="text-[11px] text-text-dim">No additional details</span>
      </Show>
    </div>
  );
}
