import { createMemo, For, Show } from 'solid-js';
import type { AgentLogEntry } from '../../lib/stores/agent';
import { shortPath } from '../../lib/utils';

type AgentActivityProps = {
  entries: AgentLogEntry[];
  visible: boolean;
};

function kindLabel(kind: string): string {
  switch (kind) {
    case 'sessionStart':
      return 'Session';
    case 'toolStart':
      return 'Tool';
    case 'toolEnd':
      return 'Done';
    case 'subagentStart':
      return 'Spawn';
    case 'subagentStop':
      return 'End';
    case 'sessionStop':
      return 'Stop';
    case 'notification':
      return 'Note';
    default:
      return kind;
  }
}

function kindColor(kind: string): string {
  switch (kind) {
    case 'sessionStart':
      return 'text-success';
    case 'toolStart':
      return 'text-primary';
    case 'toolEnd':
      return 'text-text-muted';
    case 'subagentStart':
      return 'text-info';
    case 'subagentStop':
      return 'text-info';
    case 'sessionStop':
      return 'text-error';
    case 'notification':
      return 'text-warning';
    default:
      return 'text-text-muted';
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

function entryDetail(entry: AgentLogEntry): string {
  if (entry.toolName) {
    const file = entry.filePath ? ` ${shortPath(entry.filePath, 2)}` : '';
    return `${entry.toolName}${file}`;
  }
  if (entry.message) return entry.message;
  if (entry.agentId) return entry.agentId;
  return '';
}

export function AgentActivity(props: AgentActivityProps) {
  const reversed = createMemo(() => [...props.entries].reverse());

  return (
    <Show when={props.visible}>
      <div class="border-t border-border bg-surface">
        <div class="flex items-center justify-between px-3 py-1 border-b border-border">
          <span class="text-[10px] text-text-muted uppercase tracking-wider">Agent Activity</span>
          <span class="text-[10px] text-text-muted">{props.entries.length} events</span>
        </div>
        <div class="overflow-y-auto max-h-32">
          <Show
            when={reversed().length > 0}
            fallback={
              <div class="px-3 py-2 text-xs text-text-muted">
                No agent activity yet. Open a Claude tab to see events.
              </div>
            }
          >
            <For each={reversed()}>
              {(entry) => (
                <div class="flex items-baseline gap-2 px-3 py-0.5 text-[11px] hover:bg-bg/30">
                  <span class="text-text-muted shrink-0 w-16">{formatTime(entry.ts)}</span>
                  <span class={`shrink-0 w-10 ${kindColor(entry.kind)}`}>
                    {kindLabel(entry.kind)}
                  </span>
                  <span class="text-text truncate">{entryDetail(entry)}</span>
                </div>
              )}
            </For>
          </Show>
        </div>
      </div>
    </Show>
  );
}
