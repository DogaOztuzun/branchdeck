import { createMemo, For, Show } from 'solid-js';
import type { RunLogEntry } from '../../lib/stores/task';

type RunTimelineProps = {
  entries: RunLogEntry[];
  visible: boolean;
};

function typeLabel(type: RunLogEntry['type']): string {
  switch (type) {
    case 'run_step':
      return 'Step';
    case 'assistant_text':
      return 'Text';
    case 'tool_call':
      return 'Tool';
    case 'status_change':
      return 'Status';
  }
}

function typeColor(type: RunLogEntry['type']): string {
  switch (type) {
    case 'run_step':
      return 'text-primary';
    case 'assistant_text':
      return 'text-text-muted';
    case 'tool_call':
      return 'text-info';
    case 'status_change':
      return 'text-warning';
  }
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  return d.toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

export function RunTimeline(props: RunTimelineProps) {
  const reversed = createMemo(() => [...props.entries].reverse());

  return (
    <Show when={props.visible}>
      <div class="border-t border-border bg-surface">
        <div class="flex items-center justify-between px-3 py-1 border-b border-border">
          <span class="text-[10px] text-text-muted uppercase tracking-wider">Run Timeline</span>
          <span class="text-[10px] text-text-muted">{props.entries.length} events</span>
        </div>
        <div class="overflow-y-auto max-h-32">
          <Show
            when={reversed().length > 0}
            fallback={<div class="px-3 py-2 text-xs text-text-muted">No run activity yet.</div>}
          >
            <For each={reversed()}>
              {(entry) => (
                <div class="flex items-baseline gap-2 px-3 py-0.5 text-[11px] hover:bg-bg/30">
                  <span class="text-text-muted shrink-0 w-16">{formatTime(entry.ts)}</span>
                  <span class={`shrink-0 w-10 ${typeColor(entry.type)}`}>
                    {typeLabel(entry.type)}
                  </span>
                  <span class="text-text truncate">{entry.detail}</span>
                </div>
              )}
            </For>
          </Show>
        </div>
      </div>
    </Show>
  );
}
